# Open issues: verification and implementation plan

As of 2026-11-23, verified against `main` @ `ade4894`. There are eight open issues. Each one was checked against the source, not only against what its description says.

## Verification summary

| # | Title | Verdict | Notes |
|---|---|---|---|
| 81 | Water-blocked trips grind shorelines | **Partly fixed on main; rest is in open PR #83** | `main` has only the first increment (`213c204`: >96u gate, stale-entry eviction, 2048 cap). The short-hop gate (>24u), convoy trip-start routing and stalled-caravan dissolution are on PR #83 (`98c2554`). That PR is not merged and none of its 5 tests exist on `main`. |
| 85 | Teardown audit artifacts are stale | **Confirmed** | `teardown/audit.json:555` still says "server has NO snapshot size cap", but `sim-runner/src/main.rs:440,548` enforces `MAX_VIEWER_FRAME = 4 MiB`. `audit.json:756-761` and `fix-plan.md:56` still describe URBAN_ECONOMY as unlabeled, but `docs/URBAN_ECONOMY.md:39` is labeled now. `audit.json:62` still treats `bank-v12.snapshot` as the only real fixture, while `fixtures/README.md` lists real `dev-v34` and `release-v33`. |
| 84 | Protocol coverage for streamed buildings | **Confirmed** | `testing/protocol/check_protocol.py` (138 lines) has no mention of `stream_buildings`, `building_batch`, `building_offset` or `building_count`. The server side is at `sim-runner/src/main.rs:509-537`. |
| 71 | URBAN_ECONOMY stale paths | **Confirmed, and wider than reported** | The line numbers moved after PR #86: `crates/sim-core/src/urban.rs` is now at line 5 and `--path viewer-godot` at line 66. A third stale path, `crates/neat-core/src/lib.rs`, was added at line 39. `viewer/bench/urban_ui_check.gd` exists. |
| 54 | Legacy readers embed ~18 live types | **Confirmed (detection landed, freezing not done)** | PR #65 added `legacy_reader_schema_is_pinned` (`lib.rs:4268`) and per-reader doc blocks. Readers still embed live `Patch`, `Road`, `Person`, `Council`, `Farm`, `Tavern`, `Urban`, `Banking` and others (e.g. `legacy_v12.rs:11-44`). The pin test compares **byte lengths only**: a field reorder or a type swap of the same size passes it. |
| 48 | Teacher-aligned opportunities in aspiration fixture | **Blocked: harness not in repo** | The `mortal_immortal_refinement` example and the `aspiration_*` JSONL fields are not on `main` or on any remote branch, and the history search found nothing. The Teacher → `GoalKind::Teach` mapping exists (`life.rs:897`, sensor `x[32]` at `life.rs:1093`), and so does `Education::prepare_school_demo` (`education.rs:34`). |
| 62 | Countries, diplomacy, armies | **Valid feature request; nothing exists yet** | No `Country` type exists. Settlement governance is `civic::Council`. Only 5 spare Action IDs remain: byte 12 packs `action` into its low 6 bits, so IDs 59-63 are free and 46-49 are reserved for guild quests. |
| 60 | Adventurer Guild | **Valid feature request; hooks reserved** | Action IDs 46-49 are reserved for guild quests (`lib.rs:83`). Council sensors `quest_backlog` and `guild_ready` exist (`civic.rs:48-49`). There is no guild building, roster or ration item. |

## Implementation plan

The work is ordered by risk and cost. Each item ends with the evidence its closing comment should cite (see `AGENTS.md`).

### Phase 1: close the nearly-done work (≈1 day)

**1. #81: land PR #83.**
- Rebase or merge PR #83 onto `ade4894`. The only commit since its base is docs, so no conflict is expected.
- Run `python testing/run.py test sim` and expect 213 passed. Run `python tools/dev.py test replay` and confirm the continuous and resumed digests are equal. Run `diag_shore_trace` (3000 agents, seed 42, X4) and confirm `shore_unmoved` stays at single digits at tick 2000.
- Merge, verify the merged revision, post the resolution comment and close #81.

**2. #71: fix the URBAN_ECONOMY paths.**
- Line 5: `crates/sim-core/src/urban.rs` → `sim/crates/sim-core/src/urban.rs`.
- Line 39: `crates/neat-core/src/lib.rs` → `sim/crates/neat-core/src/lib.rs`.
- Line 66: replace the raw `godot --path viewer-godot` command with `python tools/dev.py test viewer`. Keep a direct `--path viewer` form as a secondary option.
- Verify: every backticked path in the doc resolves from the repo root. A one-line grep or `Test-Path` loop is enough.

**3. #85: refresh the teardown bundle.** Do this after #81 and #71 so the audit reflects them.
- Regenerate with the `teardown-audit` skill at the new HEAD. If regeneration is too costly, revise by hand:
  - `legacy_readers`/fixtures part: three real fixtures, with real-bytes coverage by family as in `fixtures/README.md`.
  - Protocol part (`audit.json:555`): a server cap exists (`MAX_VIEWER_FRAME`) and a diagnostic reply is sent. Close the finding, or downgrade it to the reconnect/agent-0 reselect residue if that still holds.
  - `docs_drift` (`audit.json:752-805`, `fix-plan.md:56`): mark it resolved.
- Re-check every excerpt `{file, s, e, text}` against the current file contents. A small script can diff the excerpt text against the file lines and fail on any mismatch, and it can be kept under `teardown/` to stop this from recurring.
- Regenerate `teardown-report.html` from the updated JSON.

### Phase 2: test and robustness debt (≈2-4 days)

**4. #84: protocol scenario for streamed buildings.**
- In `testing/protocol/check_protocol.py`, add a `stream_buildings` scenario:
  1. Send `stream_buildings`. Assert that later XNS1 snapshots carry `building_stream: true` and no embedded building rows.
  2. Collect `building_batch` frames. Assert that `building_offset` starts at 0, is contiguous and is a multiple of 256. Rebuild the array and compare it with the server's building list from a non-streamed snapshot or a `verify` dump.
  3. Assert that the terminal `building_count` truncates. Force a removal, for example by loading a checkpoint with fewer buildings, and check that the rebuilt length shrinks.
  4. Send `reset`, then `load`, and assert that `{"building_count":0}` arrives before any new batch (`main.rs:513`).
- **Fixture requirement:** at least 257 buildings, so a second batch is forced. Check whether `bank-v12` or `dev-v34` already exceeds that. If neither does, add a synthetic scenario in `world/scenarios.toml` that seeds enough buildings, keeping it synthetic per `AGENTS.md`.
- Wire it in `tools/dev.py` (`protocol_suite`, lines 385-392) so `python tools/dev.py test protocol` runs it.

**5. #54: freeze per-version twins for the embedded live types.**
- First, strengthen detection. Replace the byte-length pins with **content hashes** of the postcard bytes (e.g. SHA-256 of a seeded `World::new(4, 7)` plus single-element instances). This catches reorders and type swaps of the same size that length pins miss. It costs little, and it protects the readers during the incremental freeze below.
- Then freeze incrementally, highest churn first. Use `git log --follow` frequency on each type's definition to decide the order. Likely first: `Person`, `Council`, `Urban`, `Banking`, `Farm`, `Tavern`.
  - Add `LegacyX` structs in one shared `legacy_types` module, versioned only where the layout actually differed between v4 and v13. Most types probably need one frozen twin, not one per version.
  - Add a `From<LegacyX> for X` conversion for each, and switch each `legacy_vN::LegacyWorld` field to the twin.
  - Add a compile-time guard: a test or `cargo deny`/grep check that `legacy_*` modules import no live type outside an allowlist, with the allowlist shrinking to empty.
- Verify each type batch the same way: run `legacy_reader` tests, `test replay` on `bank-v12`, and the `real_dev_format_fixture_loads_and_checks` and `real_release_format_fixture_loads_and_checks` tests, and record source and target format plus both hashes. Close #54 only when the allowlist is empty.

**6. #48: unblock the aspiration fixture.**
- Ask the author whether the `mortal_immortal_refinement` example can be committed. Worktree fingerprint `8cd1c187…` suggests it was never pushed. Without it, the acceptance criteria cannot be reproduced.
- Once it is available:
  - Build the fixture from `bank-v12` and add a school using the `Education::prepare_school_demo` path.
  - Make the Teacher-seeker pilot eligible to teach: skill threshold, school assignment and an open `teaching_shifts` slot.
  - Assert that `aspiration_opportunity_decisions > 0` for Teacher on seeds 17, 42 and 77, and that at least one `Teach` goal can complete.
  - Keep it synthetic and do not touch the save format.
- Until then, leave a comment on #48 stating the blocker.

### Phase 3: features (multi-week; build in the order each issue specifies)

These two items share infrastructure, so do the shared groundwork once:
- **Action ID budget.** #60 uses the reserved IDs 46-49. #62 needs at least four orders (`MusterMilitia`, `EscortConvoy`, `GuardCrossing`, `Disband`) plus villager choices, and only 59-63 are left before byte 12's 6-bit action field overflows (`audit.json:408`). Either model militia orders as *council/country* decisions, not villager `Action`s (preferred, since villagers only "accept service"), or widen the packed frame first. Widening is a wire and save change, so coordinate it with the integration owner.
- **Save format bump.** Each feature adds world state. Plan one `XXSAVE35`/`36` per feature. Do #54's freezing of the touched types first, so the new fields do not shift legacy readers.
- **Ledger-backed reservations** (rations, militia pay and supplies, treaty contributions) all need "reserve once, consume or return, idempotent across save/load". Build one reservation primitive on `Ledger` and reuse it.

**7. #60: Adventurer Guild** (first, because hooks already exist and its scope is smaller).
1. Guild building plus state: treasury, roster, job board, ration inventory, expedition records. Add a `JoinGuild` short-term goal and the Curious/Daring/Cautious traits as NEAT inputs. This needs an input-schema bump, so follow the `LEGACY_INPUTS → FRONTIER_INPUTS → INPUTS` pattern. Add the viewer panel.
2. Contracts (survey, retrieve) plus a party state machine (assembling → provisioning → traveling → investigating → returning → reporting) with 1-5 members. Handle withdrawal, death and abandonment. Use Actions 46-49 for `JoinExpedition`, `WaitForSupplies`, `Investigate` and `Return`.
3. Ration item plus a provisioning recipe that buys from the local market, using the shared reservation primitive.
4. A seeded grove site, fog-limited knowledge (reuse `worldmap` fog and the tavern report pattern), one spirit-beast and one terrain encounter, and loot settlement.
5. The issue's verification matrix: end-to-end, interrupted, save/replay mid-expedition, multi-seed including X4, and NEAT dominance checks.

**8. #62: Countries, diplomacy, armies.**
1. A `countries` module: `CountryId`, capital, charter, treasury, and authoritative membership as one `Vec<Option<CountryId>>` indexed by settlement. Add deterministic two-capital placement on the seed, migrate old saves as independent settlements, and add viewer markers.
2. A proposal/vote engine for found, accede, change dues and withdraw, reusing `civic::Council` scoring. Apply results atomically and revalidate when they resolve.
3. Pairwise treaties, starting with the shared-ferry contribution. Record events with stable IDs, and spread knowledge of them through the tavern report system.
4. Militia, modeled as country orders plus villager acceptance (see the Action budget above), reusing convoys, ferries and the reservation primitive.
5. The milestone: on several seeds including X4, reach accession, independence under a treaty, and a third-country outcome from the same rules.

## Suggested sequencing

```
Week 1   #81 (merge PR #83) → #71 → #85        (docs and audit reflect reality)
         #84                                    (independent, can run in parallel)
Week 2   #54 hash pins → freeze high-churn types
         #48 once the harness is committed
Week 3+  Shared groundwork (Action budget, reservation primitive, freeze touched types)
         #60 Guild → #62 Countries
```
