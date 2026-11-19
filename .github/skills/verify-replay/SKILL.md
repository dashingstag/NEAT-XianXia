---
name: verify-replay
description: Check deterministic continuation or a migration of a Xianxia world after changing save fields, simulation ordering or RNG.
---

Run `python tools/dev.py test replay`. It starts with the checked-in bank-v12 copy and records SHA-256 digests of both current-format checkpoints after 50 additional ticks. Return the two digests and the run directory. On divergence, compare RNG state, tick order and rebuilt indexes at the midpoint before altering the fixture. Use `test protocol` for visible bank, ledger and relationship agreement.
