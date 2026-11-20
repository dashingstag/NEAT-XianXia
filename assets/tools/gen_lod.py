#!/usr/bin/env python3
"""LOD board asset generator — three fidelity levels for every map object.

Mirrors field.gd's zoom contract (effective zoom <2 = overview, <5 = drawn,
>=5 = animated). Emits under assets/parked/lod/:

  dots/<id>.png   24x24 level-1 marker: dot or landmark shape
  anim/<id>.png   level-3 horizontal frame strip (64px cells, for integration)
  apng/<id>.png   animated level-3 preview consumed by LOD_BOARD.html
  l2/<id>.png     level-2 glyph for objects that have no parked texture yet
  scene-l{1,2,3}  the same street scene rendered at each level
  manifest.json   board rows consumed by build_lod_board.py

Rebuild: python3 assets/tools/gen_lod.py && python3 assets/tools/build_lod_board.py
"""
import json
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from PIL import Image, ImageDraw
from gen_assets import (PAL, cell, finish, px, rect, line, arc, poly, motif,
                        draw_person, draw_animal, draw_site, draw_terrain,
                        person_cell, animal_cell, site_cell, terrain_cell,
                        sheet, MORTALS, IMMORTALS, ANIMALS, SITES, TERRAIN)

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LOD = os.path.join(ROOT, "parked", "lod")
FRAMES = 8          # LOD3 frame budget; shader clock runs 10fps
FRAME_MS = 110

# field.gd patch colors, kinds 0..7 live, 8+ reserved
PATCH_COLORS = ["#a95d76", "#365a38", "#d0ccb1", "#bce1df",
                "#edc65d", "#c8d6df", "#ba7650", "#8d949b"]
# field.gd overview palette: palette[action] for actions 0..5 (+clamp),
# plus sleeping and dead overrides.
ACTION_DOTS = [("Rest", "#e5c985"), ("Gather", "#90c99a"), ("Deliver", "#e2bb78"),
               ("Buy food", "#99c8d7"), ("Build", "#cbb1de"),
               ("Trade / other", "#df9c78"), ("Sleeping", "#b5cce3"),
               ("Dead", "#686a60")]
ZONE_COLORS = {"residential": "#77b7ed", "commercial": "#f4ce75",
               "industrial": "#c49ae6"}
ANIMAL_DOT = {"rabbit": ("#f0e8d5", 2.2), "deer": ("#dcba80", 3.3),
              "boar": ("#6b5a48", 2.6), "pheasant": ("#8a5a3d", 2.2),
              "fox": ("#b06a3e", 2.6), "crane": ("#f0ede4", 2.4),
              "spirit-deer": ("#9ecfc4", 3.3), "spirit-fox": ("#b8b0d0", 2.6),
              "qilin-colt": ("#d4c9a8", 3.3)}


def save(img, *path):
    p = os.path.join(LOD, *path)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    img.save(p)
    return p


def save_apng(frames, *path):
    p = os.path.join(LOD, *path)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    frames[0].save(p, save_all=True, append_images=frames[1:],
                   duration=FRAME_MS, loop=0)
    return p


def resolve(spec):
    return {k: (PAL.get(v, v) if isinstance(v, str) else v)
            for k, v in spec.items()}


def strip(frames):
    return sheet(frames, len(frames), pad=0, bg=(0, 0, 0, 0))


def erase(d, x0, y0, x1, y1, s):
    # ImageDraw on RGBA writes values (no blend): transparent fill erases.
    d.rectangle([x0 * s, y0 * s, x1 * s, y1 * s], fill=(0, 0, 0, 0))


def fade(rgb, a):
    rgb = rgb.lstrip("#")
    return (int(rgb[0:2], 16), int(rgb[2:4], 16), int(rgb[4:6], 16), a)


# =================================================================== level 1
# 24px markers. In-game LOD1 draws colored dots for people/animals, landmark
# shapes for settlement stages and diamonds for mines; other objects reuse
# tiny silhouettes in their field.gd fallback colors.


def _d(fn, size=24):
    img, d, s = cell(size, scale=8)
    fn(d, s)
    return img.resize((size, size), Image.NEAREST)


def hut(d, s, wall, roof="#5a6166", w=13.0, x=12.0, top=5.0):
    rect(d, x - w / 2, 11, x + w / 2, 19, wall, s)
    poly(d, [(x - w / 2 - 2, 11), (x, top), (x + w / 2 + 2, 11)], roof, s)


def dot_home(d, s):
    hut(d, s, "#92c4de")


def dot_shop(sign_c):
    def go(d, s):
        hut(d, s, "#e0bd6d")
        line(d, 18.5, 9, 21, 9, "#7a5c43", s, 1)
        rect(d, 19.5, 9, 22, 12.5, sign_c, s)
    return go


def dot_workshop(d, s):
    hut(d, s, "#b89aca", roof="#4a5054")
    rect(d, 17, 4, 19, 8, "#7a5c43", s)
    px(d, (18, 2.6), 1.2, fade("b9b2a4", 200), s)


def dot_bank(d, s):
    rect(d, 7, 13, 17, 19, "#cbd2ad", s)
    poly(d, [(5, 13), (12, 8), (19, 13)], "#5a6166", s)
    rect(d, 9, 9.5, 15, 12, "#cbd2ad", s)
    poly(d, [(7.5, 9.5), (12, 5.5), (16.5, 9.5)], "#5a6166", s)
    px(d, (12, 4), 1.0, PAL["gilt"], s)


def dot_tavern(d, s):
    hut(d, s, "#d9bc87", roof="#8a4a3e")
    px(d, (19.5, 12.5), 1.8, PAL["cinnabar"], s)
    px(d, (19.5, 12.5), 0.9, PAL["fire"], s)


def dot_school(d, s):
    for x in (7, 17):
        line(d, x, 19, x, 9, "#7a5c43", s, 1.6)
    line(d, 5, 9, 19, 9, "#7a5c43", s, 1.6)
    line(d, 5, 9, 5, 7, "#7a5c43", s, 1.4)
    line(d, 19, 9, 19, 7, "#7a5c43", s, 1.4)
    rect(d, 10, 10.5, 14, 13.5, PAL["cinnabar"], s)


def dot_farm(d, s):
    rect(d, 4, 5, 20, 19, "#5d8076", s)
    for r in range(4):
        line(d, 6, 8 + r * 3.5, 18, 8 + r * 3.5, "#c8c15f", s, 1.3)


def dot_graveyard(d, s):
    rect(d, 4, 5, 20, 19, "#858a70", s, outline="#c2bca0", width=1)
    for i in range(4):
        gx = 8 + (i % 2) * 7
        gy = 9 + (i // 2) * 5
        rect(d, gx - 1, gy - 1, gx + 1, gy + 2, "#c3c9ba", s)


def dot_shelter(d, s):
    poly(d, [(5, 19), (12, 6), (19, 19)], "#c6a87c", s)
    line(d, 12, 10, 12, 19, "#614e3a", s, 1.4)
    px(d, (20, 19), 1.4, PAL["fire"], s)


def dot_jobboard(d, s):
    line(d, 8, 19, 8, 8, "#634b34", s, 1.4)
    line(d, 16, 19, 16, 8, "#634b34", s, 1.4)
    rect(d, 6, 6, 18, 12, "#ead7a6", s, outline="#634b34", width=1)


def dot_well(d, s):
    px(d, (12, 14), 6, "#b9a678", s)
    px(d, (12, 14), 4, "#5a5142", s)
    poly(d, [(7, 8), (12, 3.5), (17, 8)], "#5a6166", s)


def dot_mine(ore):
    def go(d, s):
        poly(d, [(12, 5), (19, 12), (12, 19), (5, 12)], "#3d3832", s)
        poly(d, [(12, 7), (17, 12), (12, 17), (7, 12)], ore, s)
    return go


def dot_gate(d, s):
    for x in (7, 17):
        line(d, x, 20, x, 10, "#6a5236", s, 2)
    poly(d, [(4, 9), (12, 3), (20, 9)], PAL["immortal_jade"], s)
    arc(d, (12, 15), 5, 180, 360, PAL["spirit_cyan"], s, 1.2)


def dot_pavilion(d, s):
    poly(d, [(5, 9), (12, 4), (19, 9)], PAL["immortal_jade"], s)
    for x in (8, 16):
        line(d, x, 20, x, 10, "#8a6a44", s, 1.6)
    arc(d, (12, 17), 4, 0, 180, PAL["spirit_cyan"], s, 1.0)


def dot_patch(color, r=5.0):
    def go(d, s):
        px(d, (12, 13), r, fade("26382d", 120), s)
        px(d, (12, 12), r * 0.8, color, s)
    return go


def dot_tree(d, s):
    px(d, (13.5, 12.5), 7, fade("10221a", 90), s)
    px(d, (12, 11), 7, "#365c43", s)
    px(d, (10.5, 9.5), 5, "#507448", s)


def dot_person(color):
    def go(d, s):
        px(d, (12, 13), 4.5, fade("26382d", 120), s)
        px(d, (12, 12), 4, color, s)
    return go


def dot_animal(color, r):
    def go(d, s):
        px(d, (12, 13), r + 1.2, fade("26382d", 120), s)
        px(d, (12, 12), r, color, s)
    return go


def dot_convoy(d, s):
    rect(d, 6, 8, 18, 15, "#f4d88d", s, outline="#443c30", width=1)
    px(d, (9, 16.5), 1.8, "#443c30", s)
    px(d, (15, 16.5), 1.8, "#443c30", s)


def dot_rally(d, s):
    arc(d, (12, 13), 7, 0, 360, "#e4e4be", s, 1.4)
    for i, c in enumerate(["#e5c985", "#90c99a", "#df9c78"]):
        px(d, (9 + i * 3, 12 + (i % 2) * 3), 1.6, c, s)


def dot_select(d, s):
    arc(d, (12, 12), 8, 0, 360, "#ffedaa", s, 1.2)


def dot_zone(color):
    def go(d, s):
        px(d, (12, 12), 8, fade(color, 45), s)
        arc(d, (12, 12), 8, 0, 360, color, s, 1.4)
    return go


def dot_road(d, s):
    line(d, 4, 20, 20, 5, "#a7966d", s, 5)
    line(d, 4, 20, 20, 5, "#c7b489", s, 3)


def dot_river(d, s):
    for i in range(2):
        pts = [(4 + k * 3, 9 + i * 7 + math.sin(k * 1.3) * 2) for k in range(6)]
        d.line([(x * s, y * s) for x, y in pts], fill="#5d97a6",
               width=max(1, int(2 * s)))


def dot_coast(d, s):
    rect(d, 3, 3, 13, 21, "#c7bd91", s)
    rect(d, 13, 3, 21, 21, "#477986", s)
    line(d, 15, 6, 19, 7, fade("c2e0d6", 130), s, 1.2)
    line(d, 15, 13, 19, 14, fade("c2e0d6", 130), s, 1.2)


def dot_hill(d, s):
    arc(d, (12, 20), 11, 195, 345, "#838d70", s, 6)
    arc(d, (12, 20), 8, 195, 345, "#a8ad94", s, 4)
    arc(d, (12, 20), 5, 195, 345, "#c5c9b0", s, 2.5)


def dot_grass(d, s):
    px(d, (12, 12), 6, "#78a65a", s)


def dot_fertility(d, s):
    rect(d, 4, 4, 20, 20, fade("679453", 46), s)
    for i in range(3):
        line(d, 8 + i * 4, 17, 8 + i * 4, 11, fade("679453", 140), s, 1)


def dot_fog(d, s):
    px(d, (9, 12), 6, (214, 222, 216, 150), s)
    px(d, (15, 11), 6, (214, 222, 216, 150), s)


def dot_reveal(d, s):
    arc(d, (12, 12), 8, 0, 360, PAL["spirit_cyan"], s, 1.4)
    px(d, (12, 12), 2, PAL["spirit_cyan"], s)


def dot_trade(d, s):
    line(d, 4, 18, 20, 6, "#5db4a6", s, 2)
    px(d, (20, 6), 2, "#5db4a6", s)


def dot_mist(d, s):
    for i in range(2):
        arc(d, (10 + i * 5, 10 + i * 5), 5, 180, 340, PAL["spirit_cyan"], s, 1.2)


def dot_unknown(d, s):
    px(d, (12, 12), 7, (56, 90, 120, 110), s)
    arc(d, (12, 12), 8, 0, 360, PAL["spirit_cyan"], s, 1.2)
    line(d, 12, 9, 12, 13, PAL["spirit_cyan"], s, 1.6)
    px(d, (12, 15.5), 1.0, PAL["spirit_cyan"], s)


def dot_town(d, s):
    rect(d, 3, 3, 21, 21, "#293c32", s)
    rect(d, 7, 7, 17, 17, "#eed198", s)


def dot_village(d, s):
    px(d, (12, 12), 9, "#293c32", s)
    px(d, (12, 12), 5.8, "#dcb884", s)


def dot_camp(d, s):
    poly(d, [(12, 3), (21, 20), (3, 20)], "#634d37", s)


def dot_herbs(spirit=False):
    def go(d, s):
        px(d, (12, 13), 5, fade("26382d", 120), s)
        px(d, (12, 12), 4.2, "#5a8a4e" if not spirit else "#8fc1a9", s)
        if spirit:
            arc(d, (12, 12), 7, 0, 360, PAL["spirit_cyan"], s, 1.0)
    return go


def dot_mushrooms(d, s):
    for mx, my in [(9, 12), (15, 14), (12, 9)]:
        line(d, mx, my + 3, mx, my, "#d8d0bc", s, 1.8)
        px(d, (mx, my), 1.8, "#a95d4e", s)


# =================================================================== level 3
# Animation accents drawn over the static glyph, one function per id. Each
# gets (d, s, f) with f in 0..FRAMES-1 and mutates the supersampled cell.


def glow(d, s, f, x, y, r, color, phase=0.0, lo=0.10, hi=0.55):
    a = lo + (hi - lo) * (0.5 + 0.5 * math.sin((f / FRAMES + phase) * math.tau))
    px(d, (x, y), r, fade(color, int(255 * a)), s)


def smoke(d, s, f, x, y, n=2, rise=11.0, drift=3.5):
    for k in range(n):
        ph = (f / FRAMES + k / n) % 1.0
        px(d, (x + ph * drift + k * 0.7, y - ph * rise),
           2.4 - ph * 1.5, (185, 178, 164, int(170 * (1 - ph))), s)


def sparkle(d, s, f, pts, every=2):
    for i, (x, y) in enumerate(pts):
        if (f + i) % FRAMES < every:
            c = PAL["label"]
            line(d, x - 1.6, y, x + 1.6, y, c, s, 0.9)
            line(d, x, y - 1.6, x, y + 1.6, c, s, 0.9)


def ripple(d, s, f, x, y, r0, r1, color, n=2, w=1.0):
    for k in range(n):
        ph = (f / FRAMES + k / n) % 1.0
        arc(d, (x, y), r0 + (r1 - r0) * ph, 200, 340,
            fade(color.lstrip("#") if isinstance(color, str) else "5d97a6",
                 int(200 * (1 - ph))), s, w)


def flag(d, s, f, x, y, color, wdt=5.0):
    # Small pennant fluttering on a pole.
    line(d, x, y, x, y - 7, "#7a5c43", s, 1)
    dx = math.sin(f * math.tau / FRAMES)
    poly(d, [(x, y - 7), (x + wdt, y - 6 + dx * 1.4), (x, y - 4.5)], color, s)


def _site_frames(spec, accent):
    frames = []
    for f in range(FRAMES):
        img, d, s = cell(64)
        draw_site(d, s, spec)
        accent(d, s, f, spec)
        frames.append(finish(img))
    return frames


def acc_home(d, s, f, spec):
    smoke(d, s, f, 40, 26, n=2, rise=10, drift=2.5)
    glow(d, s, f, 32, 45, 3.4, "f4c77a", lo=0.12, hi=0.5)


def acc_shop(d, s, f, spec):
    # Swinging hanging sign: erase, redraw at sway offset.
    dx = round(math.sin(f * math.tau / FRAMES) * 1.6)
    erase(d, 50, 30, 62, 46, s)
    line(d, 48, 34, 54 + dx, 34, PAL["mortal_brown"], s, 1.5)
    rect(d, 52 + dx, 34, 58 + dx, 42, spec.get("sign_c", PAL["gilt"]), s)
    if spec.get("motif"):
        motif(d, s, spec["motif"], 55 + dx, 38)
    glow(d, s, f, 32, 40, 5, "f4c77a", lo=0.05, hi=0.30)


def acc_workshop(d, s, f, spec):
    smoke(d, s, f, 46, 8, n=3, rise=12, drift=4)
    glow(d, s, f, 30, 44, 4, "ffda82", lo=0.10, hi=0.45)  # forge ember
    if spec.get("motif"):
        sparkle(d, s, f, [(55, 38)])


def acc_bank(d, s, f, spec):
    sparkle(d, s, f, [(32, 14), (32, 48)], every=3)
    glow(d, s, f, 32, 48, 4, "f4c77a", lo=0.08, hi=0.35)


def acc_tavern(d, s, f, spec):
    for i, lx in enumerate((18, 46)):
        glow(d, s, f, lx, 38, 5, "ff9a5a", phase=i * 0.5, lo=0.15, hi=0.5)
    # Sake banner edge flutter.
    erase(d, 54, 24, 58.5, 26, s)
    dx = math.sin(f * math.tau / FRAMES) * 1.2
    line(d, 54, 24, 58, 24 + dx, PAL["mortal_brown"], s, 1)


def acc_school(d, s, f, spec):
    flag(d, s, f, 46, 26, PAL["cinnabar"], 5)
    glow(d, s, f, 15, 34, 4, "f4c77a", lo=0.05, hi=0.30)


def acc_farm(d, s, f, spec):
    # Wind ripples drifting over the paddy rows.
    for k in range(2):
        x = 16 + ((f / FRAMES + k * 0.5) % 1.0) * 32
        arc(d, (x, 40), 7, 200, 330, fade("e8e2b0", 150), s, 1.1)


def acc_graveyard(d, s, f, spec):
    for k in range(2):
        x = 16 + ((f / FRAMES + k * 0.5) % 1.0) * 32
        arc(d, (x, 30 + k * 9), 6, 180, 330, fade("d6ded2", 120), s, 1.2)
    glow(d, s, f, 24, 38, 1.6, "9fc4c8", phase=0.3, lo=0.2, hi=0.7)


def acc_shelter(d, s, f, spec):
    r = 1.4 + 0.5 * math.sin(f * math.tau / FRAMES * 2)
    px(d, (48, 46), r + 0.8, fade("ff9a5a", 160), s)
    px(d, (48, 46), r, PAL["fire"], s)
    smoke(d, s, f, 48, 42, n=1, rise=8, drift=1.5)


def acc_jobboard(d, s, f, spec):
    dx = math.sin(f * math.tau / FRAMES) * 1.0
    erase(d, 33, 43, 40, 46, s)
    rect(d, 33, 43, 40 - 4 + dx, 46, "#ead7a6", s, outline=PAL["mortal_brown"],
         width=1)


def acc_market(d, s, f, spec):
    ripple(d, s, f, 32, 42, 3, 7, "5d97a6", n=2, w=0.9)
    bob = round(math.sin(f * math.tau / FRAMES) * 1.0)
    line(d, 32, 30, 32, 34 + bob, "#7a5c43", s, 1)
    rect(d, 30.5, 34 + bob, 33.5, 37 + bob, "#8a6a44", s)


def acc_mine(d, s, f, spec):
    # Headframe wheel spins; ore seams twinkle.
    a0 = f * math.tau / FRAMES
    for k in range(3):
        a = a0 + k * math.tau / 3
        line(d, 24, 24, 24 + 4 * math.cos(a), 24 + 4 * math.sin(a),
             PAL["wood"], s, 1.0)
    c = spec.get("ore", PAL["ore"][3])
    sparkle(d, s, f, [(26, 40), (34, 38), (30, 46)], every=3)
    glow(d, s, f, 40, 46, 2.2, c.lstrip("#"), phase=0.5, lo=0.0, hi=0.5)


def acc_gate(d, s, f, spec):
    r = 12 + 2.5 * math.sin(f * math.tau / FRAMES)
    arc(d, (32, 44), r, 180, 360, fade("38bdf8", 200), s, 1.0)
    smoke(d, s, f, 20, 46, n=1, rise=6, drift=2)


def acc_pavilion(d, s, f, spec):
    r = 10 + 2.0 * math.sin(f * math.tau / FRAMES)
    arc(d, (32, 46), r, 0, 180, fade("38bdf8", 190), s, 0.9)
    for k in range(2):
        x = 22 + ((f / FRAMES + k * 0.5) % 1.0) * 20
        px(d, (x, 40 - k * 6), 0.9, fade("e8b4c8", 200), s)


SITE_ANIM = {"home": acc_home, "shop": acc_shop, "workshop": acc_workshop,
             "bank": acc_bank, "tavern": acc_tavern, "school": acc_school,
             "farm": acc_farm, "graveyard": acc_graveyard,
             "shelter": acc_shelter, "jobboard": acc_jobboard,
             "market": acc_market, "mine": acc_mine, "gate": acc_gate,
             "pavilion": acc_pavilion}


# ---- flora: trees redraw with canopy sway, patches get overlay accents ----

def tree_frames(shape):
    frames = []
    for f in range(FRAMES):
        img, d, s = cell(64)
        dx = math.sin(f * math.tau / FRAMES)
        if shape == "pine":
            line(d, 32, 48, 32, 54, PAL["mortal_brown"], s, 2)
            for i, (w, y) in enumerate([(20, 46), (16, 36), (10, 26)]):
                o = dx * (0.8 + i * 0.6)
                poly(d, [(32 - w / 2 + o, y), (32 + o, y - 14),
                         (32 + w / 2 + o, y)], PAL["pine"], s)
        elif shape == "bamboo":
            for i, x in enumerate([24, 30, 36, 42]):
                top_x = x + (i - 1.5) + dx * (1.2 + i * 0.3)
                line(d, x, 54, top_x, 20 - (i % 2) * 3, PAL["bamboo"], s, 2.2)
                line(d, top_x, 30 - (i % 2) * 3, top_x + 4, 26 - (i % 2) * 3,
                     PAL["bamboo"], s, 1.2)
                line(d, top_x, 24 - (i % 2) * 3, top_x - 4, 21 - (i % 2) * 3,
                     PAL["bamboo"], s, 1.2)
        elif shape == "willow":
            arc(d, (32 + dx * 1.4, 34), 14, 180, 360, "#5a7d4e", s, 8)
            for ddx in (-8, -3, 2, 7):
                line(d, 32 + ddx + dx * 1.4, 30,
                     32 + ddx + (ddx // 4) + dx * 2.2, 46, "#5a7d4e", s, 1.4)
            line(d, 32, 40, 32, 54, PAL["mortal_brown"], s, 2)
        elif shape == "plum":
            line(d, 30, 54, 30, 34, PAL["mortal_brown"], s, 2)
            line(d, 30, 42, 38 + dx * 0.8, 32, PAL["mortal_brown"], s, 1.4)
            line(d, 30, 40, 24 + dx * 0.6, 32, PAL["mortal_brown"], s, 1.2)
            for bx, by in [(30, 30), (38, 30), (24, 30), (34, 36), (27, 35)]:
                px(d, (bx + dx * 1.4, by), 2.2, PAL["plum"], s)
        elif shape == "tree_generic":
            px(d, (33 + dx * 1.5, 30), 11, "#426540", s)
            px(d, (29 + dx * 1.5, 34), 8, "#3d603c", s)
            line(d, 32, 40, 32, 54, PAL["mortal_brown"], s, 2)
        elif shape == "timber":
            for x in (26, 32, 38):
                line(d, x, 52, x + dx * 0.6, 38, PAL["mortal_brown"], s, 3)
            px(d, (32 + dx * 1.6, 34), 9, "#365a38", s)
        frames.append(finish(img))
    return frames


def _terrain_frames(spec, accent):
    frames = []
    for f in range(FRAMES):
        img, d, s = cell(64)
        draw_terrain(d, s, spec)
        accent(d, s, f)
        frames.append(finish(img))
    return frames


def acc_berries(d, s, f):
    n = 5
    pts = [(32 + 10 * math.cos(math.pi + (i + 0.5) * math.pi / n),
            38 + 10 * math.sin(math.pi + (i + 0.5) * math.pi / n))
           for i in range(n)]
    sparkle(d, s, f, pts, every=4)


def acc_stone(d, s, f):
    sparkle(d, s, f, [(26, 38), (38, 40)], every=4)


def acc_fish(d, s, f):
    for k in range(2):
        ph = (f / FRAMES + k * 0.5) % 1.0
        arc(d, (32, 40 + ph * 4), 8 + ph * 6, 200, 340,
            fade("bce1df", int(180 * (1 - ph))), s, 1.2)
    fx = 2.5 * math.sin(f * math.tau / FRAMES)
    line(d, 28 + fx, 38, 32 + fx, 37, "#bce1df", s, 1.4)


def acc_herbs(d, s, f):
    dx = math.sin(f * math.tau / FRAMES) * 1.2
    for i, x in enumerate([26, 32, 38]):
        px(d, (x + (i - 1) * 2 + dx, 35 - i * 2), 1.2, fade("9fdc8a", 190), s)


def acc_spirit_herbs(d, s, f):
    acc_herbs(d, s, f)
    r = 14 + 1.8 * math.sin(f * math.tau / FRAMES)
    arc(d, (32, 42), r, 0, 360, fade("38bdf8", 170), s, 1.0)
    sparkle(d, s, f, [(26, 34), (38, 32)], every=3)


def acc_mushrooms(d, s, f):
    ph = (f / FRAMES) % 1.0
    px(d, (31 + ph * 3, 34 - ph * 10), 1.0, fade("d8d0bc", int(200 * (1 - ph))), s)


def acc_lotus(d, s, f):
    ripple(d, s, f, 32, 38, 6, 13, "4b8289", n=2, w=1.0)
    bob = 0.8 * math.sin(f * math.tau / FRAMES)
    px(d, (32, 30 + bob), 3.4, PAL["lotus"], s)
    px(d, (32, 30 + bob), 1.5, PAL["gilt"], s)


def acc_mist(d, s, f):
    for k in range(2):
        x = 20 + ((f / FRAMES + k * 0.5) % 1.0) * 30
        arc(d, (x, 36 + k * 8), 7, 180, 340, fade("38bdf8", 130), s, 1.2)


def acc_unknown(d, s, f):
    r = 10 + (f / FRAMES) * 8
    arc(d, (32, 36), r, 0, 360,
        fade("38bdf8", int(220 * (1 - f / FRAMES))), s, 1.2)


TERRAIN_ANIM = {"berries": acc_berries, "stone": acc_stone, "fish": acc_fish,
                "herbs": acc_herbs, "spirit-herbs": acc_spirit_herbs,
                "mushrooms": acc_mushrooms, "lotus": acc_lotus,
                "mist": acc_mist, "unknown-site": acc_unknown}
SWAY_TREES = {"pine", "bamboo", "willow", "plum", "tree-generic", "timber"}


# ---- dedicated cells for objects that have no parked glyph -----------------

def compose(parts, size=64):
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    for im, x, y in parts:
        img.paste(im, (int(x - im.width / 2), int(y - im.height / 2)), im)
    return img


def mini(im, w):
    return im.resize((w, w), Image.NEAREST)


def stamp(img, im, x, y, w, s):
    """Paste im resized to `w` local units, centred at local (x, y), on a
    supersampled cell canvas whose scale is s."""
    big = im.resize((int(w * s), int(w * s)), Image.NEAREST)
    img.paste(big, (int((x - w / 2) * s), int((y - w / 2) * s)), big)


def settlement_frames(stage):
    """LOD2/3 cell: market centre + housing ring + stage hallmark."""
    well = site_cell(dict(shape="market"))
    home = site_cell(dict(shape="home"))
    shelter = site_cell(dict(shape="shelter"))
    bank = site_cell(dict(shape="bank"))
    frames = []
    for f in range(FRAMES):
        img, d, s = cell(64)
        px(d, (32, 36), 26, fade("b7ac88", 70), s)
        arc(d, (32, 36), 26, 0, 360, fade("b7ac88", 120), s, 1.0)
        if stage == "camp":
            for (x, y) in [(18, 42), (46, 42)]:
                stamp(img, shelter, x, y, 24, s)
            r = 1.8 + 0.5 * math.sin(f * math.tau / FRAMES * 2)
            px(d, (32, 46), r + 1.2, fade("ff9a5a", 170), s)
            px(d, (32, 46), r, PAL["fire"], s)
        else:
            n = 4 if stage == "village" else 6
            for k in range(n):
                a = math.tau * k / n + 0.5
                x, y = 32 + 20 * math.cos(a), 38 + 12 * math.sin(a)
                stamp(img, home, x, y, 22, s)
                if stage == "town" and f % 4 == k % 4:
                    px(d, (x, y + 5), 1.8, fade("f4c77a", 210), s)
            if stage == "town":
                stamp(img, bank, 32, 55, 20, s)
            stamp(img, well, 32, 31, 26, s)
            ripple(d, s, f, 32, 36, 3, 7, "5d97a6", n=2, w=0.9)
        frames.append(finish(img))
    return frames


def convoy_frames():
    frames = []
    for f in range(FRAMES):
        img, d, s = cell(64)
        bob = math.sin(f * math.tau / FRAMES)
        y = bob
        # canopy + cart bed
        px(d, (32, 26 + y), 9, "#e8dfc8", s)
        rect(d, 20, 30 + y, 44, 40 + y, "#f4d88d", s,
             outline="#443c30", width=1.2)
        # wheels with rotating spokes
        for wx in (26, 38):
            px(d, (wx, 44 + y), 4, "#443c30", s)
            a0 = f * math.tau / FRAMES
            for k in range(2):
                a = a0 + k * math.pi / 2
                line(d, wx - 3.4 * math.cos(a), 44 + y - 3.4 * math.sin(a),
                     wx + 3.4 * math.cos(a), 44 + y + 3.4 * math.sin(a),
                     "#c7b489", s, 0.9)
        # cargo bundles
        for k, cx in enumerate((24, 32, 40)):
            px(d, (cx, 28 + y + (k % 2)), 3, PAL["mortal_hemp"], s)
        frames.append(finish(img))
    return frames


def rally_frames():
    specs = [resolve(dict(MORTALS[0][2], skin=PAL["skin"][0])),
             resolve(dict(MORTALS[4][2], skin=PAL["skin"][1])),
             resolve(dict(MORTALS[9][2], skin=PAL["skin"][2]))]
    frames = []
    for f in range(FRAMES):
        img, d, s = cell(64)
        r = 16 + 2 * math.sin(f * math.tau / FRAMES)
        arc(d, (32, 36), r, 0, 360, "#e4e4be", s, 1.6)
        for i, spec in enumerate(specs):
            x = 18 + i * 14
            stamp(img, person_cell(spec, "walk", f + i * 3), x, 36, 18, s)
        frames.append(finish(img))
    return frames


def road_frames():
    frames = []
    for f in range(FRAMES):
        img, d, s = cell(64)
        line(d, 6, 56, 58, 10, "#a7966d", s, 12)
        line(d, 6, 56, 58, 10, "#c7b489", s, 7)
        for k in range(3):
            t = (f / FRAMES + k / 3) % 1.0
            x = 6 + 52 * t
            y = 56 - 46 * t
            px(d, (x, y), 1.6, fade("6b5a3f", 220), s)
        frames.append(finish(img))
    return frames


def river_frames():
    frames = []
    for f in range(FRAMES):
        img, d, s = cell(64)
        pts = [(6 + k * 8, 34 + math.sin(k * 0.9) * 7) for k in range(7)]
        d.line([(x * s, y * s) for x, y in pts], fill="#5d97a6",
               width=max(1, int(9 * s)))
        for k in range(4):
            t = (f / FRAMES + k / 4) % 1.0
            x = 8 + 48 * t
            y = 34 + math.sin((x - 6) / 8 * 0.9) * 7
            arc(d, (x, y - 2), 3, 200, 340, fade("bce1df", 200), s, 1.0)
        frames.append(finish(img))
    return frames


def coast_frames():
    frames = []
    for f in range(FRAMES):
        img, d, s = cell(64)
        rect(d, 0, 0, 26, 64, "#c7bd91", s)
        rect(d, 26, 0, 64, 64, "#477986", s)
        ph = 0.5 + 0.5 * math.sin(f * math.tau / FRAMES)
        for i in range(3):
            x = 28 + ph * 4
            y = 12 + i * 18
            line(d, x, y, x + 12, y + 2, fade("c2e0d6", 150), s, 1.6)
            arc(d, (x + 4, y + 4), 4, 200, 330, fade("c2e0d6", 110), s, 1.0)
        frames.append(finish(img))
    return frames


def hill_frames():
    frames = []
    for f in range(FRAMES):
        img, d, s = cell(64)
        arc(d, (32, 58), 30, 195, 345, "#838d70", s, 16)
        arc(d, (32, 58), 22, 195, 345, "#a8ad94", s, 11)
        arc(d, (32, 58), 14, 195, 345, "#c5c9b0", s, 6)
        px(d, (32, 40), 3, "#798570", s)
        shx = 10 + (f / FRAMES) * 44
        px(d, (shx, 42), 8, fade("3a4a3a", 40), s)
        frames.append(finish(img))
    return frames


def grove_frames():
    pine = tree_frames("pine")
    gen = tree_frames("tree-generic")
    willow = tree_frames("willow")
    frames = []
    for f in range(FRAMES):
        img, d, s = cell(64)
        px(d, (32, 40), 26, fade("617f53", 160), s)
        stamp(img, gen[f], 16, 42, 26, s)
        stamp(img, pine[f], 34, 34, 28, s)
        stamp(img, willow[f], 50, 44, 26, s)
        frames.append(finish(img))
    return frames


def grass_frames():
    frames = []
    for f in range(FRAMES):
        img, d, s = cell(64)
        dx = math.sin(f * math.tau / FRAMES) * 1.6
        px(d, (32, 44), 12, fade("78a65a", 130), s)
        for i, x in enumerate([22, 28, 34, 40]):
            line(d, x, 50, x + dx * (0.6 + i * 0.25), 38 - (i % 2) * 4,
                 "#78a65a", s, 1.8)
            line(d, x, 50, x - 2 + dx * (0.4 + i * 0.2), 42, "#8fae62", s, 1.2)
        frames.append(finish(img))
    return frames


def fertility_frames():
    frames = []
    for f in range(FRAMES):
        img, d, s = cell(64)
        rect(d, 8, 12, 56, 56, fade("679453", 40), s)
        for i, x in enumerate([16, 26, 36, 46]):
            line(d, x, 52, x, 40 - (i % 2) * 5, fade("7fae62", 170), s, 1.6)
            px(d, (x, 38 - (i % 2) * 5), 1.4, fade("c8c15f", 200), s)
        ph = f / FRAMES
        px(d, (16 + ph * 30, 30 - ph * 8), 1.0, fade("f4eee0", int(160 * (1 - ph))), s)
        frames.append(finish(img))
    return frames


def zone_frames(color):
    frames = []
    for f in range(FRAMES):
        img, d, s = cell(64)
        px(d, (32, 34), 22, fade(color, 40), s)
        arc(d, (32, 34), 22, 0, 360, color, s, 1.6)
        a0 = f * math.tau / FRAMES
        for k in range(2):
            arc(d, (32, 34), 22, math.degrees(a0 + k * math.pi),
                math.degrees(a0 + k * math.pi) + 40, color, s, 2.6)
        frames.append(finish(img))
    return frames


def selection_frames():
    frames = []
    for f in range(FRAMES):
        img, d, s = cell(64)
        r = 16 + 3 * math.sin(f * math.tau / FRAMES)
        arc(d, (32, 34), r, 0, 360, "#ffedaa", s, 1.4)
        arc(d, (32, 34), r * 0.6, 0, 360, fade("ffedaa", 120), s, 0.8)
        frames.append(finish(img))
    return frames


def fog_frames():
    frames = []
    for f in range(FRAMES):
        img, d, s = cell(64)
        for k in range(3):
            x = 14 + ((f / FRAMES + k / 3) % 1.0) * 36
            px(d, (x, 30 + k * 8), 9, (214, 222, 216, 120), s)
        frames.append(finish(img))
    return frames


def trade_frames():
    frames = []
    for f in range(FRAMES):
        img, d, s = cell(64)
        line(d, 8, 52, 56, 14, "#5db4a6", s, 1.6)
        for k in range(3):
            t = (f / FRAMES + k / 3) % 1.0
            px(d, (8 + 48 * t, 52 - 38 * t), 1.8, "#5db4a6", s)
        frames.append(finish(img))
    return frames


def reveal_frames():
    frames = []
    for f in range(4):
        img, d, s = cell(64)
        for k in range(3):
            y = 30 + k * 8
            arc(d, (26 + f * 4 + k * 4, y + math.sin(f * math.tau / 4 + k) * 2),
                10, 180, 340, PAL["spirit_cyan"], s, 1.4)
        arc(d, (32, 32), 10 + f * 4, 0, 360, PAL["spirit_cyan"], s, 1.0)
        frames.append(finish(img))
    return frames


def legend_dot_strip():
    cells = []
    for name, c in ACTION_DOTS:
        im, dd, ss = cell(24, scale=8)
        px(dd, (12, 12), 4, c, ss)
        cells.append(im.resize((24, 24), Image.NEAREST))
    return sheet(cells, len(cells), pad=4, bg=(20, 24, 20, 255), cw=24, ch=24)


# ================================================================== assemble

MANIFEST = []


def row(cat, oid, name, l1, l2, l3, l3_frames, note):
    MANIFEST.append(dict(cat=cat, id=oid, name=name, l1=l1, l2=l2, l3=l3,
                         frames=l3_frames, note=note))


def rel(p):
    return os.path.relpath(p, ROOT).replace("\\", "/")


def emit_dot(oid, img):
    return rel(save(img, "dots", f"{oid}.png"))


def emit_anim(oid, frames):
    p1 = rel(save(strip(frames), "anim", f"{oid}.png"))
    p2 = rel(save_apng(frames, "apng", f"{oid}.png"))
    return p1, p2


def emit_l2(oid, frames):
    return rel(save(frames[0], "l2", f"{oid}.png"))


def parked_l2(family, name):
    return f"parked/{family}/{name}.png"


def person_spec(col, spec):
    spec = dict(spec)
    spec["skin"] = PAL["skin"][col % 4]
    return resolve(spec)


def animal_spec(aid, spec):
    spec = dict(spec)
    spec["palette"] = [PAL.get(c, c) for c in spec["palette"]]
    spec["tier"] = "immortal" if aid.startswith(("spirit", "qilin")) else "mortal"
    return spec


def main():
    site_specs = {sid: (name, resolve(spec)) for sid, name, spec in SITES}
    terr_specs = {tid: (name, spec) for tid, name, spec in TERRAIN}

    # ---------------- settlements ----------------
    for stage, label in [("camp", "Settlement — camp"),
                         ("village", "Settlement — village"),
                         ("town", "Settlement — town")]:
        l1 = emit_dot(f"settlement-{stage}",
                      _d({"camp": dot_camp, "village": dot_village,
                          "town": dot_town}[stage]))
        frames = settlement_frames(stage)
        emit_anim(f"settlement-{stage}", frames)
        l2 = emit_l2(f"settlement-{stage}", frames)
        row("settlements", f"settlement-{stage}", label, l1, l2,
            f"parked/lod/apng/settlement-{stage}.png", FRAMES,
            "LOD1 shape is the live field.gd marker; market well + housing ring drawn at LOD2+")
    # market well as its own object
    l1 = emit_dot("market-well", _d(dot_well))
    frames = _site_frames(site_specs["market"][1], acc_market)
    emit_anim("market", frames)
    row("settlements", "market-well", "Market centre well", l1,
        parked_l2("site-glyphs-xianxia", "market"),
        "parked/lod/apng/market.png", FRAMES,
        "Well ripple + bucket bob at LOD3")

    # ---------------- buildings & sites ----------------
    l2_overrides = {"mine-silver": site_cell(dict(shape="mine", ore=PAL["ore"][1])),
                    "mine-copper": site_cell(dict(shape="mine", ore=PAL["ore"][2]))}
    for oid, img in l2_overrides.items():
        save(img, "l2", f"{oid}.png")
    building_dots = {
        "home": dot_home, "shop": dot_shop(PAL["gilt"]),
        "workshop": dot_workshop, "bank": dot_bank, "tavern": dot_tavern,
        "school": dot_school, "farm": dot_farm, "graveyard": dot_graveyard,
        "shelter": dot_shelter, "jobboard": dot_jobboard,
        "gate": dot_gate, "pavilion": dot_pavilion,
        "shop-clothes": dot_shop(PAL["mortal_indigo"]),
        "shop-tools": dot_shop(PAL["gilt"]),
        "shop-furniture": dot_shop("#9a7c58"),
        "shop-butcher": dot_shop("#b04a3e"),
    }
    for wid in ["workshop-hunter", "workshop-cobbler", "workshop-blacksmith",
                "workshop-carpenter"]:
        building_dots[wid] = dot_workshop
    for mid, ore in [("mine-gold", PAL["ore"][0]), ("mine-silver", PAL["ore"][1]),
                     ("mine-copper", PAL["ore"][2]), ("mine-iron", PAL["ore"][3])]:
        building_dots[mid] = dot_mine(ore)

    site_rows = [
        ("home", "Home", "Housing blocks + kind-0 urban buildings"),
        ("shop-clothes", "Shop — clothier", "kind 1, line 0"),
        ("shop-tools", "Shop — tools", "kind 1, line 1"),
        ("shop-furniture", "Shop — furniture", "kind 1, line 2"),
        ("shop-butcher", "Shop — butcher", "kind 1, line 3"),
        ("workshop-hunter", "Workshop — hunter", "kind 2, line 0"),
        ("workshop-cobbler", "Workshop — cobbler", "kind 2, line 1"),
        ("workshop-blacksmith", "Workshop — blacksmith", "kind 2, line 2"),
        ("workshop-carpenter", "Workshop — carpenter", "kind 2, line 3"),
        ("bank", "Bank — pagoda", "town bank, rate label"),
        ("tavern", "Tavern", "visitors/32 + lantern landmark"),
        ("school", "School", "paifang gate landmark"),
        ("farm", "Rice farm", "mature vs growing paddy"),
        ("graveyard", "Graveyard", "buried_count/capacity steles"),
        ("shelter", "Roadside camp", "bed occupancy ring"),
        ("jobboard", "Job board", "vacancies + nearby bodies dot"),
        ("mine-gold", "Gold mine", "patch kind 4"),
        ("mine-silver", "Silver mine", "patch kind 5 (reuses mine glyph today)"),
        ("mine-copper", "Copper mine", "patch kind 6 (reuses mine glyph today)"),
        ("mine-iron", "Iron mine", "patch kind 7"),
        ("gate", "Mountain gate", "immortal site — dormant"),
        ("pavilion", "Pavilion", "immortal site — dormant"),
    ]
    for oid, name, note in site_rows:
        family_id = oid if oid in site_specs else (
            "mine-iron" if oid.startswith("mine-") else oid)
        spec = site_specs.get(family_id, site_specs.get("mine-iron"))[1]
        if oid in ("mine-silver", "mine-copper"):
            spec = dict(spec, ore=PAL["ore"][1 if oid == "mine-silver" else 2])
        acc = acc_mine if oid.startswith("mine-") else SITE_ANIM.get(
            site_specs.get(oid, ("", dict(shape="")))[1].get("shape"),
            acc_home)
        l1 = emit_dot(oid, _d(building_dots[oid]))
        frames = _site_frames(spec, acc)
        emit_anim(oid, frames)
        if oid in l2_overrides:
            l2 = f"parked/lod/l2/{oid}.png"
        else:
            l2 = parked_l2("site-glyphs-xianxia", oid if oid in
                           {s for s, _, _ in SITES} else family_id)
        row("buildings", oid, name, l1, l2,
            f"parked/lod/apng/{oid}.png", FRAMES, note)

    # ---------------- resource patches ----------------
    patch_rows = [
        ("patch-berries", "Berries", 0, "berries", "dots scale with stock"),
        ("patch-timber", "Timber stand", 1, "timber", "larger crown radius"),
        ("patch-stone", "Stone outcrop", 2, "stone", ""),
        ("patch-fish", "Fish shoal", 3, "fish", "coastal food source"),
        ("patch-gold", "Gold ore", 4, "mine-gold", "mine diamond at LOD1"),
        ("patch-silver", "Silver ore", 5, "l2/mine-silver", ""),
        ("patch-copper", "Copper ore", 6, "l2/mine-copper", ""),
        ("patch-iron", "Iron ore", 7, "mine-iron", ""),
        ("patch-herbs", "Herb patch", -1, "herbs", "dormant — future kind"),
        ("patch-spirit-herbs", "Spirit herb", -1, "spirit-herbs", "dormant — rare qi herb"),
        ("patch-mushrooms", "Mushroom ring", -1, "mushrooms", "dormant — future kind"),
    ]
    for oid, name, kind, glyph, note in patch_rows:
        if 4 <= kind <= 7:
            l1 = emit_dot(oid, _d(dot_mine(PATCH_COLORS[kind])))
        elif oid == "patch-mushrooms":
            l1 = emit_dot(oid, _d(dot_mushrooms))
        elif oid in ("patch-herbs", "patch-spirit-herbs"):
            l1 = emit_dot(oid, _d(dot_herbs(oid == "patch-spirit-herbs")))
        else:
            r = 9.0 if kind == 1 else 5.0
            l1 = emit_dot(oid, _d(dot_patch(PATCH_COLORS[max(0, kind)], r * 0.55)))
        if glyph.startswith("l2/"):
            l2 = f"parked/lod/{glyph}.png"
            spec = dict(shape="mine", ore=PATCH_COLORS[kind])
            frames = _site_frames(spec, acc_mine)
        else:
            l2 = parked_l2("terrain-flora-xianxia" if glyph in terr_specs
                           else "site-glyphs-xianxia", glyph)
            if glyph in terr_specs:
                acc = TERRAIN_ANIM.get(glyph)
                frames = (_terrain_frames(terr_specs[glyph][1], acc)
                          if acc else tree_frames(glyph))
            else:
                spec = site_specs[glyph][1]
                frames = _site_frames(spec, acc_mine)
        emit_anim(oid, frames)
        row("patches", oid, name, l1, l2, f"parked/lod/apng/{oid}.png",
            FRAMES, note or "resource patch")

    # ---------------- flora & terrain features ----------------
    flora_rows = [
        ("pine", "Pine", "cool groves"), ("bamboo", "Bamboo", "coastal groves"),
        ("willow", "Willow", "waterside groves"), ("plum", "Plum", "~1 in 7 accent"),
        ("tree-generic", "Generic tree", "default grove tree"),
    ]
    for oid, name, note in flora_rows:
        l1 = emit_dot(oid, _d(dot_tree))
        frames = tree_frames(oid)
        emit_anim(oid, frames)
        row("terrain", oid, name, l1,
            parked_l2("terrain-flora-xianxia", oid),
            f"parked/lod/apng/{oid}.png", FRAMES, note)
    for oid, name, frames_fn, l1img, note in [
            ("forest-grove", "Forest grove", grove_frames, _d(dot_tree),
             "canopy layer + floor, every-3rd tree at LOD1"),
            ("grass", "Grass patch", grass_frames, _d(dot_grass),
             "grazing regrowth for wildlife"),
            ("fertility", "Fertility region", fertility_frames, _d(dot_fertility),
             "farm placement tint"),
            ("hill", "Hill", hill_frames, _d(dot_hill),
             "classic terrain landmark"),
            ("coast", "Coast / water", coast_frames, _d(dot_coast),
             "shore + waterline waves"),
            ("river", "River", river_frames, _d(dot_river),
             "generated-map watercourse"),
            ("road", "Road (built)", road_frames, _d(dot_road),
             "polyline corridor; dashed when unbuilt"),
            ("lotus", "Lotus cluster", None, _d(dot_patch("#5a8a5e", 4.4)),
             "coast-edge accent flora"),
            ("mist", "Qi mist", None, _d(dot_mist),
             "immortal-site ambience"),
            ("unknown-site", "Undiscovered site", None, _d(dot_unknown),
             "procedural-expansion marker"),
            ("fog", "Unexplored mist", fog_frames, _d(dot_fog),
             "explored-chunk fog layer"),
    ]:
        l1 = emit_dot(oid, l1img)
        if frames_fn is None:
            spec = terr_specs[oid][1]
            frames = _terrain_frames(spec, TERRAIN_ANIM[oid])
            l2 = parked_l2("terrain-flora-xianxia", oid)
        else:
            frames = frames_fn()
            l2 = emit_l2(oid, frames)
        emit_anim(oid, frames)
        row("terrain", oid, name, l1, l2, f"parked/lod/apng/{oid}.png",
            FRAMES, note)

    # ---------------- people ----------------
    save(legend_dot_strip(), "l2", "action-dots.png")
    row("people", "action-dots", "Villager action colors",
        "parked/lod/dots/person.png", "parked/lod/l2/action-dots.png",
        "parked/lod/l2/action-dots.png", 0,
        "LOD1 person dot is colored by current action: " +
        ", ".join(n for n, _ in ACTION_DOTS))
    emit_dot("person", _d(dot_person("#e5c985")))
    for col, (fid, name, spec) in enumerate(MORTALS + IMMORTALS):
        rspec = person_spec(col, spec)
        l1 = "parked/lod/dots/person.png"
        frames = [person_cell(rspec, "walk", f) for f in range(FRAMES)]
        emit_anim(fid + "-walk", frames)
        wframes = [person_cell(rspec, "work", f) for f in range(FRAMES)]
        save(strip(wframes), "anim", f"{fid}-work.png")
        dormant = " (dormant)" if col >= 12 else ""
        row("people", fid, name + dormant, l1,
            f"parked/villager-atlas-v2/stances/{fid}.png",
            f"parked/lod/apng/{fid}-walk.png", FRAMES,
            f"atlas col {col}; work cycle strip also emitted")

    # ---------------- wildlife ----------------
    for col, (aid, name, spec) in enumerate(ANIMALS):
        rspec = animal_spec(aid, spec)
        color, r = ANIMAL_DOT[aid]
        l1 = emit_dot(aid, _d(dot_animal(color, r)))
        frames = [animal_cell(rspec, "walk", f) for f in range(4)]
        emit_anim(aid, frames)
        dormant = " (dormant)" if aid.startswith(("spirit", "qilin", "boar",
                                                 "pheasant", "fox", "crane")) else ""
        row("wildlife", aid, name + dormant, l1,
            f"parked/animal-table-xianxia/{aid}.png",
            f"parked/lod/apng/{aid}.png", 4,
            "atlas col %d; graze/flee/dead rows exist in atlas" % col)

    # ---------------- overlays & fx ----------------
    overlay_rows = [
        ("convoy", "Trade convoy", convoy_frames, _d(dot_convoy),
         "cart + member count; LOD1-only draw today"),
        ("rally", "Convoy rally point", rally_frames, _d(dot_rally),
         "pre-departure gathering marker"),
        ("selection", "Selection ring", selection_frames, _d(dot_select),
         "person/animal/site focus ring"),
        ("zone-residential", "Zone — residential", None, _d(dot_zone(ZONE_COLORS["residential"])), "show_zones overlay"),
        ("zone-commercial", "Zone — commercial", None, _d(dot_zone(ZONE_COLORS["commercial"])), "show_zones overlay"),
        ("zone-industrial", "Zone — industrial", None, _d(dot_zone(ZONE_COLORS["industrial"])), "show_zones overlay"),
        ("trade-route", "Trade route", trade_frames, _d(dot_trade),
         "cumulative road cargo overlay"),
        ("reveal", "Discovery reveal", reveal_frames, _d(dot_reveal),
         "existing 4-frame fx contract"),
    ]
    for oid, name, frames_fn, l1img, note in overlay_rows:
        l1 = emit_dot(oid, l1img)
        if oid.startswith("zone-"):
            frames = zone_frames(ZONE_COLORS[oid.split("-", 1)[1]])
        else:
            frames = frames_fn()
        emit_anim(oid, frames)
        l2 = (parked_l2("discovery-fx", "reveal") if oid == "reveal"
              else emit_l2(oid, frames))
        row("overlays", oid, name, l1, l2, f"parked/lod/apng/{oid}.png",
            len(frames), note)

    # ---------------- the same street, three levels ----------------
    build_scene()
    row("scene", "scene", "Street scene composite", "parked/lod/scene-l1.png",
        "parked/lod/scene-l2.png", "parked/lod/scene-l3.png", FRAMES,
        "one market street at all three levels")

    with open(os.path.join(LOD, "manifest.json"), "w") as f:
        json.dump(MANIFEST, f, indent=1)
    print(f"lod assets written: {len(MANIFEST)} board rows")


def build_scene():
    """Composite street scene: identical layout at LOD1, LOD2, LOD3."""
    W, H = 384, 216

    road_pts = [(12, 190), (130, 130), (250, 96), (372, 60)]

    sites = [("home", (150, 70)), ("shop-tools", (104, 112)),
             ("workshop-blacksmith", (292, 88)), ("tavern", (316, 140)),
             ("farm", (56, 150)), ("school", (96, 56)),
             ("market", (204, 120))]
    trees = [("pine", (26, 52)), ("tree-generic", (60, 34)),
             ("willow", (348, 40)), ("plum", (362, 84)),
             ("tree-generic", (18, 116)), ("bamboo", (336, 182))]
    patches = [("berries", (330, 170)), ("stone", (360, 196)),
               ("patch-iron", (44, 192))]

    walkers = [  # (figure col, path t offset, scale)
        (0, 0.05, 30), (4, 0.30, 30), (8, 0.55, 30), (1, 0.72, 30),
        (11, 0.88, 30), (6, 0.45, 30)]
    animals = [("deer", (350, 118), 0), ("rabbit", (24, 148), 2)]

    def road_at(t):
        t = t % 1.0
        seg = t * (len(road_pts) - 1)
        i = min(int(seg), len(road_pts) - 2)
        u = seg - i
        return (road_pts[i][0] + (road_pts[i + 1][0] - road_pts[i][0]) * u,
                road_pts[i][1] + (road_pts[i + 1][1] - road_pts[i][1]) * u)

    def draw_ground(d):
        d.rectangle([0, 0, W, H], fill="#6d7f56")
        d.rectangle([0, 0, W, 26], fill="#477986")      # coast edge top
        d.rectangle([0, 26, W, 30], fill="#c7bd91")
        for k in range(len(road_pts) - 1):
            d.line([road_pts[k], road_pts[k + 1]], fill="#a7966d", width=12)
            d.line([road_pts[k], road_pts[k + 1]], fill="#c7b489", width=7)
        px(d, (204, 124), 46, fade("b7ac88", 110), 1)   # village ground

    def cell_of(path):
        return Image.open(os.path.join(ROOT, "parked", *path.split("/")))

    # ---------- LOD1 ----------
    img, d, s = Image.new("RGBA", (W, H)), None, 1
    d = ImageDraw.Draw(img)
    draw_ground(d)
    for sid, (x, y) in sites:
        mark = {"home": "home", "shop-tools": "shop-tools",
                "workshop-blacksmith": "workshop-blacksmith",
                "tavern": "tavern", "farm": "farm", "school": "school",
                "market": "market-well"}[sid]
        if mark == "market-well":
            im = _d(dot_village)
        else:
            im = Image.open(os.path.join(LOD, "dots", f"{mark}.png"))
        img.paste(im.resize((14, 14), Image.NEAREST), (x - 7, y - 7), im.resize((14, 14), Image.NEAREST))
    for tid, (x, y) in trees:
        px(d, (x, y), 4, "#365c43", 1)
    for pid, (x, y) in patches:
        c = PATCH_COLORS[{"berries": 0, "stone": 2, "patch-iron": 7}[pid]]
        px(d, (x, y), 3, c, 1)
    for col, t, w in walkers:
        x, y = road_at(t)
        px(d, (x, y), 3, ACTION_DOTS[col % 6][1], 1)
    for aid, (x, y), _ in animals:
        px(d, (x, y), 2.4, ANIMAL_DOT[aid][0], 1)
    save(img, "scene-l1.png")

    # ---------- LOD2 ----------
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    draw_ground(d)
    glyph = lambda fid: Image.open(os.path.join(
        ROOT, "parked",
        ("site-glyphs-xianxia" if fid in {s for s, _, _ in SITES}
         else "terrain-flora-xianxia") + "/" + fid + ".png"))
    for tid, (x, y) in trees:
        im = glyph(tid).resize((30, 30), Image.NEAREST)
        img.paste(im, (x - 15, y - 15), im)
    for sid, (x, y) in sites:
        im = glyph(sid).resize((36, 36), Image.NEAREST)
        img.paste(im, (x - 18, y - 18), im)
    for pid, (x, y) in patches:
        im = glyph({"patch-iron": "mine-iron"}.get(pid, pid)).resize(
            (26, 26), Image.NEAREST)
        img.paste(im, (x - 13, y - 13), im)
    for col, t, w in walkers:
        x, y = road_at(t)
        spec = person_spec(col, (MORTALS + IMMORTALS)[col][2])
        im = person_cell(spec, "walk", 0).resize((26, 26), Image.NEAREST)
        img.paste(im, (int(x) - 13, int(y) - 13), im)
    for aid, (x, y), fr in animals:
        spec = animal_spec(aid, dict(ANIMALS[[a[0] for a in ANIMALS].index(aid)][2]))
        im = animal_cell(spec, "idle").resize((26, 26), Image.NEAREST)
        img.paste(im, (x - 13, y - 13), im)
    save(img, "scene-l2.png")

    # ---------- LOD3 (animated) ----------
    figs = MORTALS + IMMORTALS
    frames = []
    for f in range(FRAMES):
        img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        d = ImageDraw.Draw(img)
        draw_ground(d)
        # swayed trees
        for tid, (x, y) in trees:
            im = tree_frames(tid if tid != "tree-generic" else "tree-generic")[f]
            im = im.resize((30, 30), Image.NEAREST)
            img.paste(im, (x - 15, y - 15), im)
        # animated sites
        for sid, (x, y) in sites:
            spec = site_specs_scene(sid)
            acc = {"home": acc_home, "shop-tools": acc_shop,
                   "workshop-blacksmith": acc_workshop, "tavern": acc_tavern,
                   "farm": acc_farm, "school": acc_school,
                   "market": acc_market}[sid]
            cimg, cd, cs = cell(64)
            draw_site(cd, cs, spec)
            acc(cd, cs, f, spec)
            im = finish(cimg).resize((36, 36), Image.NEAREST)
            img.paste(im, (x - 18, y - 18), im)
        for pid, (x, y) in patches:
            if pid == "patch-iron":
                cimg, cd, cs = cell(64)
                draw_site(cd, cs, dict(shape="mine", ore=PAL["ore"][3]))
                acc_mine(cd, cs, f, dict(shape="mine", ore=PAL["ore"][3]))
                im = finish(cimg)
            else:
                spec = dict(TERRAIN[[t[0] for t in TERRAIN].index(pid)][2])
                cimg, cd, cs = cell(64)
                draw_terrain(cd, cs, spec)
                TERRAIN_ANIM.get(pid, lambda *a: None)(cd, cs, f)
                im = finish(cimg)
            im = im.resize((26, 26), Image.NEAREST)
            img.paste(im, (x - 13, y - 13), im)
        # walking villagers progress along the road
        for col, t0, w in walkers:
            x, y = road_at(t0 + f * 0.06)
            spec = person_spec(col, figs[col][2])
            im = person_cell(spec, "walk", f).resize((26, 26), Image.NEAREST)
            img.paste(im, (int(x) - 13, int(y) - 13), im)
        for aid, (x, y), ph in animals:
            spec = animal_spec(aid, dict(ANIMALS[[a[0] for a in ANIMALS].index(aid)][2]))
            stance = "walk" if aid == "deer" else "idle"
            im = animal_cell(spec, stance, (f + ph) % 4).resize(
                (26, 26), Image.NEAREST)
            img.paste(im, (x - 13, y - 13), im)
        frames.append(img)
    save_apng(frames, "scene-l3.png")
    save(strip([f.crop((120, 30, 264, 174)) for f in frames]), "anim", "scene-l3.png")


def site_specs_scene(sid):
    specs = {s: resolve(sp) for s, _, sp in SITES}
    return specs[sid]


if __name__ == "__main__":
    main()
