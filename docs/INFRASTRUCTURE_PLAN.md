# NEAT Xianxia development infrastructure plan

**Decision proposed for 19 November 2026.** Build a new development environment alongside the playable prototype. Keep Rust authoritative for simulation outcomes and Godot responsible for presentation. Start with one repository, one local development command and a small set of versioned examples that multiple contributors can use independently. Introduce hosted services only when they solve a demonstrated coordination problem.

The design borrows three ideas from GitHub's guide. Give a skill one repeatable job, put project-specific facts in a shared manifest, and have an independent party reproduce the evidence. Agent clients supply tools and permissions individually, so portability needs an actual trial on each supported client. [GitHub’s practical guide to agent skills](https://github.blog/ai-and-ml/github-copilot/build-once-run-on-any-agent-a-practical-guide-to-agent-skills/)

## 1. Establish the boundary

The existing launcher uses port 4176, a shared release executable and a shared runs directory. Its default backend launch starts a new simulation. The generic protocol test resets the server it connects to. Meanwhile a viewer snapshot currently carries moving people, map data, inspection details and population-wide reports together. These facts determine the order of work. Isolate execution first, then separate the data contracts, then distribute feature work. [Launcher](native-prototype/run-prototype.ps1) · [Protocol exercise](native-prototype/experiments/check_protocol.py) · [Architecture findings](native-prototype/ARCHITECTURE_REVIEW.md)

Proposed source layout, reached incrementally after the initial source import:

| Location | Owns |
|---|---|
| `sim/` | Existing Rust workspace, NEAT evaluation, domain systems, ordered tick, save readers and server |
| `viewer/` | Godot connection adapter, inspectors, rendering layers, input and camera |
| `world/` | Versioned scenario definitions, terrain, resource parameters and seeds consumed by Rust |
| `contracts/` | Protocol descriptions, stable IDs, compatibility rules and examples |
| `fixtures/` | Small synthetic old saves, recorded frames and expected outcomes with generating commands |
| `tools/`, `skills/`, `project.toml` | Development command, portable instructions and shared project configuration |
| `labs/` | Browser experiments with their own tests, usable for rapidly testing design hypotheses |

Keep the first import at its existing paths. Move one component per change once the runner can exercise it. Pin the working Rust, Godot and Python versions, retain Cargo.lock, and record how to provision dependencies for a clean or offline build. Keep player worlds, generated builds, machine paths and run output outside version control. Preserve the older Python project as a separate reference checkout.

## 2. Give people, agents and CI the same commands

Build a small Python standard-library CLI invoked as `python tools/dev.py`. Python already drives the protocol experiments. Windows is the first supported environment. Each command below is a proposed interface.

| Command | Observable result |
|---|---|
| `doctor` | Checks actual tool versions against the manifest, reports missing capabilities and explains how to supply a local tool path. |
| `test protocol` | Starts a test-owned server on an OS-assigned loopback port, waits for its readiness message, exercises the protocol, saves and reloads a synthetic world, then reaps its child processes. |
| `test sim`, `test viewer`, `test world` | Run the relevant Rust tests, headless Godot fixture or seeded placement checks. Each returns an exit code and a report identifying the failed assertion. |
| `preview --fixture town-shop` | Opens a local viewer on recorded data. An interactive developer can inspect it while another worker changes the backend. |
| `bench --scenario mature-town` | Measures tick, snapshot preparation, payload size and memory separately. Records build, hardware, seed, population and simulation duration for comparison. |
| `package` | Produces a versioned Windows backend and viewer together with a manifest of supported save and protocol versions. |

Use a unique run directory, checkpoint, logs and process handles for every invocation. Have the child bind the port itself and report its assigned address and run ID. Give reset-capable tests only the connection created by that invocation. A failed run retains its diagnostics. A concurrent run cannot reuse its processes or write its save. The player launcher uses a separate data directory and versioned installed binaries so rebuilding a worktree never requires stopping the player's game.

Commit stable project facts in `project.toml`, including component paths, tool versions, scenario names and contract versions. Write detected executable paths, ports, run IDs and output paths to an ignored per-run manifest. Put gameplay invariants and review policy in reviewed source documents. A discovered machine path or current balance constant should never silently become a project rule.

## 3. Make parallel ownership practical

Give each feature a brief containing the player-visible result, affected contract, fixture, acceptance example and owner. Give each implementation worker a separate Git worktree and run directory. An integration owner sequences changes to shared IDs, save layouts, protocol versions and tick order. Other work may merge independently once its dependencies and checks pass.

| Workstream | Owns | Independent evidence |
|---|---|---|
| Simulation logic | Feasible actions, decisions, reservations, transactions and their outcomes | A seeded last-item sale settles once, preserves coins and records the actual decision and outcome. |
| Interface | Panels, labels, selection, navigation and accessibility | A recorded shop frame opens the owner's inspector and survives an absent optional field. |
| Visuals | Sprites, animation, map layers, batching, picking and zoom | The same shop can be selected at all three levels. Capture representative frames and measure frame time on a graphics-capable machine. |
| World building | Seed sites, terrain, habitats, deposits and scenario tuning | Moving a hill in world data moves its rendered footprint and prevents farming on the new stone terrain. |

Use stable entity IDs to connect the workstreams. Publish movement frames frequently, terrain geometry on map revision, and inspector details and expensive reports on request. Version the wire format independently from the save format. A fixture records both its contract version and the producer revision. When a contract changes, land an additive example and compatibility check first, update producer and consumer, then retire the old shape at an explicit compatibility boundary.

## 4. Add skills after the commands work

Start with one portable skill, `verify-replay`. Its trigger is a change to persistence, deterministic simulation or tick ordering. Its inputs are the revision, fixture and tick count. Its procedure invokes the shared runner to compare a continuous run with a save/load continuation. Its output includes the two state digests, the first differing tick when available, the reproduction command and the report path. Record a missing capability explicitly if the check could not run.

Once that workflow has proved useful, add `verify-viewer-fixture` for inspector or rendering changes and `review-save-migration` for compatibility changes. Keep a canonical directory per skill with a SKILL.md containing its name, trigger, inputs, steps and expected evidence. Put detailed examples beside it. Generate the discovery layout required by each supported agent client from the same source. Try a positive trigger and an unrelated task in Codex and a second available client. Check discovery, fixture access, invocation and equivalent results. Until a client passes, document its support as unverified.

The CLI owns processes and state. CI records results and enforces required checks. Skills explain when to use these facilities and how to interpret failures. This division lets a human reproduce an agent’s finding from a clean checkout.

### Awesome Copilot resources for the pilot

These are adoption candidates. The first two support the baseline and isolated-runner milestones immediately. The remaining entries have explicit prerequisites. The catalog distinguishes reusable skills from configured agents and plugins. [Awesome Copilot catalog](https://awesome-copilot.github.com/)

| When and who | Catalog resource | First Xianxia trial and acceptance evidence |
|---|---|---|
| Baseline, integration owner | [Acquire Codebase Knowledge](https://awesome-copilot.github.com/skill/acquire-codebase-knowledge/) · skill | Run its Python scanner against the clean source import. Compare its findings with the existing architecture review. Confirm the player launcher, reset-capable test, save reader and Godot decoder by following each entry point. Use the verified inventory to populate the project manifest once. |
| Isolation milestone, test author | [TDD Red](https://awesome-copilot.github.com/plugin/testing-automation/) · agent supplied by the Testing Automation plugin | Give the test author the isolation ticket and two synthetic runs. Capture a failing assertion demonstrating that one run can interfere with the other in a disposable harness. A second worker implements isolation. The same test must then pass while one child server exits mid-run. |
| First shared-format change, integration owner | [Create Architectural Decision Record](https://awesome-copilot.github.com/skill/create-architectural-decision-record/) · skill | Supply the current packed position record, proposed map revision message, compatibility requirements and alternatives. Produce one decision record that a Rust producer and Godot consumer can both implement, with a linked old-frame decoding fixture. |
| First failed merge check, independent reviewer | [Diagnose CI Gate](https://awesome-copilot.github.com/skill/diagnose-ci-gate/) · skill | Provide the full job log, base revision and failing command. Trial it on a disposable change that breaks the Godot decoder. The report should identify the first script error, reproduce it against the changed fixture and show whether the base passes. Raw logs can be supplied locally. |
| Browser lab UI work, interface owner | [Webapp Testing](https://awesome-copilot.github.com/skill/webapp-testing/) · skill | With Node, Playwright and the local Architecture Lab available, move a target outside interaction range and verify that the action disappears or becomes ineligible. Capture the observed state and console errors. Use the native Godot fixture runner for the desktop inspector, picking and zoom. |
| After stable benchmarks, performance owner | [Autoresearch](https://awesome-copilot.github.com/skill/autoresearch/) · skill to adapt | Its experiment loop measures a baseline, logs attempts and uses Git resets. Trial a bounded batch in a disposable worktree with a fixed mature-town save and hardware. Target snapshot generation time. Accept a result only after repeated measurements, equal replay digests and equivalent decoded positions and reports. |

**Keep domain verification local.** The world-building worker needs a seeded check that terrain geometry, farm eligibility and finite resources agree. The logic worker needs replay and ledger checks after simultaneous claims. UI and visual workers need the actual Godot decoder, picking checks at three zoom levels and measured frame times. Package those commands in the project skills described above. The catalog provides testing and review methods. The project supplies the expected game behavior.

At adoption, record the upstream resource, revision, type, local adaptations, required tools and successful client trials. Start with individual entries so the pilot can show which ones earn their maintenance cost. Advance a candidate to the shared configuration after a second worker can reproduce its evidence from a clean checkout.

## 5. Follow a change from request to release

1. **Specify.** Write one acceptance scenario and select the smallest relevant fixture. Record whether the intended change affects rules, visuals, wire compatibility or old saves.
2. **Agree on shared data.** For a cross-domain feature, merge a reviewed fixture and contract example early. Agree on who can change shared files. Split implementation into short branches off that revision.
3. **Implement and submit evidence.** Each worker runs its own checks. Its change lists the command, fixture, observed result and any untested behavior. UI and art changes include a frame or interaction recording where appearance matters.
4. **Verify independently.** Another worker or reviewer starts with the original brief and merged candidate revision, reruns the acceptance case, and checks one plausible failure. A shop purchase example should include an absent owner or two buyers competing for the last item. The reviewer records observed outcomes. A CI result links to the exact revision tested.
5. **Integrate.** Run Rust tests and contract decoding on every proposed merge. Add deterministic replay, historical-save loading, scenario tests and headless viewer checks when their dependencies change. Changes to the contracts, runner or shared configuration run all affected producers and consumers. Inspect Godot script errors as well as process exit status. Recheck the combined revision when two individually green changes meet.
6. **Measure and release.** Run longer multi-seed survival and economy checks separately from fast review. Benchmark mature towns at 10k and 100k before scheduling a million-person soak on identified hardware. Track living population, failed jobs, sales, candidate truncation, decision age, memory and tick p95/p99. Measure viewer frames on a graphics-capable runner. Publish an identified backend/viewer pair only after installation, old-save and rollback rehearsals pass.

Proposed first CI provider is GitHub Actions if the eventual repository is hosted on GitHub. The same local commands are the fallback on another host. Keep reference performance measurements on a named machine so variation among shared runners does not drive architecture decisions.

## 6. Migrate in reversible milestones

| Order | Delivery and dependency | Exit test |
|---|---|---|
| 0 | Create a local, source-only Git baseline, tag the playable revision and inventory tool versions, dependencies and supported saves. Capture a synthetic equivalent of a populated town. | A clean checkout builds and loads the fixture. It can reproduce the baseline test results using the documented toolchain. |
| 1 | Build the isolated protocol runner and replace the test's live-port default. Wrap the existing tests before reorganizing directories. | Start two reset-and-replay tests simultaneously. Each gets a different server and checkpoint, both pass, and stopping one leaves the other running. Include an intentional assertion failure to verify reporting and cleanup. |
| 2 | Add a shop fixture, Godot offline preview, replay skill and CI checks. Extract the connection adapter behind the current protocol. | The same fixture passes locally and in CI. An independent reviewer reproduces one failure introduced on a disposable branch. |
| 3 | Pilot four parallel changes against the agreed example. Logic records the sale, UI shows the owner, visuals render and pick the shop, world building supplies the site. | The combined scenario completes a funded purchase, inspects the correct owner and picks the same shop at each zoom level. Inspect both evidence and merge conflicts before adding more skills. |
| 4 | Split movement, map and inspector updates. Migrate further domain systems and scenarios one at a time. | Compare old and new runners on matching seeds and tick counts for behavior-preserving moves. Record intended differences for rule changes. Measure zoom and snapshot costs on the same populated world. |
| 5 | Package a candidate alongside the playable release and rehearse promotion. | Copy a supported old save, load it, advance, save and reopen the copy. Rehearse rollback using the prior executable pair and its matching pre-upgrade save. Document the recovery point and potential loss of progress since upgrade before switching the player launcher. |

**First implementation ticket:** deliver milestone 1 on top of the baseline. Its demonstration is two simultaneous protocol runs and a readable failure report. That buys immediate protection for the live world and tests the central premise of this redesign before investing in more automation.
