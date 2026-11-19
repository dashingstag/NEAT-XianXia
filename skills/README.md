# Skills for this repository

Six project-owned skills describe the commands that produce evidence in this game. Edit the canonical SKILL.md here and run `python tools/sync_skills.py`. Tracked copies under .agents/skills and .github/skills support different client discovery conventions. CI checks for drift. Agents can also read any entry directly. [Verify replay](verify-replay/SKILL.md) · [Check viewer](verify-viewer-fixture/SKILL.md) · [Review old saves](review-save-migration/SKILL.md) · [Check terrain](verify-world-rules/SKILL.md) · [Verify isolation](verify-server-isolation/SKILL.md) · [Profile simulation](profile-simulation/SKILL.md)

## Awesome Copilot resources

Five upstream skill directories and the TDD Red agent are bundled with their license. The project skills above are active discovery copies. The five upstream skills are available on demand under third_party. Open the named SKILL.md when its trigger fits, or install that directory in your chosen client. The TDD Red agent also has a repository discovery copy under .github/agents. [Pinned source and license](../third_party/awesome-copilot/PROVENANCE.md)

| Resource | Trigger and project-specific trial |
|---|---|
| [Acquire Codebase Knowledge](../third_party/awesome-copilot/skills/acquire-codebase-knowledge/SKILL.md) | At first Git import, follow the launch and replay paths and compare the inventory with project.toml. |
| [TDD Red](../third_party/awesome-copilot/agents/tdd-red.agent.md) | From a GitHub issue or an equivalent local brief, write a failing test for contested ownership of the final shop item. Another worker implements the fix. |
| [Create Architectural Decision Record](../third_party/awesome-copilot/skills/create-architectural-decision-record/SKILL.md) | Record options and migration steps before changing the movement frame format. |
| [Diagnose CI Gate](../third_party/awesome-copilot/skills/diagnose-ci-gate/SKILL.md) | Compare a failing viewer job against the base revision using the raw script error and the captured frame. |
| [Webapp Testing](../third_party/awesome-copilot/skills/webapp-testing/SKILL.md) | Use Node and Playwright for the browser labs. Run the repository's Godot check for desktop UI changes. |
| [Autoresearch](../third_party/awesome-copilot/skills/autoresearch/SKILL.md) | Trial only in a disposable Git worktree. Its workflow can commit and reset experiments. Agree on scope and duration first, then check replay and population alongside timing. |
