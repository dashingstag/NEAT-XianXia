shader_type canvas_item;
render_mode unshaded;
uniform float clock = 0.0;
uniform vec4 daylight_tint = vec4(1.0);
uniform float blend_amount = 1.0;
varying vec2 atlas_cell;
varying float rested_badge;
varying float carrying_body;
void vertex() {
    float packed = INSTANCE_CUSTOM.r;
    float mode_flags = floor(packed / 16.0);
    float mode = mod(mode_flags,8.0);
    rested_badge = mod(floor(mode_flags/8.0),2.0);
    carrying_body = mod(floor(mode_flags/16.0),2.0);
    float variant = mod(packed, 16.0);
    float pose = 0.0;
    float frame = floor(mod(clock * 10.0 + INSTANCE_CUSTOM.a, 8.0));
    if (mode == 1.0) { pose = 1.0 + frame; }
    else if (mode == 2.0) { pose = 9.0 + frame; }
    else if (mode == 3.0) { pose = 17.0; }
    else if (mode == 4.0) { pose = 18.0; }
    else if (mode == 5.0) { pose = 19.0; }
    atlas_cell = vec2(variant, pose);
    VERTEX += INSTANCE_CUSTOM.gb * (1.0 - blend_amount);
}
void fragment() {
    // QuadMesh UVs point downward in this 2D setup. Flip within the cell,
    // never the entire atlas, to preserve pose rows and put heads above feet.
    vec2 sprite_uv = vec2(UV.x, 1.0-UV.y);
    COLOR = texture(TEXTURE, (sprite_uv + atlas_cell) / vec2(16.0, 20.0));
    vec2 bundle = sprite_uv-vec2(0.50,0.60);
    if (carrying_body > 0.5 && abs(bundle.x)<0.35 && abs(bundle.y)<0.12) {
        COLOR = vec4(0.40,0.29,0.19,1.0);
        if (abs(bundle.x)<0.02 || (abs(bundle.y)<0.02 && abs(bundle.x)<0.12)) { COLOR=vec4(0.72,0.63,0.45,1.0); }
    }
    vec2 badge = sprite_uv - vec2(0.80,0.12);
    if (rested_badge > 0.5 && length(badge) < 0.09) {
        bool cross = abs(badge.x) < 0.019 || abs(badge.y) < 0.019;
        COLOR = cross ? vec4(0.98,0.95,0.65,1.0) : vec4(0.25,0.65,0.90,1.0);
    }
    COLOR *= daylight_tint;
}
