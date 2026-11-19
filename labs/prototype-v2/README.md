# Xianxia Architecture Lab · Prototype II

The earlier [Mind Observatory](../mind-observatory/README.md) is available alongside this prototype.

## Run

From this folder, run `npm start` and open http://127.0.0.1:4174/. No dependency installation is needed. `npm test` runs engine checks. Node 18+ is sufficient. All computation and data stay on this computer. Export writes a JSON snapshot; import and automatic persistence are not implemented.

## What to test

1. In Decision lab, inspect an Attack pair. Move its target to position 5: it is perceived but out of interaction range. Move it beyond sensing radius 6: it disappears from candidates.
2. Set the selected NPC to Spirit: physical aid, trade and attack are blocked even if the network scores them highly. Cultivation still requires qi and energy.
3. Set target health to 100% or actor food to zero: aid is blocked. Restore injury and provisions: aid becomes possible.
4. Adjust trust and grudge, resolve ticks, and inspect actual health, resources, social events, commitment, and policy scores.
5. Open Generations. Add a neutral hidden feature; all policy scores initially remain unchanged. Run multiple generations, inspect real outcomes by scenario, then apply a champion and challenge it in other bodies and worlds.
6. Run the 100-generation unselected stress audit. It checks physical and numerical consistency while reporting policy changes and saturation separately. A pass does not mean the policy is wise.

## Architecture

`engine.mjs`: sensing, needs, pair generation, pair features, neural policy, physical validation, execution, genetic operations, selection, and audit. It has no browser dependencies.

`app.js`: in-memory world, experiment controls, candidate inspection, trace, rollout journal and fitness chart. `worker.js` runs evolution/audits off the UI thread. `server.mjs` serves an explicit local file allowlist. `tests.mjs` exercises behavior and invariants.

### Perception

Each actor sees living targets within six spatial units: distance, target injury, available food, visible hostility, relative strength, directed trust and grudges. Environment provides available qi and local danger (ambient danger × personal exposure). Sensing is exact. Actors do not receive target names or identity numbers as policy features. Identity is used only to retrieve directed memories and bind actions to targets.

### Needs and memory

Needs are health deficit, energy deficit, embodied hunger, and ambition. Trust/grudges are acquired memories; short commitments add continuity. Event logs are visible records, not recurrent neural state. Spirits do not have physical hunger. All life states have health/integrity and energy in this simplified model.

### Policy

One shared scorer evaluates each action–target pair, rather than seven unrelated action outputs lacking target context. Eighteen normalized features are need, estimated benefit, estimated risk, affinity, resentment, four traits, commitment, switching cost, and seven action flags. Five initial tanh features feed one score; direct continuity terms supplement that score. Initial node IDs reference their original lineage; their meanings are free to change.

Need/benefit/risk estimates are hand-authored, action-conditioned feature adapters. These estimates are hypotheses, not accurate future predictions, and benefit values for health/progress are not universally comparable. Feature adapters do not evolve in this prototype. Pair scores are converted to softmax shares after masking infeasible pairs; deterministic argmax selects a pair. Exact traces sum hidden and direct contributions plus bias.

### Feasibility and execution

Only living actors act. Physical actions require physical actor and target bodies and distance ≤2. Aid needs injury ≥5%, one food and energy ≥12%. Trade needs one food on both sides and actor energy ≥8%. Attack needs actor energy ≥18%. Cultivation needs access, qi >5%, and energy ≥18%. Retreat needs perceived threat and energy ≥4%. Rest/Observe are fallbacks for living actors. Spirits/Sealed lack physical bodies, Mortals/Sealed lack cultivation access. These life-state rules are experimental, not established lore.

The world validates a pair again at execution. Actors sense and act sequentially in a rotating order, avoiding a permanent first-actor advantage. This differs from prototype I’s simultaneous decision previews.

- Aid costs actor one food and 12% energy; restores up to 22% target health; recipient trust +8.
- Trade is a deliberately simple mutual provision-sharing action: each participant spends one food and gains 12% energy, with actor energy cost 8%; both trust values +4. It is not a general market exchange or consent negotiation.
- Attack costs 18% energy, damages the target according to relative power and inflicts deterministic retaliation. Recipient trust −20 and grudge +18. Killing is possible; dead targets disappear.
- Retreat moves three units away from the nearest living hostile, costs 4% energy, and halves personal exposure. It eventually ceases being available without a continuing perceived threat. There is no movement toward targets or terrain.
- Cultivate costs 18% energy and gains 0.12 × available qi progress; it does not advance a realm.
- Rest consumes one food to restore up to 20% energy and 6% health; without food it restores only 8% energy. Disembodied actors regenerate energy/integrity without food.
- Observe restores up to 2% energy. It does not change sensing accuracy yet.

Ambient exposure damage is danger × personal exposure × 0.008 per tick (0.002 on a retreat tick); grudges decay 1% per tick. Three-tick continuity starts on a new action key (two subsequent remaining ticks); it is a preference, never a hard lock. No hunger damage, birth, religion, morality, techniques, consumable item types, uncertainty, or recurrent learning is simulated.

### Heredity and topology

Weights/biases clamp to [−3,3]. Crossover aligns stable node ancestry IDs, never array positions. The first parent’s topology is retained; unmatched second-parent genes are not imported. An added node duplicates an ancestral feature and divides its output weight between the original and copy, exactly preserving all initial scores. Its deterministic ancestry ID is derived from the source node. Further independent mutation can specialize either branch. There is a ten-node cap; there is no edge-splitting innovation registry, recurrent topology, disabled-gene handling, compatibility speciation or full NEAT algorithm.

The exported `offspring` helper mixes traits and policy without returning any social memories or body. The UI’s selection experiment intentionally evolves policy alone, holding selected traits fixed. Real birth/life-state inheritance is left to the game’s entity system. Applying a champion changes an existing actor’s policy rather than pretending a new person inherits the old person’s relationships.

### Selection

Each candidate is evaluated for 12 ticks in five reset scenarios alongside three fixed seed-policy NPCs. Candidate body is Cultivator for comparable access. No existing memories, resources or realm are inherited. Selection retains the best candidate, breeds among the top third (minimum two), and adds neutral nodes with 20% probability. Initial population includes the original policy. Champions are selected from all evaluated policies including the baseline, so training reward cannot regress.

Reward formulas (averaged across scenarios):

- Balanced: remaining health + 0.7 × actual healing given + 0.8 × cultivation progress + 0.15 × remaining food/5 (clamped) − unprovoked damage − 0.1 × world switching frequency.
- Cooperation: remaining health + 2 × healing given + 0.08 × completed trades − 2 × unprovoked damage.
- Growth: remaining health + 2 × cultivation progress − 0.5 × unprovoked damage.

Unprovoked means the target has no hostile flag. This is a coarse designer definition, not moral truth. Rewards can still be exploited; cooperation can reward cycles of eating/trading, and short horizons can prefer unsafe long-term behavior. Chart and episode table use the training scenarios, with no held-out generalization claim. Future work should add multiple seeds, unseen scenarios, longer lifetimes, ecological resource production, relationship reciprocity and richer harm/benefit accounting. These are behavioral validity limitations, distinct from physical gate violations.

### Existing project connection

This builds on the earlier inspection of `C:\Workspace\XianXia`: directed −100…100 relationship conventions, realm/capability separation, and sensing/decision/execution boundaries. It does not modify or import the Python checkout. A future Python adapter should supply sensed pair features and execute validated requests through the world systems. Keep clan strategy separate from individual policy, and keep the world authoritative over resources and capabilities.
