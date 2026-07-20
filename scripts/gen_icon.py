#!/usr/bin/env python3
"""Generate a simple app icon for gpui-bar and build an .icns via iconutil.

Design: dark rounded square with a white horizontal bar motif (the app floats a
thin bar at the top-right of the screen). Run once locally; the resulting
AppIcon.icns is committed to the repo so CI does not need PIL.
"""
import subprocess
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw

ASSETS_DIR = Path(__file__).resolve().parent.parent / "assets"
ICONSET = ASSETS_DIR / "AppIcon.iconset"
ICNS = ASSETS_DIR / "AppIcon.icns"

# (filename, size)
SIZES = [
    ("icon_16x16.png", 16),
    ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128),
    ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256),
    ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512),
    ("icon_512x512@2x.png", 1024),
]


def render(size: int) -> Image.Image:
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)

    # rounded square background, dark slate
    radius = int(size * 0.22)
    pad = int(size * 0.06)
    d.rounded_rectangle(
        [pad, pad, size - pad, size - pad],
        radius=radius,
        fill=(30, 41, 59, 255),  # slate-800
    )

    # three stacked horizontal bars (the "bar" motif), white, centered
    bar_w = int(size * 0.56)
    bar_h = max(int(size * 0.07), 2)
    gap = max(int(size * 0.05), 1)
    total_h = bar_h * 3 + gap * 2
    x0 = (size - bar_w) // 2
    y0 = (size - total_h) // 2
    for i in range(3):
        # top bar full width, middle shorter, bottom medium — a little stat-row look
        w = bar_w if i == 0 else (int(bar_w * 0.7) if i == 1 else int(bar_w * 0.85))
        x = (size - w) // 2
        y = y0 + i * (bar_h + gap)
        d.rounded_rectangle([x, y, x + w, y + bar_h], radius=max(bar_h // 2, 1), fill=(255, 255, 255, 255))

    return img


def main() -> None:
    ASSETS_DIR.mkdir(parents=True, exist_ok=True)
    if ICONSET.exists():
        for f in ICONSET.iterdir():
            f.unlink()
    ICONSET.mkdir(exist_ok=True)

    for name, size in SIZES:
        render(size).save(ICONSET / name)

    if ICNS.exists():
        ICNS.unlink()
    subprocess.run(["iconutil", "-c", "icns", str(ICONSET), "-o", str(ICNS)], check=True)
    # clean up the iconset dir
    for f in ICONSET.iterdir():
        f.unlink()
    ICONSET.rmdir()
    print(f"generated {ICNS} ({ICNS.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
