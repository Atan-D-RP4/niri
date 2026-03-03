// Reference liquid-glass custom background shader for niri.
//
// This shader recreates the full liquid glass effect using the niri
// custom-shader background-effect system.
//
// Config usage:
//   window-rule {
//     match app-id=".*"
//     background-effect {
//       blur { passes 2; radius 25; }
//       custom-shader "path/to/liquid_glass.frag";
//       animate true;
//     }
//   }
//
// Uniforms available from the template (do not re-declare):
//   uniform sampler2D tex;          // backdrop texture
//   uniform vec2 v_coords;          // texture coordinates
//   uniform mat3 input_to_geo;      // maps tex coords to [0,1] element space
//   uniform vec2 geo_size;          // element size in logical pixels
//   uniform vec2 niri_pointer;      // pointer in window-local px; (-1,-1) = no pointer
//   uniform vec2 niri_window_size;  // window size in logical pixels
//   float gradient_noise(vec2 uv);  // helper: interleaved gradient noise
//
// Quality variants: edit LOW/MEDIUM/HIGH sections below to change quality.

// Baked visual constants (replaces removed lg-* config fields).
const float LG_DISTORTION = 0.04;   // convex lens distortion strength
const float LG_ABERRATION = 2.0;    // chromatic aberration amount (in pixels)
const float LG_HIGHLIGHT  = 0.25;   // specular highlight brightness
const float LG_TINT       = 0.92;   // glass tint (absorption; 1.0 = clear)

// ---------- LOW quality variant (no distortion, no CA) ----------
// Replace the HIGH variant below with this for integrated GPUs.
// vec4 custom_postprocess(vec4 input_color) {
//     return input_color * LG_TINT;
// }

// ---------- MEDIUM quality variant (distortion + 2-sample CA + highlights) ----------
// vec4 custom_postprocess(vec4 input_color) {
//     vec3 coords_geo = input_to_geo * vec3(v_coords, 1.0);
//     vec2 local_uv = coords_geo.xy;
//     vec2 from_center = local_uv - vec2(0.5);
//     float r = length(from_center);
//     vec2 radial_dir = from_center / max(r, 0.001);
//     float r2 = r * r;
//     vec2 distorted = v_coords + from_center * r2 * LG_DISTORTION;
//     if (niri_pointer.x >= 0.0) {
//         vec2 pointer_local = niri_pointer / niri_window_size;
//         vec2 to_pointer = local_uv - pointer_local;
//         float p_dist = length(to_pointer);
//         float p_influence = (1.0 - smoothstep(0.0, 0.25, p_dist)) * LG_DISTORTION * 0.5;
//         distorted += normalize(to_pointer + vec2(0.001)) * p_influence;
//     }
//     vec2 ca = radial_dir * r * (LG_ABERRATION * 0.004);
//     vec4 rg = texture2D(tex, distorted + ca);
//     vec4 gb = texture2D(tex, distorted - ca);
//     vec4 base = texture2D(tex, distorted);
//     vec4 color = vec4(rg.r, (rg.g + gb.g) * 0.5, gb.b, base.a);
//     color.rgb *= LG_TINT;
//     vec2 light = normalize(vec2(-0.5, -1.0));
//     float NdotL = max(dot(radial_dir, light), 0.0);
//     float rim = pow(NdotL, 3.0) * smoothstep(0.05, 0.25, r) * (1.0 - smoothstep(0.3, 0.65, r));
//     float top_glow = exp(-pow(local_uv.y * 5.0, 2.0)) * LG_HIGHLIGHT * 0.35;
//     float p_glow = 0.0;
//     if (niri_pointer.x >= 0.0) {
//         vec2 pointer_local = niri_pointer / niri_window_size;
//         float pointer_dist = length(local_uv - pointer_local);
//         p_glow = (1.0 - smoothstep(0.0, 0.15, pointer_dist)) * LG_HIGHLIGHT * 0.25;
//     }
//     color.rgb += rim * LG_HIGHLIGHT + top_glow + p_glow;
//     return color;
// }

// ---------- HIGH quality variant (full: distortion + 3-sample CA + specular + pointer) ----------
vec4 custom_postprocess(vec4 input_color) {
    // Convert to normalized [0,1] element geometry coordinates.
    vec3 coords_geo = input_to_geo * vec3(v_coords, 1.0);
    vec2 local_uv = coords_geo.xy;

    // Radial values from element center.
    vec2 from_center = local_uv - vec2(0.5);
    float r = length(from_center);
    // Safe radial direction — avoid divide-by-zero at exact center.
    vec2 radial_dir = from_center / max(r, 0.001);

    // ── Convex lens distortion ──────────────────────────────────────
    float r2 = r * r;
    vec2 distorted = v_coords + from_center * r2 * LG_DISTORTION;

    // Pointer-influenced lens wobble (only when pointer is in window).
    if (niri_pointer.x >= 0.0) {
        vec2 pointer_local = niri_pointer / niri_window_size;
        vec2 to_pointer = local_uv - pointer_local;
        float p_dist = length(to_pointer);
        float p_influence = (1.0 - smoothstep(0.0, 0.25, p_dist)) * LG_DISTORTION * 0.5;
        distorted += normalize(to_pointer + vec2(0.001)) * p_influence;
    }

    // ── 3-sample radial chromatic aberration ────────────────────────
    // Split grows smoothly with distance from center.
    vec2 ca = radial_dir * r * (LG_ABERRATION * 0.004);
    float r_ch = texture2D(tex, distorted + ca).r;
    vec4 base  = texture2D(tex, distorted);
    float b_ch = texture2D(tex, distorted - ca).b;
    vec4 color = vec4(r_ch, base.g, b_ch, base.a);

    // ── Glass tint ──────────────────────────────────────────────────
    // Slight absorption: 1.0 = fully clear, 0.0 = fully absorbed.
    color.rgb *= LG_TINT;

    // ── Specular crescent highlight ─────────────────────────────────
    // Convex-lens surface normal dotted with light → smooth crescent.
    vec2 light = normalize(vec2(-0.5, -1.0));
    float NdotL = max(dot(radial_dir, light), 0.0);
    // Fade near center (undefined normal) and near far edge.
    float rim = pow(NdotL, 3.0)
        * smoothstep(0.05, 0.25, r)
        * (1.0 - smoothstep(0.3, 0.65, r));

    // Soft top-edge glow: diffuse light scattering through the glass top.
    float top_glow = exp(-pow(local_uv.y * 5.0, 2.0)) * LG_HIGHLIGHT * 0.35;

    // Pointer proximity glow: gentle brightening near cursor.
    float p_glow = 0.0;
    if (niri_pointer.x >= 0.0) {
        vec2 pointer_local = niri_pointer / niri_window_size;
        float pointer_dist = length(local_uv - pointer_local);
        p_glow = (1.0 - smoothstep(0.0, 0.15, pointer_dist)) * LG_HIGHLIGHT * 0.25;
    }

    color.rgb += rim * LG_HIGHLIGHT + top_glow + p_glow;

    return color;
    // Note: saturation, noise, bg_color blending, and rounding are applied
    // by the template main() after this function returns — do not add them here.
}
