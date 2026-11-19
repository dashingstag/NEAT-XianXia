# site-glyphs-xianxia — building & site glyphs

One PNG per site type (64×64). Xianxia landmarks: pagoda bank, lantern
tavern, paifang school, plus immortal-tier mountain gate and pavilion
(parked for future immortal sites).

Integration: per-kind glyph functions in `viewer/site_art.gd` called from
`field.gd` `_draw()` — replaces inline rect+triangle drawings. Under-
construction keeps the existing outline state.
