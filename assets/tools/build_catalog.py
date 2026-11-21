#!/usr/bin/env python3
"""Build the parked-asset catalog: assets/parked/ASSET_CATALOG.md (repo view)
plus a self-contained HTML copy for review."""
import base64, os, html

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
P = os.path.join(ROOT, "parked")

# (family, [(id, file, usage)], header note)
DATA = [
    ("villager-atlas-v2", "Villager atlas v2 — `atlas.png` (1024×1280), 16 cols × 20 pose rows. Stance sheets per figure show: idle, walk×2, work×2, carry, dead, lying.", [
        ("farmer-straw",   "stances/farmer-straw.png",   "Atlas col 0 — farmers on Farm/HarvestRice actions"),
        ("farmer-douli",   "stances/farmer-douli.png",   "Atlas col 1 — gatherers, ForageMeal"),
        ("fieldhand",      "stances/fieldhand.png",      "Atlas col 2 — WorkForFood laborers"),
        ("reaper",         "stances/reaper.png",         "Atlas col 3 — harvest workers"),
        ("porter",         "stances/porter.png",         "Atlas col 4 — Deliver/Trade carriers"),
        ("laborer",        "stances/laborer.png",        "Atlas col 5 — Build/construction crews"),
        ("artisan",        "stances/artisan.png",        "Atlas col 6 — CraftTool artisans"),
        ("smith",          "stances/smith.png",          "Atlas col 7 — blacksmith workshop"),
        ("merchant",       "stances/merchant.png",       "Atlas col 8 — OperateBusiness shopkeepers"),
        ("trader",         "stances/trader.png",         "Atlas col 9 — Trade/Barter at market"),
        ("fisher",         "stances/fisher.png",         "Atlas col 10 — coastal fish gatherers"),
        ("elder",          "stances/elder.png",          "Atlas col 11 — elderly villagers"),
        ("swordsman",      "stances/swordsman.png",      "Atlas col 12 — cultivator (dormant)"),
        ("cultivator",     "stances/cultivator.png",     "Atlas col 13 — robed cultivator (dormant)"),
        ("elder-sage",     "stances/elder-sage.png",     "Atlas col 14 — elder sage (dormant)"),
        ("spirit-envoy",   "stances/spirit-envoy.png",   "Atlas col 15 — spirit envoy (dormant)"),
    ]),
    ("animal-table-xianxia", "Animal stance sheets — idle, walk×2, graze, flee, dead. `atlas.png` = 9×6 grid.", [
        ("rabbit",      "rabbit.png",      "Wildlife renderer — grove herbivore (existing)"),
        ("deer",        "deer.png",        "Wildlife renderer — grove herbivore (existing)"),
        ("boar",        "boar.png",        "Future species — forest forager"),
        ("pheasant",    "pheasant.png",    "Future species — grove bird"),
        ("fox",         "fox.png",         "Future species — first predator"),
        ("crane",       "crane.png",       "Future species — coast/wetland bird"),
        ("spirit-deer", "spirit-deer.png", "Dormant immortal-tier spirit beast"),
        ("spirit-fox",  "spirit-fox.png",  "Dormant immortal-tier spirit beast"),
        ("qilin-colt",  "qilin-colt.png",  "Dormant rare spirit beast"),
    ]),
    ("site-glyphs-xianxia", "Building & site glyphs (64×64).", [
        ("home",      "home.png",      "Housing blocks in `field.gd`"),
        ("shop",      "shop.png",      "Shop buildings (clothes/tools/furniture/butcher sign variants)"),
        ("workshop",  "workshop.png",  "Workshop buildings (hunter/cobbler/blacksmith/carpenter)"),
        ("bank",      "bank.png",      "Town bank — pagoda landmark"),
        ("tavern",    "tavern.png",    "Village tavern — lantern landmark"),
        ("school",    "school.png",    "School — paifang gate"),
        ("farm",      "farm.png",      "Rice farm plots"),
        ("graveyard", "graveyard.png", "Graveyard — stele rows"),
        ("shelter",   "shelter.png",   "Roadside camps"),
        ("jobboard",  "jobboard.png",  "Job posting board"),
        ("market",    "market.png",    "Market centre well"),
        ("mine-iron", "mine-iron.png", "Iron mine — headframe + ore"),
        ("mine-gold", "mine-gold.png", "Gold mine (silver/copper share the glyph, ore color swaps)"),
        ("gate",      "gate.png",      "Immortal site — mountain gate (dormant)"),
        ("pavilion",  "pavilion.png",  "Immortal site — pavilion (dormant)"),
    ]),
    ("terrain-flora-xianxia", "Terrain, patch and flora glyphs (64×64).", [
        ("pine",         "pine.png",         "Forest canopy archetype (cool groves)"),
        ("bamboo",       "bamboo.png",       "Forest canopy archetype (coastal-biased)"),
        ("willow",       "willow.png",       "Forest canopy archetype (near water/valley)"),
        ("plum",         "plum.png",         "Rare accent tree (~1 in 7 groves)"),
        ("tree-generic", "tree-generic.png", "Default grove tree"),
        ("berries",      "berries.png",      "Berry patch (kind 0) — dots scale with stock"),
        ("timber",       "timber.png",       "Timber patch (kind 1)"),
        ("stone",        "stone.png",        "Stone outcrop (kind 2)"),
        ("fish",         "fish.png",         "Fish shoal (kind 3)"),
        ("herbs",        "herbs.png",        "Future patch kind — herb forage"),
        ("spirit-herbs", "spirit-herbs.png", "Future rare patch kind — spirit herb"),
        ("mushrooms",    "mushrooms.png",    "Future patch kind — forest mushrooms"),
        ("lotus",        "lotus.png",        "Coast-edge accent flora"),
        ("mist",         "mist.png",         "Qi mist wisps — immortal-site ambience"),
        ("unknown-site", "unknown-site.png", "Undiscovered site marker — procedural-expansion engine"),
    ]),
    ("icons-brush", "Brush-stroke panel icons (24×24).", [
        ("berries", "berries.png", "Resource menus / ledgers — food good"),
        ("timber",  "timber.png",  "Resource menus — material good"),
        ("stone",   "stone.png",   "Resource menus — material good"),
        ("fish",    "fish.png",    "Resource menus — food good"),
        ("gold",    "gold.png",    "Resource menus — ore good"),
        ("silver",  "silver.png",  "Resource menus — ore good"),
        ("copper",  "copper.png",  "Resource menus — ore good"),
        ("iron",    "iron.png",    "Resource menus — ore good"),
        ("rice",    "rice.png",    "Resource menus — food good"),
        ("axe",     "axe.png",     "Tool inventories"),
        ("pickaxe", "pickaxe.png", "Tool inventories"),
        ("hoe",     "hoe.png",     "Tool inventories / farm jobs"),
        ("shovel",  "shovel.png",  "Tool inventories"),
        ("home",    "home.png",    "Inspector title — housing"),
        ("shop",    "shop.png",    "Inspector title — shops"),
        ("workshop","workshop.png","Inspector title — workshops"),
        ("bank",    "bank.png",    "Inspector title — bank"),
        ("school",  "school.png",  "Inspector title — school"),
        ("tavern",  "tavern.png",  "Inspector title — tavern"),
        ("farm",    "farm.png",    "Inspector title — farm"),
        ("graveyard","graveyard.png","Inspector title — graveyard"),
        ("shelter", "shelter.png", "Inspector title — camps"),
        ("jobboard","jobboard.png","Job board panel"),
        ("market",  "market.png",  "Market inspector"),
        ("road",    "road.png",    "Road inspector"),
        ("sword",   "sword.png",   "Hunt / cultivator actions (dormant)"),
        ("scroll",  "scroll.png",  "School / contract rows"),
        ("lantern", "lantern.png", "Tavern / night rows"),
        ("ingot",   "ingot.png",   "Treasury / bank balances"),
        ("lotus",   "lotus.png",   "Spirit-site rows (dormant)"),
        ("seal",    "seal.png",    "UI signature stamp (cinnabar seal)"),
    ]),
    ("discovery-fx", "Mist & reveal animation frames.", [
        ("reveal", "reveal.png", "4-frame expanding-ring reveal + qi wisps — procedural-expansion engine (art-spec §9)"),
    ]),
]


# Actual texture files loaded by the Godot viewer. Individual atlas source
# sheets remain in the catalog, but the viewer loads the combined atlas.
RUNTIME_FILES = {
    "villager-atlas-v2": {"atlas.png"},
    "animal-table-xianxia": {"atlas.png"},
    "site-glyphs-xianxia": {name + ".png" for name in (
        "home", "bank", "tavern", "school", "farm", "graveyard", "shelter",
        "jobboard", "market", "mine-iron", "mine-gold",
    )},
    "terrain-flora-xianxia": {name + ".png" for name in (
        "pine", "bamboo", "willow", "plum", "tree-generic", "berries",
        "timber", "stone", "fish",
    )},
    "icons-brush": {name + ".png" for name in (
        "berries", "timber", "gold", "silver", "copper", "iron",
    )},
}


def b64(path):
    with open(path, "rb") as f:
        return "data:image/png;base64," + base64.b64encode(f.read()).decode()


def main():
    md = ["# NEAT Xianxia — Parked Asset Catalog\n",
          "**Runtime** files are loaded by Godot. **Parked** files are source",
          "art, contact sheets, or assets awaiting integration.",
          "The use column includes planned uses. Regenerate with",
          "`python3 assets/tools/gen_assets.py && python3 assets/tools/build_catalog.py`.\n"]
    h = ["<!doctype html><meta charset=utf-8><title>Parked Asset Catalog</title>",
         "<style>body{font:14px/1.5 system-ui;background:#10151c;color:#e8e2d2;max-width:1100px;margin:24px auto;padding:0 16px}",
         "h1{font-size:22px}h2{margin-top:36px;border-bottom:1px solid #38bdf8;padding-bottom:4px}",
         "table{border-collapse:collapse;width:100%}td,th{border:1px solid #2a3442;padding:6px 10px;vertical-align:middle}",
         "th{text-align:left;color:#38bdf8}code{background:#1b2430;padding:1px 5px;border-radius:3px;color:#d4a94e}",
         "img{image-rendering:pixelated;max-height:96px;background:#1b2430;border-radius:4px}",
         ".note{color:#9aa8b5}</style>",
         "<h1>NEAT Xianxia — Parked Asset Catalog</h1>",
         "<p class=note><b>Runtime</b> files are loaded by Godot. <b>Parked</b> files are source art, contact sheets, or assets awaiting integration. The use column includes planned uses.</p>"]

    for fam, note, rows in DATA:
        d = os.path.join(P, fam)
        md += [f"\n## `{fam}`", "", note, "", "| ID | Image | Runtime status | Current or planned use |", "|---|---|---|---|"]
        h += [f"<h2><code>{fam}</code></h2><p class=note>{note}</p>",
              "<table><tr><th>ID</th><th>Image</th><th>Runtime status</th><th>Current or planned use</th></tr>"]
        runtime = RUNTIME_FILES.get(fam, set())
        for aid, rel, use in rows:
            p = os.path.join(d, rel)
            status = "Runtime" if rel in runtime else "Parked"
            md.append(f"| `{fam}/{aid}` | ![]({fam}/{rel}) | {status} | {use} |")
            h.append(f"<tr><td><code>{fam}/{aid}</code></td>"
                     f"<td><img src=\"{b64(p)}\"></td><td>{status}</td><td>{html.escape(use)}</td></tr>")
        # family-level extras
        for extra, label in [("atlas.png", "Full atlas grid"), ("sheet.png", "Contact sheet")]:
            ep = os.path.join(d, extra)
            if os.path.exists(ep):
                status = "Runtime" if extra in runtime else "Parked"
                md.append(f"| `{fam}/{extra.split('.')[0]}` | ![]({fam}/{extra}) | {status} | {label} |")
                h.append(f"<tr><td><code>{fam}/{extra.split('.')[0]}</code></td>"
                         f"<td><img src=\"{b64(ep)}\" style=max-height:320px></td><td>{status}</td><td>{label}</td></tr>")
        md.append("")
        h.append("</table>")

    open(os.path.join(P, "ASSET_CATALOG.md"), "w", encoding="utf-8").write("\n".join(md))
    open(os.path.join(ROOT, "ASSET_CATALOG.html"), "w", encoding="utf-8").write("\n".join(h))
    print("catalog written")


if __name__ == "__main__":
    main()
