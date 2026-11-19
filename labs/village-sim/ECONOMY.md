# Jobs, leadership and conscription — implemented first version

Open http://127.0.0.1:4175/ and scroll to **Jobs, council & village market**. Run `npm start` from this directory if needed. Existing worlds in open browser tabs need a reload to load the extension; snapshots from older versions cannot be imported.

## Try the complete loop

1. Pause and press **Advance one day** twice. The default world should have a village, houses, market, crafted supplies and completed exchanges. The action uses the same 0.1-second steps as live playback and preserves pause state.
2. Change a resident's civilian occupation. Inspect its input/output description and task preferences. All jobs can still eat, recover and take other useful work.
3. Inspect the leader, quartermaster, spendable treasury, reserved order money, actual orders and recent trades. A public buy order without coins cannot be posted.
4. Call up two militia. Resume briefly so the call is resolved, or advance the world. Recruits must be healthy, willing and supplied. Officers, two foragers, and one healer/artisan are exempt. Low trust produces recorded refusals. Changing a recruit's civilian occupation does not silently cancel duty.
5. Watch civilian counts fall while militia patrol. The optional threat test sends an existing hill creature toward camp; it creates no creature or loot. It is a scenario intervention rather than a naturally occurring raid system.
6. Stand down the militia. They return unused issued tools/provisions at camp and resume their civilian jobs. A call expires after 90 simulated seconds even without manual stand-down.
7. Open the resource ledger. All eight resources should have zero balance error, including goods inside orders and interrupted recipes.

## Jobs and actual flows

Occupation is a +1.1 preference bonus for matching jobs, not an exclusive profession or an extra action per tick. Each person has exactly one current task and a separate civilian job and duty status. New arrivals receive a balanced initial mix of professions. The council's food-first priority raises food-gathering utility; construction priority raises hauling/building utility.

| Job | Inputs | Actual output |
| --- | --- | --- |
| Forager | Finite berry/herb stock, time, energy | Carried food and herbs |
| Woodcutter | Finite trees, time, optional tool | Carried timber |
| Miner | Nonrenewable stone, time, optional tool | Carried stone |
| Hauler | Existing carried/dropped goods and travel | Goods deposited, not duplicated |
| Builder | Prepaid project materials and labor | Construction progress |
| Artisan | Reserved recipe inputs and work time | Tools or medicine in the public store |
| Healer | One reserved public medicine and treatment time | Up to 30 health restored to a living patient |
| Merchant | Existing goods, funds and market access | Completed exchanges |
| Hunter | Energy, health risk, optional equipment | One creature's finite carcass loot |

Eight herb patches start with three herbs each, regenerate at 0.008 herbs/second per patch at normal productivity, and cap at three. They are part of the ledger like berry bushes.

**Tool recipe:** 2 timber + 1 stone + 7 seconds work → 1 tool. **Medicine recipe:** 2 herbs + 1 food + 6 seconds work → 1 medicine. Inputs move to task escrow on starting work. No ingredients are consumed and no output appears until completion. Cancellation returns unused ingredients; worker death puts them into the recoverable pack.

Tools make gathering 35% faster and combat damage 20% greater. One tool is consumed after twelve completed harvest actions or successful hunts, creating replacement demand. Medicine can be purchased for self-treatment or reserved from public stock by a healer. Cancelled or unnecessary treatment refunds unused medicine.

## Public leadership and finance

On founding, living assembled civilians are ranked by care, a smaller bravery contribution, and recorded contributions. The highest becomes leader and the next quartermaster. This is an appointment/ranking model, not a voting simulation. Vacancies are replaced automatically from eligible civilians. Officers can still work; administration does not yet consume separate labor time.

Each assembled resident contributes up to two existing coins once after founding. A manual levy transfers up to one coin per assembled living resident with a 60-second cooldown. Public funds remain in the warehouse's coin account. Reserved order coins are unavailable for wages, construction or a second order.

When funds are available, public work pays one coin per five seconds of construction, per completed craft/treatment, per deposit of at least three goods, or per twenty seconds of actual guard-post service. Wages are contingent transfers from the spendable treasury, not guaranteed escrow-backed employment contracts. No arrears are accumulated. Unpaid civic work can still be selected. Goods donated through hauling remain communal; a funded procurement order instead buys an existing private unit at its posted price.

## Market reserves and prices

The market must be built before any order can be posted or filled. Orders expire and return unused reserves to the owner. A private owner's death releases reserves into their inventory before the death pack is created. A dead crafter similarly preserves reserved ingredients.

- Public buy orders: food 2 coins, timber 1, stone 2, herbs 1. Up to three units are funded at once per material, within actual treasury limits and stock targets.
- Public sell offers: tools 5 coins, medicine 4. One actual unit is reserved for each offer.
- Resident food offers: a resident near camp with at least five food and low hunger may reserve one food at 2 coins, leaving at least four in their pack.
- Residents travel to camp to buy or fulfill a funded order. Offers cannot be oversold; trades transfer one unit at a time. Private sellers must remain alive and within the village vicinity.

Stock targets express limited public demand; they do not cause goods to appear. There is no bank, mint, unlimited NPC buyer, dynamic pricing or external caravan market. Crafting targets account for already planned crafting tasks to limit redundant production. Lack of funds can leave demand unmet.

## Militia mobilization

A call requests 1–4 people for up to 90 seconds in the UI. Candidates need at least 65 health, 35 energy, low hunger, and a completed arrival. Officers, the first two living foragers, and one healer/artisan are protected from recruitment. The model has no age or pregnancy system, so those exemptions are not claimed.

Willingness is a deterministic combination of leadership trust, bravery and care. An insufficient score records a refusal for that call. Candidates lacking supplies remain civilians; they are not immobilized waiting for nonexistent equipment. Equipment/provision demand raises the council's craft targets. An accepted recruit needs one actual carried tool and two carried food; personal supplies count, and only shortfalls are issued from public stores.

Active duty removes civilian production candidates. Basic eating/recovery remains available. Patrols take travel time and energy; nearby aggressive creatures can trigger defense using normal combat resolution. Soldiers receive no duplicated household food allowance: the existing hunger clock continues. Consumed or worn issued supplies are tracked so stand-down does not reclaim them a second time. Unused issued goods are returned physically at camp; private equipment remains private.

Duty can end through the deadline, manual stand-down or death. Survivors retain their occupation and injuries. Dead soldiers' remaining goods are recoverable, including public equipment. No conscription penalties, imprisonment, desertion, loyalty learning or political rebellion is simulated yet.

## Accounting and verification

For each resource: **initial + imports + biological renewal + crafted outputs = current inventory + consumption + construction**. Current inventory includes wild goods, creature loot, packs, dropped goods, public stock, market escrow and recipe/treatment escrow. Consumed ingredients and produced outputs are recorded explicitly; crafting is a conversion, not unexplained renewal. Coin's produced/renewed counters remain zero.

`npm test` covers the original world and the economy: funded orders, expiry, duplicate fills, cancelled/dead-worker recipes, treatment refunds, wear, levy cooldown, supply-limited call-ups, refusal, exemptions, civilian labor displacement, demobilization and conservation during extended runs. `economy-validation.json` records a seeded stress comparison, including short-lived mobilizations and threat tests.

This implementation deliberately leaves farming, water/seed accounting, dynamic prices, promised employment contracts, elections, taxation consent and neural evolution for later releases. Those systems should build on the same authoritative transfer and escrow mechanisms.
