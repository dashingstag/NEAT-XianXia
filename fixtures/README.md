# Fixture provenance

- **bank-v12.snapshot** is the synthetic bank-demo checkpoint from the prototype experiments. It includes an active lending economy and exercises the schema 12 reader. SHA-256 `d0862fd21cf4d13bc30bbabbdc2b3c891c5973921251f0ea340b745d2d8d9dff`. Preserve the original bytes as a compatibility input.
- **dev-v34.snapshot** is real binary output: the release sim-runner loaded `bank-v12.snapshot`, stepped 75 ticks and re-saved as `XXSAVE34`. SHA-256 `187298481d3eea0162737060e85671cfc8f10ec894e48ca3acc7d3a22967e91d` — identical to the `test replay` digest it was produced under. It freezes the current tuple+tails layout (versions 20-29/34 family) as historical bytes: a `World` serde change that forgets to bump the format or breaks v34 reading fails against it. Regenerate by running `python tools/dev.py test replay` and copying the run's `world.snapshot`.
- **town-shop.json** is the recorded relationships UI fixture from a synthetic bank-demo continuation. SHA-256 `59b63d65748384458412caeb973a7fce3bed42446484fb0dabcf47cd4c8f8c08`. It contains authentic snapshot metadata and selected-person data. The headless viewer harness constructs a frame with zero moving samples to exercise panels, world sites and picking.

## Real-bytes coverage by format family

- **Legacy readers (v4-18):** `bank-v12.snapshot` is the only real artifact; other versions have synthetic readers tests only.
- **Development tuple+tails (v20-29, 34):** `dev-v34.snapshot` covers the tail-chain layout with real bytes; v20-29 tail ordering is pinned synthetically by `tests::dev_format_tail_walk_is_pinned`.
- **Release tuple+Action-remap (v19, 30-33):** no historical release build survives to produce real bytes — this gap is documented. The synthetic `tests::release_checkpoints_restore_their_original_action_indices` pins that family's tail order and index remap instead.

`python tools/dev.py test protocol` writes a fresh town-shop.json in its own run directory. Compare the changed fields and run the viewer check before deliberately updating a checked-in fixture. Use the replay command for full-state equality on the same build and machine. Game balance changes may legitimately change a newly generated frame.
