"""Exercise real Ruffle hit testing, cursor visibility and click hotspot through libretro."""
import argparse
from pathlib import Path
import struct
import tempfile
from types import SimpleNamespace
import zlib

from make_cursor_swf import make_swf
from probe_game import run


def read_png(path):
    data = path.read_bytes()
    width, height = struct.unpack_from(">II", data, 16)
    compressed = b""
    offset = 8
    while offset < len(data):
        size = struct.unpack_from(">I", data, offset)[0]
        if data[offset+4:offset+8] == b"IDAT":
            compressed += data[offset+8:offset+8+size]
        offset += size + 12
    raw = zlib.decompress(compressed)
    stride = width*3 + 1
    assert all(raw[y*stride] == 0 for y in range(height))
    return width, height, b"".join(raw[y*stride+1:(y+1)*stride] for y in range(height))


def verify(output, scale):
    width, height, background = read_png(output / "frame-00050.png")
    def cursor_bounds(frame):
        w, h, pixels = read_png(output / f"frame-{frame:05d}.png")
        assert (w,h) == (width,height)
        changed = [(i % width, i // width) for i in range(width*height)
                   if pixels[i*3:i*3+3] != background[i*3:i*3+3]]
        assert changed, f"No visible cursor at frame {frame}"
        return min(x for x,y in changed), min(y for x,y in changed), max(x for x,y in changed), max(y for x,y in changed)
    assert cursor_bounds(10) == (30*scale-5,30*scale-5,30*scale+5,30*scale+5), "Original cross/hotspot changed"
    assert cursor_bounds(20)[3] > 100*scale+5, "Hover did not select pointing hand"
    assert cursor_bounds(30) == (101*scale-5,100*scale-5,101*scale+5,100*scale+5), "useHandCursor=false should restore cross"
    assert cursor_bounds(40)[3] > 100*scale+5, "Hand did not return after re-enabling"
    assert read_png(output / "frame-00070.png")[2] == background, "Click near left edge missed button/hide action"
    assert cursor_bounds(60)[3] > 100*scale+5, "Mouse.show did not restore hand"
    assert cursor_bounds(80)[3] > 100*scale+5, "Mouse.show after click failed"
    assert cursor_bounds(90) == (width-6,height-6,width-1,height-1), "Cross clipping/hotspot incorrect"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("library")
    parser.add_argument("--output", default="target/cursor-test")
    args = parser.parse_args()
    root = Path(args.output).resolve()
    with tempfile.TemporaryDirectory(prefix="ruffle-cursor-") as temporary:
        fixture = Path(temporary) / "cursor.swf"
        fixture.write_bytes(make_swf())
        for scale in (1, 2):
            output = root / f"scale-{scale}"
            run(SimpleNamespace(
                library=args.library, game=str(fixture), output=str(output), frames=100,
                capture="10,20,30,40,50,60,70,80,90", check_cache=True, set_at=[],
                option=[f"ruffle_resolution={'Native' if scale==1 else '640x480'}"],
                move=[f"2,{30*scale},{30*scale}", f"15,{100*scale},{100*scale}",
                      f"29,{101*scale},{100*scale}", f"39,{102*scale},{100*scale}",
                      f"85,{320*scale-1},{240*scale-1}"],
                click=[f"65,{81*scale},{100*scale}"],
                key=["25,10,100", "35,10,101", "45,10,104", "55,10,115", "75,10,115"],
            ))
            verify(output, scale)
    print("PASS: real Flash button hover, useHandCursor, hide/show, click hotspot, edge clipping, 2 resolutions, paused frame lifetime")


if __name__ == "__main__":
    main()
