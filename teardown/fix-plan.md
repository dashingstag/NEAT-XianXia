# Teardown remediation plan

Refreshed 2026-11-23 against the revision that carries this file. The original plan (2026-11-22, `d786376`) listed P0-P9. Nine of them have since landed. This version records what resolved each one and lists only the work still open. `python teardown/check_excerpts.py` confirms that every excerpt in `audit.json` still matches the source it quotes.

## Status of the original items

| Item | Audit part | Status | Resolved by |
|---|---|---|---|
| P0 | `load_ladder` | Done. The header is parsed once into `SaveVersion` (`lib.rs:563-610`), and every gate reads a named capability. | #61 (`1401b6b`) |
| P1 | `travel_module`, `validate_gate` | Done. Founding stops at `MAX_MARKETS`, the same constant the load gate uses, and `check()` runs `validate_schema()`. | #63 / #78 (`42e979a`) |
| P2 | `action_enum` | Done. `every_action_remapped_below_the_six_bit_wire_budget` and `release_remap_declaration_indices_are_pinned`. | #64 (`fa2a3aa`) |
| P3 | `legacy_readers` | **Done for v4-13.** Exact schema snapshot per reader, and 47 frozen twins in four batches; no reader v4-13 decodes a live type. v14-18 (bare `World`) are detected but not frozen; that is a follow-up issue. | #91 and the batch PRs after it |
| P4 | `fixture_bank_v12` | Done. Real `dev-v34` and `release-v33` writer output is checked in and loaded by tests. | #79 (`0badb84`), #80 (`c11da31`) |
| P5 | `house_origin_mirror` | **Mostly done.** `house_positions` ships on the wire. The GDScript fallback remains for frames recorded without it. | #67 (`179b191`) |
| P6 | `viewer_tcp` | Done. `MAX_VIEWER_FRAME` is shared, and the server drops oversize snapshots with a diagnostic. | #57 / #77 (`191672e`) |
| P7 | `memory_pilot` | Done for save/load: `checkpoint_payload` refuses to save during a trial, and the example is feature-gated. | #75 (`c7d1aaa`) |
| P8 | `world_monolith` | Done. The append-tails rule is documented on `struct World` (`lib.rs:388-392`). | #61 |
| P9 | `docs_drift` | Done. URBAN_ECONOMY is labeled historical, annotates the sensor counts and its paths resolve. | #59 / #70 (`3bd7e1d`), #71 |

## Open work

### R1 — `legacy_readers`: v14-18 bare-World saves (follow-up to #54)
- **Status:** readers v4-13 are fully frozen. `legacy_types.rs` holds 47 twins, `STILL_LIVE` is empty, and `legacy_schema.json` pins every reader's traced schema.
- **Remaining:** v14 saves (and v15-18) decode the live `World` directly, so a change to a type inside `World` changes how they decode. The `v14-bare-world` entry in the snapshot fails on any such change.
- **Fix:** when a type inside `World` first needs to change shape, freeze a `World` twin for v14-18 that matches the snapshot's `v14-bare-world` entry. Agents and Market need current-layout twins, and `map` needs its `default_checkpoint_map` default.
- **Verify:** `cargo test -p sim-core --lib legacy_`, plus `python tools/dev.py test replay` and `test protocol` on `bank-v12`.

### R2 — `house_origin_mirror`: retire the fallback (#89)
- **Class:** residual hand-mirror. `field.gd:447-457` prefers the sim's `house_positions`. Frames without them use `_house_origin`, which copies the `rest.rs:587` constants and has no parity test.
- **Fix:** re-record any fixture that lacks `house_positions`, then delete the fallback. Otherwise add a parity check for a ring of indices.
- **Verify:** `python tools/dev.py test viewer` and `test connection`.

### R3 — `main_gd` / `urban_module`: render exported thresholds (#87) — done
- `urban.rs` names `GAZETTE_THRESHOLD` and `LEASE_REVIEW_THRESHOLD`. The two rules, the snapshot export and the viewer text (`main.gd`) all read them, and `gazette_offer_and_viewer_export_use_the_threshold_constants` pins the boundary and the export.
- Left over: the town promotion text ("3 completed homes, shops and workshops") is still literal in `main.gd`.

### Lower priority (no fix planned; watch on re-audit)
- `urban_module` and `world_monolith` stay orange as fusion findings. They are mandates to keep moving logic out incrementally, not defects.
- `contract_readme`: the save-history paragraph is hand-written prose. `SaveVersion` is the authority. Update both together when the format is bumped.

## Keeping the audit current

- Run `python teardown/check_excerpts.py` after changes that touch cited code. Run `--fix` to move excerpts whose text only moved. Rewrite any excerpt reported as `text not found`, and re-score its part.
- Re-render the report with the teardown-audit skill: `node emit-report.mjs teardown/audit.json teardown/teardown-report.html`.
- When a finding resolves, update its score and `whyScore` with the PR that fixed it. Drop its `fixes` entry, and update the status table above.
