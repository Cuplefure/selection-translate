"""Generate simple flat app icons using only the Python standard library."""

import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "src-tauri" / "icons"
OUT.mkdir(parents=True, exist_ok=True)


def make_png(size: int) -> bytes:
    rows = []
    for y in range(size):
        row = bytearray()
        for x in range(size):
            r, g, b, a = 83, 74, 183, 255
            fx, fy = x / size, y / size
            if 0.20 <= fx <= 0.80 and 0.24 <= fy <= 0.62:
                r, g, b = 255, 255, 255
            if 0.34 <= fx <= 0.50 and 0.62 < fy <= 0.76:
                r, g, b = 255, 255, 255
            row.extend((r, g, b, a))
        rows.append(bytes([0]) + bytes(row))
    raw = b"".join(rows)

    def chunk(tag: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def make_ico(sizes: list[int]) -> bytes:
    images = [(s, make_png(s)) for s in sizes]
    header = struct.pack("<HHH", 0, 1, len(images))
    offset = 6 + 16 * len(images)
    entries = b""
    for size, data in images:
        entries += struct.pack(
            "<BBBBHHII",
            size if size < 256 else 0,
            size if size < 256 else 0,
            0,
            0,
            1,
            32,
            len(data),
            offset,
        )
        offset += len(data)
    return header + entries + b"".join(data for _, data in images)


(OUT / "icon.png").write_bytes(make_png(256))
(OUT / "icon.ico").write_bytes(make_ico([16, 32, 48, 256]))
print(f"icons written to {OUT}")
