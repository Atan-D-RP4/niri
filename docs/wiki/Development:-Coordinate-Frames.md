# Coordinate frames

This page documents the compositor-wide coordinate-frame convention: the `Global` / `Local`
markers, the conversion traits, and the immutable output-view context used at every
geometry boundary. It is contributor documentation — normal niri configuration never
mentions any of these types.

Screen zoom ([user docs](./Configuration:-Zoom.md), state in `src/layout/zoom.rs`) is one
consumer of this machinery, not its topic: magnification is an optional operation *within*
`Local`, described in its own section below.

The implementation lives in two shared files plus per-feature consumers:

- `src/utils/geometry.rs` — `Global` / `Local` frame markers and conversion traits.
- `src/utils/view.rs` — `OutputViewCtx` (immutable output geometry context) and
  `ViewportTransform` (immutable sampled view operation).
- Consumers: `src/layout/zoom.rs` (`OutputZoomState`), input hit-testing (`src/input/`),
  screencasting (`src/screencasting/`), render boundaries (`src/render_helpers/`, `src/ui/`).

## Pipeline

```
compositor scene (Space, Global)
        │  Global ↔ Local (pure translation by output origin)
        ▼
output-local logical scene (Local, presented orientation)
        │  optional view operation within Local (e.g. zoom magnification)
        ▼
output Transform (rotation/reflection, render stage only)
        │  Logical → Physical (output scale)
        ▼
render target
```

These stages stay separately represented and separately composable.
Do not collapse them into one matrix or one render-element wrapper.

## Frames

Smithay geometry types carry a single `Kind` marker parameter, and niri reuses that slot
for frame-of-reference where it matters:

- `Global` — compositor-absolute logical coordinates (the `Space` scene).
- `Local` — output-local logical coordinates, origin at the output's top-left corner,
  in presented orientation. Hit-testing, layout, and view operations work in this frame.
- `Physical` — output-local physical pixels. Already implies its frame; never retyped.
- `Buffer` — surface/render buffer pixels. Already implies its frame; never retyped.
- `SurfaceLocal` — short-lived surface-relative input offsets (`pub(crate)`).
  Unlike `Global`/`Local` it is not a persistent compositor frame; it exists to keep
  surface-relative offsets from being mistaken for global positions. Do not add
  further markers (WorkspaceLocal, BackdropLocal, …) without a call-site-driven
  reason — screen/content/surface stay nominal until a second consumer
  demonstrably confuses them.

Units and frames are conceptually separate axes, but they share Smithay's single `Kind`
slot, so `Point<_, Global>` and `Point<_, Physical>` are alternative marker choices,
not a Cartesian product. In practice the ambiguous frame axis is the Logical one:
`Global`/`Local` mark compositor logical frames, while `Physical`/`Buffer` need no
frame marker.

## Extension traits (`src/utils/geometry.rs`)

| Trait | Visibility | Receiver | Methods | Meaning |
|---|---|---|---|---|
| `PointExt` | `pub(crate)` | `Point<C, Logical>` | `assume_global`, `assume_local` | Zero-cost relabel, no translation. Boundary-only. |
| `PointGlobalExt` | `pub(crate)` | `Point<C, Global>` | `to_local(&ctx)` | Genuine conversion: subtract output origin. |
| `PointGlobalExt` | `pub(crate)` | `Point<C, Global>` | `as_logical()` | Relabel for Logical-only Smithay APIs. No translation. |
| `PointLocalExt` | `pub(crate)` | `Point<C, Local>` | `to_global(&ctx)` | Genuine conversion: add output origin. |
| `PointLocalExt` | `pub(crate)` | `Point<C, Local>` | `as_logical`, `to_physical`, `to_physical_precise_round` | Smithay interop / unit conversion via `Logical`. |
| `PointSurfaceLocalExt` | `pub(crate)` | `Point<C, SurfaceLocal>` | `as_logical()` | Relabel at the surface boundary. |
| `RectExt` | `pub(crate)` | `Rectangle<C, Logical>` | `assume_global`, `assume_local` | Relabel loc + size. Boundary-only. |
| `RectGlobalExt` | `pub(crate)` | `Rectangle<C, Global>` | `to_local(&ctx)`, `as_logical` | Translates loc, preserves size. |
| `RectLocalExt` | `pub(crate)` | `Rectangle<C, Local>` | `to_global(&ctx)`, `as_logical`, `to_physical…` | Translates loc, preserves size. |
| `SizeExt` | `pub` (`niri-visual-tests` uses `assume_local` for `Tile` sizes) | `Size<C, Logical/Global/Local>` | `as_logical`, `assume_global`, `assume_local` | Relabels only: sizes have no position and are frame-invariant. |

Two `pub(crate)` helpers convert between the global frame and surface-relative offsets:
`surface_offset(point, surface_origin)` and `surface_position(surface_origin, surface_location)`.

### Rules

- `to_local` / `to_global` are genuine coordinate transformations and require the
  output-view context. They are a pure translation by the output origin — output
  rotation/reflection and scaling do not participate, because `OutputViewCtx::for_output`
  bakes the output transform and scale into `local_geo` (presented orientation) at
  construction time. These extension-trait conversions are the live path used by all
  non-test callers (input, hit-testing, screencasting, zoom viewport computation).
- `assume_local` / `assume_global` are not conversions. They relabel a bare `Logical`
  value whose frame is already established by surrounding invariants, and must not become
  general-purpose conveniences to silence the compiler.
- `as_logical` relabels a typed value for a Logical-only Smithay API (surface-tree input,
  render elements, protocol rects). Bare `Logical` is allowed only at such explicit
  interoperability boundaries — new compositor logic stays in `Global`/`Local` once the
  frame is known.
- There is deliberately no `to_zoomed()`-style conversion coupled to view state.
  View operations apply visibly as `Global → Local → transform → Local`,
  never hidden inside a geometry trait.

## `OutputViewCtx` (`src/utils/view.rs`)

Immutable geometric context answering "where is this output, geometrically?"
It owns no zoom level, focal-point animation, mutable viewport state, renderers, or damage state.

```rust
pub struct OutputViewCtx {
    pub global_geo: Rectangle<f64, Global>,
    pub local_geo: Rectangle<f64, Local>,
    pub output_transform: Transform,
    pub scale: Scale<f64>,
}
```

- `OutputViewCtx::for_output(&global_space, &output)` is the canonical constructor.
- `OutputViewCtx::from_origin(origin)` builds a minimal context where only the origin is
  meaningful (transform `Normal`, scale 1.0).
- `output_origin()` exposes the translation offset used by the `to_global` / `to_local`
  trait conversions. Prefer the trait conversions over recomputing
  `global_space.output_geometry(output).loc` by hand.
- `physical_rect_to_local` / `local_rect_to_physical` (and the point-level counterparts)
  convert between `Physical` pixels and `Local` logical geometry by output scale only.
  The output transform is intentionally ignored there: element geometry is already in
  presented orientation, and rotation composes after the viewport.

## Example consumer: screen zoom

Magnification is a transformation *within* `Local`, which is why it needs no third
coordinate marker:

```rust
pub struct ViewportTransform {
    pub focal: Point<f64, Local>,
    pub factor: f64,
}
```

- `ViewportTransform::new(focal, factor)` (`factor` must be finite and `> 0`),
  `ViewportTransform::identity()` for the 1.0× case.
- `apply` / `apply_inverse`: `focal + (point - focal) * factor` (and `/ factor`).
- `apply_rect` / `apply_inverse_rect`: axis-aligned bounding box of the transformed
  rectangle's image / preimage (exact for the current uniform scale + translation;
  stated geometrically so a future non-axis-preserving transform cannot silently change
  the contract). Results are unrounded — rounding is the consumer's job.
- `to_matrix()`: equivalent 2D affine matrix for rendering.

Rules, which generalize to any future view operation in `Local`:

- Construct it only through `OutputZoomState::viewport_transform(now)` (`src/layout/zoom.rs`).
  No consumer reaches into raw animation state to build one.
- Never branch on "is zoom active?". At 1.0× the transform is `IDENTITY` and
  `apply(p) == p`, so consumers apply it unconditionally.
- It is a point-in-time value for one output: do not store it across frames and do not
  share it across outputs. Rendering and input each sample it when needed.
- It owns no policy: focal clamping, movement modes, and animation targets belong to
  `OutputZoomState`, the mutable per-output policy + animation state stored in
  `Layout::zoom_states` keyed by `Output` (`level`, `focal`, `locked`,
  `level_transition: Idle | Animating | Gesturing`, `focal_animation`).

### Ownership of the four view concepts

The implementation already separates the four responsibilities the pipeline implies —
they just live under two owners, which is load-bearing rather than accidental:

- `OutputViewCtx` = static output geometry/presentation. Stored per-output in
  `Niri::OutputState::view_ctx` (`src/niri.rs`), refreshed by `reposition_outputs` /
  `output_resized`. Render code reaches it through `output_state`.
- `OutputZoomState` = dynamic view policy. Stored per-output in `Layout::zoom_states`
  (`src/layout/mod.rs`), **not** in `OutputState`: its animation tick runs inside
  `Layout::advance_animations` and focal tracking depends on `Layout::output_size_for_focal`.
  Co-locating it under `OutputState` would split the tick or drag layout-size logic
  across the boundary for purely organizational gain.
- `ViewportTransform` = dynamic view snapshot. Sampled via `viewport_transform(now)` and
  passed as an explicit parameter into `render` / `render_inner` alongside `RenderCtx`,
  never stored in it — including the `IDENTITY` selection for non-output captures.
- `RenderCtx` = rendering machinery only (`renderer`, `target`, `xray`).

So the render signature already embodies the conceptual split: machinery, static
geometry, and sampled view arrive as three separate things. Do not merge the viewport
into `RenderCtx` or the zoom state into `OutputState` without a call-site-driven reason.

### Screen vs content: a nominal distinction, deliberately

`Niri::screen_to_content` (`src/niri.rs`) maps a screen-space `Local` point through the
inverse viewport to the content-space `Local` point hit-testing and effects consume.
Both sides are `Point<f64, Local>` — the screen/content distinction lives in function
and variable names, not in marker types, and the current call sites are all input
hit-testing paths. That is the correct stopping point for now: marks like
screen / content / surface / render-target should only become types when a second
consumer demonstrably confuses them. Candidates that would justify new markers if they
arrive: blur regions needing content-space rects at render, xray backdrop sampling in
backdrop-frame coordinates, or persistent window-local shader frames. Until such a call
site exists, the named function plus the `Local` frame carry the semantics.
