---
name: verify-server-isolation
description: Reproduce accidental sharing between simultaneous Xianxia test sessions or validate launcher changes.
---

Use `python tools/dev.py test isolation` with the normal runner-owned child servers. Read isolation.json and check each child readiness record. The interrupted child's socket must close while the survivor loads its unchanged save and advances. For a regression, give the implementer a failing command, affected process or checkpoint and expected survivor state. Re-run from a separate worktree.
