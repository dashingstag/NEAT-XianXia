# NEAT Brain Inspector

A single-file, dependency-free HTML tool that visualizes a real game brain and how it reacts to its surroundings. Open `index.html` directly in a browser — no server needed.

## What it does

- **Faithful port of `sim/crates/neat-core`**: the same `Rng` (splitmix64), `Genome::seeded` topology and seed weights, `compile()` topological ordering, `Compiled::evaluate` feed-forward inference, and the `mutate_weights` / `add_node` / `add_connection` mutations. A seed produces the same genome as the game (verified: seed 42 scores 3.600756 on `[0.2; 48]` in both ports; JS evaluates in f64 so expect ≤1e-6 divergence from the game's f32 math).
- **Topology graph**: nodes laid out by topological depth — the 48 named sensors on the left, hidden Tanh/Identity nodes in the middle, the single score output on the right. Edge colour shows weight sign, width shows magnitude, disabled genes are dashed, node size tracks live activation.
- **Node inspector**: click any node to open its internals — bias, every incoming edge's `value × weight` contribution sorted by magnitude, the pre-activation sum, the post-activation value after `tanh` (or identity/output passthrough), and its fan-out list. Selecting a node dims all unrelated edges so you can trace its signal path.
- **Candidate slate**: each row is an action offer (`Gather`, `BuyFood`, `Rest`, `Hunt`, `UrbanWork`, …) whose 48-sensor vector is built exactly the way `sim-core`'s candidate closure and `personal_inputs` do — need, benefit, risk, travel, price, resource one-hots, wallet/currency, night/rest sensors, aspiration flags, goal alignment/urgency, happiness, traits, adult, iron/rice. Scores are ranked and the argmax highlighted, mirroring how the game commits to the highest-scoring candidate.
- **Surroundings**: agent-state controls (hunger, coins, energy, rested, happiness, social, load, night, traits, aspiration, goal) update every candidate's sensors and the graph live. Presets model "Hungry at night", "Morning market run", and so on. Sensors 42–47 (dread, qi pressure, technique memory, bond, quest reward, unexplored) are exposed as sliders, and any sensor can be pinned manually for ablation.
- **Real genomes**: *Import genome JSON* accepts the serde shape used in world checkpoints (`{generation, nodes: [{id, kind, bias}], genes: [{innovation, from, to, weight, enabled}]}` — the `genomes` array in a save). *Export genome* downloads the current brain. The mutate buttons apply the game's own mutation operators with the same RNG.

## Layout

| Panel | Contents |
|---|---|
| Left | Agent state, aspiration/traits/goal, frontier sensors, sensor overrides, scenario presets |
| Centre | Live activation graph, node inspector (click a node), and the selected candidate's full sensor readout |
| Right | Editable candidate slate, per-candidate scores, winner highlight |

Related: `labs/mind-observatory` is a separate hand-authored prototype and does not use the game's genome format.
