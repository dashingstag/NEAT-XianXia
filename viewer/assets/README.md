# viewer/assets

Runtime copies of the Xianxia textures committed at `assets/parked/`.
`res://` resolves to this directory's project root, so the files the
renderer loads must live inside `viewer/`. Regenerate the source set with
`python3 assets/tools/gen_assets.py` (repo root), then re-copy with
`cp -r assets/parked viewer/assets/parked`.
