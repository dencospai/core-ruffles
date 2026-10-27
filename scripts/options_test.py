"""Regression: live options, geometry callback order, scaling and paused video lifetime."""
import ctypes as C
import hashlib
from pathlib import Path
import sys
import tempfile

from smoke_test import ENV, VIDEO, BATCH, POLL, INPUT, GameInfo, Geometry, AvInfo
from probe_game import Variable
from make_test_swf import make_swf


def run(library):
    core = C.CDLL(str(Path(library).resolve()))
    values, registered, errors = {}, {}, []
    changed = False
    expected = (320, 240)
    cached = None
    pixels = b""
    geometry_events = []
    saves = None

    def check(condition, message):
        if not condition:
            errors.append(message)

    @ENV
    def environment(command, data):
        nonlocal changed
        if command == 10:
            return C.cast(data, C.POINTER(C.c_uint))[0] == 1
        if command == 16:
            variables = C.cast(data, C.POINTER(Variable))
            index = 0
            while variables[index].key:
                registered[variables[index].key.decode()] = C.string_at(variables[index].value).decode()
                index += 1
            return True
        if command == 15:
            variable = C.cast(data, C.POINTER(Variable)).contents
            value = values.get(variable.key.decode())
            variable.value = C.addressof(value) if value is not None else None
            return value is not None
        if command == 17:
            C.cast(data, C.POINTER(C.c_bool))[0] = changed
            changed = False
            return True
        if command == 31:
            C.cast(data, C.POINTER(C.c_void_p))[0] = C.addressof(saves)
            return True
        if command == 37:
            geometry = C.cast(data, C.POINTER(Geometry)).contents
            geometry_events.append((geometry.width, geometry.height))
            check(geometry_events[-1] == expected, "Geometry changed to unexpected size")
            av = AvInfo()
            core.retro_get_system_av_info(C.byref(av))  # Reentrant: no STATE borrow allowed.
            check((av.geometry.width, av.geometry.height) == expected, "Stale AV info")
            check(abs(geometry.aspect - expected[0]/expected[1]) < 1e-5, "Wrong aspect")
            return True
        if command == 6:
            errors.append(C.cast(data, C.POINTER(C.c_char_p))[0].decode())
            return True
        return command in (11, 12)

    @VIDEO
    def video(data, width, height, pitch):
        nonlocal cached, pixels
        if cached:
            # The old frame must still exist while its replacement is submitted.
            check(hashlib.sha256(C.string_at(cached[0], cached[1])).digest() == cached[2], "Previous frame released too early")
        check((width, height) == expected, "Wrong video dimensions")
        check(pitch == width * 4, "Wrong pitch")
        if expected != (320, 240):
            check(bool(geometry_events) and geometry_events[-1] == expected, "Video preceded geometry notification")
        pixels = C.string_at(data, height * pitch)
        cached = (data, len(pixels), hashlib.sha256(pixels).digest())

    @BATCH
    def audio(_data, count):
        return count

    @POLL
    def poll():
        pass

    @INPUT
    def state(_port, _device, _index, _id):
        return 0

    for name, kind, callback in [("environment", ENV, environment), ("video_refresh", VIDEO, video),
                                 ("audio_sample_batch", BATCH, audio), ("input_poll", POLL, poll), ("input_state", INPUT, state)]:
        fn = getattr(core, "retro_set_" + name)
        fn.argtypes, fn.restype = [kind], None
        fn(callback)
    core.retro_load_game.argtypes, core.retro_load_game.restype = [C.POINTER(GameInfo)], C.c_bool
    core.retro_get_system_av_info.argtypes = [C.POINTER(AvInfo)]
    core.retro_get_system_av_info.restype = None
    for name in ("retro_init", "retro_run", "retro_reset", "retro_unload_game", "retro_deinit"):
        getattr(core, name).argtypes, getattr(core, name).restype = [], None

    def tick():
        core.retro_run()
        assert cached
        churn = [C.create_string_buffer(b"\xa5" * cached[1]) for _ in range(3)]
        check(hashlib.sha256(C.string_at(cached[0], cached[1])).digest() == cached[2], "Paused framebuffer changed")
        assert churn
        assert not errors, errors

    def set_options(**options):
        nonlocal changed
        values.update({"ruffle_" + k: C.create_string_buffer(v.encode()) for k, v in options.items()})
        changed = True

    with tempfile.TemporaryDirectory(prefix="ruffle-options-") as temporary:
        path = Path(temporary) / "smoke.swf"
        path.write_bytes(make_swf())
        saves = C.create_string_buffer(temporary.encode())
        core.retro_init()
        try:
            assert len(registered) == 6, registered
            assert core.retro_load_game(C.byref(GameInfo(str(path).encode(), None, 0, None)))
            tick()
            expected = (1280, 720)
            set_options(resolution="1280x720", scale_mode="Zoom to Fit", force_scale="On")
            for _ in range(5):
                tick()
            assert pixels[:4] == b"\0" * 4, "ShowAll should letterbox 4:3 in 16:9"
            set_options(scale_mode="Stretch to Fit")
            tick()
            assert pixels[:4] != b"\0" * 4, "Stretch should fill the canvas"
            for mode in ("Unscaled (100%)", "Crop to Fit", "Zoom to Fit"):
                set_options(scale_mode=mode)
                tick()
            for quality in ("Low", "Medium", "High", "Best", "High (8x8)", "High (8x8) Linear", "High (16x16)", "High (16x16) Linear"):
                set_options(quality=quality)
                tick()
            set_options(letterbox="Off", force_scale="Off")
            tick()
            for resolution in ("640x480", "800x600", "1024x768", "1920x1080"):
                expected = tuple(map(int, resolution.split("x")))
                set_options(resolution=resolution)
                tick()
            expected = (320, 240)
            set_options(resolution="Native", quality="High")
            tick()
            core.retro_reset()
            tick()
            core.retro_unload_game()
            check(hashlib.sha256(C.string_at(cached[0], cached[1])).digest() == cached[2], "Unload invalidated cached frame")
            assert not errors, errors
        finally:
            core.retro_deinit()
    print("PASS: 6 options; all resolutions, scale/quality modes; live geometry; retained frame across pause/reset/unload")


if __name__ == "__main__":
    run(sys.argv[1])
