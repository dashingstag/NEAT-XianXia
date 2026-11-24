# Contributing to NEAT Xianxia

Start with an observable result. For example, two villagers compete for the last item at a shop. One receives it, the seller receives a single payment, and the ledger balances. A seed, a tick and the expected outcome make simulation changes much easier to review.

## Local development

Fork the repository, clone your fork and create a branch from the latest upstream main. Follow the [setup instructions](README.md#run-from-source). To try an earlier browser prototype with Node alone, run `npm start` from its folder under labs. Run `npm test` only in labs that define a test script.

| If you change | Start with |
|---|---|
| NPC choices, resources, transactions or family rules | Add a deterministic case for the changed behavior and check account conservation. |
| Save fields, tick order or randomness | Verify defaults for old saves and deterministic continuation. |
| Inspectors, drawing, animation or picking | Exercise fixture-backed UI behavior and include a screenshot or short recording for visible changes. |
| Terrain or settlement placement | Record the seed and check that drawing and placement agree. |
| Launchers or server lifetime | Verify concurrent runs keep separate backends and saves, and that the viewer connects to its own backend. |

Use the [testing guide](testing/README.md) for current command syntax. For independent agent work, use separate branches and worktrees. Agree on the example fixture and an owner for shared protocol or save-format edits. The [agent guide](AGENTS.md) lists the entry points.

## Issues and pull requests

Use a bug report for reproducible problems and a feature proposal for changes to game behavior. Include synthetic or minimal examples. Check screenshots and logs before sharing personal file paths or player data.

Open a pull request to main with the player-visible result, test commands and observed output. Mention save compatibility or performance effects when applicable. Windows CI runs the combined suites and builds a runtime ZIP. Contributors can retrieve the results from the Actions run attached to the pull request.
