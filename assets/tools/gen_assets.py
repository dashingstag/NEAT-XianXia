#!/usr/bin/env python3
"""NEAT Xianxia parked-asset generator.

Renders the Xianxia sprite set to PNG with PIL. The script is the procedural
source of truth for the parked textures; every figure is parameterised so
runtime-generated variants can reuse the same grammar later.

Layout produced under assets/parked/<family>/:
  <id>.png            individual sprite / stance sheet
  spec.md             integration note (written by hand, not generated)
  sheet.png           contact sheet used in the catalog doc
"""
import math
import os
from PIL import Image, ImageDraw

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PARKED = os.path.join(ROOT, "parked")

# ---------------------------------------------------------------- palette
PAL = {
    "mortal_earth":    "#b08d57",
    "mortal_indigo":   "#3d5a73",
    "mortal_greygreen":"#7d8b74",
    "mortal_hemp":     "#c9b896",
    "mortal_brown":    "#7a5c43",
    "immortal_white":  "#f4f6f0",
    "immortal_jade":   "#8fc1a9",
    "spirit_cyan":     "#38bdf8",
    "cinnabar":        "#e63946",
    "gilt":            "#d4a94e",
    "bamboo":          "#4d7c57",
    "pine":            "#3d5c40",
    "plum":            "#c25e7c",
    "lotus":           "#e8b4c8",
    "timber_wall":     "#e6dcc4",
    "roof_tile":       "#5a6166",
    "skin":            ["#cf9c74", "#dcb58c", "#a97855", "#edc5a0"],
    "hair":            "#45352c",
    "dark":            "#3d3832",
    "belt":            "#524538",
    "metal":           "#c0c4b6",
    "wood":            "#7a5535",
    "dead":            "#77776d",
    "shadow":          (18, 28, 18, 60),
    "stone":           "#8d949b",
    "stone2":          "#aab7b0",
    "water":           "#4b8289",
    "sand":            "#c7bf89",
    "field":           "#8b9f6b",
    "ore": ["#edc65d", "#c8d6df", "#ba7650", "#8d949b"],
    "fire":            "#ffda82",
    "label":           "#f7eed0",
}

S = 4  # supersample factor: draw at 64*S, downscale NEAREST for crisp pixels


def cell(size=64, scale=S):
    img = Image.new("RGBA", (size * scale, size * scale), (0, 0, 0, 0))
    return img, ImageDraw.Draw(img), scale


def finish(img, size=64):
    return img.resize((size, size), Image.NEAREST)


def px(d, xy, r, fill, scale, outline=None, width=1):
    x, y = xy
    d.ellipse([(x - r) * scale, (y - r) * scale, (x + r) * scale,
               (y + r) * scale], fill=fill, outline=outline,
              width=max(1, int(width * scale)) if outline else 1)


def rect(d, x0, y0, x1, y1, fill, scale, outline=None, width=1):
    d.rectangle([x0 * scale, y0 * scale, x1 * scale, y1 * scale], fill=fill,
                outline=outline, width=max(1, int(width * scale)) if outline else 1)


def line(d, x0, y0, x1, y1, fill, scale, w=1.0):
    d.line([x0 * scale, y0 * scale, x1 * scale, y1 * scale], fill=fill,
           width=max(1, int(w * scale)))


def arc(d, xy, r, a0, a1, fill, scale, w=1.0):
    x, y = xy
    d.arc([(x - r) * scale, (y - r) * scale, (x + r) * scale,
           (y + r) * scale], a0, a1, fill=fill, width=max(1, int(w * scale)))


def poly(d, pts, fill, scale, outline=None, width=1.0):
    d.polygon([(x * scale, y * scale) for x, y in pts], fill=fill)
    if outline:
        ring = [(x * scale, y * scale) for x, y in pts]
        d.line(ring + [ring[0]], fill=outline,
               width=max(1, int(width * scale)), joint="curve")


# ------------------------------------------------------------- human figure
# local coords inside a 64x64 cell; feet rest near y=52, head near y=14.

def draw_person(d, s, spec, pose, frame=0):
    """spec: dict with keys robe, robe2, skin, hat, prop, wide_sleeves,
    long_hem, beard, topknot, aura. pose: 'idle','walk','work','dead',
    'carry','lie'. frame: animation phase 0..7."""
    skin = spec["skin"]
    robe = spec["robe"]
    robe2 = spec.get("robe2", spec["belt"] if "belt" in spec else PAL["belt"])
    cx = 32.0
    if pose == "dead":
        # X marker body
        line(d, cx - 6, 30, cx + 6, 30, PAL["dead"], s, 3)
        line(d, cx - 4, 24, cx + 4, 36, PAL["dead"], s, 3)
        return
    swing = math.sin(frame * math.tau / 8.0)
    leg_a = swing * 5 if pose == "walk" else (swing * 2 if pose == "work" else 0)
    arm_a = -swing * 4 if pose == "walk" else (swing * 5 if pose == "work" else 0)

    top = 20.0            # shoulder line
    hip = 38.0            # hem of short robe
    hem = hip + (12 if spec.get("long_hem") else 0)
    head_y = 12.0

    # shadow
    px(d, (cx + 2, 54), 10, PAL["shadow"], s)

    if pose == "lie":
        pass  # drawn rotated by caller

    # legs
    legw = 2.4
    if spec.get("long_hem"):
        line(d, cx - 2, hem - 2, cx - 2 - leg_a * 0.4, 52, PAL["dark"], s, legw)
        line(d, cx + 2, hem - 2, cx + 2 + leg_a * 0.4, 52, PAL["dark"], s, legw)
    else:
        line(d, cx - 3, hip - 2, cx - 3 - leg_a, 52, PAL["dark"], s, legw)
        line(d, cx + 3, hip - 2, cx + 3 + leg_a, 52, PAL["dark"], s, legw)

    # robe body: rect for mortal, trapezoid for immortal
    if spec.get("long_hem"):
        poly(d, [(cx - 7, top), (cx + 7, top), (cx + 10, hem), (cx - 10, hem)],
             robe, s)
    else:
        rect(d, cx - 7, top, cx + 7, hip, robe, s)
    # sash
    rect(d, cx - 7, top + 8, cx + 7, top + 10, robe2, s)

    # arms / sleeves
    if spec.get("wide_sleeves"):
        poly(d, [(cx - 7, top + 2), (cx - 7, top + 10),
                 (cx - 13 - arm_a, top + 16), (cx - 10 - arm_a, top + 8)],
             robe, s)
        poly(d, [(cx + 7, top + 2), (cx + 7, top + 10),
                 (cx + 13 + arm_a, top + 16), (cx + 10 + arm_a, top + 8)],
             robe, s)
        px(d, (cx - 13 - arm_a, top + 17), 1.8, skin, s)
        px(d, (cx + 13 + arm_a, top + 17), 1.8, skin, s)
    else:
        line(d, cx - 7, top + 4, cx - 10 - arm_a, top + 14 - arm_a, skin, s, 2.2)
        line(d, cx + 7, top + 4, cx + 10 + arm_a, top + 14 + arm_a, skin, s, 2.2)

    # head
    px(d, (cx, head_y), 6.5, skin, s)
    # hair base
    arc(d, (cx, head_y - 1), 5.8, 180, 360, spec.get("hair", PAL["hair"]), s, 3)

    # headgear / features
    hat = spec.get("hat")
    if hat == "straw":            # wide flat farmer hat
        line(d, cx - 11, head_y - 4, cx + 11, head_y - 4, PAL["mortal_hemp"], s, 2.5)
        poly(d, [(cx - 5, head_y - 4), (cx + 5, head_y - 4), (cx, head_y - 9)],
             PAL["mortal_hemp"], s)
    elif hat == "douli":          # conical bamboo hat
        poly(d, [(cx - 9, head_y - 3), (cx + 9, head_y - 3), (cx, head_y - 12)],
             "#b9a06a", s)
        line(d, cx, head_y - 12, cx, head_y - 3, "#8a744a", s, 0.8)
    elif hat == "wrap":           # headwrap knot
        arc(d, (cx, head_y - 1), 5.8, 180, 360, PAL["mortal_indigo"], s, 4)
        px(d, (cx + 5, head_y - 6), 1.6, PAL["mortal_indigo"], s)
    elif hat == "cap":
        arc(d, (cx, head_y - 1), 6.0, 180, 360, PAL["mortal_brown"], s, 4)
    elif hat == "futou":          # square merchant hat
        rect(d, cx - 4, head_y - 10, cx + 4, head_y - 3, "#2e2a26", s)
        line(d, cx - 4, head_y - 6, cx - 7, head_y - 7, "#2e2a26", s, 1.5)
        line(d, cx + 4, head_y - 6, cx + 7, head_y - 7, "#2e2a26", s, 1.5)
    elif hat == "reed":           # rain cape fringe
        poly(d, [(cx - 9, head_y - 2), (cx + 9, head_y - 2), (cx + 6, head_y - 8),
                 (cx - 6, head_y - 8)], "#9aa06b", s)
        for i in range(-3, 4):
            line(d, cx + i * 2, head_y - 2, cx + i * 2, head_y + 1, "#7d8352", s, 0.7)
    elif hat == "bun":            # elder / lady hair bun
        arc(d, (cx, head_y - 1), 5.8, 180, 360, "#6a6a66", s, 3)
        px(d, (cx, head_y - 7), 2.2, "#6a6a66", s)
    elif hat == "topknot":
        px(d, (cx, head_y - 8), 2.4, spec.get("hair", PAL["hair"]), s)
        line(d, cx - 3, head_y - 9, cx + 3, head_y - 9, PAL["gilt"], s, 1.2)  # hairpin
    elif hat == "guan":           # crown for high tier
        rect(d, cx - 3, head_y - 11, cx + 3, head_y - 5, "#3a352f", s)
        line(d, cx - 5, head_y - 9, cx + 5, head_y - 9, PAL["gilt"], s, 1.2)

    if spec.get("beard"):
        poly(d, [(cx - 2, head_y + 5), (cx + 2, head_y + 5), (cx, head_y + 15)],
             "#d8d4ca", s)

    # face hint (eye)
    px(d, (cx + 2.4, head_y - 0.5), 0.8, PAL["dark"], s)

    # aura ring for spirit figures
    if spec.get("aura"):
        arc(d, (cx, 32), 18, 0, 360, PAL["spirit_cyan"], s, 1.2)

    # prop
    prop = spec.get("prop")
    if prop == "hoe" and pose in ("work", "idle", "walk"):
        line(d, cx + 11, top + 6 + arm_a, cx + 15, top - 8 + arm_a, PAL["wood"], s, 1.6)
        line(d, cx + 15, top - 8 + arm_a, cx + 19, top - 6 + arm_a, PAL["metal"], s, 1.8)
    elif prop == "pole":
        line(d, cx - 14, top + 3, cx + 14, top + 3, PAL["wood"], s, 1.6)
        px(d, (cx - 14, top + 8), 3, PAL["mortal_hemp"], s)
        px(d, (cx + 14, top + 8), 3, PAL["mortal_hemp"], s)
    elif prop == "basket":
        rect(d, cx + 10, top + 12, cx + 15, top + 17, PAL["mortal_hemp"], s,
             outline=PAL["mortal_brown"])
        line(d, cx + 10, top + 12, cx + 15, top + 12, PAL["mortal_brown"], s, 1)
    elif prop == "hammer":
        line(d, cx + 10 + arm_a, top + 14 + arm_a, cx + 13 + arm_a, top + 9 + arm_a,
             PAL["wood"], s, 1.6)
        rect(d, cx + 12 + arm_a, top + 7 + arm_a, cx + 17 + arm_a, top + 11 + arm_a,
             PAL["metal"], s)
    elif prop == "tongs":
        line(d, cx + 10 + arm_a, top + 14 + arm_a, cx + 15 + arm_a, top + 10 + arm_a,
             PAL["metal"], s, 1)
        line(d, cx + 10 + arm_a, top + 15 + arm_a, cx + 15 + arm_a, top + 12 + arm_a,
             PAL["metal"], s, 1)
        px(d, (cx + 16, top + 11 + arm_a), 1.5, PAL["fire"], s)
    elif prop == "staff":
        line(d, cx + 11, top - 4, cx + 11, 52, PAL["wood"], s, 1.8)
        px(d, (cx + 11, top - 5), 1.8, PAL["mortal_brown"], s)
    elif prop == "dragonstaff":
        line(d, cx + 12, top - 10, cx + 12, 52, "#6a5236", s, 2)
        px(d, (cx + 12, top - 12), 2.4, PAL["gilt"], s)
        line(d, cx + 10, top - 14, cx + 8, top - 16, PAL["gilt"], s, 1.2)
    elif prop == "rod":
        line(d, cx + 10 + arm_a, top + 14 + arm_a, cx + 18 + arm_a, top - 12 + arm_a,
             PAL["wood"], s, 1.2)
        line(d, cx + 18 + arm_a, top - 12 + arm_a, cx + 18 + arm_a, top - 2 + arm_a,
             "#c8c4b6", s, 0.7)
    elif prop == "sword":
        line(d, cx - 6, top - 4, cx + 8, top + 14, PAL["dark"], s, 2.2)  # scabbard on back
        line(d, cx - 8, top - 6, cx - 4, top - 2, PAL["gilt"], s, 1.6)  # hilt
    elif prop == "whisk":
        line(d, cx + 11 + arm_a, top + 16 + arm_a, cx + 15 + arm_a, top + 8 + arm_a,
             PAL["wood"], s, 1.4)
        for i in range(3):
            line(d, cx + 15 + arm_a, top + 8 + arm_a,
                 cx + 17 + i + arm_a, top + 4 + i + arm_a, "#f0ede4", s, 0.9)
    elif prop == "lantern":
        line(d, cx + 11, top + 4, cx + 11, top + 10, PAL["wood"], s, 1.2)
        px(d, (cx + 11, top + 13), 3.2, PAL["lotus"], s)
        px(d, (cx + 11, top + 13), 1.4, PAL["fire"], s)
    elif prop == "sickle":
        line(d, cx + 10 + arm_a, top + 14 + arm_a, cx + 13 + arm_a, top + 8 + arm_a,
             PAL["wood"], s, 1.6)
        arc(d, (cx + 15 + arm_a, top + 7 + arm_a), 3, 300, 90, PAL["metal"], s, 1.6)
    elif prop == "book":
        rect(d, cx + 9, top + 10, cx + 15, top + 15, "#e8dfc8", s,
             outline=PAL["mortal_brown"])
        line(d, cx + 12, top + 10, cx + 12, top + 15, PAL["mortal_brown"], s, 0.8)
    elif prop == "scroll":
        rect(d, cx + 10 + arm_a, top + 12 + arm_a, cx + 16 + arm_a, top + 15 + arm_a,
             "#e8dfc8", s, outline=PAL["mortal_brown"])
    elif prop == "abacus":
        rect(d, cx + 9, top + 11, cx + 15, top + 16, "#8a6a44", s,
             outline=PAL["dark"])
        for i in range(3):
            px(d, (cx + 10.5 + i * 2, top + 13.5), 0.8, PAL["label"], s)

    # carry pose: bundle overhead
    if pose == "carry":
        px(d, (cx, top - 8), 5, "#6b5236", s)
        line(d, cx - 8, top + 4, cx - 4, top - 4, skin, s, 2.2)
        line(d, cx + 8, top + 4, cx + 4, top - 4, skin, s, 2.2)


def person_cell(spec, pose, frame=0, size=64):
    img, d, s = cell(size)
    draw_person(d, s, spec, pose, frame)
    if pose == "lie":
        base = img.copy().rotate(-90, expand=False, resample=Image.NEAREST)
        img = base
    return finish(img, size)


# ------------------------------------------------------------------ animals
def draw_animal(d, s, spec, stance, frame=0):
    b = spec.get("body_r", 9)
    h = spec.get("head_r", 5)
    cx, cy = 32.0, 40.0
    pal = spec["palette"]
    legw = 1.6
    lift = math.sin(frame * math.tau / 4.0) * 2 if stance == "walk" else 0
    stretch = 1.25 if stance == "flee" else 1.0
    headdown = stance == "graze"

    if stance == "dead":
        px(d, (cx, cy), b, pal[0], s)
        line(d, cx - 3, cy - 2, cx + 3, cy + 2, PAL["dead"], s, 1.5)
        line(d, cx - 3, cy + 2, cx + 3, cy - 2, PAL["dead"], s, 1.5)
        return

    px(d, (cx, cy + b + 6), b * stretch * 0.9, PAL["shadow"], s)

    # legs
    for i, dx in enumerate(spec.get("leg_x", [-b * 0.6, b * 0.6])):
        step = lift if i % 2 == 0 else -lift
        line(d, cx + dx * stretch, cy + b * 0.6,
             cx + dx * stretch + step, cy + b + spec.get("leg_l", 8),
             spec.get("leg_c", pal[1] if len(pal) > 1 else pal[0]), s, legw)

    # body
    d.ellipse([(cx - b * stretch) * s, (cy - b * 0.8) * s,
               (cx + b * stretch) * s, (cy + b * 0.8) * s], fill=pal[0])

    # tail
    tail = spec.get("tail")
    if tail == "long":
        line(d, cx - b * stretch, cy, cx - b * stretch - 10, cy - 4 + lift, pal[1], s, 2)
    elif tail == "bushy":
        d.ellipse([(cx - b * stretch - 12) * s, (cy - 8) * s,
                   (cx - b * stretch) * s, (cy + 4) * s], fill=pal[1])
        px(d, (cx - b * stretch - 10, cy - 5), 2.4, "#f0ede4", s)
    elif tail == "tails3":
        for i, dy in enumerate([-6, 0, 6]):
            d.ellipse([(cx - b * stretch - 13) * s, (cy - 4 + dy) * s,
                       (cx - b * stretch - 2) * s, (cy + 4 + dy) * s],
                      fill=pal[1])
            px(d, (cx - b * stretch - 11, cy + dy), 2.0, "#f0ede4", s)
    elif tail == "fan":
        poly(d, [(cx - b * stretch + 2, cy - 2), (cx - b * stretch - 14, cy - 12),
                 (cx - b * stretch - 12, cy + 6)], pal[1], s)
    elif tail == "stub":
        px(d, (cx - b * stretch - 1, cy - 2), 2, "#f0ede4", s)

    # head
    hx = cx + b * stretch + h * 0.6
    hy = cy + (4 if headdown else -b * 0.35)
    px(d, (hx, hy), h, pal[0], s)
    # snout/beak
    if spec.get("beak"):
        poly(d, [(hx + h, hy - 1), (hx + h + 4, hy + 1), (hx + h, hy + 3)],
             PAL["gilt"], s)
    else:
        px(d, (hx + h * 0.9, hy + h * 0.3), h * 0.5, pal[1] if len(pal) > 1 else pal[0], s)
    # ear / horn / antler
    ear = spec.get("ear")
    if ear == "rabbit":
        line(d, hx - 1, hy - h, hx - 3, hy - h - 7, pal[1], s, 1.6)
        line(d, hx + 2, hy - h, hx + 2, hy - h - 8, pal[1], s, 1.6)
    elif ear == "antler":
        for side in (-1, 1):
            line(d, hx + side * 1.5, hy - h + 1, hx + side * 3, hy - h - 5,
                 "#65513c" if spec.get("tier") == "mortal" else PAL["spirit_cyan"], s, 1.4)
            line(d, hx + side * 3, hy - h - 5, hx + side * 5, hy - h - 7,
                 "#65513c" if spec.get("tier") == "mortal" else PAL["spirit_cyan"], s, 1.2)
            line(d, hx + side * 3, hy - h - 5, hx + side * 2, hy - h - 8,
                 "#65513c" if spec.get("tier") == "mortal" else PAL["spirit_cyan"], s, 1.0)
    elif ear == "horn":
        line(d, hx, hy - h, hx + 1, hy - h - 5, PAL["gilt"], s, 1.6)
    elif ear == "pointy":
        poly(d, [(hx - 3, hy - h + 1), (hx - 1, hy - h - 4), (hx, hy - h + 1)], pal[0], s)
        poly(d, [(hx + 1, hy - h + 1), (hx + 3, hy - h - 4), (hx + 4, hy - h + 1)], pal[0], s)
    elif ear == "crest":
        px(d, (hx - 1, hy - h - 2), 2, PAL["cinnabar"], s)
    elif ear == "fin":
        poly(d, [(hx - 2, hy - h), (hx + 1, hy - h - 4), (hx + 3, hy - h)], pal[1], s)

    # eye
    px(d, (hx + h * 0.4, hy - h * 0.2), 0.9,
       PAL["spirit_cyan"] if spec.get("spirit_eyes") else PAL["dark"], s)

    # extras
    if spec.get("bristle"):
        for i in range(5):
            line(d, cx - b * 0.6 + i * b * 0.3, cy - b * 0.75,
                 cx - b * 0.6 + i * b * 0.3, cy - b * 0.75 - 2.5, pal[1], s, 1)
    if spec.get("scale_arcs"):
        for i in range(3):
            arc(d, (cx - 3 + i * 5, cy - 2), 4, 200, 340,
                PAL["gilt"], s, 1.0)
    if spec.get("mist"):
        for i in range(2):
            arc(d, (cx - 8 + i * 16, cy + b + 2), 6, 180, 360,
                PAL["spirit_cyan"], s, 0.9)


def animal_cell(spec, stance, frame=0, size=64):
    img, d, s = cell(size)
    draw_animal(d, s, spec, stance, frame)
    return finish(img, size)


def motif(d, s, name, cx, cy):
    """Tiny trade pictogram centred on (cx, cy) — used on shop signs and
    workshop walls so each store kind reads differently at map scale."""
    k = name
    if k == "clothes":        # hanging robe
        line(d, cx, cy - 4, cx, cy - 2, PAL["label"], s, 1)
        poly(d, [(cx - 3, cy + 4), (cx - 2, cy - 2), (cx + 2, cy - 2),
                 (cx + 3, cy + 4)], PAL["mortal_indigo"], s)
        line(d, cx - 2, cy - 2, cx - 4, cy - 4, PAL["mortal_indigo"], s, 1.4)
        line(d, cx + 2, cy - 2, cx + 4, cy - 4, PAL["mortal_indigo"], s, 1.4)
    elif k == "tools":        # crossed axe & pick
        line(d, cx - 3, cy - 3, cx + 3, cy + 3, PAL["wood"], s, 1.4)
        line(d, cx - 3, cy + 3, cx + 3, cy - 3, PAL["wood"], s, 1.4)
        rect(d, cx - 4, cy - 4, cx - 2, cy - 2, PAL["metal"], s)
        rect(d, cx + 2, cy + 2, cx + 4, cy + 4, PAL["metal"], s)
    elif k == "furniture":    # low table & stool
        rect(d, cx - 4, cy - 2, cx + 4, cy - 1, PAL["mortal_brown"], s)
        line(d, cx - 3, cy - 1, cx - 3, cy + 3, PAL["mortal_brown"], s, 1.4)
        line(d, cx + 3, cy - 1, cx + 3, cy + 3, PAL["mortal_brown"], s, 1.4)
        rect(d, cx - 1, cy + 1, cx + 1, cy + 3, "#9a7c58", s)
    elif k == "butcher":      # haunch with bone
        px(d, (cx - 1, cy), 3, "#b04a3e", s)
        px(d, (cx + 3, cy - 3), 1.2, PAL["label"], s)
        line(d, cx + 1, cy - 1, cx + 3, cy - 3, PAL["label"], s, 1.4)
        px(d, (cx + 3, cy - 3), 1.0, PAL["label"], s)
    elif k == "hunter":       # strung bow & arrow
        arc(d, (cx - 2, cy), 4, 90, 270, PAL["wood"], s, 1.4)
        line(d, cx - 2, cy - 4, cx - 2, cy + 4, PAL["label"], s, 0.9)
        line(d, cx - 4, cy, cx + 4, cy, PAL["wood"], s, 1.0)
        poly(d, [(cx + 4, cy), (cx + 1, cy - 1), (cx + 1, cy + 1)],
             PAL["metal"], s)
    elif k == "cobbler":      # boot last
        poly(d, [(cx - 3, cy - 4), (cx - 1, cy - 4), (cx - 1, cy + 1),
                 (cx + 4, cy + 3), (cx - 3, cy + 3)], PAL["mortal_brown"], s)
        line(d, cx - 3, cy + 3, cx + 4, cy + 3, PAL["dark"], s, 1.2)
    elif k == "blacksmith":   # anvil with ember
        poly(d, [(cx - 4, cy - 1), (cx + 1, cy - 1), (cx + 4, cy - 3),
                 (cx + 4, cy - 1), (cx + 1, cy + 1), (cx - 4, cy + 1)],
             PAL["metal"], s)
        rect(d, cx - 1, cy + 1, cx + 1, cy + 3, PAL["metal"], s)
        px(d, (cx - 3, cy - 3), 1.2, PAL["fire"], s)
    elif k == "carpenter":    # saw & plank
        rect(d, cx - 4, cy - 3, cx + 4, cy - 1, PAL["wood"], s)
        poly(d, [(cx - 4, cy + 1), (cx + 2, cy + 1), (cx + 4, cy + 4),
                 (cx - 4, cy + 4)], PAL["metal"], s)
        rect(d, cx + 2, cy, cx + 4, cy + 2, PAL["mortal_brown"], s)


# -------------------------------------------------------------------- sites
def draw_site(d, s, spec):
    """spec = {'shape': 'home'|'shop'|..., 'palette': [...]} each centred ~32,32."""
    shape = spec["shape"]
    p = spec.get("palette", [])

    def roof(x, y, w, color, eaves=True):
        poly(d, [(x - w / 2 - 3, y), (x, y - 10), (x + w / 2 + 3, y)], color, s)
        if eaves:  # upturned eave flicks
            line(d, x - w / 2 - 3, y, x - w / 2 - 6, y - 3, color, s, 2)
            line(d, x + w / 2 + 3, y, x + w / 2 + 6, y - 3, color, s, 2)

    if shape == "home":
        rect(d, 18, 32, 46, 50, PAL["timber_wall"], s)
        roof(32, 32, 28, PAL["roof_tile"])
        rect(d, 29, 40, 35, 50, PAL["mortal_brown"], s)
    elif shape == "shop":
        rect(d, 16, 30, 48, 50, PAL["timber_wall"], s)
        roof(32, 30, 32, PAL["roof_tile"])
        # hanging sign
        line(d, 48, 34, 54, 34, PAL["mortal_brown"], s, 1.5)
        rect(d, 52, 34, 58, 42, spec.get("sign_c", PAL["gilt"]), s)
        rect(d, 22, 36, 42, 44, "#b9a678", s)  # counter
        if spec.get("motif"):
            motif(d, s, spec["motif"], 55, 38)
    elif shape == "workshop":
        rect(d, 16, 30, 48, 50, PAL["timber_wall"], s)
        roof(32, 30, 32, "#4a5054")
        rect(d, 44, 18, 48, 26, PAL["mortal_brown"], s)  # chimney
        arc(d, (46, 14), 3, 0, 180, "#b9b2a4", s, 1.2)
        arc(d, (44, 10), 4, 0, 180, "#b9b2a4", s, 1.0)
        if spec.get("motif"):
            motif(d, s, spec["motif"], 32, 43)
    elif shape == "bank":
        # two-tier pagoda
        rect(d, 20, 40, 44, 52, PAL["timber_wall"], s)
        roof(32, 40, 28, PAL["roof_tile"])
        rect(d, 24, 28, 40, 36, PAL["timber_wall"], s)
        roof(32, 28, 20, PAL["roof_tile"])
        px(d, (32, 14), 1.8, PAL["gilt"], s)
        line(d, 32, 18, 32, 14, PAL["gilt"], s, 1.2)
        rect(d, 29, 44, 35, 52, PAL["gilt"], s)
    elif shape == "tavern":
        rect(d, 16, 30, 48, 50, PAL["timber_wall"], s)
        roof(32, 30, 32, "#8a4a3e")
        for lx in (18, 46):
            line(d, lx, 30, lx, 36, PAL["mortal_brown"], s, 1)
            px(d, (lx, 38), 3, PAL["cinnabar"], s)
            line(d, lx, 41, lx, 43, PAL["gilt"], s, 0.8)
        rect(d, 54, 24, 58, 40, "#e8dfc8", s, outline=PAL["mortal_brown"])
    elif shape == "school":
        rect(d, 18, 32, 46, 50, PAL["timber_wall"], s)
        roof(32, 32, 28, PAL["roof_tile"])
        # paifang gate
        for gx in (10, 20):
            line(d, gx, 50, gx, 30, PAL["mortal_brown"], s, 2)
        line(d, 8, 30, 22, 30, PAL["mortal_brown"], s, 2)
        line(d, 8, 30, 8, 28, PAL["mortal_brown"], s, 2)
        line(d, 22, 30, 22, 28, PAL["mortal_brown"], s, 2)
        rect(d, 12, 32, 18, 36, PAL["cinnabar"], s)
    elif shape == "farm":
        rect(d, 14, 28, 50, 52, "#5d8076", s)
        for r in range(5):
            c = "#c8c15f" if spec.get("mature") else "#739550"
            line(d, 17, 32 + r * 4, 47, 32 + r * 4, c, s, 1.8)
    elif shape == "graveyard":
        rect(d, 14, 26, 50, 52, "#858a70", s, outline="#c2bca0", width=2)
        for i in range(6):
            gx = 20 + (i % 3) * 9
            gy = 34 + (i // 3) * 9
            rect(d, gx - 2, gy - 2, gx + 2, gy + 4, "#c3c9ba", s)
            arc(d, (gx, gy - 2), 2, 180, 360, "#c3c9ba", s, 1.5)
    elif shape == "shelter":
        poly(d, [(22, 48), (32, 30), (42, 48)], PAL["mortal_earth"], s)
        line(d, 32, 36, 32, 48, PAL["mortal_brown"], s, 1.5)
        line(d, 46, 44, 50, 44, PAL["wood"], s, 1.2)
        line(d, 48, 40, 48, 44, PAL["wood"], s, 1.2)
        px(d, (48, 46), 1.6, PAL["fire"], s)
    elif shape == "jobboard":
        line(d, 26, 52, 26, 34, PAL["mortal_brown"], s, 2)
        line(d, 38, 52, 38, 34, PAL["mortal_brown"], s, 2)
        roof(32, 34, 20, PAL["roof_tile"], eaves=False)
        rect(d, 24, 34, 40, 46, "#ead7a6", s, outline=PAL["mortal_brown"])
        line(d, 27, 38, 37, 38, PAL["mortal_brown"], s, 0.8)
        line(d, 27, 41, 35, 41, PAL["mortal_brown"], s, 0.8)
    elif shape == "market":
        px(d, (32, 42), 10, "#b9a678", s)             # well ring
        px(d, (32, 42), 7, "#5a5142", s)
        roof(32, 30, 22, PAL["roof_tile"])            # well roof
        line(d, 24, 30, 24, 40, PAL["mortal_brown"], s, 1.5)
        line(d, 40, 30, 40, 40, PAL["mortal_brown"], s, 1.5)
    elif shape == "mine":
        c = spec.get("ore", PAL["ore"][3])
        poly(d, [(16, 52), (26, 30), (38, 36), (48, 52)], "#5d5a52", s)
        for ox, oy in [(26, 40), (34, 38), (30, 46), (40, 46)]:
            px(d, (ox, oy), 2, c, s)
        line(d, 20, 52, 20, 26, PAL["wood"], s, 2)   # headframe
        line(d, 30, 52, 30, 30, PAL["wood"], s, 2)
        line(d, 20, 26, 30, 30, PAL["wood"], s, 2)
        arc(d, (24, 24), 4, 0, 360, PAL["wood"], s, 1.5)
    elif shape == "grave_site":  # roadside camp variant (unused today)
        pass
    elif shape == "gate":        # immortal mountain gate
        for gx in (20, 44):
            line(d, gx, 52, gx, 28, "#6a5236", s, 3)
        roof(32, 26, 32, PAL["immortal_jade"])
        rect(d, 28, 30, 36, 34, PAL["cinnabar"], s)
        arc(d, (32, 44), 12, 180, 360, PAL["spirit_cyan"], s, 1.0)
    elif shape == "pavilion":    # immortal pavilion
        roof(32, 26, 30, PAL["immortal_jade"])
        for gx in (22, 42):
            line(d, gx, 52, gx, 28, "#8a6a44", s, 2)
        arc(d, (32, 46), 10, 0, 180, PAL["spirit_cyan"], s, 0.9)


def site_cell(spec, size=64):
    img, d, s = cell(size)
    draw_site(d, s, spec)
    return finish(img, size)


# ------------------------------------------------------------------ terrain
def draw_terrain(d, s, spec):
    shape = spec["shape"]
    if shape == "pine":
        for i, (w, y) in enumerate([(20, 46), (16, 36), (10, 26)]):
            poly(d, [(32 - w / 2, y), (32, y - 14), (32 + w / 2, y)],
                 PAL["pine"], s)
        line(d, 32, 48, 32, 54, PAL["mortal_brown"], s, 2)
    elif shape == "bamboo":
        for i, x in enumerate([24, 30, 36, 42]):
            line(d, x, 54, x + (i - 1.5), 20 - (i % 2) * 3, PAL["bamboo"], s, 2.2)
            line(d, x + (i - 1.5), 30 - (i % 2) * 3, x + (i - 1.5) + 4,
                 26 - (i % 2) * 3, PAL["bamboo"], s, 1.2)
            line(d, x + (i - 1.5), 24 - (i % 2) * 3, x + (i - 1.5) - 4,
                 21 - (i % 2) * 3, PAL["bamboo"], s, 1.2)
    elif shape == "willow":
        arc(d, (32, 34), 14, 180, 360, "#5a7d4e", s, 8)
        for dx in (-8, -3, 2, 7):
            line(d, 32 + dx, 30, 32 + dx + (dx // 4), 46, "#5a7d4e", s, 1.4)
        line(d, 32, 40, 32, 54, PAL["mortal_brown"], s, 2)
    elif shape == "plum":
        line(d, 30, 54, 30, 34, PAL["mortal_brown"], s, 2)
        line(d, 30, 42, 38, 32, PAL["mortal_brown"], s, 1.4)
        line(d, 30, 40, 24, 32, PAL["mortal_brown"], s, 1.2)
        for bx, by in [(30, 30), (38, 30), (24, 30), (34, 36), (27, 35)]:
            px(d, (bx, by), 2.2, PAL["plum"], s)
    elif shape == "tree_generic":
        px(d, (33, 30), 11, "#426540", s)
        px(d, (29, 34), 8, "#3d603c", s)
        line(d, 32, 40, 32, 54, PAL["mortal_brown"], s, 2)
    elif shape == "berries":
        arc(d, (32, 40), 12, 180, 360, "#4a6a3e", s, 10)
        n = spec.get("n", 5)
        for i in range(n):
            a = math.pi + (i + 0.5) * math.pi / n
            px(d, (32 + 10 * math.cos(a), 38 + 10 * math.sin(a)), 1.6,
               "#a95d76", s)
    elif shape == "timber":
        for x in (26, 32, 38):
            line(d, x, 52, x, 38, PAL["mortal_brown"], s, 3)
        px(d, (32, 34), 9, "#365a38", s)
    elif shape == "stone":
        poly(d, [(18, 50), (26, 34), (36, 40), (34, 50)], PAL["stone"], s)
        poly(d, [(30, 50), (38, 36), (48, 46), (46, 50)], PAL["stone2"], s)
        line(d, 26, 40, 30, 46, "#6a7178", s, 1)
    elif shape == "fish":
        arc(d, (32, 40), 12, 200, 340, PAL["water"], s, 2)
        arc(d, (32, 44), 9, 200, 340, PAL["water"], s, 1.5)
        for fx, fy in [(28, 38), (36, 40), (32, 42)]:
            line(d, fx, fy, fx + 4, fy - 1, "#bce1df", s, 1.4)
    elif shape == "herbs":
        for i, x in enumerate([26, 32, 38]):
            line(d, x, 50, x + (i - 1) * 2, 36 - i * 2, "#5a8a4e", s, 1.6)
            px(d, (x + (i - 1) * 2, 35 - i * 2), 2, "#7ab56a", s)
        if spec.get("spirit"):
            arc(d, (32, 42), 14, 0, 360, PAL["spirit_cyan"], s, 1.0)
    elif shape == "mushrooms":
        for mx, my, r in [(26, 44, 4), (36, 46, 5), (31, 40, 3)]:
            line(d, mx, my + r + 4, mx, my, "#d8d0bc", s, 2.4)
            arc(d, (mx, my), r, 180, 360, "#a95d4e", s, r)
    elif shape == "lotus":
        for i in range(3):
            a = -math.pi / 2 + (i - 1) * 0.6
            x0, y0 = 32 + 10 * math.cos(a), 40 + 8 * math.sin(a)
            px(d, (x0, y0), 3.5, "#5a8a5e", s)
        px(d, (32, 30), 3, PAL["lotus"], s)
        px(d, (32, 30), 1.4, PAL["gilt"], s)
    elif shape == "mist":
        for i in range(3):
            y = 34 + i * 7
            arc(d, (28 + i * 6, y), 10, 180, 340, PAL["spirit_cyan"], s, 1.4)
    elif shape == "unknown_site":
        px(d, (32, 36), 12, (56, 90, 120, 90), s)
        arc(d, (32, 36), 14, 0, 360, PAL["spirit_cyan"], s, 1.2)
        line(d, 32, 32, 32, 38, PAL["spirit_cyan"], s, 2)
        px(d, (32, 42), 1.4, PAL["spirit_cyan"], s)


def terrain_cell(spec, size=64):
    img, d, s = cell(size)
    draw_terrain(d, s, spec)
    return finish(img, size)


# -------------------------------------------------------------------- icons
def draw_icon(d, s, spec):
    shape = spec["shape"]
    ink = spec.get("ink", "#f4eee0")
    w = spec.get("w", 2.5)
    cx = cy = 12.0
    if shape == "berries":
        arc(d, (cx, cy + 2), 7, 180, 360, ink, s, w)
        for a in (-0.6, 0, 0.6):
            px(d, (cx + 6 * math.sin(a), cy - 1 + 3 * abs(a)), 1.4, ink, s)
    elif shape == "timber":
        for x in (8, 12, 16):
            line(d, x, 19, x, 9, ink, s, w - 0.6)
        arc(d, (12, 9), 6, 180, 360, ink, s, w - 1)
    elif shape == "stone":
        poly(d, [(6, 18), (10, 9), (16, 12), (17, 18)], None, s, outline=ink, width=w)
        poly(d, [(13, 18), (16, 10), (20, 16), (19, 18)], None, s, outline=ink, width=w)
    elif shape == "fish":
        arc(d, (cx, cy), 7, 200, 160, ink, s, w)
        poly(d, [(cx - 7, cy), (cx - 11, cy - 3), (cx - 11, cy + 3)], None, s,
             outline=ink, width=w)
        px(d, (cx + 3, cy - 2), 0.9, ink, s)
    elif shape in ("gold", "silver", "copper", "iron"):
        poly(d, [(cx, 5), (19, 10), (19, 15), (cx, 20), (5, 15), (5, 10)],
             None, s, outline=ink, width=w)
        px(d, (cx, 12.5), 2, ink, s)
    elif shape == "rice":
        for dx in (-3, 0, 3):
            line(d, cx + dx, 20, cx + dx, 8, ink, s, 1.4)
        for i in range(3):
            px(d, (cx - 3 + i * 3, 6 + (i % 2) * 2), 1.6, None, s, outline=ink, width=w)
    elif shape == "axe":
        line(d, 9, 20, 14, 6, ink, s, w - 0.8)
        poly(d, [(12, 6), (18, 6), (17, 10), (12, 10)], None, s, outline=ink, width=w)
    elif shape == "pickaxe":
        line(d, 12, 20, 12, 8, ink, s, w - 0.8)
        arc(d, (12, 9), 6, 200, 340, ink, s, w - 0.8)
    elif shape == "hoe":
        line(d, 10, 20, 13, 7, ink, s, w - 0.8)
        line(d, 13, 7, 18, 9, ink, s, w - 0.8)
        line(d, 18, 9, 17, 12, ink, s, w - 1.4)
    elif shape == "shovel":
        line(d, 12, 6, 12, 15, ink, s, w - 0.8)
        poly(d, [(9, 15), (15, 15), (13.5, 21), (10.5, 21)], None, s,
             outline=ink, width=w)
    elif shape == "home":
        poly(d, [(5, 13), (12, 6), (19, 13)], None, s, outline=ink, width=w)
        rect(d, 7, 13, 17, 19, None, s, outline=ink, width=w)
    elif shape == "shop":
        rect(d, 6, 12, 18, 19, None, s, outline=ink, width=w)
        poly(d, [(4, 12), (6, 8), (18, 8), (20, 12)], None, s, outline=ink, width=w)
        line(d, 12, 15, 12, 19, ink, s, 1.4)
    elif shape == "workshop":
        rect(d, 6, 11, 17, 19, None, s, outline=ink, width=w)
        line(d, 14, 6, 14, 11, ink, s, w - 1)
        arc(d, (16, 5), 2, 180, 360, ink, s, 1.2)
    elif shape == "bank":
        poly(d, [(5, 10), (12, 5), (19, 10)], None, s, outline=ink, width=w)
        for x in (7, 11, 15):
            line(d, x, 11, x, 17, ink, s, w - 1.2)
        line(d, 5, 18, 19, 18, ink, s, w - 0.8)
    elif shape == "school":
        rect(d, 7, 9, 17, 14, None, s, outline=ink, width=w)
        line(d, 12, 9, 12, 14, ink, s, 1.2)
        line(d, 6, 19, 18, 19, ink, s, w - 0.8)
    elif shape == "tavern":
        rect(d, 8, 8, 16, 17, None, s, outline=ink, width=w)
        arc(d, (17, 11), 3, 270, 90, ink, s, 1.6)
        line(d, 8, 12, 16, 12, ink, s, 1.2)
    elif shape == "farm":
        for r in range(3):
            line(d, 6, 10 + r * 4, 18, 10 + r * 4, ink, s, 1.2)
        line(d, 6, 8, 18, 8, ink, s, w - 1)
    elif shape == "graveyard":
        rect(d, 8, 7, 16, 18, None, s, outline=ink, width=w)
        arc(d, (12, 7), 4, 180, 360, ink, s, w - 0.8)
        line(d, 12, 10, 12, 15, ink, s, 1.2)
    elif shape == "shelter":
        poly(d, [(6, 18), (12, 7), (18, 18)], None, s, outline=ink, width=w)
        line(d, 12, 11, 12, 18, ink, s, 1.2)
    elif shape == "jobboard":
        line(d, 8, 20, 8, 9, ink, s, w - 1)
        line(d, 16, 20, 16, 9, ink, s, w - 1)
        rect(d, 6, 6, 18, 12, None, s, outline=ink, width=w)
    elif shape == "market":
        px(d, (12, 14), 6, None, s, outline=ink, width=w)
        px(d, (12, 14), 3, None, s, outline=ink, width=w)
    elif shape == "road":
        line(d, 5, 18, 19, 6, ink, s, w - 0.6)
        for t in (0.3, 0.55, 0.8):
            px(d, (5 + 14 * t, 18 - 12 * t), 0.9, ink, s)
    elif shape == "sword":
        line(d, 7, 18, 16, 9, ink, s, w - 0.8)
        line(d, 9, 16, 5, 20, ink, s, w - 1)
        line(d, 14, 11, 18, 7, ink, s, 1.4)
    elif shape == "scroll":
        rect(d, 7, 8, 17, 16, None, s, outline=ink, width=w)
        line(d, 9, 11, 15, 11, ink, s, 1)
        line(d, 9, 13.5, 14, 13.5, ink, s, 1)
    elif shape == "lantern":
        px(d, (12, 12), 5, None, s, outline=ink, width=w)
        line(d, 12, 4, 12, 7, ink, s, 1.2)
        line(d, 12, 17, 12, 20, ink, s, 1.2)
    elif shape == "ingot":
        poly(d, [(8, 12), (11, 8), (13, 8), (16, 12), (14, 17), (10, 17)],
             None, s, outline=ink, width=w)
    elif shape == "lotus":
        for a in (-0.7, -0.35, 0, 0.35, 0.7):
            x0 = 12 + 8 * math.sin(a)
            y0 = 15 - 7 * math.cos(a)
            line(d, 12, 17, x0, y0, ink, s, 1.2)
        arc(d, (12, 9), 3, 180, 360, ink, s, 1.4)
    elif shape == "seal":
        rect(d, 6, 6, 18, 18, PAL["cinnabar"], s)
        line(d, 9, 12, 15, 12, "#f4ede0", s, 1.6)
        line(d, 12, 9, 12, 15, "#f4ede0", s, 1.6)
        line(d, 9.5, 15.5, 14.5, 15.5, "#f4ede0", s, 1.6)


def icon_cell(spec, size=24):
    img, d, s = cell(size, scale=8)
    draw_icon(d, s, spec)
    return img.resize((size, size), Image.NEAREST)


# ================================================================ inventory
MORTALS = [
    ("farmer-straw",   "Farmer — straw hat",      dict(robe="mortal_earth", hat="straw", prop="hoe")),
    ("farmer-douli",   "Farmer — douli hat",      dict(robe="mortal_hemp", hat="douli", prop="basket")),
    ("fieldhand",      "Field hand — headwrap",   dict(robe="mortal_earth", robe2="mortal_indigo", hat="wrap", prop="hoe")),
    ("reaper",         "Reaper",                  dict(robe="mortal_earth", hat="cap", prop="sickle")),
    ("porter",         "Porter — shoulder pole",  dict(robe="mortal_indigo", hat="wrap", prop="pole")),
    ("laborer",        "Laborer",                 dict(robe="mortal_greygreen", hat=None, prop="pole")),
    ("artisan",        "Artisan — apron",         dict(robe="mortal_indigo", robe2="mortal_hemp", hat="cap", prop="hammer")),
    ("smith",          "Smith",                   dict(robe="dark", robe2="mortal_indigo", hat="wrap", prop="tongs")),
    ("merchant",       "Merchant — futou hat",    dict(robe="#4a6a6e", hat="futou", prop="abacus")),
    ("trader",         "Trader — ledger scroll",  dict(robe="#5a5a48", hat="cap", prop="scroll")),
    ("fisher",         "Fisher — reed cape",      dict(robe="mortal_greygreen", hat="reed", prop="rod")),
    ("elder",          "Elder — staff",           dict(robe="#6a6a60", hat="bun", prop="staff", beard=True)),
]
IMMORTALS = [
    ("swordsman",      "Swordsman cultivator",    dict(robe="immortal_white", robe2="immortal_jade", hat="topknot", prop="sword", long_hem=True, wide_sleeves=True)),
    ("cultivator",     "Robed cultivator",        dict(robe="immortal_white", robe2="spirit_cyan", hat="topknot", prop="whisk", long_hem=True, wide_sleeves=True)),
    ("elder-sage",     "Elder sage",              dict(robe="#e8e2d2", robe2="gilt", hat="guan", prop="dragonstaff", long_hem=True, wide_sleeves=True, beard=True)),
    ("spirit-envoy",   "Spirit envoy",            dict(robe="immortal_jade", robe2="immortal_white", hat="topknot", prop="lantern", long_hem=True, wide_sleeves=True, aura=True)),
]

ANIMALS = [
    ("rabbit",       "Rabbit",        dict(body_r=6, head_r=3.5, palette=["#e5ddc8", "#f4ecdc"], ear="rabbit", tail="stub", leg_l=5, leg_x=[-3, 3])),
    ("deer",         "Deer",          dict(body_r=9, head_r=4.5, palette=["#af8558", "#ccaa78"], ear="antler", tail="stub", leg_l=10, leg_x=[-5, 5])),
    ("boar",         "Boar",          dict(body_r=8, head_r=5, palette=["#6b5a48", "#4e4238"], ear="pointy", tail=None, leg_l=6, bristle=True)),
    ("pheasant",     "Pheasant",      dict(body_r=6, head_r=3.5, palette=["#8a5a3d", "#c25e7c"], ear="crest", tail="fan", leg_l=6, beak=True)),
    ("fox",          "Fox",           dict(body_r=7, head_r=4, palette=["#b06a3e", "#8a4e2e"], ear="pointy", tail="bushy", leg_l=6)),
    ("crane",        "Crane",         dict(body_r=7, head_r=3.5, palette=["#f0ede4", "#3d3a36"], ear="crest", tail=None, leg_l=14, beak=True)),
    ("spirit-deer",  "Spirit deer",   dict(body_r=9, head_r=4.5, palette=["#cfe8e0", "#9ecfc4"], ear="antler", tail="stub", leg_l=10, leg_x=[-5, 5], spirit_eyes=True, mist=True)),
    ("spirit-fox",   "Spirit fox",    dict(body_r=7, head_r=4, palette=["#e8e4f0", "#b8b0d0"], ear="pointy", tail="tails3", leg_l=6, spirit_eyes=True)),
    ("qilin-colt",   "Qilin colt",    dict(body_r=9, head_r=4.5, palette=["#d4c9a8", "#b8a878"], ear="horn", tail="long", leg_l=10, leg_x=[-5, 5], scale_arcs=True, spirit_eyes=True, mist=True)),
]

POSES = ["idle"] + [("walk", f) for f in range(8)] + [("work", f) for f in range(8)] + ["dead", "carry", "lie"]
STANCES = ["idle", "walk", "walk2", "graze", "flee", "dead"]

SITES = [
    ("home",      "Home",            dict(shape="home")),
    ("shop",      "Shop",            dict(shape="shop")),
    ("workshop",  "Workshop",        dict(shape="workshop")),
    ("shop-clothes",    "Shop — clothier",    dict(shape="shop", motif="clothes",    sign_c=PAL["mortal_indigo"])),
    ("shop-tools",      "Shop — tools",       dict(shape="shop", motif="tools",      sign_c=PAL["gilt"])),
    ("shop-furniture",  "Shop — furniture",   dict(shape="shop", motif="furniture",  sign_c="#9a7c58")),
    ("shop-butcher",    "Shop — butcher",     dict(shape="shop", motif="butcher",    sign_c="#b04a3e")),
    ("workshop-hunter",     "Workshop — hunter",     dict(shape="workshop", motif="hunter")),
    ("workshop-cobbler",    "Workshop — cobbler",    dict(shape="workshop", motif="cobbler")),
    ("workshop-blacksmith", "Workshop — blacksmith", dict(shape="workshop", motif="blacksmith")),
    ("workshop-carpenter",  "Workshop — carpenter",  dict(shape="workshop", motif="carpenter")),
    ("bank",      "Bank — pagoda",   dict(shape="bank")),
    ("tavern",    "Tavern — lanterns", dict(shape="tavern")),
    ("school",    "School — paifang", dict(shape="school")),
    ("farm",      "Rice farm",       dict(shape="farm")),
    ("graveyard", "Graveyard — steles", dict(shape="graveyard")),
    ("shelter",   "Roadside camp",   dict(shape="shelter")),
    ("jobboard",  "Job board",       dict(shape="jobboard")),
    ("market",    "Market well",     dict(shape="market")),
    ("mine-iron", "Iron mine",       dict(shape="mine", ore=PAL["ore"][3])),
    ("mine-gold", "Gold mine",       dict(shape="mine", ore=PAL["ore"][0])),
    ("gate",      "Mountain gate (immortal)", dict(shape="gate")),
    ("pavilion",  "Pavilion (immortal)",      dict(shape="pavilion")),
]

TERRAIN = [
    ("pine",          "Pine tree",            dict(shape="pine")),
    ("bamboo",        "Bamboo grove",         dict(shape="bamboo")),
    ("willow",        "Willow",               dict(shape="willow")),
    ("plum",          "Plum blossom",         dict(shape="plum")),
    ("tree-generic",  "Generic tree",         dict(shape="tree_generic")),
    ("berries",       "Berry bush",           dict(shape="berries")),
    ("timber",        "Timber stand",         dict(shape="timber")),
    ("stone",         "Stone outcrop",        dict(shape="stone")),
    ("fish",          "Fish shoal",           dict(shape="fish")),
    ("herbs",         "Herb patch",           dict(shape="herbs")),
    ("spirit-herbs",  "Spirit herb patch",    dict(shape="herbs", spirit=True)),
    ("mushrooms",     "Mushroom ring",        dict(shape="mushrooms")),
    ("lotus",         "Lotus cluster",        dict(shape="lotus")),
    ("mist",          "Qi mist wisps",        dict(shape="mist")),
    ("unknown-site",  "Undiscovered site",    dict(shape="unknown_site")),
]

ICONS = ["berries", "timber", "stone", "fish", "gold", "silver", "copper",
         "iron", "rice", "axe", "pickaxe", "hoe", "shovel", "home", "shop",
         "workshop", "bank", "school", "tavern", "farm", "graveyard",
         "shelter", "jobboard", "market", "road", "sword", "scroll",
         "lantern", "ingot", "lotus", "seal"]


def sheet(images, cols, pad=4, bg=(20, 24, 20, 255), cw=None, ch=None):
    cw = cw or max(i.width for i in images)
    ch = ch or max(i.height for i in images)
    rows = (len(images) + cols - 1) // cols
    out = Image.new("RGBA", (cols * (cw + pad) + pad, rows * (ch + pad) + pad), bg)
    for i, im in enumerate(images):
        x = pad + (i % cols) * (cw + pad)
        y = pad + (i // cols) * (ch + pad)
        out.paste(im, (x, y))
    return out


def save(img, *path):
    p = os.path.join(PARKED, *path)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    img.save(p)
    return p


def main():
    # ---- villager atlas: 16 figures x 20 poses -> 1024x1280 atlas + sheets
    figures = MORTALS + IMMORTALS
    atlas = Image.new("RGBA", (1024, 1280), (0, 0, 0, 0))
    previews = []
    for col, (fid, name, spec) in enumerate(figures):
        spec = dict(spec)
        spec["skin"] = PAL["skin"][col % 4]
        spec = {k: (PAL.get(v, v) if isinstance(v, str) else v)
                for k, v in spec.items()}
        for row, p in enumerate(POSES):
            pose, frame = (p, 0) if isinstance(p, str) else p
            c = person_cell(spec, pose, frame)
            atlas.paste(c, (col * 64, row * 64))
        # preview row: idle, walk f0, walk f4, work f0, work f4, carry, dead, lie
        previews.append(sheet([
            person_cell(spec, "idle"), person_cell(spec, "walk", 0),
            person_cell(spec, "walk", 4), person_cell(spec, "work", 0),
            person_cell(spec, "work", 4), person_cell(spec, "carry"),
            person_cell(spec, "dead"), person_cell(spec, "lie")], 8))
    save(atlas, "villager-atlas-v2", "atlas.png")
    for (fid, name, _), pv in zip(figures, previews):
        save(pv, "villager-atlas-v2", "stances", f"{fid}.png")

    # ---- animals: stance sheet per species (6 stances) + atlas
    aanimal_atlas = Image.new("RGBA", (9 * 64, 6 * 64), (0, 0, 0, 0))
    for col, (aid, name, spec) in enumerate(ANIMALS):
        spec = dict(spec)
        spec["palette"] = [PAL.get(c, c) for c in spec["palette"]]
        spec["tier"] = "immortal" if aid.startswith(("spirit", "qilin")) else "mortal"
        for row, st in enumerate(STANCES):
            stance, frame = ("walk", 2) if st == "walk" else (("walk", 6) if st == "walk2" else (st, 0))
            c = animal_cell(spec, stance, frame)
            aanimal_atlas.paste(c, (col * 64, row * 64))
            if row == 0:
                pass
        sh = sheet([animal_cell(spec, st, f) for st, f in
                    [("idle", 0), ("walk", 2), ("walk", 6), ("graze", 0),
                     ("flee", 0), ("dead", 0)]], 6)
        save(sh, "animal-table-xianxia", f"{aid}.png")
    save(aanimal_atlas, "animal-table-xianxia", "atlas.png")

    # ---- sites
    site_imgs = []
    for sid, name, spec in SITES:
        im = site_cell(spec)
        save(im, "site-glyphs-xianxia", f"{sid}.png")
        site_imgs.append(im)
    save(sheet(site_imgs, 5), "site-glyphs-xianxia", "sheet.png")

    # ---- terrain / flora
    t_imgs = []
    for tid, name, spec in TERRAIN:
        im = terrain_cell(spec)
        save(im, "terrain-flora-xianxia", f"{tid}.png")
        t_imgs.append(im)
    save(sheet(t_imgs, 5), "terrain-flora-xianxia", "sheet.png")

    # ---- icons
    i_imgs = []
    for ic in ICONS:
        im = icon_cell(dict(shape=ic))
        save(im, "icons-brush", f"{ic}.png")
        i_imgs.append(im.resize((32, 32), Image.NEAREST))
    save(sheet(i_imgs, 8, cw=32, ch=32), "icons-brush", "sheet.png")

    # ---- discovery fx frames
    fx_imgs = []
    for i in range(4):
        img, d, s = cell(64)
        for k in range(3):
            y = 30 + k * 8
            arc(d, (26 + i * 4 + k * 4, y + math.sin(i * math.tau / 4 + k) * 2),
                10, 180, 340, PAL["spirit_cyan"], s, 1.4)
        arc(d, (32, 32), 10 + i * 4, 0, 360, PAL["spirit_cyan"], s, 1.0)
        fx_imgs.append(finish(img))
    save(sheet(fx_imgs, 4), "discovery-fx", "reveal.png")

    print("done")


if __name__ == "__main__":
    main()
