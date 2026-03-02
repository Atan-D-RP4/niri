// Your shader must contain one function (see the bottom of this file).
//
// It should not contain any uniform definitions, varying declarations, or
// anything else that niri already provides. All niri-defined symbols use a
// niri_ prefix, so avoid that prefix for your own names.
//
// The shader is compiled as GLSL ES 1.0 (#version 100). ES 3.0 features such
// as texture(), in/out qualifiers, or layout are not available.

// The function you must define looks like this:
vec4 custom_postprocess() {
    vec4 color = /* ...compute the color... */;
    return color;
}

// It takes no arguments.
//
// Sample the composited backdrop texture yourself using texture2D(tex, v_coords)
// or any modified coordinates.
//
// The function must return the color of the pixel in sRGB with premultiplied
// alpha. Keep RGB and alpha consistent: if you reduce alpha, reduce RGB by the
// same factor. Adding values to RGB without multiplying by alpha brightens
// transparent areas incorrectly.
//
// After your function returns, niri applies saturation, noise dithering, the
// background color blend, and corner rounding on top. Do not apply these
// yourself.

// Now let's go over the uniforms and varyings that niri provides.
//
// You should only rely on the symbols documented here. Any others can change
// or be removed without notice.

// The composited backdrop texture (blurred background, etc.).
// Sample it with: texture2D(tex, v_coords)
uniform sampler2D tex;

// Texture coordinates of the current fragment.
//
// Goes from (0.0, 0.0) at the top-left of the rendered area to (1.0, 1.0) at
// the bottom-right. The rendered area is the full window including any padding
// niri adds for the effect, which may be slightly larger than the window itself.
varying vec2 v_coords;

// Homogeneous matrix that converts texture coordinates (v_coords) into
// normalized geometry coordinates.
//
// After the transform, (0.0, 0.0) is the top-left of the window geometry and
// (1.0, 1.0) is the bottom-right. Use it to get window-relative UVs:
//
//   vec3 coords_geo = input_to_geo * vec3(v_coords, 1.0);
//   vec2 uv = coords_geo.xy;   // (0,0) top-left, (1,1) bottom-right
uniform mat3 input_to_geo;

// Window geometry size in logical pixels.
uniform vec2 geo_size;

// HiDPI scale factor (physical pixels per logical pixel).
uniform float niri_scale;

// Corner radii in logical pixels: (top-left, top-right, bottom-right, bottom-left).
uniform vec4 corner_radius;

// Pointer position in window-local logical pixels.
//
// The origin (0, 0) is the top-left of the window. Set to (-1, -1) when:
//   - The pointer is not over this window, or
//   - animate is not set in the background-effect block.
//
// Always check niri_pointer.x >= 0.0 before using this uniform.
// To enable pointer tracking, add `animate true` to your background-effect block.
uniform vec2 niri_pointer;

// Window size in logical pixels.
//
// Divide niri_pointer by this to get a normalized [0, 1] pointer position:
//   vec2 pointer_local = niri_pointer / niri_window_size;
uniform vec2 niri_window_size;

// Noise dither amount from the `noise` window rule (0.0 to 1.0).
// Applied by niri after your function returns; provided here if you want to
// use it for your own dithering effects.
uniform float noise;

// Saturation value from the `saturation` window rule.
// Applied by niri after your function returns; provided here for reference.
uniform float saturation;

// Background color hint (premultiplied sRGB, from the `bg-color` window rule).
// Applied by niri after your function returns; provided here for reference.
uniform vec4 bg_color;

// Helper functions provided by niri:

// Interleaved gradient noise. Returns a float in [0, 1). Useful for dithering.
float gradient_noise(vec2 uv);

// Returns the anti-aliased rounding alpha for a pixel at `coords` (logical
// pixels from the top-left of the window) inside a box of `size` (logical
// pixels) with the given `corner_radius`.
//   1.0 = inside the rounded area
//   0.0 = clipped by a corner
float niri_rounding_alpha(vec2 coords, vec2 size, vec4 corner_radius);

// Now let's look at some examples. You can copy everything below this line
// into your custom-shader to experiment.

// Example: passthrough — return the backdrop unchanged.
// This is the default behavior when no custom-shader is set.
vec4 passthrough() {
    return texture2D(tex, v_coords);
}

// Example: color inversion.
// Inverts RGB channels while preserving premultiplied alpha.
vec4 invert() {
    vec4 color = texture2D(tex, v_coords);
    // Unpremultiply → invert → premultiply.
    // For fully opaque pixels (a == 1) this simplifies to (1 - r, 1 - g, 1 - b, 1).
    if (color.a > 0.0) {
        vec3 straight = color.rgb / color.a;
        straight = 1.0 - straight;
        color.rgb = straight * color.a;
    }
    return color;
}

// Example: vignette.
// Darkens the edges of the backdrop, drawing focus toward the window center.
// Uses geometry coordinates so the shape tracks the window correctly.
vec4 vignette() {
    vec4 color = texture2D(tex, v_coords);

    // Convert to geometry-relative UVs: (0,0) = top-left, (1,1) = bottom-right.
    vec3 coords_geo = input_to_geo * vec3(v_coords, 1.0);
    vec2 uv = coords_geo.xy;

    // Distance from center.
    vec2 from_center = uv - vec2(0.5);
    float dist = length(from_center * 2.0);

    // Bright center, dark edges.
    float factor = 1.0 - smoothstep(0.5, 1.4, dist);

    return vec4(color.rgb * factor, color.a);
}

// Example: pointer highlight (requires animate true).
// Brightens the area around the cursor. Gracefully a no-op when pointer is absent.
vec4 pointer_highlight() {
    vec4 color = texture2D(tex, v_coords);

    // Guard: no pointer (or animate not set).
    if (niri_pointer.x < 0.0) {
        return color;
    }

    vec3 coords_geo = input_to_geo * vec3(v_coords, 1.0);
    vec2 uv = coords_geo.xy;

    // Normalize pointer to [0, 1] window space.
    vec2 pointer_local = niri_pointer / max(niri_window_size, vec2(1.0));
    float dist = length(uv - pointer_local);

    // Soft radial glow up to 0.2 units from the pointer.
    float glow = (1.0 - smoothstep(0.0, 0.2, dist)) * 0.15;

    // Premultiplied alpha: add to RGB only, scaled by alpha to avoid
    // brightening transparent pixels.
    return vec4(color.rgb + glow * color.a, color.a);
}

// For a full-featured example showing convex-lens distortion, chromatic
// aberration, a specular crescent highlight, and pointer proximity glow, see
// the liquid_glass.frag reference shader in this directory.

// This is the function you must define.
vec4 custom_postprocess() {
    // Pick one of the examples above or write your own.
    return passthrough();
}
