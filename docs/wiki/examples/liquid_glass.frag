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
//   uniform vec2 v_coords;          // texture coordinates (use texture2D(tex, ...) yourself)
//   uniform mat3 input_to_geo;      // maps tex coords to [0,1] element space
//   uniform vec2 geo_size;          // element size in logical pixels
//   uniform vec2 niri_pointer;      // pointer in window-local px; (-1,-1) = no pointer
//   uniform vec2 niri_window_size;  // window size in logical pixels
//   float gradient_noise(vec2 uv);  // helper: interleaved gradient noise
//
// Quality variants: edit LOW/MEDIUM/HIGH sections below to change quality.

// Baked visual constants (replaces removed lg-* config fields).
const float LG_DISTORTION = 0.020;                 // convex lens distortion strength
const float LG_ABERRATION = 1.2;                   // chromatic aberration amount
const float LG_HIGHLIGHT  = 0.26;                  // specular highlight brightness
const vec3  LG_TINT       = vec3(0.93, 0.96, 0.98); // cool glass absorption tint
const vec2  LG_UV_EPS     = vec2(0.0015);          // keeps samples safely in-bounds

vec2 lg_safe_uv(vec2 uv) {
    return clamp(uv, LG_UV_EPS, vec2(1.0) - LG_UV_EPS);
}

vec4 lg_sample(vec2 uv) {
    return texture2D(tex, lg_safe_uv(uv));
}

// ---------- LOW quality variant (no distortion, no CA) ----------
// Replace the HIGH variant below with this for integrated GPUs.
// vec4 custom_postprocess() {
//     vec4 color = lg_sample(v_coords);
//     float luma = dot(color.rgb, vec3(0.2126, 0.7152, 0.0722));
//     color.rgb = mix(vec3(luma), color.rgb, 0.95);
//     color.rgb *= LG_TINT;
//     return color;
// }

// ---------- MEDIUM quality variant (distortion + 2-sample CA + highlights) ----------
// vec4 custom_postprocess() {
//     vec3 coords_geo = input_to_geo * vec3(v_coords, 1.0);
//     vec2 local_uv = coords_geo.xy;
//     vec2 from_center = local_uv - vec2(0.5);
//     float r = length(from_center);
//     vec2 radial_dir = from_center / max(r, 0.001);
//     float r2 = r * r;
//     vec2 dist_vec = from_center * r2 * LG_DISTORTION;
//     vec2 pointer_vec = vec2(0.0);
//     if (niri_pointer.x >= 0.0) {
//         vec2 pointer_local = niri_pointer / max(niri_window_size, vec2(1.0));
//         vec2 to_pointer = local_uv - pointer_local;
//         float p_dist = length(to_pointer);
//         float p_influence = (1.0 - smoothstep(0.0, 0.25, p_dist)) * 0.007;
//         pointer_vec = normalize(to_pointer + vec2(0.001)) * p_influence;
//     }
//     vec2 ca = radial_dir * r * (LG_ABERRATION * 0.0025);
//     vec2 shift = dist_vec + pointer_vec;
//     vec2 excursion = abs(shift) + abs(ca);
//     vec2 budget = max(min(v_coords, vec2(1.0) - v_coords) - LG_UV_EPS, vec2(0.0));
//     float uv_scale = min(1.0, min(budget.x / max(excursion.x, 1e-5), budget.y / max(excursion.y, 1e-5)));
//     shift *= uv_scale;
//     ca *= uv_scale;
//     vec2 distorted = v_coords + shift;
//     vec4 rg = lg_sample(distorted + ca);
//     vec4 gb = lg_sample(distorted - ca);
//     vec4 base = lg_sample(distorted);
//     vec4 color = vec4(rg.r, (rg.g + gb.g) * 0.5, gb.b, base.a);
//     color.rgb *= LG_TINT;
//     vec2 light = normalize(vec2(-0.5, -1.0));
//     float NdotL = max(dot(radial_dir, light), 0.0);
//     float rim = pow(NdotL, 3.0) * smoothstep(0.05, 0.25, r) * (1.0 - smoothstep(0.3, 0.65, r));
//     float top_glow = exp(-pow(local_uv.y * 5.0, 2.0)) * LG_HIGHLIGHT * 0.35;
//     float p_glow = 0.0;
//     if (niri_pointer.x >= 0.0) {
//         vec2 pointer_local = niri_pointer / max(niri_window_size, vec2(1.0));
//         float pointer_dist = length(local_uv - pointer_local);
//         p_glow = (1.0 - smoothstep(0.0, 0.15, pointer_dist)) * LG_HIGHLIGHT * 0.25;
//     }
//     color.rgb += (rim * LG_HIGHLIGHT + top_glow + p_glow) * color.a;
//     return color;
// }

// ---------- HIGH quality variant (full: distortion + 3-sample CA + specular + pointer) ----------
vec4 custom_postprocess() {
    // Convert to normalized [0,1] element geometry coordinates.
    vec3 coords_geo = input_to_geo * vec3(v_coords, 1.0);
    vec2 local_uv = coords_geo.xy;

    // Radial values from element center.
    vec2 from_center = local_uv - vec2(0.5);
    float r = length(from_center);
    // Safe radial direction — avoid divide-by-zero at exact center.
    vec2 radial_dir = from_center / max(r, 0.001);

    // Soft convex refraction. Keep strength low for polished, glassy depth.
    float r2 = r * r;
    float warp = LG_DISTORTION * (0.25 + 0.75 * smoothstep(0.0, 0.65, r));
    vec2 dist_vec = from_center * r2 * warp;
    vec2 pointer_vec = vec2(0.0);

    // Cursor interaction: subtle local lens wobble.
    if (niri_pointer.x >= 0.0) {
        vec2 pointer_local = niri_pointer / max(niri_window_size, vec2(1.0));
        vec2 to_pointer = local_uv - pointer_local;
        float p_dist = length(to_pointer);
        float p_influence = (1.0 - smoothstep(0.0, 0.28, p_dist)) * DISTORTION * 0.008;
        pointer_vec = normalize(to_pointer + vec2(0.001)) * p_influence;
    }

    // Radial chromatic split with bounded sampling.
    vec2 ca = radial_dir * r * (LG_ABERRATION * 0.0025);
    vec2 shift = dist_vec + pointer_vec;
    vec2 excursion = abs(shift) + abs(ca);
    vec2 budget = max(min(v_coords, vec2(1.0) - v_coords) - LG_UV_EPS, vec2(0.0));
    float uv_scale = min(1.0, min(budget.x / max(excursion.x, 1e-5), budget.y / max(excursion.y, 1e-5)));
    shift *= uv_scale;
    ca *= uv_scale;
    vec2 distorted = v_coords + shift;
    float r_ch = lg_sample(distorted + ca).r;
    vec4 base = lg_sample(distorted);
    float b_ch = lg_sample(distorted - ca).b;
    vec4 color = vec4(r_ch, base.g, b_ch, base.a);

    // Frosted appearance: slight desaturation + cool tint.
    // Note: the desaturation is technically optional, since the chromatic
    // aberration already reduces saturation — but it helps unify the look and
    // smooth out some remaining color noise from the CA sampling.
    float luma = dot(color.rgb, vec3(0.2126, 0.7152, 0.0722));
    color.rgb = mix(vec3(luma), color.rgb, 0.94);
    color.rgb *= LG_TINT;
    color.rgb += 0.02 * color.a;

    // Apple-like glazing: fresnel edge lift + narrow specular + top sheen.
    vec3 n = normalize(vec3(from_center * 2.0, 1.25));
    vec3 v = vec3(0.0, 0.0, 1.0);
    vec3 l = normalize(vec3(-0.45, -0.85, 0.35));
    float fresnel = pow(1.0 - max(dot(n, v), 0.0), 3.5);
    float specular = pow(max(dot(reflect(-l, n), v), 0.0), 22.0);
    float top_sheen = exp(-pow((local_uv.y - 0.05) * 7.0, 2.0)) * 0.20;
    float edge_lift = fresnel * 0.14;

    float highlight = (specular * 0.45 + edge_lift + top_sheen) * LG_HIGHLIGHT;

    // Pointer proximity glow: subtle halo around the cursor, using the same
    // fresnel highlight for consistency.
    float p_glow = 0.0;
    if (niri_pointer.x >= 0.0) {
        vec2 pointer_local = niri_pointer / max(niri_window_size, vec2(1.0));
        float pointer_dist = length(local_uv - pointer_local);
        p_glow = (1.0 - smoothstep(0.0, 0.16, pointer_dist)) * LG_HIGHLIGHT * 0.23;
    }

    color.rgb += (highlight + p_glow) * color.a;

    return color;
    // Note: saturation, noise, bg_color blending, and rounding are applied
    // by the template main() after this function returns — do not add them here.
}
