#version 100

precision highp float;

uniform sampler2D tex;
uniform float lg_tint;
uniform int lg_quality;
uniform vec2 lg_window_size;
uniform vec2 lg_local_origin;

uniform float niri_alpha;
uniform vec4 geo_to_tex;

varying vec2 v_coords;

// Forward declaration for rounding alpha (implementation included via mod.rs)
float niri_rounding_alpha(vec2 coords, vec2 size, vec4 corner_radius);

void main() {
    vec2 tex_coords = v_coords * geo_to_tex.xy + geo_to_tex.zw;

    // LOD-based rendering quality branching
    vec4 color;
    if (lg_quality == 0) {
        // LOW: no distortion, no chromatic aberration — placeholder
        color = texture2D(tex, tex_coords);
    } else if (lg_quality == 1) {
        // MEDIUM: basic distortion, 2-sample CA — placeholder
        color = texture2D(tex, tex_coords);
    } else {
        // HIGH: full distortion, 3-sample CA, specular — placeholder
        color = texture2D(tex, tex_coords);
    }

    // Glass tint: darken to simulate glass absorption
    color.rgb *= lg_tint;

    // Premultiply alpha
    color *= niri_alpha;

    gl_FragColor = color;
}
