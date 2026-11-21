# NEAT Xianxia — Parked Asset Catalog

**Runtime** files are loaded by Godot. **Parked** files are source
art, contact sheets, or assets awaiting integration.
The use column includes planned uses. Regenerate with
`python3 assets/tools/gen_assets.py && python3 assets/tools/build_catalog.py`.


## `villager-atlas-v2`

Villager atlas v2 — `atlas.png` (1024×1280), 16 cols × 20 pose rows. Stance sheets per figure show: idle, walk×2, work×2, carry, dead, lying.

| ID | Image | Runtime status | Current or planned use |
|---|---|---|---|
| `villager-atlas-v2/farmer-straw` | ![](villager-atlas-v2/stances/farmer-straw.png) | Parked | Atlas col 0 — farmers on Farm/HarvestRice actions |
| `villager-atlas-v2/farmer-douli` | ![](villager-atlas-v2/stances/farmer-douli.png) | Parked | Atlas col 1 — gatherers, ForageMeal |
| `villager-atlas-v2/fieldhand` | ![](villager-atlas-v2/stances/fieldhand.png) | Parked | Atlas col 2 — WorkForFood laborers |
| `villager-atlas-v2/reaper` | ![](villager-atlas-v2/stances/reaper.png) | Parked | Atlas col 3 — harvest workers |
| `villager-atlas-v2/porter` | ![](villager-atlas-v2/stances/porter.png) | Parked | Atlas col 4 — Deliver/Trade carriers |
| `villager-atlas-v2/laborer` | ![](villager-atlas-v2/stances/laborer.png) | Parked | Atlas col 5 — Build/construction crews |
| `villager-atlas-v2/artisan` | ![](villager-atlas-v2/stances/artisan.png) | Parked | Atlas col 6 — CraftTool artisans |
| `villager-atlas-v2/smith` | ![](villager-atlas-v2/stances/smith.png) | Parked | Atlas col 7 — blacksmith workshop |
| `villager-atlas-v2/merchant` | ![](villager-atlas-v2/stances/merchant.png) | Parked | Atlas col 8 — OperateBusiness shopkeepers |
| `villager-atlas-v2/trader` | ![](villager-atlas-v2/stances/trader.png) | Parked | Atlas col 9 — Trade/Barter at market |
| `villager-atlas-v2/fisher` | ![](villager-atlas-v2/stances/fisher.png) | Parked | Atlas col 10 — coastal fish gatherers |
| `villager-atlas-v2/elder` | ![](villager-atlas-v2/stances/elder.png) | Parked | Atlas col 11 — elderly villagers |
| `villager-atlas-v2/swordsman` | ![](villager-atlas-v2/stances/swordsman.png) | Parked | Atlas col 12 — cultivator (dormant) |
| `villager-atlas-v2/cultivator` | ![](villager-atlas-v2/stances/cultivator.png) | Parked | Atlas col 13 — robed cultivator (dormant) |
| `villager-atlas-v2/elder-sage` | ![](villager-atlas-v2/stances/elder-sage.png) | Parked | Atlas col 14 — elder sage (dormant) |
| `villager-atlas-v2/spirit-envoy` | ![](villager-atlas-v2/stances/spirit-envoy.png) | Parked | Atlas col 15 — spirit envoy (dormant) |
| `villager-atlas-v2/atlas` | ![](villager-atlas-v2/atlas.png) | Runtime | Full atlas grid |


## `animal-table-xianxia`

Animal stance sheets — idle, walk×2, graze, flee, dead. `atlas.png` = 9×6 grid.

| ID | Image | Runtime status | Current or planned use |
|---|---|---|---|
| `animal-table-xianxia/rabbit` | ![](animal-table-xianxia/rabbit.png) | Parked | Wildlife renderer — grove herbivore (existing) |
| `animal-table-xianxia/deer` | ![](animal-table-xianxia/deer.png) | Parked | Wildlife renderer — grove herbivore (existing) |
| `animal-table-xianxia/boar` | ![](animal-table-xianxia/boar.png) | Parked | Future species — forest forager |
| `animal-table-xianxia/pheasant` | ![](animal-table-xianxia/pheasant.png) | Parked | Future species — grove bird |
| `animal-table-xianxia/fox` | ![](animal-table-xianxia/fox.png) | Parked | Future species — first predator |
| `animal-table-xianxia/crane` | ![](animal-table-xianxia/crane.png) | Parked | Future species — coast/wetland bird |
| `animal-table-xianxia/spirit-deer` | ![](animal-table-xianxia/spirit-deer.png) | Parked | Dormant immortal-tier spirit beast |
| `animal-table-xianxia/spirit-fox` | ![](animal-table-xianxia/spirit-fox.png) | Parked | Dormant immortal-tier spirit beast |
| `animal-table-xianxia/qilin-colt` | ![](animal-table-xianxia/qilin-colt.png) | Parked | Dormant rare spirit beast |
| `animal-table-xianxia/atlas` | ![](animal-table-xianxia/atlas.png) | Runtime | Full atlas grid |


## `site-glyphs-xianxia`

Building & site glyphs (64×64).

| ID | Image | Runtime status | Current or planned use |
|---|---|---|---|
| `site-glyphs-xianxia/home` | ![](site-glyphs-xianxia/home.png) | Runtime | Housing blocks in `field.gd` |
| `site-glyphs-xianxia/shop` | ![](site-glyphs-xianxia/shop.png) | Parked | Shop buildings (clothes/tools/furniture/butcher sign variants) |
| `site-glyphs-xianxia/workshop` | ![](site-glyphs-xianxia/workshop.png) | Parked | Workshop buildings (hunter/cobbler/blacksmith/carpenter) |
| `site-glyphs-xianxia/bank` | ![](site-glyphs-xianxia/bank.png) | Runtime | Town bank — pagoda landmark |
| `site-glyphs-xianxia/tavern` | ![](site-glyphs-xianxia/tavern.png) | Runtime | Village tavern — lantern landmark |
| `site-glyphs-xianxia/school` | ![](site-glyphs-xianxia/school.png) | Runtime | School — paifang gate |
| `site-glyphs-xianxia/farm` | ![](site-glyphs-xianxia/farm.png) | Runtime | Rice farm plots |
| `site-glyphs-xianxia/graveyard` | ![](site-glyphs-xianxia/graveyard.png) | Runtime | Graveyard — stele rows |
| `site-glyphs-xianxia/shelter` | ![](site-glyphs-xianxia/shelter.png) | Runtime | Roadside camps |
| `site-glyphs-xianxia/jobboard` | ![](site-glyphs-xianxia/jobboard.png) | Runtime | Job posting board |
| `site-glyphs-xianxia/market` | ![](site-glyphs-xianxia/market.png) | Runtime | Market centre well |
| `site-glyphs-xianxia/mine-iron` | ![](site-glyphs-xianxia/mine-iron.png) | Runtime | Iron mine — headframe + ore |
| `site-glyphs-xianxia/mine-gold` | ![](site-glyphs-xianxia/mine-gold.png) | Runtime | Gold mine (silver/copper share the glyph, ore color swaps) |
| `site-glyphs-xianxia/gate` | ![](site-glyphs-xianxia/gate.png) | Parked | Immortal site — mountain gate (dormant) |
| `site-glyphs-xianxia/pavilion` | ![](site-glyphs-xianxia/pavilion.png) | Parked | Immortal site — pavilion (dormant) |
| `site-glyphs-xianxia/sheet` | ![](site-glyphs-xianxia/sheet.png) | Parked | Contact sheet |


## `terrain-flora-xianxia`

Terrain, patch and flora glyphs (64×64).

| ID | Image | Runtime status | Current or planned use |
|---|---|---|---|
| `terrain-flora-xianxia/pine` | ![](terrain-flora-xianxia/pine.png) | Runtime | Forest canopy archetype (cool groves) |
| `terrain-flora-xianxia/bamboo` | ![](terrain-flora-xianxia/bamboo.png) | Runtime | Forest canopy archetype (coastal-biased) |
| `terrain-flora-xianxia/willow` | ![](terrain-flora-xianxia/willow.png) | Runtime | Forest canopy archetype (near water/valley) |
| `terrain-flora-xianxia/plum` | ![](terrain-flora-xianxia/plum.png) | Runtime | Rare accent tree (~1 in 7 groves) |
| `terrain-flora-xianxia/tree-generic` | ![](terrain-flora-xianxia/tree-generic.png) | Runtime | Default grove tree |
| `terrain-flora-xianxia/berries` | ![](terrain-flora-xianxia/berries.png) | Runtime | Berry patch (kind 0) — dots scale with stock |
| `terrain-flora-xianxia/timber` | ![](terrain-flora-xianxia/timber.png) | Runtime | Timber patch (kind 1) |
| `terrain-flora-xianxia/stone` | ![](terrain-flora-xianxia/stone.png) | Runtime | Stone outcrop (kind 2) |
| `terrain-flora-xianxia/fish` | ![](terrain-flora-xianxia/fish.png) | Runtime | Fish shoal (kind 3) |
| `terrain-flora-xianxia/herbs` | ![](terrain-flora-xianxia/herbs.png) | Parked | Future patch kind — herb forage |
| `terrain-flora-xianxia/spirit-herbs` | ![](terrain-flora-xianxia/spirit-herbs.png) | Parked | Future rare patch kind — spirit herb |
| `terrain-flora-xianxia/mushrooms` | ![](terrain-flora-xianxia/mushrooms.png) | Parked | Future patch kind — forest mushrooms |
| `terrain-flora-xianxia/lotus` | ![](terrain-flora-xianxia/lotus.png) | Parked | Coast-edge accent flora |
| `terrain-flora-xianxia/mist` | ![](terrain-flora-xianxia/mist.png) | Parked | Qi mist wisps — immortal-site ambience |
| `terrain-flora-xianxia/unknown-site` | ![](terrain-flora-xianxia/unknown-site.png) | Parked | Undiscovered site marker — procedural-expansion engine |
| `terrain-flora-xianxia/sheet` | ![](terrain-flora-xianxia/sheet.png) | Parked | Contact sheet |


## `icons-brush`

Brush-stroke panel icons (24×24).

| ID | Image | Runtime status | Current or planned use |
|---|---|---|---|
| `icons-brush/berries` | ![](icons-brush/berries.png) | Runtime | Resource menus / ledgers — food good |
| `icons-brush/timber` | ![](icons-brush/timber.png) | Runtime | Resource menus — material good |
| `icons-brush/stone` | ![](icons-brush/stone.png) | Parked | Resource menus — material good |
| `icons-brush/fish` | ![](icons-brush/fish.png) | Parked | Resource menus — food good |
| `icons-brush/gold` | ![](icons-brush/gold.png) | Runtime | Resource menus — ore good |
| `icons-brush/silver` | ![](icons-brush/silver.png) | Runtime | Resource menus — ore good |
| `icons-brush/copper` | ![](icons-brush/copper.png) | Runtime | Resource menus — ore good |
| `icons-brush/iron` | ![](icons-brush/iron.png) | Runtime | Resource menus — ore good |
| `icons-brush/rice` | ![](icons-brush/rice.png) | Parked | Resource menus — food good |
| `icons-brush/axe` | ![](icons-brush/axe.png) | Parked | Tool inventories |
| `icons-brush/pickaxe` | ![](icons-brush/pickaxe.png) | Parked | Tool inventories |
| `icons-brush/hoe` | ![](icons-brush/hoe.png) | Parked | Tool inventories / farm jobs |
| `icons-brush/shovel` | ![](icons-brush/shovel.png) | Parked | Tool inventories |
| `icons-brush/home` | ![](icons-brush/home.png) | Parked | Inspector title — housing |
| `icons-brush/shop` | ![](icons-brush/shop.png) | Parked | Inspector title — shops |
| `icons-brush/workshop` | ![](icons-brush/workshop.png) | Parked | Inspector title — workshops |
| `icons-brush/bank` | ![](icons-brush/bank.png) | Parked | Inspector title — bank |
| `icons-brush/school` | ![](icons-brush/school.png) | Parked | Inspector title — school |
| `icons-brush/tavern` | ![](icons-brush/tavern.png) | Parked | Inspector title — tavern |
| `icons-brush/farm` | ![](icons-brush/farm.png) | Parked | Inspector title — farm |
| `icons-brush/graveyard` | ![](icons-brush/graveyard.png) | Parked | Inspector title — graveyard |
| `icons-brush/shelter` | ![](icons-brush/shelter.png) | Parked | Inspector title — camps |
| `icons-brush/jobboard` | ![](icons-brush/jobboard.png) | Parked | Job board panel |
| `icons-brush/market` | ![](icons-brush/market.png) | Parked | Market inspector |
| `icons-brush/road` | ![](icons-brush/road.png) | Parked | Road inspector |
| `icons-brush/sword` | ![](icons-brush/sword.png) | Parked | Hunt / cultivator actions (dormant) |
| `icons-brush/scroll` | ![](icons-brush/scroll.png) | Parked | School / contract rows |
| `icons-brush/lantern` | ![](icons-brush/lantern.png) | Parked | Tavern / night rows |
| `icons-brush/ingot` | ![](icons-brush/ingot.png) | Parked | Treasury / bank balances |
| `icons-brush/lotus` | ![](icons-brush/lotus.png) | Parked | Spirit-site rows (dormant) |
| `icons-brush/seal` | ![](icons-brush/seal.png) | Parked | UI signature stamp (cinnabar seal) |
| `icons-brush/sheet` | ![](icons-brush/sheet.png) | Parked | Contact sheet |


## `discovery-fx`

Mist & reveal animation frames.

| ID | Image | Runtime status | Current or planned use |
|---|---|---|---|
| `discovery-fx/reveal` | ![](discovery-fx/reveal.png) | Parked | 4-frame expanding-ring reveal + qi wisps — procedural-expansion engine (art-spec §9) |
