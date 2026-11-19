# Contributing to NEAT Xianxia

Start with an observable result. For example, two villagers compete for the last item at a shop. One receives it, the seller receives a single payment, and the ledger balances. A seed, a tick and the expected outcome make simulation changes much easier to review.

## Local development

Fork the repository, clone your fork and create a branch from the latest upstream main. Follow the [setup instructions](README.md#run-from-source). To try an earlier browser prototype with Node alone, run `npm start` from its folder under labs. Run `npm test` there to check the experiment.

| If you change | Start with |
|---|---|
| NPC choices, resources, transactions or family rules | `python tools/dev.py test sim`. Add a deterministic case for the behavior you changed. |
| Save fields, tick order or randomness | `python tools/dev.py test replay` and `python tools/dev.py test protocol`. State the default for people loaded from an old save. |
| Inspectors, drawing, animation or picking | `python tools/dev.py test viewer`. Include a screenshot or short recording for visible changes. |
| Terrain or settlement placement | `python tools/dev.py test world`. Record the seed and check that drawing and placement agree. |
| Launchers or server lifetime | `python tools/dev.py test isolation` and `python tools/dev.py test connection`. |

For independent agent work, use separate branches and worktrees. Agree on the example fixture and an owner for shared protocol or save-format edits. The [agent guide](AGENTS.md) lists the entry points.

## Issues and pull requests

Use a bug report for reproducible problems and a feature proposal for changes to game behavior. Include synthetic or minimal examples. Check screenshots and logs before sharing personal file paths or player data.

Open a pull request to main with the player-visible result, test commands and observed output. Mention save compatibility or performance effects when applicable. Windows CI runs the combined suites and builds a runtime ZIP. Contributors can retrieve the results from the Actions run attached to the pull request.
