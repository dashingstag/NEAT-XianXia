# Teardown remediation plan

Refreshed 2026-11-23 against the revision that carries this file. The original plan (2026-11-22, `d786376`) listed P0-P9. Nine of them have since landed. This version records what resolved each one and lists only the work still open. `python teardown/check_excerpts.py` confirms that every excerpt in `audit.json` still matches the source it quotes.

## Status of the original items

| Item | Audit part | Status | Resolved by |
|---|---|---|---|
| P0 | `load_ladder` | Done. The header is parsed once into `SaveVersion` (`lib.rs:563-610`), and every gate reads a named capability. | #61 (`1401b6b`) |
| P1 | `travel_module`, `validate_gate` | Done. Founding stops at `MAX_MARKETS`, the same constant the load gate uses, and `check()` runs `validate_schema()`. | #63 / #78 (`42e979a`) |
| P2 | `action_enum` | Done. `every_action_remapped_below_the_six_bit_wire_budget` and `release_remap_declaration_indices_are_pinned`. | #64 (`fa2a3aa`) |
| P3 | `legacy_readers` | Done. Every historical layout is frozen and traced: reader twins (#54), the v14-v33 `World` base (#96) and the v15-v33 tail payloads (#97). | #91-#94, #96, #97 |
| P4 | `fixture_bank_v12` | Done. Real `dev-v34` and `release-v33` writer output is checked in and loaded by tests. | #79 (`0badb84`), #80 (`c11da31`) |
| P5 | `house_origin_mirror` | Done. `house_positions` ships on the wire, and the GDScript fallback is removed. | #67 (`179b191`), #89 |
| P6 | `viewer_tcp` | Done. `MAX_VIEWER_FRAME` is shared, and the server drops oversize snapshots with a diagnostic. | #57 / #77 (`191672e`) |
| P7 | `memory_pilot` | Done for save/load: `checkpoint_payload` refuses to save during a trial, and the example is feature-gated. | #75 (`c7d1aaa`) |
| P8 | `world_monolith` | Done. The append-tails rule is documented on `struct World` (`lib.rs:388-392`). | #61 |
| P9 | `docs_drift` | Done. URBAN_ECONOMY is labeled historical, annotates the sensor counts and its paths resolve. | #59 / #70 (`3bd7e1d`), #71 |

## Open work

### R1 — `legacy_readers`: every historical layout frozen (#54, #96, #97) — done
- Readers v4-v13 use the frozen reader twins (#54). Saves v14-v33 decode their `World` base through `LegacyWorldBase` (#96), and the payloads v15-v33 append after it through per-module `Legacy*` twins via `World::take_as` (#97). Only the current format decodes live types.
- `legacy_schema.json` traces all of it. A change to a type the current format writes fails a `current-world*` entry with a "bump the save format" message.
- **On the next format bump:** set `SaveVersion::CURRENT`, update the writer's header, move the replaced version onto the twins, and regenerate the snapshot.

### R2 — `house_origin_mirror`: retire the fallback (#89) — done
- `field.gd` now reads only the sim's `house_positions`; the hand copy of `rest.rs` `house_origin` is deleted, and a missing entry draws at the town centre. `urban_ui_check.gd` asserts every fixture house position and click target against `house_positions` (33 houses).

### R3 — `main_gd` / `urban_module`: render exported thresholds (#87) — done
- `urban.rs` names `GAZETTE_THRESHOLD` and `LEASE_REVIEW_THRESHOLD`. The two rules, the snapshot export and the viewer text (`main.gd`) all read them, and `gazette_offer_and_viewer_export_use_the_threshold_constants` pins the boundary and the export.
- The town promotion requirement followed in #99: `TOWN_BUILDINGS_PER_KIND` is exported as `urban.town_buildings_per_kind`, and the viewer renders both the explanation and the progress denominators from it.

### Lower priority (no fix planned; watch on re-audit)
- `urban_module` and `world_monolith` stay orange as fusion findings. They are mandates to keep moving logic out incrementally, not defects.
- `contract_readme`: the save-history paragraph is hand-written prose. `SaveVersion` is the authority. Update both together when the format is bumped.

## Keeping the audit current

- Run `python teardown/check_excerpts.py` after changes that touch cited code. Run `--fix` to move excerpts whose text only moved. Rewrite any excerpt reported as `text not found`, and re-score its part.
- Re-render the report with the teardown-audit skill: `node emit-report.mjs teardown/audit.json teardown/teardown-report.html`.
- When a finding resolves, update its score and `whyScore` with the PR that fixed it. Drop its `fixes` entry, and update the status table above.
