## Background Shaders

<sup>Since: next release</sup>

You can write a custom GLSL shader to post-process the background effect on a per-window basis.
The shader runs after all background processing is complete and samples the composited backdrop texture directly.

If a custom shader fails to compile, niri will print a warning and fall back to no post-processing (equivalent to the passthrough shader).
When running niri as a systemd service, you can see the warnings in the journal: `journalctl -ef /usr/bin/niri`

> [!WARNING]
>
> Custom shaders do not have a backwards compatibility guarantee.
> I may need to change their interface as I'm developing new features.

### Shader Contract

Your shader must define exactly one function:

```glsl
vec4 custom_postprocess() {
    return texture2D(tex, v_coords);
}
```

The function takes no arguments. Sample the backdrop texture yourself using `texture2D(tex, v_coords)` (or any modified coordinates). Return a color in sRGB with premultiplied alpha.

Do not declare any `uniform` variables or `varying` inputs yourself. Niri provides them all.
All niri-defined symbols use a `niri_` prefix, so avoid that prefix for your own names.
The shader is compiled as GLSL ES 1.0 (`#version 100`). ES 3.0 features are not available.

#### Available Uniforms

**Geometry and scale**

| Uniform | Type | Description |
|---------|------|-------------|
| `niri_scale` | `float` | HiDPI scale factor (physical pixels per logical pixel) |
| `geo_size` | `vec2` | Window geometry size in logical pixels |
| `corner_radius` | `vec4` | Corner radii in logical pixels: (top-left, top-right, bottom-right, bottom-left) |
| `input_to_geo` | `mat3` | Homogeneous transform from texture coords to geometry coords (0..1 range) |

**Pointer and animation**

| Uniform | Type | Description |
|---------|------|-------------|
| `niri_pointer` | `vec2` | Pointer position in window-local logical pixels; `(-1, -1)` when pointer is absent or `animate` is not set |
| `niri_window_size` | `vec2` | Window size in logical pixels |

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

Use the `custom-shader` property inside a `background-effect` block, with inline GLSL as a KDL raw string (`r"..."`), or a path to a `.frag` file.

Set `animate true` to make niri continuously track the pointer for this window and pass its position as `niri_pointer`. Without it, `niri_pointer` is always `(-1, -1)`. Default: `false`.

```kdl
window-rule {
    match app-id="^foot$"

    background-effect {
        blur true
        animate true
        custom-shader r"
            vec4 custom_postprocess() {
                return texture2D(tex, v_coords);
            }
        "
    }
}
```

### Examples

#### Passthrough (no-op)

Returns the background output unchanged. This is the default behavior when no `custom-shader` is set.

```kdl
window-rule {
    match app-id="^foot$"

    background-effect {
        blur true
        custom-shader r"
            vec4 custom_postprocess() {
                return texture2D(tex, v_coords);
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
        custom-shader r"
            vec4 custom_postprocess() {
                vec4 color = texture2D(tex, v_coords);
                return vec4(1.0 - color.rgb, color.a);
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
        custom-shader r"
            vec4 custom_postprocess() {
                vec4 color = texture2D(tex, v_coords);
                vec3 coords_geo = input_to_geo * vec3(v_coords, 1.0);
                vec2 uv = coords_geo.xy;

                // Distance from center in [0, 1] space.
                vec2 from_center = uv - vec2(0.5);
                float dist = length(from_center * 2.0);

                // Smooth falloff: bright center, dark edges.
                float vignette = 1.0 - smoothstep(0.5, 1.4, dist);

                return vec4(color.rgb * vignette, color.a);
            }
        "
    }
}
```

#### Pointer Highlight

Brightens the area around the cursor when `animate true` is set. Falls back gracefully when the pointer is absent.

```kdl
window-rule {
    match app-id="^foot$"

    background-effect {
        blur true
        animate true
        custom-shader r"
            vec4 custom_postprocess() {
                vec4 color = texture2D(tex, v_coords);
                if (niri_pointer.x < 0.0) {
                    return color;
                }

                vec3 coords_geo = input_to_geo * vec3(v_coords, 1.0);
                vec2 uv = coords_geo.xy;
                vec2 pointer_local = niri_pointer / niri_window_size;
                float dist = length(uv - pointer_local);

                float glow = (1.0 - smoothstep(0.0, 0.2, dist)) * 0.15;

                // Premultiplied alpha: add to RGB only, not alpha.
                return vec4(color.rgb + glow * color.a, color.a);
            }
        "
    }
}
```

### Liquid Glass Reference Shader

niri ships a ready-to-use liquid glass shader at `docs/wiki/examples/liquid_glass.frag` in the repository.
It implements convex-lens distortion, chromatic aberration, a specular crescent highlight, and a pointer proximity glow.

Copy the file to a location of your choice, then point your config at it:

```kdl
window-rule {
    match app-id=".*"

    background-effect {
        blur true
        custom-shader "/path/to/liquid_glass.frag"
        animate true
    }
}
```

The global blur settings (passes, radius) are configured separately in the `blur` section of your niri config, not per-window.

Do not set `xray true` with custom background shaders. Custom shaders are rendered through the framebuffer path (`xray false`) so the shader coordinate space matches element geometry.

After copying, open `liquid_glass.frag` and edit the `LG_*` constants near the top to tune the look:

- `LG_DISTORTION` — convex lens warp strength (default `0.04`)
- `LG_ABERRATION` — chromatic aberration spread in pixels (default `2.0`)
- `LG_HIGHLIGHT` — specular highlight brightness (default `0.25`)
- `LG_TINT` — glass tint / absorption (`1.0` = fully clear, default `0.92`)

#### Performance Tuning

The shader ships with three quality variants. Only one should be active at a time (the others are commented out).

**HIGH** (default) — full 3-sample chromatic aberration, specular crescent, and pointer proximity glow. Best on dedicated GPUs.

**MEDIUM** — 2-sample CA and highlights, no separate pointer glow pass. Good middle ground.

**LOW** — samples `texture2D(tex, v_coords) * LG_TINT` only, no distortion or aberration. Suitable for integrated GPUs.

To switch variants, comment out the HIGH function body and uncomment the desired variant. The file has clear `// ---------- LOW/MEDIUM/HIGH ----------` markers.

### Migration Guide (from `liquid-glass` config)

The `liquid-glass` config field and all associated parameters have been removed. Replace them with the custom shader approach:

```kdl
window-rule {
    match app-id=".*"

    background-effect {
        blur true
        custom-shader "/path/to/liquid_glass.frag"
        animate true
    }
}
```

Field mapping:

| Removed config field | Replacement |
|---------------------|-------------|
| `lg-tint 0.92` | `const float LG_TINT = 0.92;` in shader file |
| `lg-distortion 0.04` | `const float LG_DISTORTION = 0.04;` in shader file |
| `lg-aberration 2.0` | `const float LG_ABERRATION = 2.0;` in shader file |
| `lg-highlight 0.25` | `const float LG_HIGHLIGHT = 0.25;` in shader file |
| `lg-quality` | Edit LOW/MEDIUM/HIGH variant in shader file (see Performance Tuning) |
| `lg-animate true` | `animate true` in `background-effect {}` block |

### Troubleshooting

**Shader fails to compile**

niri logs a warning and silently falls back to passthrough (no post-processing). The background effect itself still renders normally.

Check the journal for the error message and the relevant GLSL line number:

```
journalctl -ef /usr/bin/niri
```

Common causes:
- Missing `vec4 custom_postprocess()` function definition (must use this exact signature with no arguments).
- Using GLSL ES 3.0 features (`texture()`, `in`/`out` qualifiers, etc.) — use `texture2D()` and GLSL ES 1.0 syntax instead.
- Declaring a `uniform` that niri already provides — remove it from your shader.

**Effect not visible**

- The window must be semitransparent; an opaque window covers the background entirely.

**Pointer uniforms always `(-1, -1)`**

Add `animate true` to the `background-effect` block. Without it, niri does not track the pointer for this window and `niri_pointer` stays at `(-1, -1)`.

**Color looks wrong or washed out**

All colors are sRGB with premultiplied alpha. When modifying color channels, keep RGB and alpha consistent: if you reduce alpha, also reduce the RGB channels by the same factor. Adding a value to RGB without multiplying by alpha will appear to brighten transparent areas incorrectly.
