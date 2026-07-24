#!/usr/bin/env python3
"""Sinh icon nguon 1024x1024 cho `tauri icon`. Chi dung stdlib (zlib + struct)
nen khong can cai them gi. Hinh: vien bo tron nen toi + vong donut xanh —
dung mo-tip cua chinh cai panel.

Chay:  python3 scripts/make_icon.py  ->  app-icon.png
"""
import math
import struct
import zlib

SIZE = 1024
OUT = "app-icon.png"

BG = (26, 26, 25, 255)        # #1a1a19 — surface toi cua panel
RING = (57, 135, 229, 255)    # #3987e5 — series slot 1 (dark)
GAP_DEG = 8.0                 # khe ho tren vong, cho no khong phai vong kin

CORNER = SIZE * 0.22
CX = CY = SIZE / 2.0
R_OUT = SIZE * 0.33
R_IN = SIZE * 0.21


def blend(dst, src, a):
    """src phu len dst voi do phu a (0..1)."""
    return tuple(round(d + (s - d) * a) for d, s in zip(dst[:3], src[:3])) + (255,)


def coverage(d, edge, soft=1.2):
    """Anti-alias 1 chieu: tra ve 0..1 theo khoang cach toi bien."""
    return min(1.0, max(0.0, (edge - d) / soft + 0.5))


def rounded_rect_alpha(x, y):
    """Alpha cua hinh vuong bo goc, co anti-alias."""
    hw = SIZE / 2.0
    dx = abs(x - CX) - (hw - CORNER)
    dy = abs(y - CY) - (hw - CORNER)
    if dx <= 0 and dy <= 0:
        return 1.0
    dx = max(dx, 0.0)
    dy = max(dy, 0.0)
    return coverage(math.hypot(dx, dy), CORNER)


def ring_alpha(x, y):
    dx, dy = x - CX, y - CY
    r = math.hypot(dx, dy)
    if r > R_OUT + 2 or r < R_IN - 2:
        return 0.0
    # be day vong
    a = coverage(r, R_OUT) * coverage(R_IN, r)
    if a <= 0:
        return 0.0
    # cat mot khe ho o dinh vong (goc -90 do)
    ang = (math.degrees(math.atan2(dy, dx)) + 450.0) % 360.0
    half = GAP_DEG / 2.0
    if ang < half or ang > 360.0 - half:
        return 0.0
    return a


def main():
    rows = bytearray()
    for py in range(SIZE):
        rows.append(0)  # filter byte: None
        y = py + 0.5
        for px in range(SIZE):
            x = px + 0.5
            ra = rounded_rect_alpha(x, y)
            if ra <= 0.0:
                rows.extend((0, 0, 0, 0))
                continue
            px_col = BG
            ka = ring_alpha(x, y)
            if ka > 0.0:
                px_col = blend(px_col, RING, ka)
            rows.extend((px_col[0], px_col[1], px_col[2], round(255 * ra)))

    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(bytes(rows), 9))
    png += chunk(b"IEND", b"")

    with open(OUT, "wb") as f:
        f.write(png)
    print(f"da ghi {OUT} ({SIZE}x{SIZE}, {len(png) // 1024} KB)")


if __name__ == "__main__":
    main()
