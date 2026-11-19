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
