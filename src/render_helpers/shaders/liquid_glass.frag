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

// ============ USER CUSTOM SHADER SECTION ============
vec4 custom_postprocess() {
    return texture2D(tex, v_coords);
}
// ============ USER CUSTOM SHADER SECTION ============

void main() {
    vec3 coords_geo = input_to_geo * vec3(v_coords, 1.0);

    // Clip check: coords outside element geometry → transparent.
    if (coords_geo.x < 0.0 || 1.0 < coords_geo.x || coords_geo.y < 0.0 || 1.0 < coords_geo.y) {
        gl_FragColor = vec4(0.0);
        return;
    }

    // User custom shader.
    vec4 color = custom_postprocess();

    // Saturation adjustment (BT.709 luminance weights — same as postprocess.frag).
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

    // Rounding + clipping.
    color *= niri_rounding_alpha(coords_geo.xy * geo_size, geo_size, corner_radius);

    gl_FragColor = color * alpha;

    #if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        gl_FragColor = vec4(0.0, 0.2, 0.0, 0.2) + gl_FragColor * 0.8;
    #endif
}
