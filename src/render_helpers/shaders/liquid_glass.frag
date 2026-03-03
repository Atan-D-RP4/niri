#version 100

//_DEFINES_

#if defined(EXTERNAL)
#extension GL_OES_EGL_image_external : require
uniform samplerExternalOES tex;
#else
uniform sampler2D tex;
#endif

precision highp float;

uniform float alpha;

// Liquid glass parameters
uniform float lg_tint;
uniform float lg_distortion;
uniform float lg_aberration;
uniform float lg_highlight;
uniform int lg_quality;
uniform vec2 lg_window_size;
// Pointer in window-local logical pixels; (-1,-1) = no pointer (animate disabled).
uniform vec2 lg_pointer;
// Pointer in window-local logical pixels for custom shader; (-1,-1) = no pointer.
uniform vec2 niri_pointer;
// Window size in logical pixels for custom shader.
uniform vec2 niri_window_size;

// Reused from postprocess
uniform float noise;
uniform float saturation;
uniform vec4 bg_color;

// Rounding + clipping (same as clipped_surface.frag)
uniform float niri_scale;
uniform vec2 geo_size;
uniform vec4 corner_radius;
uniform mat3 input_to_geo;

#if defined(DEBUG_FLAGS)
uniform float tint;
#endif

varying vec2 v_coords;

// Forward declaration (implementation included via concat!)
float niri_rounding_alpha(vec2 coords, vec2 size, vec4 corner_radius);

// Interleaved Gradient Noise (same as postprocess.frag)
float gradient_noise(vec2 uv) {
    const vec3 magic = vec3(0.06711056, 0.00583715, 52.9829189);
    return fract(magic.z * fract(dot(uv, magic.xy)));
}

vec4 liquid_glass_effect() {
    vec3 coords_geo = input_to_geo * vec3(v_coords, 1.0);

    // Normalized [0,1] coords over element geometry — use for all position logic.
    vec2 local_uv = coords_geo.xy;

    // Radial values from element center.
    vec2 from_center = local_uv - vec2(0.5);
    float r = length(from_center);
    // Safe radial direction — avoid divide-by-zero at exact center.
    vec2 radial_dir = from_center / max(r, 0.001);

    vec4 color;

    if (lg_quality == 0) {
        // LOW: no distortion, no chromatic aberration.
        color = texture2D(tex, v_coords);
    } else if (lg_quality == 1) {
        // MEDIUM: convex lens distortion + 2-sample radial chromatic aberration.
        float r2 = r * r;
        vec2 distorted = v_coords + from_center * r2 * lg_distortion;

        // Pointer-influenced lens wobble.
        if (lg_pointer.x >= 0.0) {
            vec2 pointer_local = lg_pointer / lg_window_size;
            vec2 to_pointer = local_uv - pointer_local;
            float p_dist = length(to_pointer);
            float p_influence = (1.0 - smoothstep(0.0, 0.25, p_dist)) * lg_distortion * 0.5;
            distorted += normalize(to_pointer + vec2(0.001)) * p_influence;
        }

        // Radial CA: split grows smoothly with distance from center.
        vec2 ca = radial_dir * r * (lg_aberration * 0.004);
        vec4 rg = texture2D(tex, distorted + ca);
        vec4 gb = texture2D(tex, distorted - ca);
        vec4 base = texture2D(tex, distorted);

        color = vec4(rg.r, (rg.g + gb.g) * 0.5, gb.b, base.a);
    } else {
        // HIGH: convex lens distortion + 3-sample radial chromatic aberration.
        float r2 = r * r;
        vec2 distorted = v_coords + from_center * r2 * lg_distortion;

        if (lg_pointer.x >= 0.0) {
            vec2 pointer_local = lg_pointer / lg_window_size;
            vec2 to_pointer = local_uv - pointer_local;
            float p_dist = length(to_pointer);
            float p_influence = (1.0 - smoothstep(0.0, 0.25, p_dist)) * lg_distortion * 0.5;
            distorted += normalize(to_pointer + vec2(0.001)) * p_influence;
        }

        vec2 ca = radial_dir * r * (lg_aberration * 0.004);
        float r_ch = texture2D(tex, distorted + ca).r;
        vec4 base = texture2D(tex, distorted);
        float b_ch = texture2D(tex, distorted - ca).b;
        color = vec4(r_ch, base.g, b_ch, base.a);
    }

    // Glass tint: slight absorption (1.0 = no change, 0.0 = fully absorbed).
    color.rgb *= lg_tint;

    // Specular highlight (medium + high).
    if (lg_quality >= 1) {
        // Convex-lens surface normal: points outward from center.
        // Dotted with light direction → smooth crescent highlight.
        vec2 light = normalize(vec2(-0.5, -1.0));
        float NdotL = max(dot(radial_dir, light), 0.0);
        // Fade near center (undefined normal) and near far edge.
        float rim = pow(NdotL, 3.0)
            * smoothstep(0.05, 0.25, r)
            * (1.0 - smoothstep(0.3, 0.65, r));
        float spec = rim * lg_highlight;

        // Soft top-edge glow: diffuse light scattering through the glass top.
        float top_glow = exp(-pow(local_uv.y * 5.0, 2.0)) * lg_highlight * 0.35;

        // Pointer proximity: gentle brightening near cursor.
        float p_glow = 0.0;
        if (lg_pointer.x >= 0.0) {
            vec2 pointer_local = lg_pointer / lg_window_size;
            float pointer_dist = length(local_uv - pointer_local);
            p_glow = (1.0 - smoothstep(0.0, 0.15, pointer_dist)) * lg_highlight * 0.25;
        }

        color.rgb += spec + top_glow + p_glow;
    }

    // Saturation (BT.709, same as postprocess.frag).
    if (saturation != 1.0) {
        const vec3 w = vec3(0.2126, 0.7152, 0.0722);
        color.rgb = mix(vec3(dot(color.rgb, w)), color.rgb, saturation);
    }

    // Noise dithering.
    if (noise > 0.0) {
        color.rgb += (gradient_noise(gl_FragCoord.xy) - 0.5) * noise;
    }

    // Mix bg_color behind the texture (both premultiplied alpha).
    color = color + bg_color * (1.0 - color.a);

    if (coords_geo.x < 0.0 || 1.0 < coords_geo.x || coords_geo.y < 0.0 || 1.0 < coords_geo.y) {
        return vec4(0.0);
    }

    color *= niri_rounding_alpha(coords_geo.xy * geo_size, geo_size, corner_radius);

    color = color * alpha;

#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) + color * 0.8;
#endif

    return color;
}

// ============ USER CUSTOM SHADER SECTION ============
vec4 custom_postprocess(vec4 input_color) {
    return input_color;
}
// ============ USER CUSTOM SHADER SECTION ============

void main() {
    gl_FragColor = custom_postprocess(liquid_glass_effect());
}
