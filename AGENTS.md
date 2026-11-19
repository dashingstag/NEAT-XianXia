# Working on NEAT Xianxia

## GitHub issue lifecycle

When work reveals a reproducible defect or a confirmed limitation, search open and closed GitHub issues first. Add evidence to an existing match or open an issue describing the problem, expected and observed behavior, affected code or reproduction, and how to verify a fix. Clearly label suspected impacts. Link related issues in the pull request. For partial fixes use a reference that leaves the issue open.

After the complete fix merges, verify the merged revision and add a closing comment listing the steps taken, the merged PR or commit and the checks that passed. Close the issue then. If GitHub already closed it on merge, still add the resolution comment. Keep temporary logs and personal saves outside commits and public reports.

Start with README.md and the command table. Run from the repository root. Use synthetic fixtures for verification. The runner owns test processes, ports and checkpoints. The interactive play command uses a persistent profile.

After the first Git commit, give concurrent contributors separate branches and worktrees. Agree on the player-visible outcome, affected data, owner and a small example before cross-domain work. A passing test report should identify the revision, scenario, command and observed result. Recheck the combined revision before release.

| Area | Entry points and expectations |
|---|---|
| Simulation | sim/crates/sim-core and neat-core. Preserve physical feasibility, ordered resolution, conservation, stable IDs and reproducible randomness. Include a contested, interrupted or stale action when relevant. |
| UI | viewer/main.gd and viewer/bench. Exercise panels against fixtures. For cross-panel navigation, check the resolved entity ID. |
| Visuals | viewer/field.gd, shaders and atlas. Verify orientation, camera following, site picking and zoom. Headless tests cover logic. Capture interactive frame time and screenshots for appearance or performance claims. |
| World | world/scenarios.toml and sim-core landscape, agriculture and wildlife modules. Check placement, finite stocks and multi-seed outcomes. Economic labels derive from actual sales. |

Coordinate edits to tick order, shared IDs, wire format, save layout and the two large Godot scripts with the integration owner for that change. Move panels or map layers out incrementally when a task gives them independent ownership. Reviewers can reproduce workflows from [the skill index](skills/README.md). Use the original brief and a fresh run directory during independent verification.
