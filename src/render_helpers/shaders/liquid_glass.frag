#version 100

precision highp float;

uniform sampler2D tex;
uniform float niri_alpha;
uniform vec4 geo_to_tex;

// Liquid glass parameters
uniform float lg_tint;
uniform float lg_distortion;
uniform float lg_aberration;
uniform float lg_highlight;
uniform int lg_quality;
uniform vec2 lg_window_size;
uniform vec2 lg_local_origin;

// Reused from postprocess
uniform float noise;
uniform float saturation;

// Rounding + clipping (same as clipped_surface.frag)
uniform float niri_scale;
uniform vec2 geo_size;
uniform vec4 corner_radius;
uniform mat3 input_to_geo;

varying vec2 v_coords;

// Forward declaration (implementation included via concat!)
float niri_rounding_alpha(vec2 coords, vec2 size, vec4 corner_radius);

// Interleaved Gradient Noise (same as postprocess.frag)
float gradient_noise(vec2 uv) {
    const vec3 magic = vec3(0.06711056, 0.00583715, 52.9829189);
    return fract(magic.z * fract(dot(uv, magic.xy)));
}

// Signed distance from fragment to rounded-rect window edge.
// Negative inside, positive outside.
float window_sdf(vec2 local_uv) {
    vec2 pos = local_uv * lg_window_size;
    vec2 half_size = lg_window_size * 0.5;
    vec2 d = abs(pos - half_size) - half_size;
    return length(max(d, 0.0)) + min(max(d.x, d.y), 0.0);
}

// Normal direction at fragment (toward nearest edge).
vec2 sdf_gradient(vec2 local_uv) {
    float eps = 1.0 / max(lg_window_size.x, lg_window_size.y);
    float dx = window_sdf(local_uv + vec2(eps, 0.0)) - window_sdf(local_uv - vec2(eps, 0.0));
    float dy = window_sdf(local_uv + vec2(0.0, eps)) - window_sdf(local_uv - vec2(0.0, eps));
    return normalize(vec2(dx, dy));
}

// Quadratic bulge from center — simulates convex glass.
vec2 distort_uv(vec2 uv, float strength) {
    vec2 center = vec2(0.5);
    vec2 offset = uv - center;
    float r2 = dot(offset, offset);
    return uv + offset * r2 * strength;
}

void main() {
    vec3 coords_geo = input_to_geo * vec3(v_coords, 1.0);
    vec2 local_uv = v_coords - lg_local_origin / lg_window_size;
    vec2 base_tex = v_coords * geo_to_tex.xy + geo_to_tex.zw;

    vec4 color;

    float sdf = window_sdf(local_uv);
    // Edge factor: 1 at edges, 0 at center
    float edge_factor = clamp(-sdf / max(lg_window_size.x, lg_window_size.y) * 4.0, 0.0, 1.0);
    edge_factor = 1.0 - edge_factor;

    if (lg_quality == 0) {
        // LOW: No distortion, no chromatic aberration
        color = texture2D(tex, base_tex);
    } else if (lg_quality == 1) {
        // MEDIUM: Distortion + 2-sample chromatic aberration (swizzle trick)
        vec2 distorted_uv = distort_uv(v_coords, lg_distortion);
        vec2 distorted_tex = distorted_uv * geo_to_tex.xy + geo_to_tex.zw;

        float ca_offset = lg_aberration / max(lg_window_size.x, lg_window_size.y) * edge_factor;
        vec2 ca_dir = sdf_gradient(local_uv) * ca_offset;

        vec4 sample_rg = texture2D(tex, distorted_tex + ca_dir);
        vec4 sample_gb = texture2D(tex, distorted_tex - ca_dir);

        color = vec4(sample_rg.r, (sample_rg.g + sample_gb.g) * 0.5, sample_gb.b, 1.0);
    } else {
        // HIGH: Full distortion + 3-sample chromatic aberration
        vec2 distorted_uv = distort_uv(v_coords, lg_distortion);
        vec2 distorted_tex = distorted_uv * geo_to_tex.xy + geo_to_tex.zw;

        float ca_offset = lg_aberration / max(lg_window_size.x, lg_window_size.y) * edge_factor;
        vec2 ca_dir = sdf_gradient(local_uv) * ca_offset;

        float r = texture2D(tex, distorted_tex + ca_dir).r;
        float g = texture2D(tex, distorted_tex).g;
        float b = texture2D(tex, distorted_tex - ca_dir).b;
        color = vec4(r, g, b, 1.0);
    }

    // Glass tint (all LOD levels)
    color.rgb *= lg_tint;

    // Specular rim (medium + high)
    if (lg_quality >= 1) {
        vec2 grad = sdf_gradient(local_uv);
        vec2 light_dir = normalize(vec2(-0.5, -0.7));
        float spec = max(dot(grad, light_dir), 0.0);
        spec = pow(spec, 4.0) * lg_highlight * edge_factor;
        color.rgb += spec;
    }

    // Saturation (BT.709, same as postprocess.frag)
    if (saturation != 1.0) {
        const vec3 w = vec3(0.2126, 0.7152, 0.0722);
        color.rgb = mix(vec3(dot(color.rgb, w)), color.rgb, saturation);
    }

    // Noise dithering
    if (noise > 0.0) {
        color.rgb += (gradient_noise(gl_FragCoord.xy) - 0.5) * noise;
    }

    // Clip outside geometry (same as clipped_surface.frag)
    if (coords_geo.x < 0.0 || 1.0 < coords_geo.x || coords_geo.y < 0.0 || 1.0 < coords_geo.y) {
        gl_FragColor = vec4(0.0);
        return;
    }

    // Corner rounding
    color *= niri_rounding_alpha(coords_geo.xy * geo_size, geo_size, corner_radius);

    // Final alpha
    color *= niri_alpha;

    gl_FragColor = color;
}
