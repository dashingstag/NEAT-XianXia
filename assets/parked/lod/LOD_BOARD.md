# NEAT Xianxia — LOD Asset Board

Pre-integration review of all drawable objects at the three gameplay fidelities (L1 overview dots/landmarks, L2 drawn individuals, L3 animation). Animated previews are APNG files; open `assets/LOD_BOARD.html` for the live board.

## Settlements

Stage shapes match the live LOD1 markers in `field.gd`; at LOD2+ the market well and housing ring draw in full.

| ID | L1 | L2 | L3 | Notes |
|---|---|---|---|---|
| `settlement-camp` | ![](dots/settlement-camp.png) | ![](../lod/l2/settlement-camp.png) | ![](apng/settlement-camp.png) | LOD1 shape is the live field.gd marker; market well + housing ring drawn at LOD2+ |
| `settlement-village` | ![](dots/settlement-village.png) | ![](../lod/l2/settlement-village.png) | ![](apng/settlement-village.png) | LOD1 shape is the live field.gd marker; market well + housing ring drawn at LOD2+ |
| `settlement-town` | ![](dots/settlement-town.png) | ![](../lod/l2/settlement-town.png) | ![](apng/settlement-town.png) | LOD1 shape is the live field.gd marker; market well + housing ring drawn at LOD2+ |
| `market-well` | ![](dots/market-well.png) | ![](../site-glyphs-xianxia/market.png) | ![](apng/market.png) | Well ripple + bucket bob at LOD3 |

## Buildings & sites

Glyph = parked `site-glyphs-xianxia` texture. LOD3 adds a per-site ambient accent (smoke, lantern glow, swinging sign, turning mill wheel).

| ID | L1 | L2 | L3 | Notes |
|---|---|---|---|---|
| `home` | ![](dots/home.png) | ![](../site-glyphs-xianxia/home.png) | ![](apng/home.png) | Housing blocks + kind-0 urban buildings |
| `shop-clothes` | ![](dots/shop-clothes.png) | ![](../site-glyphs-xianxia/shop-clothes.png) | ![](apng/shop-clothes.png) | kind 1, line 0 |
| `shop-tools` | ![](dots/shop-tools.png) | ![](../site-glyphs-xianxia/shop-tools.png) | ![](apng/shop-tools.png) | kind 1, line 1 |
| `shop-furniture` | ![](dots/shop-furniture.png) | ![](../site-glyphs-xianxia/shop-furniture.png) | ![](apng/shop-furniture.png) | kind 1, line 2 |
| `shop-butcher` | ![](dots/shop-butcher.png) | ![](../site-glyphs-xianxia/shop-butcher.png) | ![](apng/shop-butcher.png) | kind 1, line 3 |
| `workshop-hunter` | ![](dots/workshop-hunter.png) | ![](../site-glyphs-xianxia/workshop-hunter.png) | ![](apng/workshop-hunter.png) | kind 2, line 0 |
| `workshop-cobbler` | ![](dots/workshop-cobbler.png) | ![](../site-glyphs-xianxia/workshop-cobbler.png) | ![](apng/workshop-cobbler.png) | kind 2, line 1 |
| `workshop-blacksmith` | ![](dots/workshop-blacksmith.png) | ![](../site-glyphs-xianxia/workshop-blacksmith.png) | ![](apng/workshop-blacksmith.png) | kind 2, line 2 |
| `workshop-carpenter` | ![](dots/workshop-carpenter.png) | ![](../site-glyphs-xianxia/workshop-carpenter.png) | ![](apng/workshop-carpenter.png) | kind 2, line 3 |
| `bank` | ![](dots/bank.png) | ![](../site-glyphs-xianxia/bank.png) | ![](apng/bank.png) | town bank, rate label |
| `tavern` | ![](dots/tavern.png) | ![](../site-glyphs-xianxia/tavern.png) | ![](apng/tavern.png) | visitors/32 + lantern landmark |
| `school` | ![](dots/school.png) | ![](../site-glyphs-xianxia/school.png) | ![](apng/school.png) | paifang gate landmark |
| `farm` | ![](dots/farm.png) | ![](../site-glyphs-xianxia/farm.png) | ![](apng/farm.png) | mature vs growing paddy |
| `graveyard` | ![](dots/graveyard.png) | ![](../site-glyphs-xianxia/graveyard.png) | ![](apng/graveyard.png) | buried_count/capacity steles |
| `shelter` | ![](dots/shelter.png) | ![](../site-glyphs-xianxia/shelter.png) | ![](apng/shelter.png) | bed occupancy ring |
| `jobboard` | ![](dots/jobboard.png) | ![](../site-glyphs-xianxia/jobboard.png) | ![](apng/jobboard.png) | vacancies + nearby bodies dot |
| `mine-gold` | ![](dots/mine-gold.png) | ![](../site-glyphs-xianxia/mine-gold.png) | ![](apng/mine-gold.png) | patch kind 4 |
| `mine-silver` | ![](dots/mine-silver.png) | ![](../lod/l2/mine-silver.png) | ![](apng/mine-silver.png) | patch kind 5 (reuses mine glyph today) |
| `mine-copper` | ![](dots/mine-copper.png) | ![](../lod/l2/mine-copper.png) | ![](apng/mine-copper.png) | patch kind 6 (reuses mine glyph today) |
| `mine-iron` | ![](dots/mine-iron.png) | ![](../site-glyphs-xianxia/mine-iron.png) | ![](apng/mine-iron.png) | patch kind 7 |
| `gate` | ![](dots/gate.png) | ![](../site-glyphs-xianxia/gate.png) | ![](apng/gate.png) | immortal site — dormant |
| `pavilion` | ![](dots/pavilion.png) | ![](../site-glyphs-xianxia/pavilion.png) | ![](apng/pavilion.png) | immortal site — dormant |

## Resource patches

LOD1 keeps the in-game convention: color dots scale with stock, mines are ore-colored diamonds.

| ID | L1 | L2 | L3 | Notes |
|---|---|---|---|---|
| `patch-berries` | ![](dots/patch-berries.png) | ![](../terrain-flora-xianxia/berries.png) | ![](apng/patch-berries.png) | dots scale with stock |
| `patch-timber` | ![](dots/patch-timber.png) | ![](../terrain-flora-xianxia/timber.png) | ![](apng/patch-timber.png) | larger crown radius |
| `patch-stone` | ![](dots/patch-stone.png) | ![](../terrain-flora-xianxia/stone.png) | ![](apng/patch-stone.png) | resource patch |
| `patch-fish` | ![](dots/patch-fish.png) | ![](../terrain-flora-xianxia/fish.png) | ![](apng/patch-fish.png) | coastal food source |
| `patch-gold` | ![](dots/patch-gold.png) | ![](../site-glyphs-xianxia/mine-gold.png) | ![](apng/patch-gold.png) | mine diamond at LOD1 |
| `patch-silver` | ![](dots/patch-silver.png) | ![](../lod/l2/mine-silver.png) | ![](apng/patch-silver.png) | resource patch |
| `patch-copper` | ![](dots/patch-copper.png) | ![](../lod/l2/mine-copper.png) | ![](apng/patch-copper.png) | resource patch |
| `patch-iron` | ![](dots/patch-iron.png) | ![](../site-glyphs-xianxia/mine-iron.png) | ![](apng/patch-iron.png) | resource patch |
| `patch-herbs` | ![](dots/patch-herbs.png) | ![](../terrain-flora-xianxia/herbs.png) | ![](apng/patch-herbs.png) | dormant — future kind |
| `patch-spirit-herbs` | ![](dots/patch-spirit-herbs.png) | ![](../terrain-flora-xianxia/spirit-herbs.png) | ![](apng/patch-spirit-herbs.png) | dormant — rare qi herb |
| `patch-mushrooms` | ![](dots/patch-mushrooms.png) | ![](../terrain-flora-xianxia/mushrooms.png) | ![](apng/patch-mushrooms.png) | dormant — future kind |

## Flora & terrain features

Tree canopies sway, water ripples, mist drifts. Features that the engine draws procedurally (roads, coast, hill, fog) get LOD2 cells under `parked/lod/anim/`.

| ID | L1 | L2 | L3 | Notes |
|---|---|---|---|---|
| `pine` | ![](dots/pine.png) | ![](../terrain-flora-xianxia/pine.png) | ![](apng/pine.png) | cool groves |
| `bamboo` | ![](dots/bamboo.png) | ![](../terrain-flora-xianxia/bamboo.png) | ![](apng/bamboo.png) | coastal groves |
| `willow` | ![](dots/willow.png) | ![](../terrain-flora-xianxia/willow.png) | ![](apng/willow.png) | waterside groves |
| `plum` | ![](dots/plum.png) | ![](../terrain-flora-xianxia/plum.png) | ![](apng/plum.png) | ~1 in 7 accent |
| `tree-generic` | ![](dots/tree-generic.png) | ![](../terrain-flora-xianxia/tree-generic.png) | ![](apng/tree-generic.png) | default grove tree |
| `forest-grove` | ![](dots/forest-grove.png) | ![](../lod/l2/forest-grove.png) | ![](apng/forest-grove.png) | canopy layer + floor, every-3rd tree at LOD1 |
| `grass` | ![](dots/grass.png) | ![](../lod/l2/grass.png) | ![](apng/grass.png) | grazing regrowth for wildlife |
| `fertility` | ![](dots/fertility.png) | ![](../lod/l2/fertility.png) | ![](apng/fertility.png) | farm placement tint |
| `hill` | ![](dots/hill.png) | ![](../lod/l2/hill.png) | ![](apng/hill.png) | classic terrain landmark |
| `coast` | ![](dots/coast.png) | ![](../lod/l2/coast.png) | ![](apng/coast.png) | shore + waterline waves |
| `river` | ![](dots/river.png) | ![](../lod/l2/river.png) | ![](apng/river.png) | generated-map watercourse |
| `road` | ![](dots/road.png) | ![](../lod/l2/road.png) | ![](apng/road.png) | polyline corridor; dashed when unbuilt |
| `lotus` | ![](dots/lotus.png) | ![](../terrain-flora-xianxia/lotus.png) | ![](apng/lotus.png) | coast-edge accent flora |
| `mist` | ![](dots/mist.png) | ![](../terrain-flora-xianxia/mist.png) | ![](apng/mist.png) | immortal-site ambience |
| `unknown-site` | ![](dots/unknown-site.png) | ![](../terrain-flora-xianxia/unknown-site.png) | ![](apng/unknown-site.png) | procedural-expansion marker |
| `fog` | ![](dots/fog.png) | ![](../lod/l2/fog.png) | ![](apng/fog.png) | explored-chunk fog layer |

## People

LOD1 is the existing action-colored dot (MultiMesh quad); LOD2 shows the villager-atlas stance sheet; LOD3 is the 8-frame walk cycle. Work-cycle strips ship alongside as `anim/<id>-work.png`.

| ID | L1 | L2 | L3 | Notes |
|---|---|---|---|---|
| `action-dots` | ![](dots/person.png) | ![](../lod/l2/action-dots.png) | ![](l2/action-dots.png) | LOD1 person dot is colored by current action: Rest, Gather, Deliver, Buy food, Build, Trade / other, Sleeping, Dead |
| `farmer-straw` | ![](dots/person.png) | ![](../villager-atlas-v2/stances/farmer-straw.png) | ![](apng/farmer-straw-walk.png) | atlas col 0; work cycle strip also emitted |
| `farmer-douli` | ![](dots/person.png) | ![](../villager-atlas-v2/stances/farmer-douli.png) | ![](apng/farmer-douli-walk.png) | atlas col 1; work cycle strip also emitted |
| `fieldhand` | ![](dots/person.png) | ![](../villager-atlas-v2/stances/fieldhand.png) | ![](apng/fieldhand-walk.png) | atlas col 2; work cycle strip also emitted |
| `reaper` | ![](dots/person.png) | ![](../villager-atlas-v2/stances/reaper.png) | ![](apng/reaper-walk.png) | atlas col 3; work cycle strip also emitted |
| `porter` | ![](dots/person.png) | ![](../villager-atlas-v2/stances/porter.png) | ![](apng/porter-walk.png) | atlas col 4; work cycle strip also emitted |
| `laborer` | ![](dots/person.png) | ![](../villager-atlas-v2/stances/laborer.png) | ![](apng/laborer-walk.png) | atlas col 5; work cycle strip also emitted |
| `artisan` | ![](dots/person.png) | ![](../villager-atlas-v2/stances/artisan.png) | ![](apng/artisan-walk.png) | atlas col 6; work cycle strip also emitted |
| `smith` | ![](dots/person.png) | ![](../villager-atlas-v2/stances/smith.png) | ![](apng/smith-walk.png) | atlas col 7; work cycle strip also emitted |
| `merchant` | ![](dots/person.png) | ![](../villager-atlas-v2/stances/merchant.png) | ![](apng/merchant-walk.png) | atlas col 8; work cycle strip also emitted |
| `trader` | ![](dots/person.png) | ![](../villager-atlas-v2/stances/trader.png) | ![](apng/trader-walk.png) | atlas col 9; work cycle strip also emitted |
| `fisher` | ![](dots/person.png) | ![](../villager-atlas-v2/stances/fisher.png) | ![](apng/fisher-walk.png) | atlas col 10; work cycle strip also emitted |
| `elder` | ![](dots/person.png) | ![](../villager-atlas-v2/stances/elder.png) | ![](apng/elder-walk.png) | atlas col 11; work cycle strip also emitted |
| `swordsman` | ![](dots/person.png) | ![](../villager-atlas-v2/stances/swordsman.png) | ![](apng/swordsman-walk.png) | atlas col 12; work cycle strip also emitted |
| `cultivator` | ![](dots/person.png) | ![](../villager-atlas-v2/stances/cultivator.png) | ![](apng/cultivator-walk.png) | atlas col 13; work cycle strip also emitted |
| `elder-sage` | ![](dots/person.png) | ![](../villager-atlas-v2/stances/elder-sage.png) | ![](apng/elder-sage-walk.png) | atlas col 14; work cycle strip also emitted |
| `spirit-envoy` | ![](dots/person.png) | ![](../villager-atlas-v2/stances/spirit-envoy.png) | ![](apng/spirit-envoy-walk.png) | atlas col 15; work cycle strip also emitted |

## Wildlife

LOD1 species dot (size/color per `field.gd`); LOD3 is the 4-frame walk cycle. Graze/flee/dead rows already exist in the animal atlas.

| ID | L1 | L2 | L3 | Notes |
|---|---|---|---|---|
| `rabbit` | ![](dots/rabbit.png) | ![](../animal-table-xianxia/rabbit.png) | ![](apng/rabbit.png) | atlas col 0; graze/flee/dead rows exist in atlas |
| `deer` | ![](dots/deer.png) | ![](../animal-table-xianxia/deer.png) | ![](apng/deer.png) | atlas col 1; graze/flee/dead rows exist in atlas |
| `boar` | ![](dots/boar.png) | ![](../animal-table-xianxia/boar.png) | ![](apng/boar.png) | atlas col 2; graze/flee/dead rows exist in atlas |
| `pheasant` | ![](dots/pheasant.png) | ![](../animal-table-xianxia/pheasant.png) | ![](apng/pheasant.png) | atlas col 3; graze/flee/dead rows exist in atlas |
| `fox` | ![](dots/fox.png) | ![](../animal-table-xianxia/fox.png) | ![](apng/fox.png) | atlas col 4; graze/flee/dead rows exist in atlas |
| `crane` | ![](dots/crane.png) | ![](../animal-table-xianxia/crane.png) | ![](apng/crane.png) | atlas col 5; graze/flee/dead rows exist in atlas |
| `spirit-deer` | ![](dots/spirit-deer.png) | ![](../animal-table-xianxia/spirit-deer.png) | ![](apng/spirit-deer.png) | atlas col 6; graze/flee/dead rows exist in atlas |
| `spirit-fox` | ![](dots/spirit-fox.png) | ![](../animal-table-xianxia/spirit-fox.png) | ![](apng/spirit-fox.png) | atlas col 7; graze/flee/dead rows exist in atlas |
| `qilin-colt` | ![](dots/qilin-colt.png) | ![](../animal-table-xianxia/qilin-colt.png) | ![](apng/qilin-colt.png) | atlas col 8; graze/flee/dead rows exist in atlas |

## Overlays & FX

Interface marks: convoys, rally points, zones, selection, trade routes and the discovery reveal.

| ID | L1 | L2 | L3 | Notes |
|---|---|---|---|---|
| `convoy` | ![](dots/convoy.png) | ![](../lod/l2/convoy.png) | ![](apng/convoy.png) | cart + member count; LOD1-only draw today |
| `rally` | ![](dots/rally.png) | ![](../lod/l2/rally.png) | ![](apng/rally.png) | pre-departure gathering marker |
| `selection` | ![](dots/selection.png) | ![](../lod/l2/selection.png) | ![](apng/selection.png) | person/animal/site focus ring |
| `zone-residential` | ![](dots/zone-residential.png) | ![](../lod/l2/zone-residential.png) | ![](apng/zone-residential.png) | show_zones overlay |
| `zone-commercial` | ![](dots/zone-commercial.png) | ![](../lod/l2/zone-commercial.png) | ![](apng/zone-commercial.png) | show_zones overlay |
| `zone-industrial` | ![](dots/zone-industrial.png) | ![](../lod/l2/zone-industrial.png) | ![](apng/zone-industrial.png) | show_zones overlay |
| `trade-route` | ![](dots/trade-route.png) | ![](../lod/l2/trade-route.png) | ![](apng/trade-route.png) | cumulative road cargo overlay |
| `reveal` | ![](dots/reveal.png) | ![](../discovery-fx/reveal.png) | ![](apng/reveal.png) | existing 4-frame fx contract |
