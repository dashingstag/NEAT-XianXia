# Teardown remediation plan

Ordered by importance (silent-corruption risk first, then coverage gaps, then mirrors/dual-identity, then docs). Each item names the audit part, the defect class, the fix shape, and how to verify. Per AGENTS.md: before fixing a confirmed defect, search open/closed GitHub issues; coordinate wire/save/tick-order changes with the integration owner; keep repro logs out of commits.

## P0 — `load_ladder`: collapse the repeated header lists
- **Class:** silent-corruption surface. ~10 per-version fixup gates each repeat a hand-maintained non-sequential magic-byte list; missing the newest header silently re-runs init passes on fresh state (the failure the test at `lib.rs:4017` was written for).
- **Fix:** parse the header once into a `SaveVersion` enum/ordinal, then derive each fixup gate as `version < V24`-style comparisons or one `const` capability table. Every gate reads from the same source of truth — "which versions skip init_wildlife" gets written once.
- **Verify:** `cargo test -p sim-core` — `checkpoint_load_preserves_world_state_exactly`, `checkpoint_replays_exactly` must stay green; add a synthetic-header test per gate asserting the boundary version both ways.
- **Risk:** low — refactor only, no format change. Do not touch `checkpoint_payload` order in the same change.

## P1 — `travel_module`/`validate_gate`: prove or close the markets bound gap
- **Class:** coverage gap (verify). Runtime waycamp founding (`travel.rs:3612`, `markets.push`) is uncapped; `validate()` bounds markets to 3..=64 on load. A 62nd waycamp saves fine, then fails reload.
- **Step 1 — prove:** add a soak-style scenario seeding founders until `markets.len() > 64`, or unit-test `settlement_step`'s founding branch directly with a saturated `markets` vec. If unreachable, document why (e.g. founder scarcity) and mark the bound carve-out.
- **Step 2 — fix (if proven):** gate founding at `markets.len() < 64` with a visible outcome (founders stay in camp state / market deferred), since load rejects >64.
- **Also:** make `check()` run `self.validate()`'s pool-length/market sweep so runtime and load-time invariants agree — then the gap fails loudly at save time, not at next reload.
- **Verify:** `python tools/dev.py test sim` + targeted unit test asserting founding refusal at the bound.

## P2 — `action_enum`: make the three encodings machine-checked
- **Class:** hand-maintained contract held by convention. Enum serves wire repr (6-bit ceiling + remap), postcard declaration index, and the release-layout remap (39–41). Today it's consistent — verified in audit — but nothing mechanical stops a violation.
- **Fix:** (a) a test that iterates all `Action` variants, asserts `wire_action(v) < 64` after the remap for every variant ≥64, and asserts the remap match is exhaustive; (b) a test pinning the declaration indices of `HuntWildlife=39, EatCarcass=40, CraftSpear=41` (postcard encodes decl index — the release remap depends on it); (c) `#[repr(u8)]` + compile-fail comment is already there — add the coverage test, that's the missing half.
- **Verify:** new tests + `cargo test -p sim-core`. Cheap, high-leverage — do early.

## P3 — `legacy_readers`: freeze the remaining embedded types
- **Class:** half-done freeze. `legacy_v12/v13` froze Agents+Market but ~18 nested types (Person, Patch, Council, Urban, Wildlife, Banking, Education, Genome, Innovations…) are still live imports — a field edit to any of them changes what old bytes deserialize to.
- **Fix:** move embedded types used by legacy readers into `legacy_types` frozen twins, per the ARCHITECTURE_REVIEW recommendation; prioritize the types most likely to evolve (Person, Urban, Wildlife, Council). Add per-reader comment blocks naming any live type still trusted.
- **Verify:** `python tools/dev.py test protocol` + `replay` — the v12 fixture must still load and step; add unit deserializers for the frozen types.
- **Effort:** largest item — mechanical but wide. Sequence after P0 (shared code region).

## P4 — `fixture_bank_v12`: real bytes for each format family
- **Class:** single-version real-bytes coverage. Only XXSAVE12 exists; dev-format (20–29,34) and release-format (19,30–33) paths are exercised only via synthetic bytes built from current types.
- **Fix:** check in one real save per family (one dev-format ≤29, one release-format 30–33) into `fixtures/` and add them to `project.toml [contracts]` + the protocol/replay suites. Source them from historical builds/releases if available; if none exist, mark the gap `verify` in perpetuity and strengthen the synthetic tests.
- **Verify:** both fixtures load + step 50 ticks + `check()` passes in the protocol suite.

## P5 — `house_origin_mirror`: replace the duplicated geometry with a shared source
- **Class:** hand-mirrored cross-language contract. `rest.rs:587` and `field.gd:965` are byte-identical math today; the generated-map paths already diverge (`map.dry_ground` vs `terrain.dry_point`).
- **Fix (preferred):** export bed/house origins in the static `map_layers` payload — the viewer reads positions, doesn't compute them. If too invasive now, add a parity test: emit 30 test vectors (house idx → xy) from Rust, assert `field.gd`'s function matches within ε in a headless bench script.
- **Verify:** new parity check in `dev.py test viewer`/`connection`; picking and visuals unchanged.

## P6 — `viewer_tcp`: reconcile the 4MB frame cap
- **Class:** asymmetric bound (verify). Server has no snapshot cap; viewer rejects frames >4MB → reconnect loop on oversized worlds.
- **Step 1 — prove:** construct a snapshot exceeding 4MB (large metadata + 20k records + animals) in a bench/soak run; confirm the viewer's "Invalid native snapshot frame" path fires.
- **Step 2 — fix:** either server-side bound with an explicit error reply, or raise/share the constant in one place (a `const` in `contracts` + comment cross-references in both languages).
- **Verify:** targeted test asserting frame < cap at max world size, or the explicit error path.

## P7 — `memory_pilot`: isolate the experiment from production state
- **Class:** dual-identity fusion. `Trial::adjust` mutates candidate scores and `execute_with_memory_pilot` swaps the executor inside `step()`; `watch` is `#[serde(skip)]` so a save/load silently drops an active trial while genomes shaped by it persist.
- **Fix:** persist the trial's active flag (or its seed/config) in a checkpoint tail so save/load preserves the experimental state; alternatively feature-gate the pilot behind a build flag / `--pilot` flag documented as non-replay-safe. Minimum: comment on `watch` noting trials don't survive save/load.
- **Verify:** save→load during an active trial resumes identical decisions (digest equality over 50 ticks).

## P8 — `world_monolith`: document the append-tails schema convention
- **Class:** implicit contract. Checkpoint state past `relationships` serializes via `#[serde(skip)]` + `checkpoint_payload` tuple tails; a new non-skip field silently breaks all saves ≥v19 (loud at deserialize, but the convention deserves to be explicit).
- **Fix:** one doc comment on `struct World`: "checkpoint state added after `relationships` must be `#[serde(skip)]` and appended to `checkpoint_payload`'s tail tuples." Optionally a compile-time marker test asserting the base struct's serde layout is stable.
- **Verify:** trivial — no behavior change. Pair with P0's version-table work since it's the same code region.

## P9 — `docs_drift`: date the stale design docs
- **Class:** documentation fiction. `URBAN_ECONOMY.md:37` claims a "stable 42-input contract" (now 52).
- **Fix:** add an "as-of <date/rev>" header marking it historical (the `ARCHITECTURE_REVIEW.md` pattern), or update the numbers and cross-link `neat_core::INPUTS`.
- **Verify:** none needed beyond review.

## Sequencing

```
P0 load_ladder ──► P3 legacy_readers ──► P4 fixtures      (save-boundary chain)
P1 markets bound (prove → cap)                            (independent)
P2 action tests    P5 house parity    P6 4MB cap          (mirror guards, parallel)
P7 memory_pilot                        P8 doc comment     (smaller, any time)
P9 docs drift                                             (any time)
```

P0→P3→P4 share the save-load code region and should land in order to avoid rewrites. P1 needs its proving experiment first — it may resolve as "documented unreachable" (cheap) or "cap founding" (a gameplay-visible change → integration-owner coordination). P2, P5, P6 are independent test-additions that can run in parallel. P8 is one comment — fold into whichever PR touches `struct World` first.

## Repo verification skills (skills/README.md)

- **P0, P3, P4** → `review-save-migration`: establish old-field defaults + invariants, run `test replay` + `test protocol`, verify a fixture copy with the affected field absent, record source/target format + both hashes.
- **P0, P1, P3, P7** → `verify-replay`: `test replay` SHA-256 digests after 50 ticks on the bank-v12 fixture; divergence triage order = RNG state → tick order → rebuilt indexes.
- **P5, P6** → `verify-viewer-fixture`: `test viewer` (completion marker, counts, fidelity levels) + `preview --fixture town-shop`; the fixture frame has no moving humans — live-session check needed for movement claims.
- **P1** → `verify-world-rules`: settlement/camp founding behavior lives in its remit — use it for the "markets > 64 reachable?" proving experiment.
- **profile-simulation** → only if any fix touches the tick hot path; P0's gate-table refactor is the likeliest candidate.

After fixes land: regenerate `teardown/audit.json` findings that changed, rerun `python tools/dev.py test all`, and per AGENTS.md file/close GitHub issues for anything that was a real reproducible defect (P1 if proven, P6 if proven).
