"""Generate an original SWF fixture: changing backgrounds and a short PCM tone.

No Flash SDK or copyrighted game assets are required. SWF runs at 30 fps,
which also exercises the core's conversion from the 60 Hz libretro clock.
"""
import math
from pathlib import Path
import struct
import sys


def rect(width, height):
    values = (0, width * 20, 0, height * 20)
    size = max(values).bit_length() + 1
    bits = f"{size:05b}" + "".join(f"{v:0{size}b}" for v in values)
    bits += "0" * (-len(bits) % 8)
    return int(bits, 2).to_bytes(len(bits) // 8, "big")


def tag(code, data=b""):
    size = len(data)
    header = struct.pack("<H", (code << 6) | min(size, 63))
    return header + (struct.pack("<I", size) if size >= 63 else b"") + data


def shape(character, color):
    # One solid fill and four straight edges covering the stage.
    bits = "00010000"  # NumFillBits=1, NumLineBits=0.
    bits += "0" + "00101" + "00001" + "00" + "1"  # Move to 0,0; FillStyle1=1.
    for dx, dy in ((6400, 0), (0, 4800), (-6400, 0), (0, -4800)):
        size = max(abs(dx), abs(dy)).bit_length() + 1
        bits += "11" + f"{size - 2:04b}" + "1"
        bits += f"{dx & ((1 << size) - 1):0{size}b}{dy & ((1 << size) - 1):0{size}b}"
    bits += "000000"  # EndShapeRecord.
    bits += "0" * (-len(bits) % 8)
    records = int(bits, 2).to_bytes(len(bits) // 8, "big")
    return tag(2, struct.pack("<H", character) + rect(320, 240) + b"\x01\x00" + color + b"\x00" + records)


def make_swf():
    body = rect(320, 240) + struct.pack("<HH", 30 * 256, 3)
    samples = [int(math.sin(i * 2 * math.pi * 440 / 44100) * 6000) for i in range(4410)]
    pcm = struct.pack(f"<{len(samples)}h", *samples)
    body += tag(14, struct.pack("<HBI", 1, 0x3E, len(samples)) + pcm)
    body += tag(15, struct.pack("<HB", 1, 0))
    colors = (b"\xcc\x22\x33", b"\x22\xcc\x33", b"\x22\x33\xcc")
    for character, color in enumerate(colors, start=2):
        body += shape(character, color)
    for character in range(2, 5):
        if character != 2:
            body += tag(28, struct.pack("<H", 1))  # RemoveObject2 at depth 1.
        body += tag(4, struct.pack("<HH", character, 1) + b"\x00")  # Identity matrix.
        body += tag(1)
    body += tag(0)
    return b"FWS\x08" + struct.pack("<I", len(body) + 8) + body


if __name__ == "__main__":
    path = Path(sys.argv[1] if len(sys.argv) > 1 else "tests/fixtures/smoke.swf")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(make_swf())
    print(path)

