# lod — three-fidelity sprite set for every map object

Companion set to the existing families, organised by `field.gd`'s zoom
fidelity contract (effective zoom <2 = L1 overview, <5 = L2 drawn,
>=5 = L3 animated). Parked like the rest of `assets/parked/` pending
integration approval.

- `dots/<id>.png` — 24px L1 markers: dots + landmark shapes matching the
  live overview conventions (settlement stage shapes, ore diamonds,
  action-colored people dots, species animal dots).
- `l2/<id>.png` — 64x64 L2 cells for objects that have no parked glyph
  yet: settlement clusters, convoy, rally, roads, rivers, coast, hill,
  fog, zones, silver/copper mine variants.
- `anim/<id>.png` — horizontal 8x64px frame strips (4 for animals/reveal),
  the format the viewer would consume at L3.
- `apng/<id>.png` — animated previews at ~10fps (the shader clock rate)
  for the review board.
- `scene-l{1,2,3}.png` — one market street composed at each level.
- `manifest.json` + `LOD_BOARD.md` — board index.

Review board: `assets/LOD_BOARD.html` (self-contained, animated).
Regenerate: `python3 assets/tools/gen_lod.py && python3
assets/tools/build_lod_board.py` — grammar and palette shared with
`gen_assets.py`.
