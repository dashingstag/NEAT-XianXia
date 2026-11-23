# Teardown remediation plan

Refreshed 2026-11-23 against the revision that carries this file. The original plan (2026-11-22, `d786376`) listed P0-P9. Nine of them have since landed. This version records what resolved each one and lists only the work still open. `python teardown/check_excerpts.py` confirms that every excerpt in `audit.json` still matches the source it quotes.

## Status of the original items

| Item | Audit part | Status | Resolved by |
|---|---|---|---|
| P0 | `load_ladder` | Done. The header is parsed once into `SaveVersion` (`lib.rs:563-610`), and every gate reads a named capability. | #61 (`1401b6b`) |
| P1 | `travel_module`, `validate_gate` | Done. Founding stops at `MAX_MARKETS`, the same constant the load gate uses, and `check()` runs `validate_schema()`. | #63 / #78 (`42e979a`) |
| P2 | `action_enum` | Done. `every_action_remapped_below_the_six_bit_wire_budget` and `release_remap_declaration_indices_are_pinned`. | #64 (`fa2a3aa`) |
| P3 | `legacy_readers` | **Partial.** Exact schema snapshot per reader; frozen twins landing in batches (batch 1: Person, Council, Ledger, Statistics; batch 2: urban, banking, education, farms, taverns). 26 shared types remain. |
| P4 | `fixture_bank_v12` | Done. Real `dev-v34` and `release-v33` writer output is checked in and loaded by tests. | #79 (`0badb84`), #80 (`c11da31`) |
| P5 | `house_origin_mirror` | **Mostly done.** `house_positions` ships on the wire. The GDScript fallback remains for frames recorded without it. | #67 (`179b191`) |
| P6 | `viewer_tcp` | Done. `MAX_VIEWER_FRAME` is shared, and the server drops oversize snapshots with a diagnostic. | #57 / #77 (`191672e`) |
| P7 | `memory_pilot` | Done for save/load: `checkpoint_payload` refuses to save during a trial, and the example is feature-gated. | #75 (`c7d1aaa`) |
| P8 | `world_monolith` | Done. The append-tails rule is documented on `struct World` (`lib.rs:388-392`). | #61 |
| P9 | `docs_drift` | Done. URBAN_ECONOMY is labeled historical, annotates the sensor counts and its paths resolve. | #59 / #70 (`3bd7e1d`), #71 |

## Open work

### R1 — `legacy_readers`: finish the freeze (#54)
- **Status:** detection is exact and the first batch is frozen. `legacy_schema::legacy_reader_schemas_are_pinned` traces every reader's full serde schema with `serde-reflection` and compares it with `sim-core/src/legacy_schema.json`. This replaced the byte-length pins. `legacy_types.rs` freezes `Person`, `Council`, `Ledger`, `Statistics` and their nested types for readers 6-13.
- **Remaining:** 26 shared types, listed in `STILL_LIVE` (`legacy_schema.rs`). `legacy_readers_trust_no_new_live_types` stops the list from growing.
- **Fix:** freeze the rest in batches. Batch 3 is wildlife, the world fixtures (`Patch`, `Road`, `Shelter`, `Convoy`), civic records (`Corpse`, `Contract`, `Graveyard`, `JobKind`) and `Relationships`. Batch 4 is the neat-core brain types (`Genome`, `Gene`, `Node`, `Kind`, `Innovations`, `Rng`), `Action`, and the v4/v5 `Lot`/`Project`/`Quote`. Each twin removes its name from `STILL_LIVE`; regenerate the snapshot with `UPDATE_LEGACY_SCHEMA=1` only after switching a reader to a twin.
- **Verify:** `cargo test -p sim-core --lib legacy_`, plus `python tools/dev.py test replay` and `test protocol` on `bank-v12`. The replay digest must not change.

### R2 — `house_origin_mirror`: retire the fallback (#89)
- **Class:** residual hand-mirror. `field.gd:447-457` prefers the sim's `house_positions`. Frames without them use `_house_origin`, which copies the `rest.rs:587` constants and has no parity test.
- **Fix:** re-record any fixture that lacks `house_positions`, then delete the fallback. Otherwise add a parity check for a ring of indices.
- **Verify:** `python tools/dev.py test viewer` and `test connection`.

### R3 — `main_gd` / `urban_module`: render exported thresholds (#87)
- **Class:** fragile seam. `main.gd:1017` prints "Gazette at 500 copper. Lease review below 100." The snapshot already exports `urban.wealth_threshold` and `reserve_threshold` (`urban.rs:2369`).
- **Fix:** build the text from the exported fields.
- **Verify:** `python tools/dev.py test viewer`.

### Lower priority (no fix planned; watch on re-audit)
- `urban_module` and `world_monolith` stay orange as fusion findings. They are mandates to keep moving logic out incrementally, not defects.
- `contract_readme`: the save-history paragraph is hand-written prose. `SaveVersion` is the authority. Update both together when the format is bumped.

## Keeping the audit current

- Run `python teardown/check_excerpts.py` after changes that touch cited code. Run `--fix` to move excerpts whose text only moved. Rewrite any excerpt reported as `text not found`, and re-score its part.
- Re-render the report with the teardown-audit skill: `node emit-report.mjs teardown/audit.json teardown/teardown-report.html`.
- When a finding resolves, update its score and `whyScore` with the PR that fixed it. Drop its `fixes` entry, and update the status table above.
