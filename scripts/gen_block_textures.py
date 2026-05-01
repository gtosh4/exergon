#!/usr/bin/env python3
"""
Generate placeholder 128x128 block textures for the voxel atlas.

Usage:
    python3 scripts/gen_block_textures.py

Writes PNGs to assets/textures/blocks/. Add new entries to TEXTURES dict
to generate more. After adding, also update:
  - assets/textures/blocks/manifest.ron  (append name, note atlas index)
  - src/world/generation.rs texture_index_mapper (map voxel_id -> atlas index)
"""

from PIL import Image, ImageDraw
import os

SIZE = 128
OUT = "assets/textures/blocks"


def make_metal(draw, bg, fg, bolt_color):
    draw.rectangle([0, 0, SIZE-1, SIZE-1], fill=bg)
    for i in range(0, SIZE, 16):
        draw.line([(i, 0), (i, SIZE-1)], fill=fg, width=1)
        draw.line([(0, i), (SIZE-1, i)], fill=fg, width=1)
    for bx, by in [(8, 8), (SIZE-8, 8), (8, SIZE-8), (SIZE-8, SIZE-8)]:
        draw.ellipse([bx-4, by-4, bx+4, by+4], fill=bolt_color, outline=fg)


def make_cable(draw, bg, stripe):
    draw.rectangle([0, 0, SIZE-1, SIZE-1], fill=bg)
    for i in range(0, SIZE, 16):
        draw.rectangle([i, 0, i+8, SIZE-1], fill=stripe)
    draw.rectangle([SIZE//2-8, SIZE//2-8, SIZE//2+8, SIZE//2+8], fill=stripe)


def make_crate(draw):
    draw.rectangle([0, 0, SIZE-1, SIZE-1], fill=(139, 100, 60))
    for i in range(0, SIZE, 32):
        draw.rectangle([i, 0, i+6, SIZE-1], fill=(90, 60, 30))
        draw.rectangle([0, i, SIZE-1, i+6], fill=(90, 60, 30))
    draw.rectangle([0, 0, SIZE-1, 4], fill=(90, 60, 30))
    draw.rectangle([0, SIZE-5, SIZE-1, SIZE-1], fill=(90, 60, 30))


def make_glowing_machine(draw, base, glow):
    draw.rectangle([0, 0, SIZE-1, SIZE-1], fill=base)
    fg = tuple(min(255, c+30) for c in base)
    for i in range(0, SIZE, 16):
        draw.line([(i, 0), (i, SIZE-1)], fill=fg, width=1)
        draw.line([(0, i), (SIZE-1, i)], fill=fg, width=1)
    cx, cy = SIZE//2, SIZE//2
    draw.ellipse([cx-28, cy-28, cx+28, cy+28], fill=glow)
    mid = tuple((g + b) // 2 for g, b in zip(glow, (255, 255, 255)))
    draw.ellipse([cx-14, cy-14, cx+14, cy+14], fill=mid)


# name -> draw function
# atlas index = position in manifest.ron (0-indexed)
TEXTURES = {
    # name              atlas  voxel_id
    "smelter_core":     lambda d: make_glowing_machine(d, (50,  45,  40),  (220, 90,  20)),   # 9   vox 7
    "machine_casing":   lambda d: make_metal(d, (100, 100, 110), (70, 70, 80), (160, 160, 170)),  # 10  vox 8
    "assembler_core":   lambda d: make_glowing_machine(d, (35,  40,  55),  (40,  120, 220)),  # 11  vox 9
    "refinery_core":    lambda d: make_glowing_machine(d, (40,  55,  40),  (40,  200, 80)),   # 12  vox 10
    "gateway_core":     lambda d: make_glowing_machine(d, (45,  30,  60),  (80,  220, 220)),  # 13  vox 11
    "logistics_cable":  lambda d: make_cable(d, (180, 140, 20),  (230, 200, 60)),             # 14  vox 12
    "power_cable":      lambda d: make_cable(d, (160, 30,  30),  (220, 60,  60)),             # 15  vox 13
    "storage_crate":    lambda d: make_crate(d),                                               # 16  vox 14
    "generator":        lambda d: make_glowing_machine(d, (55,  50,  40),  (255, 160, 0)),    # 17  vox 15
}


def main():
    os.makedirs(OUT, exist_ok=True)
    for name, fn in TEXTURES.items():
        img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 255))
        draw = ImageDraw.Draw(img)
        fn(draw)
        path = f"{OUT}/{name}.png"
        img.save(path)
        print(f"wrote {path}")
    print(f"done — {len(TEXTURES)} textures")


if __name__ == "__main__":
    main()
