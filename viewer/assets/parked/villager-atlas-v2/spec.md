# villager-atlas-v2 — 16-column x 20-row villager atlas

`atlas.png` (1024×1280) replaces the runtime-generated `viewer/person_atlas.gd`
texture. Same contract: 16 columns (`id % 16` variant) × 20 rows
(0 idle, 1–8 walk, 9–16 work, 17 dead, 18 carry, 19 lying). Cols 0–11 mortal
villagers, 12–15 immortal cultivators (dormant until a cultivation signal lands).

Integration: load `atlas.png` as `detail_mesh.texture` instead of the
SubViewport (committed-texture path), or port the param recipes to draw_*.
Source: `assets/tools/gen_assets.py` (regenerates all PNGs).
