# Runtime contracts

## Proposed watched-purchase inspector contract

For the last-item acceptance scenario, expose an optional `last_decision` on the selected person. A missing or null value means no captured decision. Keep the existing `choices` as a preview computed at inspection time. Capture the shop and owner IDs when deciding so a later ownership change cannot rewrite history.

Example for the buyer who reached an empty shelf:

```json
{"decision_tick":120,"actor_id":4,"action":"ShopGoods","shop_id":7,"owner_id":2,"product_id":17,"resolution_tick":124,"outcome":"out_of_stock","quantity":0,"buyer_charge":0,"shop_proceeds":0,"market_levy":0}
```

Before arrival, resolution tick and outcome are null. For the successful buyer, outcome is `purchased`, quantity is one, and the charge equals shop proceeds plus the levy for this meal. Read actual settled amounts from the transaction. Missing history stays distinguishable from an attempt that failed. These fields specify the intended inspector extension and are pending implementation.

The server owns the world and accepts a local viewer. The handoff launcher binds each backend to loopback port zero and reads its assigned port and process ID from the child's atomic readiness file. It passes that port to the viewer via XIANXIA_PORT. Reset-capable experiments receive a socket from their owning launcher.

Each wire message has a four-byte little-endian payload length. Commands are UTF-8 JSON. Replies begin with XNC1 and JSON. Snapshot payloads begin with XNS1, followed by little-endian version (u16), flags (u16, bit zero means paused), tick (u64), allocated count (u32), sample count (u32), and metadata byte length (u32). JSON metadata and sample records follow. Each 16-byte sample contains ID (u32), x/y (f32), and four bytes for action/flags, cargo/flags, health/age flag, and home. Action IDs occupy six bits. Coordinate and ID changes must agree across the encoder, field renderer and picking implementation.

Current save format is XXSAVE13 with explicitly migrated historical versions. The included v12 fixture exercises the introduction of relationship state. Saves 1 through 3 predate the supported migration range. A wire change and a save change require separate compatibility decisions. For a new persistent field, define the old-world default and test load, advance, save, reopen and replay. For a new optional inspector field, exercise both an older frame and a frame containing the field. Keep new global reports out of the fast path where possible.
