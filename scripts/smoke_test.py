"""Exercise the real shared library via the C ABI, without RetroArch.

python scripts/smoke_test.py path/to/ruffle_libretro.dll [--abi-only]
Full mode requires a working D3D12/Vulkan/Metal adapter (software is OK).
"""
import argparse
import ctypes as C
from pathlib import Path
import tempfile

from make_test_swf import make_swf


class SystemInfo(C.Structure):
    _fields_ = [("name", C.c_char_p), ("version", C.c_char_p), ("extensions", C.c_char_p),
                ("fullpath", C.c_bool), ("block_extract", C.c_bool)]


class GameInfo(C.Structure):
    _fields_ = [("path", C.c_char_p), ("data", C.c_void_p), ("size", C.c_size_t), ("meta", C.c_char_p)]


class Geometry(C.Structure):
    _fields_ = [("width", C.c_uint), ("height", C.c_uint), ("max_width", C.c_uint),
                ("max_height", C.c_uint), ("aspect", C.c_float)]


class Timing(C.Structure):
    _fields_ = [("fps", C.c_double), ("sample_rate", C.c_double)]


class AvInfo(C.Structure):
    _fields_ = [("geometry", Geometry), ("timing", Timing)]


ENV = C.CFUNCTYPE(C.c_bool, C.c_uint, C.c_void_p)
VIDEO = C.CFUNCTYPE(None, C.c_void_p, C.c_uint, C.c_uint, C.c_size_t)
AUDIO = C.CFUNCTYPE(None, C.c_int16, C.c_int16)
BATCH = C.CFUNCTYPE(C.c_size_t, C.POINTER(C.c_int16), C.c_size_t)
POLL = C.CFUNCTYPE(None)
INPUT = C.CFUNCTYPE(C.c_int16, C.c_uint, C.c_uint, C.c_uint, C.c_uint)


def run(library, abi_only):
    core = C.CDLL(str(Path(library).resolve()))
    # Set signatures explicitly: this catches size/pointer mistakes on Windows x64.
    signatures = {
        "retro_api_version": (C.c_uint, []),
        "retro_get_system_info": (None, [C.POINTER(SystemInfo)]),
        "retro_get_system_av_info": (None, [C.POINTER(AvInfo)]),
        "retro_load_game": (C.c_bool, [C.POINTER(GameInfo)]),
        "retro_serialize_size": (C.c_size_t, []),
        "retro_serialize": (C.c_bool, [C.c_void_p, C.c_size_t]),
        "retro_unserialize": (C.c_bool, [C.c_void_p, C.c_size_t]),
        "retro_get_memory_data": (C.c_void_p, [C.c_uint]),
        "retro_get_memory_size": (C.c_size_t, [C.c_uint]),
        "retro_get_region": (C.c_uint, []),
        "retro_set_controller_port_device": (None, [C.c_uint, C.c_uint]),
        "retro_cheat_set": (None, [C.c_uint, C.c_bool, C.c_char_p]),
        "retro_load_game_special": (C.c_bool, [C.c_uint, C.POINTER(GameInfo), C.c_size_t]),
    }
    for name in ("retro_init", "retro_deinit", "retro_run", "retro_reset", "retro_unload_game", "retro_cheat_reset"):
        signatures[name] = (None, [])
    for name, cb in (("environment", ENV), ("video_refresh", VIDEO), ("audio_sample", AUDIO),
                     ("audio_sample_batch", BATCH), ("input_poll", POLL), ("input_state", INPUT)):
        signatures["retro_set_" + name] = (None, [cb])
    for name, (result, arguments) in signatures.items():
        fn = getattr(core, name)
        fn.restype, fn.argtypes = result, arguments

    frames, sample_frames, nonzero, colors, errors = 0, 0, False, set(), []
    accept_pixels = True
    with tempfile.TemporaryDirectory(prefix="ruffle-smoke-") as temporary:
        root = Path(temporary)
        save_path = C.create_string_buffer(str(root).encode("utf-8"))

        @ENV
        def environment(command, data):
            if command == 10:
                return accept_pixels and C.cast(data, C.POINTER(C.c_uint))[0] == 1
            if command == 31:
                C.cast(data, C.POINTER(C.c_void_p))[0] = C.addressof(save_path)
                return True
            if command == 6:
                print("core:", C.cast(data, C.POINTER(C.c_char_p))[0].decode("utf-8"))
                return True
            return command in (11, 12)

        @VIDEO
        def video(data, width, height, pitch):
            nonlocal frames
            if not data or (width, height, pitch) != (320, 240, 1280):
                errors.append("Invalid video frame dimensions/pitch")
                return
            frames += 1
            colors.add(C.cast(data, C.POINTER(C.c_uint32))[0] & 0xFFFFFF)

        @BATCH
        def batch(data, count):
            nonlocal sample_frames, nonzero
            # Deliberately consume partial batches to check the core's handling.
            consumed = min(count, 137)
            sample_frames += consumed
            nonzero |= any(data[i] != 0 for i in range(consumed * 2))
            return consumed

        @AUDIO
        def audio(left, right):
            nonlocal sample_frames, nonzero
            sample_frames += 1
            nonzero |= left != 0 or right != 0

        @POLL
        def poll():
            pass

        @INPUT
        def input_state(port, device, index, key):
            return 0

        core.retro_set_environment(environment)
        core.retro_set_video_refresh(video)
        core.retro_set_audio_sample(audio)
        core.retro_set_audio_sample_batch(batch)
        core.retro_set_input_poll(poll)
        core.retro_set_input_state(input_state)
        core.retro_init()
        try:
            assert core.retro_api_version() == 1
            info = SystemInfo()
            core.retro_get_system_info(C.byref(info))
            assert info.extensions == b"swf" and info.fullpath
            assert core.retro_serialize_size() == 0
            assert not core.retro_serialize(None, 0)
            assert not core.retro_unserialize(None, 0)
            assert not core.retro_get_memory_data(0)
            assert core.retro_get_memory_size(0) == 0
            core.retro_run()  # Safe with no content.
            assert not core.retro_load_game(None)
            invalid = root / "invalid.swf"
            invalid.write_bytes(b"not a SWF")
            assert not core.retro_load_game(C.byref(GameInfo(str(invalid).encode(), None, 0, None)))
            swf_path = root / "teste a\u00e7\u00e3o.swf"
            swf_path.write_bytes(make_swf())
            game = GameInfo(str(swf_path).encode("utf-8"), None, 0, None)
            accept_pixels = False
            assert not core.retro_load_game(C.byref(game))
            accept_pixels = True
            if abi_only:
                print("PASS: exported ABI, metadata, unsupported features and load errors")
                return
            assert core.retro_load_game(C.byref(game)), "SWF/GPU initialization failed"
            av = AvInfo()
            core.retro_get_system_av_info(C.byref(av))
            assert (av.geometry.width, av.geometry.height) == (320, 240)
            assert (av.timing.fps, av.timing.sample_rate) == (60.0, 48000.0)
            for _ in range(60):
                core.retro_run()
            assert not errors, errors
            assert frames == 60, frames
            assert sample_frames == 48000, sample_frames
            assert nonzero, "Expected the generated PCM tone"
            assert {0xCC2233, 0x22CC33, 0x2233CC}.issubset(colors), colors
            core.retro_reset()
            core.retro_run()
            assert frames == 61
            core.retro_unload_game()
            core.retro_run()
            assert frames == 61
            assert core.retro_load_game(C.byref(game))
            core.retro_set_audio_sample_batch(BATCH())  # Exercise sample callback fallback.
            core.retro_run()
            assert frames == 62 and sample_frames == 49600
            assert not errors, errors
            print("PASS: SWF, RGB frames, PCM audio, timing, reset, unload and reload")
        finally:
            core.retro_deinit()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("library")
    parser.add_argument("--abi-only", action="store_true")
    args = parser.parse_args()
    run(args.library, args.abi_only)

