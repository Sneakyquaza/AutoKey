"""Convertit des captures .ppm (P6) en .png, sans dépendance (pour les lire facilement)."""
import struct
import sys
import zlib


def chunk(tag, data):
    return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)


def convert(src):
    raw = open(src, "rb").read()
    _, w, h, _, px = raw.split(maxsplit=4)
    w, h = int(w), int(h)
    rows = b"".join(b"\x00" + px[y * w * 3:(y + 1) * w * 3] for y in range(h))
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(rows, 9)) + chunk(b"IEND", b"")
    open(src[:-4] + ".png", "wb").write(png)


for f in sys.argv[1:]:
    convert(f)
