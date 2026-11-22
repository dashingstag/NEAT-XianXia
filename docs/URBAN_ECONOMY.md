# Districts, property and private industry

Historical prototype notes. The save-format numbers, paths and command examples below describe an earlier implementation snapshot; they are not current compatibility claims or run instructions. Use the [root README](../README.md) for current commands and the [contract notes](../contracts/README.md) for supported save formats.

The native Rust simulation owns all decisions, property, money, inventories and contracts. Godot displays snapshots and sends player commands. `crates/sim-core/src/urban.rs` contains the new system; the existing council and individual NEAT evaluators rank its feasible actions alongside previous activities.

## Rules in this prototype

- A formed village with at least **500 copper in its coffers** can choose to gazette residential, commercial and industrial districts. Coffers include money reserved for market bids, so committing a purchase does not falsely make the village poor. Construction and wages require actual uncommitted cash.
- The marketplace remains the centre. District sites are placed around it, avoiding other buildings, fields, graveyards, the sea and forecast communal housing. Sites inside forests need a paid clearance contract before construction.
- Homes cost 12 timber + 4 stone + 16 builder wages. Shops cost 14 timber + 6 stone + 20 wages. Workshops cost 18 timber + 8 stone + 24 wages. Materials and wages are reserved when commissioned. Builders travel to the site and perform work before the building becomes usable.
- A settlement becomes a **Town after three completed homes, three completed shops and three completed workshops**. Empty zone outlines do not count. The initial prototype permits eight buildings of each category per village and one construction site per category at a time.
- New private homes provide four physical beds. A person can buy for 120 copper or rent for 3/day; their spouse and dependent children may use the beds. Existing 32-bed communal blocks remain available for existing residents. These older blocks do not count toward Town promotion.
- Business premises are village property leased to an operator. Entry costs 8 copper plus 50 copper transferred into the business's working capital. Shop rent is 3/day and workshop rent 2/day. A shop operator can earn a funded 2-copper daily marketplace service payment after working on site, in addition to sales proceeds. Qualified educated workers retain their existing village-job bonus entitlement.
- With coffers below 100 copper, the village NEAT policy can review commercial leases and evict a tenant after a minimum day-long lease. Active delivery contracts prevent eviction of the affected premises. Rent arrears influence which tenant is reviewed. Evicted money and stock remain the person's property at that location; recovering money requires returning, and goods require funded physical transport. A one-day cooldown prevents immediate reletting to the same evicted operator.
- Empty temporary camps can receive paid removal jobs. Occupied camps or camps being approached for rest cannot be removed. Salvage comes from previously accounted building materials.

## Goods and production

The original thirteen raw goods/tool types retain their identifiers and save layouts. Five manufactured/animal wares have a separate conserved ledger: hides (weight 3), leather clothes (2), shoes (2), furniture (8), and meat (1). A nonempty wallet still weighs one regardless of its balance.

| Workshop | Actual input | Output |
| --- | --- | --- |
| Hunter lodge | A physically reached, living adult wild animal | 1 hide + 2 meat, carried back to the lodge |
| Cobbler/leatherworker | 1 hide | 1 leather clothing item or pair of shoes |
| Blacksmith | 2 iron + 1 timber; pickaxes use 3 iron | Axe, pickaxe, hoe or shovel |
| Carpenter | 3 timber | 1 furniture item |

Hunters leave at least six living animals in a forest. Wildlife births still require the existing food and breeding conditions. Leatherworking is deliberately combined with cobbling for this prototype; separate tanning, textiles and garment trades can be added later.

Commercial premises specialize in clothes/shoes, tools, furniture or meat. Shops buy existing workshop stock at wholesale prices and sell it at retail. Workshops buy actual central-market raw stocks. Orders reserve stock and pay the supplier immediately, then escrow a separate delivery reward. The buyer cannot use the parcel until a haulier collects and delivers it. Suppliers can be in another settlement.

Individuals also post private transport contracts when buying furniture or recovering stored property. Furniture is delivered to a private home; it does not teleport into the buyer's carried inventory. Clothes and shoes add carried weight; shoes provide a small travel bonus. Meat is consumed as food. Repeated furniture orders for the same furnished home or pending delivery are blocked.

Shop retail purchases pay one copper to the marketplace and the remainder to the business. Owner withdrawals use actual business profits above working capital. Rent, retainers, builder rewards, private haulage and evictions never mint money. Manufactured wares appear in observed sales profiles, so the village's economic identity continues to follow products actually sold rather than its persona label.

## Brains, aspirations and profiles

New individual choices are `UrbanWork`, `LeaseBusiness`, `OperateBusiness`, `OrderGoods`, `BuyHome`, `RentHome`, `ShopGoods`, `Hunt` and `CollectEstate`. They use the existing stable 42-input contract (as of this brief — the sensor schema has since grown: `LEGACY_INPUTS` 42 → `FRONTIER_INPUTS` 48 → `INPUTS` 52 plus 4 memory inputs; see `crates/neat-core/src/lib.rs`): need, benefit, risk, distance, signed transaction value, continuity, funds, carrying capacity, traits, aspiration and goals. Costs are negative transaction values; earned contract rewards are positive. Infeasible choices are excluded before neural scoring. A mutated brain cannot spend missing money, create goods or skip delivery distance.

New aspirations are Homeowner, Shopkeeper, Artisan, Hunter and Haulier. Corresponding short goals track home ownership, leasing a business, production, deliveries and hunts. Existing saved aspirations are preserved; new births can receive the expanded set. Survival and dependent care can interrupt work. A carried contract parcel is dropped at the worker's actual position and can be recovered by another worker.

Village policies choose gazetting, commissioning each category, advertising vacancies, forest clearance, lease reviews and shelter removal. Trace scores appear in the existing village board. Forest, Coastal and Rocky profiles add small reversible coefficient biases to the same council network; Balanced adds none. They do not assign products or force an industry. `PERSONA_BIAS` and `PERSONA_INPUTS` are the tuning table. The Village tab's profile control applies to the focused village (Willow Reach when no village is focused).

The village profiles are initial environmental presets, not a claim of trained optimal behaviour. Production volumes, lease viability, market retainers, housing prices and resource competition remain balance parameters for future experiments. Sale prices and rent are fixed prototype values; dynamic property auctions, insolvency proceedings, maintenance, wear, transport routing and full estate inheritance remain future extensions.

## Viewing and testing

Use **Zones** beside Wildlife to show blue residential, gold commercial and purple industrial districts. Use **Map selects → Buildings** and click a new building to inspect its owner, rent, working cash, inputs, wares and production. The Village tab shows promotion counts, vacancies, coffers, contracts and policy scores. Centre/Follow also work for new buildings. Cleared forest sites display open ground and exhausted cleared timber patches no longer regenerate through the building.

Save format **11** stores private property, jobs, cargo, wares, personas and urban weights. The explicit format-10 reader preserves existing people, schools, resources and money and initializes empty urban records. Older migrations remain supported. Backups of the user's pre-upgrade world are in `runs/pre-urban-existing.snapshot` and `runs/pre-urban-live.snapshot`.

For an isolated scenario, run:

```powershell
target/release/sim-runner.exe urban-demo --agents 1000 --steps 18000 --threads 8 --out runs/urban-flow.json
```

This fixture transfers existing money from inhabitants and finite materials from resource patches into a funded hamlet. It does not manufacture initial assets, force district choices, pre-complete buildings or assign tenants. It writes `runs/urban-demo.snapshot`, separate from the user's world, and checks save/replay continuation. Use `serve --load --checkpoint runs/urban-demo.snapshot` to view that experiment separately.

Focused tests cover wealth/material gates, the nine-building promotion condition, private beds, rent, one-time builder payment, real manufacture and wholesale purchase, funded retail furniture delivery, parcel recovery, eviction asset preservation and exact checkpoint continuation. The complete workspace suite also covers the previous family, wildlife, school, caravan and conservation behaviour.

An isolated Godot interface check consumes the native report without connecting to the live backend:

```powershell
godot --headless --path viewer-godot --quit-after 120 --script res://bench/urban_ui_check.gd -- C:/absolute/path/to/runs/urban-flow.json
```

On 19 November 2026 this checked 47 property inspectors, building hit detection, camera centring, nine zone overlays, profile selection, the account display and all three fidelity levels. It finished without errors or warnings. A repeated completed-building drawing warning found during this check was corrected; an already running viewer loads that correction when reopened.


### Vacancy recruitment

Completed commercial and industrial buildings without an operator are counted separately from posted openings. Vacancy pressure raises the NEAT council's need/benefit inputs for `AdvertisePremises`. Choosing that policy publishes one role-specific operator posting per available building on the Village board, with its entry fee, working capital, daily rent and income terms. An individual's `LeaseBusiness` choice accepts the posting upon arrival and payment; acceptance removes it immediately, preventing duplicate tenants. Eviction or loss of an operator makes it eligible for a new council posting. Dead operators are detected each tick once deliveries touching their premises have cleared. Houses and unfinished buildings never generate operator jobs. Existing saves use their existing advertised flags, so no save-format migration is necessary.

### Individual wealth leaderboard

The **Wealth** inspector tab lists the top 30 living people by estimated net worth in copper. Click a name to select and inspect that individual. Ties use total coins, then ascending person ID for stable ordering.

Net worth includes wallet coins, owned business cash, stored estate cash, owned homes at their listed sale prices, business inventory, personal belongings and privately owned parcels in transit; rent arrears are deducted. Leased commercial/industrial premises remain village property, so their building prices are excluded from the operator's assets. Unpaid job rewards and village funds are excluded. A haulier does not own the parcel they carry. Rental-home contents are attributed to the current tenant; this remains a prototype valuation convention until furniture ownership is tracked separately.

Inventory uses fixed reference values shown in the snapshot's `leaderboard` metadata, rather than fluctuating market quotes. These are comparison estimates, not guaranteed liquidation proceeds. The ranking is read-only and needs no new save fields. Its bounded selection retains only 30 candidates rather than sorting the whole population. Tests cover stable ordering, dead-person exclusion, cash and possession breakdowns, leased-premises exclusion, inventory in transit and arrears. The isolated interface check also verifies every displayed name, the net-worth equation and clicking through to the individual inspector.

### Town banks and business loans

Save format **12** adds banking; format 11 is read explicitly and retains existing people, money, resources, buildings and education. Earlier format migrations remain supported. No banks or bank cash are invented during migration.

After reaching Town, the village NEAT policy can choose **FundBank**. A bank needs an open commercial-district site, 20 timber, 12 stone, a 30-copper builder contract and 500 copper of lending reserves, all transferred from existing village stocks and funds. At least 800 uncommitted copper is required to commission it. A person must physically build it before lending starts. One bank is allowed per town. The council can later choose **CapitaliseBank**, moving another 500 from the treasury when the bank has less than 100 and the treasury has at least 700. Bank cash and unearned builder escrow are included in conserved world currency.

An adult with an available local business opening can seek enough money to reach the 58-copper lease-entry requirement. Existing operators with low working capital can seek up to 50 copper of business funding; this goes into their firm's cash. Only one outstanding loan per person is allowed. A bank must retain at least 50 copper after lending. Both the applicant's individual brain and the village's lending evaluation must favour the transaction. The applicant physically visits the bank; there is no remote disbursement or new currency creation. Borrowers remain responsible for how they use their funds.

The starting annual rate is **5%**, fixed for each loan for **365 simulation days**. Interest accrues proportionally over elapsed simulation time, using fractional remainders to avoid rounding every update. It is simple interest on unpaid principal, not compounded interest on unpaid interest. A loan's rate renews only at its own annual anniversary.

Each bank reviews its advertised rate once per simulation year. It compares that year's completed local purchase volume with the previous year's: growth raises the rate by **one percentage point**, decline lowers it by one point, and equal volume leaves it unchanged. Rates are bounded at 1–20%. An existing loan keeps its previous rate until its next anniversary. The first annual comparison uses zero as its prior-year baseline. Rate changes are deterministic economic rules; commissioning, recapitalisation, lending and individual acceptance/repayment use the existing NEAT policy mechanisms.

Repayment is an individual action at the bank. It uses wallet funds above an 8-copper personal reserve and business funds above 50 working capital. Interest is paid before principal. A final fractional copper rounds up when settling a loan, so even a very short fully repaid loan can cost one copper. After a year, an unpaid loan is marked overdue; automatic repossession, collateral, inherited debt, deposit accounts and write-off policy are future extensions. A deceased borrower's unpaid balance remains recorded rather than being magically repaid.

Shopkeeper aspirations can generate the 14-day **BusinessLoan** short-term goal when funded local credit and a business opportunity are available. Its progress increases only after actual loan approval and disbursement; the next objective can remain leasing the business. Borrowing itself does not create net wealth: the leaderboard now deducts outstanding principal and interest as well as rent arrears.

Banks appear on the map, can be selected, centred and followed, and show reserves, annual rates, the next review and outstanding loans. The Village board shows construction and lending status; individual inspectors show their loans and next rate reset. Tests cover Town gating, actual construction and funding, cash conservation, duplicate/remote-loan rejection, yearly fixed rates and economic reviews, repayment, contextual goals, save migration, active-loan replay and debt-aware wealth totals.

### Learned relationships

Save format **13** adds a sparse relationship graph. Format 12 and earlier saves initialize an empty graph while retaining the existing world, banks and money. Relationship scores are learned during life, not inherited by children.

- Each pair has one **mutual score between -100 and +100**, visible to both individuals. Each person can retain at most **50 contacts**. Existing contacts remain usable at capacity; a new pair is ignored if either person's list is full. Zero-valued relationships are removed from both lists, and deaths free their contacts' slots.
- A normal interaction has a **50% +1 / 50% -1** roll. Spouses, parents/children and siblings add **15 percentage points** to the positive chance. Sharing any trait adds another **15 points once**, regardless of how many traits match. Thus family sharing a trait has an 80% positive and 20% negative roll. A shared surname alone does not establish kinship.
- While the score is exactly **-100 or +100**, both directional roll chances decrease by **15 percentage points**, leaving a 30% explicit no-change outcome. Outward rolls also clamp at the limit. For example, a family/shared-trait pair at +100 rolls positive 65%, negative 5%, or neither 30%; positive rolls stay at +100. The normal chances resume after leaving the limit.
- Each relationship receives a **50% chance every 30 simulation days** to move **one point toward zero**: positive scores lose one, negative scores gain one. This cadence begins at creation and is not reset by continuing contact. A score reaching zero is removed. This 50% drift probability is the prototype default for the user's unspecified decay chance.
- Caravan departure, arriving to sleep beside other sleepers, completed tavern social time and successful face-to-face shop purchases can trigger contact. Longer shared time receives further checks once per game hour. A pair can roll at most once per game hour across all contexts, preventing repeated purchase/update loops from creating relationships every frame.
- Caravan contacts require physical presence with the group. Sleeping contacts use the actual house's 32-bed chunk, the separate private four-bed house, or a shared campsite; sharing a settlement or merely reserving a bed is insufficient. Tavern contacts require actual guests present. Shop contacts pair the customer with an owner physically nearby; remote wholesale orders do not count as meetings.

The individual inspector lists contacts, signed scores, family/shared-trait indicators, encounter contexts, directional roll chances and the next drift check. Names can be clicked to inspect the other person. Familiarity adds a small bounded adjustment (at most ±0.1) to the existing NEAT benefit input for courtship, tavern visits and retail purchases; it does not force marriage, purchases or hostility and adds no new neural node IDs.

Shared-time checks are staggered across the game hour, contacts are stored sparsely, and an ordered due index processes only relationships whose monthly review is due. Relationship randomness has its own saved stream rather than consuming the existing evolutionary or family random stream. Neighbour and decay indices are rebuilt when loading; score history, cooldowns and random state survive replay exactly.

The relationship tests cover combined probabilities, both limits, once-per-hour rolls, mutual scores, capacity on either side, zero removal, both signs of monthly drift, actual house grouping, caravan/tavern presence, retail encounters, death cleanup and exact save/replay. `experiments/check_relationships.py` additionally exercises the isolated TCP backend and writes an authentic inspector fixture for the Godot UI check.
