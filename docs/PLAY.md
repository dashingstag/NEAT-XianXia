# Play and transfer

From the repository root or extracted runtime ZIP, run `python tools/dev.py play`. Supply Godot 4.6.3 using `GODOT_BIN` or a `godot` command on PATH. A source checkout builds the Rust binary first. A runtime ZIP uses its included binary. It is platform-specific. Each session starts its own backend, chooses a free port and stops the backend when the viewer closes.

Use the viewer's Save button before exiting. The default save is `~/.xianxia-handoff/world.snapshot`. Set `XIANXIA_PLAYER_DIR` to choose another profile. A process lock protects concurrent launches using the same profile. A failed load leaves the checkpoint available for investigation.

To continue an existing world, save it in the old game, create a new player profile directory and copy that checkpoint into it as `world.snapshot`. Set `XIANXIA_PLAYER_DIR` to the new directory and launch. Check population, day, treasury and ledger in the paused world before resuming. Keep the prior game and its original checkpoint together as the recovery point. Restoring that pair restores progress to the time of the copy.
