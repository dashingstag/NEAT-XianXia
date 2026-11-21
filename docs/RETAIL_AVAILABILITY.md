# Retail availability audit

The candidate audit compares 750 ticks starting at 10:00 or 21:00. It covers three 1,000-person urban-demo seeds (17, 42, 77) after 1,500 warm-up ticks and a mature 820-person bank town at tick 24,000. See the reproducible command in the root README.

In the eight ordinary windows, no occupied shop held a sellable catalog item at any pre-tick observation. Across the whole population, shop sale counters recorded zero purchases. The general decision sample contained 4,013 committed choices. At the first supply gate, empty shelves fully explain why shopping was absent from that sample.

In the mature fixture, eight occupied shops contributed 6,000 shop-ticks per window. At least one was open for 653 daytime ticks and 454 overnight ticks. In both windows a real, funded wholesale order was available somewhere on every tick. No shop had a pending firm job on any observed tick. The younger towns had an eligible supplier for only 56, 82 and 311 daytime ticks respectively, and zero overnight ticks.

## What shop owners chose

A second pass sampled only scheduled owners of occupied shops. The audit captured 342 committed daytime decisions and 315 overnight. During the day, 271 included OrderGoods. OperateBusiness won 270 and BuyFood won one. Overnight, 195 included OrderGoods. OperateBusiness won 116 and Rest won 79. This identifies competition at the owner's choice stage before physical hauling. The sample does not establish how much a changed policy would increase sales.

## Controlled openings

Starting from the same mature daytime checkpoint, the inventory control added 20 accounted items per occupied shop in its existing specialty. The opening control set the last-service tick once. Neither control forced an action by an owner, buyer or haulier.

| Condition | Ticks with any open, stocked shop | Sampled decisions with shopping admitted | Shopping selected in sample | Completed shop sales, whole town |
|---|---:|---:|---:|---:|
| Ordinary | 0 | 0 | 0 | 0 |
| Opening only | 0 | 0 | 0 | 0 |
| Stock only | 653 | 33 | 0 | 0 |
| Opening and stock | 653 | 33 | 0 | 0 |

In the stock-only case, the sampled actor could afford and carry at least one open shop item on 133 ticks. Pre-tick urban offer generation produced a shopping candidate on 78 ticks. The 33 admitted decisions were captured during the actual simulation step. Those counts use different observation points and denominators. Inventory is the first constraint and stocking a shelf alone did not produce a completed sale in this six-game-hour window.

All 13 observed parallel continuations matched the serialized state of their unobserved serial controls. Raw output belongs in the ignored .work directory.

## Empty-counter owner experiment

Run from the repository root with `cargo run --release --locked --manifest-path sim/Cargo.toml -p sim-core --example restocking -- fixtures/bank-v12.snapshot`. Repeat with `--features experimental-restocking` before the example argument. This opt-in feature adjusts only the benefit inputs for a shopkeeper with empty catalog shelves and a feasible, funded order. Tending the counter drops from 0.9 to 0.1 and ordering increases from 0.85 to 1.0.

Two days (6,000 ticks) from the same mature seed-42 checkpoint gave the following cumulative results. Each day passed conservation checks and reproduced the same serialized state after a checkpoint reload.

| Two-day result | Default | Experimental |
|---|---:|---:|
| Orders / pickups / deliveries | 0 / 0 / 0 | 7 / 7 / 7 |
| Wholesale and freight spending | 0 | 104 |
| Stocked-and-open shop-ticks | 0 | 11,070 |
| Retail units sold | 0 | 0 |
| Occupied shops' ending cash | 213 | 147 |
| Sum of occupied shops' arrears | 0 | 1 |
| Initial owners alive | 8/8 | 8/8 |

Shop-ticks sum across shops. End cash also reflects other business activity.
Owners triggered physical restocking in this fixture. Sustainable retail
demand remains unverified.

### Observed buyer decisions

A second experimental run reproduced 11,070 stocked-and-open shop-ticks,
seven deliveries, and zero sales. In a rotating sample of 3,520 committed
decisions, shopping was admitted 56 times, omitted at the candidate cap
zero times, and selected zero times. Rest won 26 of the admitted decisions,
operating a business 11, and six other actions won the remaining 19.
The sample yielded zero shopping travel or checkout events. The pre-tick
probe found
an affordable item with room to carry it on 111 sampled actor-ticks
and a feasible urban shopping offer on 85. These probe counts use a
different observation point from committed decisions. Conservation and
checkpoint replay passed on both days. Choice competition was the largest
observed drop after offer admission in this seed.


### Rerun after the Frontier food-access changes

The same mature fixture with experimental restocking on PR #43 produced
three completed deliveries and 1,311 stocked-and-open shop-ticks on day
two. The cumulative decision sample saw 3,468 decisions, 32 actor-ticks
with an affordable item that fit, and 24 pre-tick urban shopping offers.
Of the 12 admitted shopping choices, none won. Rest won six, farming won
two, and four other actions each won once. The candidate cap omitted none.
No sampled shopper started travel or reached checkout, and the entire
town made zero retail sales. Conservation and exact checkpoint replay
passed on both days.

The controlled last-item test exercises checkout with a stocked shop and
eligible buyers. In the organic run, the first observed failure after
offer admission was action selection. The experiment does not establish
whether a change to buyer preferences would produce sustained demand.
