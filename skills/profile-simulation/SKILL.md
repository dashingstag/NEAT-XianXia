---
name: profile-simulation
description: Compare mature-town simulation timings while preserving game behavior and replay.
---

Run `python tools/dev.py bench --scenario mature-town` on the base and candidate revisions on the same machine. Repeat with the same worker count and fixture. Report median and spread across runs, tick p95/p99 and living population. Use replay digests to catch behavior changes for optimizations. Snapshot assembly, Godot frame time and million-agent workloads require their own measurements. Keep the experiment bounded in a disposable worktree.
