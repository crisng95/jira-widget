#!/usr/bin/env python3
"""Sinh tray icon TEMPLATE cho menu bar macOS -> src-tauri/icons/tray.png.

Template icon = hinh DEN tren nen trong suot; macOS tu doi mau theo sang/toi
cua menu bar. Dung icon app mau (o vuong dac) voi set_icon_as_template(true)
la ra dung mot cuc trang — bug da gap.

Hinh: vong donut co khe ho — cung mo-tip voi app icon (make_icon.py).
Kich thuoc 44x44 (22pt @2x). Chay: python3 scripts/make_tray_icon.py
"""
from PIL import Image, ImageDraw

SS = 8  # supersample cho net
SIZE = 44
BIG = SIZE * SS

img = Image.new("RGBA", (BIG, BIG), (0, 0, 0, 0))
d = ImageDraw.Draw(img)

cx = cy = BIG / 2
r_out = BIG * 0.46
r_in = BIG * 0.26
# vong tron day: ve annulus bang 2 ellipse
d.ellipse([cx - r_out, cy - r_out, cx + r_out, cy + r_out], fill=(0, 0, 0, 255))
d.ellipse([cx - r_in, cy - r_in, cx + r_in, cy + r_in], fill=(0, 0, 0, 0))

# khe ho 40 do phia tren-phai — cho giong donut cua panel, khong phai chu O
d.pieslice([cx - r_out * 1.05, cy - r_out * 1.05, cx + r_out * 1.05, cy + r_out * 1.05],
           start=-72, end=-32, fill=(0, 0, 0, 0))

img = img.resize((SIZE, SIZE), Image.LANCZOS)
img.save("src-tauri/icons/tray.png")
print("da ghi src-tauri/icons/tray.png")
