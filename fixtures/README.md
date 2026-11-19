# Fixture provenance

- **bank-v12.snapshot** is the synthetic bank-demo checkpoint from the prototype experiments. It includes an active lending economy and exercises the schema 12 reader. SHA-256 `d0862fd21cf4d13bc30bbabbdc2b3c891c5973921251f0ea340b745d2d8d9dff`. Preserve the original bytes as a compatibility input.
- **town-shop.json** is the recorded relationships UI fixture from a synthetic bank-demo continuation. SHA-256 `59b63d65748384458412caeb973a7fce3bed42446484fb0dabcf47cd4c8f8c08`. It contains authentic snapshot metadata and selected-person data. The headless viewer harness constructs a frame with zero moving samples to exercise panels, world sites and picking.

`python tools/dev.py test protocol` writes a fresh town-shop.json in its own run directory. Compare the changed fields and run the viewer check before deliberately updating a checked-in fixture. Use the replay command for full-state equality on the same build and machine. Game balance changes may legitimately change a newly generated frame.
