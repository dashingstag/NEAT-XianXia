---
name: verify-world-rules
description: Validate Xianxia terrain placement, renewable resources, mines and emerging settlement behavior.
---

Run `python tools/dev.py test world` for the fixed-seed placement and finite-stock regressions. Add a second seed when changing geography. Inspect Rust placement rules alongside the Godot geometry; moving only the art can leave invisible rocky ground. For an economy claim, report completed sales, finite stock and living population at a stated tick. Use `test sim` for farming, wildlife and accounting.
