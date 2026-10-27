"""Check live gamepad mouse options and real Flash clicks through libretro."""
import ctypes as C
from pathlib import Path
import sys
import tempfile

from smoke_test import ENV, VIDEO, BATCH, POLL, INPUT, GameInfo
from probe_game import Variable
from make_cursor_swf import make_swf


def run(library):
    core = C.CDLL(str(Path(library).resolve()))
    values, registered, readings, errors = {}, {}, {}, []
    changed = False
    pixels = b""
    saves = None

    @ENV
    def environment(command, data):
        nonlocal changed
        if command == 10:
            return C.cast(data, C.POINTER(C.c_uint))[0] == 1
        if command == 16:
            variables = C.cast(data, C.POINTER(Variable))
            i = 0
            while variables[i].key:
                registered[variables[i].key.decode()] = C.string_at(variables[i].value).decode()
                i += 1
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
        if command == 6:
            errors.append(C.cast(data, C.POINTER(C.c_char_p))[0].decode())
            return True
        return command in (11, 12)

    @VIDEO
    def video(data, width, height, pitch):
        nonlocal pixels
        if (width, height, pitch) != (320, 240, 1280):
            errors.append("Unexpected geometry")
        pixels = C.string_at(data, height * pitch)

    @BATCH
    def audio(_data, count):
        return count

    @POLL
    def poll():
        pass

    @INPUT
    def state(port, device, index, button):
        return readings.get((device, index, button), 0) if port == 0 else 0

    for name, kind, callback in [("environment", ENV, environment), ("video_refresh", VIDEO, video),
                                ("audio_sample_batch", BATCH, audio), ("input_poll", POLL, poll),
                                ("input_state", INPUT, state)]:
        fn = getattr(core, "retro_set_" + name)
        fn.argtypes, fn.restype = [kind], None
        fn(callback)
    core.retro_load_game.argtypes, core.retro_load_game.restype = [C.POINTER(GameInfo)], C.c_bool
    for name in ("retro_init", "retro_run", "retro_reset", "retro_deinit"):
        getattr(core, name).argtypes, getattr(core, name).restype = [], None

    def tick(inputs=None, count=1):
        readings.clear()
        readings.update(inputs or {})
        for _ in range(count):
            core.retro_run()
        assert not errors, errors
        assert pixels

    def options(**settings):
        nonlocal changed
        values.update({"ruffle_" + k: C.create_string_buffer(v.encode()) for k, v in settings.items()})
        changed = True

    def bounds():
        points = [(i % 320, i // 320) for i in range(320 * 240)
                  if pixels[4*i:4*i+4] != background[4*i:4*i+4]]
        return (min(x for x, y in points), min(y for x, y in points),
                max(x for x, y in points), max(y for x, y in points)) if points else None

    with tempfile.TemporaryDirectory(prefix="ruffle-gamepad-") as temporary:
        fixture = Path(temporary) / "cursor.swf"
        fixture.write_bytes(make_swf())
        saves = C.create_string_buffer(temporary.encode())
        core.retro_init()
        try:
            assert registered["ruffle_gamepad_mouse"].endswith("; Off|On")
            assert registered["ruffle_mouse_stick"].endswith("; Left|Right")
            assert registered["ruffle_mouse_click"].endswith("; R3|L3")
            assert core.retro_load_game(C.byref(GameInfo(str(fixture).encode(), None, 0, None)))
            tick({(3, 0, 104): 1}, 12)  # H: hidden baseline.
            background = pixels
            tick({(2, 0, 0): -130, (2, 0, 1): -90})  # Physical mouse to 30,30.
            tick({(3, 0, 115): 1}, 6)  # S: show.
            assert bounds() == (25, 25, 35, 35)
            tick({(5, 0, 0): -32768, (5, 1, 1): -32768, (1, 0, 15): 1})
            assert bounds() == (25, 25, 35, 35), "Disabled gamepad moved cursor"
            options(gamepad_mouse="On")
            tick({(5, 0, 0): -32768})
            assert bounds() == (17, 25, 27, 35), "Default left stick or live enable failed"
            tick({(5, 1, 0): -32768})
            assert bounds() == (17, 25, 27, 35), "Unselected stick moved cursor"
            options(mouse_stick="Right")
            tick({(5, 1, 1): -32768})
            assert bounds() == (17, 17, 27, 27), "Live right stick selection failed"
            options(gamepad_mouse="Off")
            tick({(5, 1, 0): -32768, (2, 0, 0): 8, (2, 0, 1): 8})
            assert bounds() == (25, 25, 35, 35), "Disable blocked physical mouse or kept stick active"
            tick({(2, 0, 0): 70, (2, 0, 1): 70})  # Hover real Flash button.
            assert bounds()[3] > 105, "Hover did not show hand"
            for stick in ("Left", "Right"):
                for click, selected, other in (("R3", 15, 14), ("L3", 14, 15)):
                    options(gamepad_mouse="On", mouse_stick=stick, mouse_click=click)
                    tick({(3, 0, 115): 1}, 6)
                    tick({(1, 0, other): 1}, 2)
                    tick(count=6)
                    assert bounds(), "Unselected stick button clicked"
                    tick({(1, 0, selected): 1}, 2)
                    tick(count=6)
                    assert bounds() is None, f"{stick}/{click} failed to click Flash button"
            options(gamepad_mouse="Off")
            tick({(3, 0, 115): 1}, 6)
            tick({(1, 0, 14): 1, (1, 0, 15): 1}, 2)
            tick(count=6)
            assert bounds(), "Disabled gamepad still clicked"
            tick({(2, 0, 2): 1}, 2)
            tick(count=6)
            assert bounds() is None, "Physical mouse stopped clicking"
            # Reset re-reads the chosen options, including the selected stick.
            options(gamepad_mouse="On", mouse_stick="Right", mouse_click="L3")
            core.retro_reset()
            tick(count=12)
            tick({(5, 1, 0): -32768, (5, 1, 1): -32768})
            assert bounds()[0] == 147, "Reset lost right-stick setting"
        finally:
            core.retro_deinit()
    print("PASS: default Off/Left/R3; live enable/disable/stick/click; all 4 mappings; physical mouse; reset")


if __name__ == "__main__":
    run(sys.argv[1])
