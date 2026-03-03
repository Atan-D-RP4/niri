## Background Shaders

<sup>Since: next release</sup>

You can write a custom GLSL shader to post-process the liquid glass background effect on a per-window basis.
The shader runs after all liquid glass processing is complete and receives the final composited color as input.

This feature requires `liquid-glass true` in the same `background-effect` block.

If a custom shader fails to compile, niri will print a warning and fall back to no post-processing (equivalent to the passthrough shader).
When running niri as a systemd service, you can see the warnings in the journal: `journalctl -ef /usr/bin/niri`

> [!WARNING]
>
> Custom shaders do not have a backwards compatibility guarantee.
> I may need to change their interface as I'm developing new features.

### Shader Contract

Your shader must define exactly one function:

```glsl
vec4 custom_postprocess(vec4 input_color) {
    return input_color;
}
```

The function receives the fully-processed liquid glass color (sRGB, premultiplied alpha) and must return a color in the same format.

Do not declare any `uniform` variables or `varying` inputs yourself. Niri provides them all.
All niri-defined symbols use a `niri_` or `lg_` prefix, so avoid those for your own names.

The shader is compiled as GLSL ES 1.0 (`#version 100`). ES 3.0 features are not available.

#### Available Uniforms

**Geometry and scale**

| Uniform | Type | Description |
|---------|------|-------------|
| `niri_scale` | `float` | HiDPI scale factor (physical pixels per logical pixel) |
| `geo_size` | `vec2` | Window geometry size in logical pixels |
| `corner_radius` | `vec4` | Corner radii in logical pixels: (top-left, top-right, bottom-right, bottom-left) |
| `input_to_geo` | `mat3` | Homogeneous transform from texture coords to geometry coords (0..1 range) |

**Liquid glass parameters** (reflect the values set in the window rule)

| Uniform | Type | Description |
|---------|------|-------------|
| `lg_tint` | `float` | Glass tint opacity (`0.92` default) |
| `lg_distortion` | `float` | Lens distortion strength (`0.04` default) |
| `lg_aberration` | `float` | Chromatic aberration spread in pixels (`2.0` default) |
| `lg_highlight` | `float` | Specular rim highlight intensity (`0.25` default) |
| `lg_quality` | `int` | Quality level: `0` = low, `1` = medium, `2` = high |
| `lg_window_size` | `vec2` | Window size in physical pixels |
| `lg_pointer` | `vec2` | Pointer position in window-local logical pixels; `(-1, -1)` when pointer is absent or animation is disabled |

**Background effect parameters**

| Uniform | Type | Description |
|---------|------|-------------|
| `noise` | `float` | Noise dither amount (from `noise` window rule) |
| `saturation` | `float` | Color saturation (from `saturation` window rule) |
| `bg_color` | `vec4` | Background color hint (premultiplied sRGB) |

**Fragment input**

| Varying | Type | Description |
|---------|------|-------------|
| `v_coords` | `vec2` | Texture coordinates of the current fragment |

#### Available Helper Functions

```glsl
// Returns the rounding alpha (0.0 outside corners, 1.0 inside) for a pixel
// at `coords` (in logical pixels) within a box of `size` with `corner_radius`.
float niri_rounding_alpha(vec2 coords, vec2 size, vec4 corner_radius);

// Interleaved gradient noise — useful for dithering.
float gradient_noise(vec2 uv);
```

### Config Syntax

Use the `custom-shader` property inside a `background-effect` block, with inline GLSL as a KDL raw string (`r"..."`).
`liquid-glass true` must also be set in the same block.

```kdl
window-rule {
    match app-id="^foot$"

    background-effect {
        blur true
        liquid-glass true
        custom-shader r"
            vec4 custom_postprocess(vec4 input_color) {
                return input_color;
            }
        "
    }
}
```

### Examples

#### Passthrough (no-op)

Returns the liquid glass output unchanged. This is the default behavior when no `custom-shader` is set.

```kdl
window-rule {
    match app-id="^foot$"

    background-effect {
        blur true
        liquid-glass true
        custom-shader r"
            vec4 custom_postprocess(vec4 input_color) {
                return input_color;
            }
        "
    }
}
```

#### Color Inversion

Inverts the RGB channels of the background while preserving alpha. Works correctly with premultiplied alpha because we invert only the color channels.

```kdl
window-rule {
    match app-id="^foot$"

    background-effect {
        blur true
        liquid-glass true
        custom-shader r"
            vec4 custom_postprocess(vec4 input_color) {
                return vec4(1.0 - input_color.rgb, input_color.a);
            }
        "
    }
}
```

#### Vignette

Darkens the edges of the background, drawing attention to the window center. Uses geometry coordinates so the effect tracks the window shape correctly.

```kdl
window-rule {
    match app-id="^foot$"

    background-effect {
        blur true
        liquid-glass true
        custom-shader r"
            vec4 custom_postprocess(vec4 input_color) {
                vec3 coords_geo = input_to_geo * vec3(v_coords, 1.0);
                vec2 uv = coords_geo.xy;

                // Distance from center in [0, 1] space.
                vec2 from_center = uv - vec2(0.5);
                float dist = length(from_center * 2.0);

                // Smooth falloff: bright center, dark edges.
                float vignette = 1.0 - smoothstep(0.5, 1.4, dist);

                return vec4(input_color.rgb * vignette, input_color.a);
            }
        "
    }
}
```

#### Pointer Highlight

Brightens the area around the cursor when `lg-animate true` is set. Falls back gracefully when the pointer is absent.

```kdl
window-rule {
    match app-id="^foot$"

    background-effect {
        blur true
        liquid-glass true
        lg-animate true
        custom-shader r"
            vec4 custom_postprocess(vec4 input_color) {
                if (lg_pointer.x < 0.0) {
                    return input_color;
                }

                vec3 coords_geo = input_to_geo * vec3(v_coords, 1.0);
                vec2 uv = coords_geo.xy;
                vec2 pointer_local = lg_pointer / lg_window_size;
                float dist = length(uv - pointer_local);

                float glow = (1.0 - smoothstep(0.0, 0.2, dist)) * 0.15;

                // Premultiplied alpha: add to RGB only, not alpha.
                return vec4(input_color.rgb + glow * input_color.a, input_color.a);
            }
        "
    }
}
```

### Troubleshooting

**Shader fails to compile**

niri logs a warning and silently falls back to passthrough (no post-processing). The liquid glass effect itself still renders normally.

Check the journal for the error message and the relevant GLSL line number:

```
journalctl -ef /usr/bin/niri
```

Common causes:
- Missing `vec4 custom_postprocess(vec4 input_color)` function signature (must match exactly).
- Using GLSL ES 3.0 features (`texture()`, `in`/`out` qualifiers, etc.) — use `texture2D()` and GLSL ES 1.0 syntax instead.
- Declaring a `uniform` that niri already provides — remove it from your shader.

**Effect not visible**

- Make sure `liquid-glass true` is set in the same `background-effect` block. The `custom-shader` property has no effect without it.
- The window must be semitransparent; an opaque window covers the background entirely.

**Pointer uniforms always `(-1, -1)`**

Add `lg-animate true` to the `background-effect` block. Without it, niri does not track the pointer for this window and `lg_pointer` stays at `(-1, -1)`.

**Color looks wrong or washed out**

All colors are sRGB with premultiplied alpha. When modifying color channels, keep RGB and alpha consistent: if you reduce alpha, also reduce the RGB channels by the same factor. Adding a value to RGB without multiplying by alpha will appear to brighten transparent areas incorrectly.
