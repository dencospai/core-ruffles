"""Original cursor fixture: a button at (80,80)-(160,140).

Click hides the pointer. H/S hide/show; D/E disable/enable the button's hand.
No third-party game assets or ActionScript compiler needed.
"""
from pathlib import Path
import struct
import sys
from make_test_swf import rect, shape, tag


def action(code, payload=b""):
    return bytes([code]) + (struct.pack("<H", len(payload)) + payload if code >= 0x80 else b"")


def push(*values):
    data = b""
    for value in values:
        if isinstance(value, bool):
            data += b"\x05" + bytes([value])
        elif isinstance(value, int):
            data += b"\x07" + struct.pack("<i", value)
        else:
            data += b"\0" + value.encode() + b"\0"
    return action(0x96, data)


def mouse(method):
    return push(0, "Mouse") + action(0x1C) + push(method) + action(0x52) + action(0x17)


def on_key(key, body):
    condition = push(ord(key), 1, "Key") + action(0x1C) + push("isDown") + action(0x52)
    return condition + action(0x12) + action(0x9D, struct.pack("<h", len(body))) + body


def matrix(scale=1, x=0, y=0):
    def signed(value, n):
        return f"{value & ((1 << n) - 1):0{n}b}"
    bits = "0"
    if scale != 1:
        value = round(scale * 65536)
        n = abs(value).bit_length() + 1
        bits = "1" + f"{n:05b}" + signed(value, n) * 2
    bits += "0"  # No skew/rotation.
    n = max(abs(x*20), abs(y*20)).bit_length() + 1
    bits += f"{n:05b}" + signed(x*20, n) + signed(y*20, n)
    bits += "0" * (-len(bits) % 8)
    return int(bits, 2).to_bytes(len(bits)//8, "big")


def make_swf():
    body = rect(320, 240) + struct.pack("<HH", 30*256, 3)
    body += tag(9, bytes.fromhex("3a5678"))
    body += shape(1, bytes.fromhex("55aa33"))
    # DefineButton: Up/Over/Down/Hit share the same shape, scaled to 80x60.
    button = struct.pack("<H", 2) + b"\x0f" + struct.pack("<HH", 1, 1) + matrix(.25)
    button += b"\0" + mouse("hide") + b"\0"
    body += tag(7, button)
    # Named PlaceObject2 at stage position 80,80.
    body += tag(26, b"\x26" + struct.pack("<HH", 1, 2) + matrix(x=80,y=80) + b"btn\0")
    body += tag(1)
    handlers = on_key("H", mouse("hide")) + on_key("S", mouse("show"))
    for key, enabled in [("D", False), ("E", True)]:
        handlers += on_key(key, push("btn") + action(0x1C) + push("useHandCursor", enabled) + action(0x4F))
    body += tag(12, handlers + b"\0") + tag(1)
    body += tag(12, action(0x81, struct.pack("<H", 1)) + action(0x06) + b"\0") + tag(1) + tag(0)
    return b"FWS\x06" + struct.pack("<I", len(body)+8) + body


if __name__ == "__main__":
    path = Path(sys.argv[1] if len(sys.argv)>1 else "tests/fixtures/cursor.swf")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(make_swf())
    print(path)
