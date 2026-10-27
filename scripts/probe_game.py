"""Run a local SWF through the real libretro core and capture framebuffer PNGs.

This is a diagnostic frontend, not an assertion of game compatibility.
"""
import argparse
import ctypes as C
import hashlib
import json
from pathlib import Path
import struct
import zlib

from smoke_test import ENV, VIDEO, BATCH, POLL, INPUT, GameInfo, AvInfo


class Variable(C.Structure):
    _fields_ = [("key", C.c_char_p), ("value", C.c_void_p)]


def png(path, width, height, pixels):
    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))
    rows = bytearray()
    for y in range(height):
        rows.append(0)
        for x in range(width):
            i = (y * width + x) * 4
            rows.extend((pixels[i + 2], pixels[i + 1], pixels[i]))
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
                     + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b""))


def run(args):
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    options = dict(item.split("=", 1) for item in args.option)
    values = {key: C.create_string_buffer(value.encode()) for key, value in options.items()}
    saves = C.create_string_buffer(str(output).encode("utf-8"))
    core = C.CDLL(str(Path(args.library).resolve()))
    frame = 0
    changes = {int(item.split(":", 1)[0]): item.split(":", 1)[1].split("=", 1) for item in args.set_at}
    clicks = [tuple(map(int, item.split(","))) for item in args.click]
    moves = [tuple(map(int, item.split(","))) for item in args.move]
    keys = [tuple(map(int, item.split(","))) for item in args.key]
    capture = set(map(int, args.capture.split(","))) | {args.frames}
    current_mouse = [0, 0]
    delta = [0, 0]
    updated = False
    cached = [None, 0, b""]
    results = {"game": Path(args.game).name, "frames": [], "geometry_updates": [], "messages": [], "options": options}

    @ENV
    def environment(command, data):
        nonlocal updated
        if command == 10:
            return C.cast(data, C.POINTER(C.c_uint))[0] == 1
        if command == 31:
            C.cast(data, C.POINTER(C.c_void_p))[0] = C.addressof(saves)
            return True
        if command == 6:
            message = C.cast(data, C.POINTER(C.c_char_p))[0].decode("utf-8", "replace")
            results["messages"].append(message)
            print(message, flush=True)
            return True
        if command == 15:
            variable = C.cast(data, C.POINTER(Variable)).contents
            value = values.get(variable.key.decode())
            variable.value = C.addressof(value) if value else None
            return value is not None
        if command == 17:
            C.cast(data, C.POINTER(C.c_bool))[0] = updated
            updated = False
            return True
        if command in (32, 37):
            from smoke_test import Geometry
            geometry = C.cast(data, C.POINTER(Geometry)).contents
            results["geometry_updates"].append([frame, geometry.width, geometry.height, geometry.aspect])
            return True
        return command in (11, 12, 16)

    @VIDEO
    def video(data, width, height, pitch):
        if data and args.check_cache:
            cached[:] = [data, height * pitch, hashlib.sha256(C.string_at(data, height * pitch)).digest()]
        if data and frame in capture:
            pixels = C.string_at(data, height * pitch)
            if pitch != width * 4:
                pixels = b"".join(pixels[y*pitch:y*pitch+width*4] for y in range(height))
            png(output / f"frame-{frame:05d}.png", width, height, pixels)
            colors = len(set(struct.iter_unpack("<I", pixels)))
            results["frames"].append({"frame": frame, "width": width, "height": height, "colors": colors,
                                       "sha256": hashlib.sha256(pixels).hexdigest()})
            print(results["frames"][-1], flush=True)

    @BATCH
    def audio(_data, count):
        return count

    @POLL
    def poll():
        pass

    @INPUT
    def state(_port, device, _index, key):
        if device == 2:
            if key in (0, 1):
                return delta[key]
            if key == 2:
                return int(any(start <= frame < start + 2 for start, _, _ in clicks))
        if device == 3:
            return int(any(start <= frame < start + duration and code == key for start, duration, code in keys))
        return 0

    callbacks = [("environment", ENV, environment), ("video_refresh", VIDEO, video),
                 ("audio_sample_batch", BATCH, audio), ("input_poll", POLL, poll), ("input_state", INPUT, state)]
    for name, kind, callback in callbacks:
        function = getattr(core, "retro_set_" + name)
        function.argtypes = [kind]
        function.restype = None
        function(callback)
    core.retro_load_game.argtypes, core.retro_load_game.restype = [C.POINTER(GameInfo)], C.c_bool
    core.retro_get_system_av_info.argtypes = [C.POINTER(AvInfo)]
    core.retro_get_system_av_info.restype = None
    for name in ("retro_init", "retro_run", "retro_deinit"):
        getattr(core, name).restype = None
        getattr(core, name).argtypes = []
    core.retro_init()
    try:
        game = GameInfo(str(Path(args.game).resolve()).encode("utf-8"), None, 0, None)
        assert core.retro_load_game(C.byref(game)), "Could not load game"
        av = AvInfo()
        core.retro_get_system_av_info(C.byref(av))
        current_mouse[:] = [av.geometry.width // 2, av.geometry.height // 2]
        results["initial_geometry"] = [av.geometry.width, av.geometry.height, av.geometry.aspect]
        print("Initial geometry:", results["initial_geometry"], flush=True)
        for frame in range(1, args.frames + 1):
            delta[:] = [0, 0]
            for start, x, y in moves + clicks:
                if frame == start:
                    delta[:] = [x - current_mouse[0], y - current_mouse[1]]
                    current_mouse[:] = [x, y]
            if frame in changes:
                key, value = changes[frame]
                values[key] = C.create_string_buffer(value.encode())
                updated = True
            core.retro_run()
            if args.check_cache and cached[0]:
                # RetroArch's paused menu reuses the pointer AFTER retro_run.
                # Churn similarly sized allocations to expose freed buffers.
                churn = [C.create_string_buffer(b"\xa5" * cached[1]) for _ in range(3)]
                actual = hashlib.sha256(C.string_at(cached[0], cached[1])).digest()
                assert actual == cached[2], f"Cached frame invalid after retro_run at frame {frame}"
                del churn
        results["cached_frame_check"] = "passed" if args.check_cache else "not run"
    finally:
        core.retro_deinit()
        (output / "report.json").write_text(json.dumps(results, indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("library")
    parser.add_argument("game")
    parser.add_argument("--output", required=True)
    parser.add_argument("--frames", type=int, default=600)
    parser.add_argument("--capture", default="1,60,180,600")
    parser.add_argument("--option", action="append", default=[])
    parser.add_argument("--set-at", action="append", default=[], metavar="FRAME:KEY=VALUE")
    parser.add_argument("--click", action="append", default=[], metavar="FRAME,X,Y")
    parser.add_argument("--move", action="append", default=[], metavar="FRAME,X,Y", help="Move without clicking")
    parser.add_argument("--key", action="append", default=[], metavar="FRAME,DURATION,KEYCODE")
    parser.add_argument("--check-cache", action="store_true", help="Read cached video after retro_run, like a paused menu")
    run(parser.parse_args())
