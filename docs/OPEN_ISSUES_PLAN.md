# Open issues: verification and implementation plan

As of 2026-11-23, verified against `main` @ `ade4894`. There are eight open issues. Each one was checked against the source, not only against what its description says.

## Verification summary

| # | Title | Verdict | Notes |
|---|---|---|---|
| 81 | Water-blocked trips grind shorelines | **Fixed and closed** | PR #82 (`d5a8299`) plus PR #83, merged as `24604f0` on 2026-11-23. Checked on the merged revision: 213 lib tests pass and replay digests match. |
| 85 | Teardown audit artifacts are stale | **Confirmed** | `teardown/audit.json:555` still says "server has NO snapshot size cap", but `sim-runner/src/main.rs:440,548` enforces `MAX_VIEWER_FRAME = 4 MiB`. `audit.json:756-761` and `fix-plan.md:56` still describe URBAN_ECONOMY as unlabeled, but `docs/URBAN_ECONOMY.md:39` is labeled now. `audit.json:62` still treats `bank-v12.snapshot` as the only real fixture, while `fixtures/README.md` lists real `dev-v34` and `release-v33`. |
| 84 | Protocol coverage for streamed buildings | **Confirmed** | `testing/protocol/check_protocol.py` (138 lines) has no mention of `stream_buildings`, `building_batch`, `building_offset` or `building_count`. The server side is at `sim-runner/src/main.rs:509-537`. |
| 71 | URBAN_ECONOMY stale paths | **Confirmed, and wider than reported** | The line numbers moved after PR #86: `crates/sim-core/src/urban.rs` is now at line 5 and `--path viewer-godot` at line 66. A third stale path, `crates/neat-core/src/lib.rs`, was added at line 39. `viewer/bench/urban_ui_check.gd` exists. |
| 54 | Legacy readers embed ~18 live types | **Done for v4-13** | Week 2: the length-only pins are replaced by an exact per-reader schema snapshot (`legacy_schema.rs`, `legacy_schema.json`). 47 frozen twins in `legacy_types.rs` landed in four batches (people and councils; urban, banking, education and agriculture; wildlife, world fixtures, civic records and relationships; brain types, `Action` and market lots). No reader v4-13 decodes a live type. v14-18 saves decode the live `World` and are detected but not frozen (follow-up). |
| 48 | Teacher-aligned opportunities in aspiration fixture | **Blocked: harness not in repo** | The `mortal_immortal_refinement` example and the `aspiration_*` JSONL fields are not on `main` or on any remote branch, and the history search found nothing. The Teacher → `GoalKind::Teach` mapping exists (`life.rs:897`, sensor `x[32]` at `life.rs:1093`), and so does `Education::prepare_school_demo` (`education.rs:34`). |
| 62 | Countries, diplomacy, armies | **Valid feature request; nothing exists yet** | No `Country` type exists. Settlement governance is `civic::Council`. Action IDs 0-69 are in use or reserved. New actions take IDs 70 and up, and each one must be mapped down to an existing activity by `Action::wire()` so it fits the wire's 6-bit action field (see the Action ID note under Phase 3). |
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
- **Action IDs.** #60 uses the reserved IDs 46-49. Every other ID up to 69 is taken (`lib.rs:120-141`). New actions get IDs 70 and up, and the viewer will show them as whatever existing activity `Action::wire()` (`lib.rs:148`) maps them to, because the snapshot packs actions into 6 bits; `every_action_remapped_below_the_six_bit_wire_budget` enforces the mapping, and its variant count must be bumped. That is acceptable for background choices but loses detail for headline ones. #62's militia orders (`MusterMilitia`, `EscortConvoy`, `GuardCrossing`, `Disband`) fit better as *council/country* decisions, with villagers only choosing to accept service. Showing new actions distinctly in the viewer means widening the packed frame, which is a wire and save change to coordinate with the integration owner.
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

## Issues raised while verifying (2026-11-23)

- #87: urban zoning thresholds (500 / 100 copper) are literals in four places, including viewer text.
- #88: a viewer reconnect silently resets the selection to agent 0.
- #89: retire the GDScript `house_origin` fallback left after #56.
- #96: saves v14-v33 decode the live `World` base struct (follow-up to #54). Fixed by freezing `LegacyWorldBase`; only the current format decodes the live `World`.
- Comments added to #54 (the length-only schema pin misses same-size reorders) and #48 (the experiment harness is not in the repository).

## Suggested sequencing

```
Week 1   #81 (merge PR #83) → #71 → #85        (docs and audit reflect reality)
         #84                                    (independent, can run in parallel)
Week 2   #54 hash pins → freeze high-churn types
         #48 once the harness is committed
Week 3+  Shared groundwork (Action budget, reservation primitive, freeze touched types)
         #60 Guild → #62 Countries
```
