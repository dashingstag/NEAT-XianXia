# Play and transfer

From the repository root or extracted runtime ZIP, run `python tools/dev.py play`. Supply Godot 4.6.3 using `GODOT_BIN` or a `godot` command on PATH. A source checkout builds the Rust binary first. A runtime ZIP uses its included binary. It is platform-specific. Each session starts its own backend, chooses a free port and stops the backend when the viewer closes.

Use the viewer's Save button before exiting. The default save is `~/.xianxia-handoff/world.snapshot`. Set `XIANXIA_PLAYER_DIR` to choose another profile. A process lock protects concurrent launches using the same profile. A failed load leaves the checkpoint available for investigation.

To continue an existing world, save it in the old game, create a new player profile directory and copy that checkpoint into it as `world.snapshot`. Set `XIANXIA_PLAYER_DIR` to the new directory and launch. Check population, day, treasury and ledger in the paused world before resuming. Keep the prior game and its original checkpoint together as the recovery point. Restoring that pair restores progress to the time of the copy.

## Settlement leadership

Founders select a provisional Camp Leader. A village first elects its leader
after five game years and every five years thereafter. The 30-day nomination
window uses a fixed fee of 10% of the richest living resident's net worth,
rounded up to a whole copper. Residents aged 11 or older can pay the fee into
the settlement treasury and nominate themselves. The fee is non-refundable.
On election day, each eligible resident votes using their relationship with the
candidate and whether they share an aspiration. Ties use a seed tied to the
settlement and election date. The council governs through vacancies.

Town status follows the existing completed-building rule. A town with 500
stable residents and 50 completed buildings qualifies as a city. Promotion
updates the office title without changing the election date. Leaders can
favour aligned public job postings, while residents still decide which jobs
to accept. Living residents with the winner's aspiration at inauguration
receive +8 effective happiness for one game year, capped at 100.
