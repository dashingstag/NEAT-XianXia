# Xianxia Mind Observatory

The playable Rust and Godot game includes families, inherited brains, industry, schools and banking. Launch it from the repository root with `python tools/dev.py play`. [Game instructions](../../README.md)

Willow Crossing explores settlement founding, finite resources, wildlife and construction. Run `npm start --prefix ../village-sim` from this folder. [Willow Crossing](../village-sim/README.md)

The Architecture Lab tests target-aware perception, physical feasibility and multi-generation policy evolution on port 4174. This original prototype uses port 4173. [Architecture Lab](../prototype-v2/README.md)

A local, dependency-free browser prototype for exploring NPC neural inputs, hidden drives, actions, life states, relationships, and inheritance.

## Open

Run `npm start` in this directory, then visit http://127.0.0.1:4173. Run `npm test` for engine checks. No installation or account is needed. The server binds only to this computer. Export experiment downloads the current state, weights, ablations, journal, and evolution settings as JSON. Refreshing resets the session; JSON import is not implemented.

## Try it

1. Select Lin Yue, switch between Roadside ambush and Spirit spring, and compare action shares.
2. Change Mortal to Cultivator: Cultivate becomes available, without assigning moral alignment.
3. Open Traits & ties, choose a target, and adjust trust, grudges, compassion, or aggression.
4. Return to Neural map. Disable trust or danger, or select a hidden drive to edit its input weights.
5. Advance a tick. Every NPC decides from the pre-tick social state; effects then update recipients’ relationships. Compare the new preview with the recorded prior decision.
6. Preview offspring under Inheritance lab; explicitly apply it if desired. Add hidden nodes or compare 24 offspring under different reward objectives.

## Architecture

`engine.mjs` is pure, browser-independent model logic; `app.js` owns the UI and in-memory experiment; `index.html` and `style.css` provide the responsive interface. `server.mjs` serves an explicit file allowlist using Node built-ins. `tests.mjs` validates inference, masks, directional memory, crossover and mutation, and simulation steps.

11 normalized inputs → initially 5 tanh hidden nodes → 6 output logits → capability mask → stable softmax → deterministic argmax. Inputs are danger, scarcity, qi, target distress, directed trust, directed grudge, compassion, ambition, aggression, caution, and life-state power. Trust is signed −1…1; other inputs are 0…1. Disabled inputs become zero. Hidden labels are human interpretations of hand-authored seed weights. They do not constrain node behavior after editing or mutation.

Outputs are Aid, Trade, Cultivate, Attack, Retreat, Observe. Softmax shares are relative network preferences, not calibrated confidence. Decision traces report every hidden contribution plus bias to the winning logit. They are arithmetic decompositions rather than causal explanations. All NPCs currently share one environmental context; target distress is a scenario abstraction.

Mortal, Cultivator, Immortal, Spirit, and Sealed are prototype life states. Mortal/Sealed cannot cultivate. Spirit/Sealed lack a physical body and cannot aid, trade, or attack. Observe and Retreat remain available. These are editable domain assumptions, not claims about canonical Xianxia lore. Lifespan, hunger, inventory, techniques, spatial sensing, mortality, and actual combat are future inputs/systems. Actions currently apply only social effects: Aid +8, Trade +4, Attack −16 to the recipient’s trust in the actor; Attack also adds 12 grudge. Values clamp to their bounds. Other actions have no world effects. Good/bad or friendly/hostile are not fixed labels; infer behavioral patterns across scenarios.

## Existing project inspection and reuse

Inspected `C:\Workspace\XianXia` on 2026-11-17. It is a Python world simulation, not a browser application. This prototype is intentionally isolated in the requested working directory and does not modify that checkout. Reused its domain conventions and interface concepts rather than copying Python runtime code:

- `core/macro_entities.py`: directed clan relationships in −100…100; `CultivationRealm` progresses through Qi Condensation to True Immortal. The UI compresses that progression into broad capability states.
- `core/clan_ai.py`: situation evaluation → possible decisions → personality-adjusted priorities → selected decision, with `AIDecision` target and history. The lab replaces scoring with an inspectable neural policy while retaining separation of sensing, admissibility, scoring, and choice.
- `README.md`: component-based world, spiritual energy and cultivation, settlements, and Python/Pygame visualization.

Suggested integration: build a Python sensing adapter around an individual cultivator and a target; normalize world signals and relationships; evaluate a serialized genome; return an action request with actor/target/tick plus trace; let game systems validate resources and resolve effects. Clan strategic actions in `ClanAI` should remain a separate level of control. Add precise realm power and lifespan pressure rather than equating immortality with compassion. No live bridge or Python parity implementation is included.

## Evolution and heredity

Seeded crossover chooses each scalar weight/bias from either parent, then applies bounded-amplitude weight mutation with a controllable probability. Traits average with small mutation and clamp to 0…1. Life state follows the selected parent only for experimentation; social memories reset. Offspring replace a selected cast slot only when explicitly applied. Other NPCs retain their opinions of that slot: IDs model social positions, not newly born identities.

Different-topology crossover retains the selected parent’s shape and uses its value where the other parent lacks a homologous scalar. Node indices provide limited homology; there are no historical innovation IDs. Add-hidden-node disables an incoming edge, inserts a tanh node, and adds outgoing paths. This approximate mutation is not function-preserving and is capped at 10 hidden nodes in the UI.

The sweep compares 24 independent offspring across four scenario presets using expected policy reward. Cooperation rewards Aid+Trade; Cultivation rewards Cultivate; Survival rewards danger×Retreat + (1−danger)×(Trade+Observe). It reports the top five candidates and does not apply a winner. It is a one-generation sensitivity experiment without rollouts, resource costs, population persistence, speciation, innovation tracking, disabled-edge genes, recurrent memory, or full NEAT. Mutation and seed affect experiments, not inference. A production NEAT implementation needs those mechanisms, multi-seed evaluation, ecological fitness, diversity preservation, and checks against reward exploitation.

The font stylesheet optionally requests Google Fonts; system fonts work offline. All experiment data stays in memory unless exported; the local server has no persistence or telemetry.
