### Overview

<sup>Since: next release</sup>

Niri supports screen zoom (magnification) triggered by a pinch gesture or a key binding.

### Using Zoom

Zoom starts at 1.0× (no zoom). The level is clamped to `1.0`–`max-zoom`. There are several ways to zoom:

- **Touchpad pinch**: Perform a three-finger pinch gesture on a touchpad to zoom
  in and out.
- **Touchscreen pinch**: Pinch-to-zoom with two fingers also works on touchscreens
  (the gesture cancels if a third finger touches down).
- **Key binding**: See the [Key Bindings](./Configuration:-Key-Bindings.md#set-zoom-level) page.

Zoom remains active (viewport stays zoomed in) until a gesture or action resets it back to 1.0×.

Each output zooms independently around its own focal point.
The focal point tracks the cursor according to the configured [`movement-mode`](./Configuration:-Miscellaneous.md#movement-mode),
or stays put while [locked](./Configuration:-Key-Bindings.md#toggle-zoom-lock).

### Zoom lock and app gestures

While zoom is unlocked, a three-finger touchpad pinch (or a two-finger touchscreen
pinch) is consumed by the compositor for zoom, so apps with their own pinch gestures —
browsers, art tools, PDF/e-book readers — never see it.
Locking the zoom is the first-pass answer to that conflict: while an output's zoom is
locked, niri stops starting zoom gestures there and forwards the gesture events to the
Wayland client instead, so apps receive complete begin/update/end sequences.
On touchscreens, clients always receive the raw touch events; locking only stops the
compositor from zooming alongside them.

Notes:

- Key-bind zoom still adjusts the level while locked; only focal tracking is frozen.
- The touchpad finger count is fixed at three for now.
- The lock is per output: pinch gestures act on the output under the cursor
  (touch midpoint on touchscreens), while `set-zoom-level` / `toggle-zoom-lock`
  act on the focused output, or the named output if given (the screenshot
  selection output while the screenshot UI is open).

### Configuration

All zoom settings are configured in the [`zoom {}` config block](./Configuration:-Miscellaneous.md#zoom):
Zoom can also be animated. See the [`animations {}` settings](./Configuration:-Animations.md)

The cursor can optionally scale with zoom via the [`scale-with-zoom`](./Configuration:-Miscellaneous.md#scale-with-zoom) cursor setting.

### IPC

#### State Query

You can query the current zoom state of all outputs, or of one output:

```sh
niri msg zoom-state
niri msg zoom-state "eDP-1"
```

This returns a map from output name to zoom state, where each state contains:

- `level`: the current zoom level (1.0 = no zoom).
- `is_locked`: whether zoom is locked.

#### Events

The compositor emits a `ZoomChanged` event whenever the zoom state changes (at
commit granularity, not per animation frame). This event contains the output
name, current zoom level, and whether zoom is locked.

### Interaction with Other Features

- **Screencasting**: Output screencasts render what you see, including the live
  zoom viewport. Window screencasts are never zoomed.
- **Screenshots**: Direct screenshots capture the unzoomed output. Screencopy
  captures what you see, including the live zoom viewport. Inside the
  screenshot UI, the captured texture stays unzoomed but the
  live preview is shown through the current zoom viewport, and the confirmed
  export matches the preview (WYSIWYG). The screenshot UI can optionally scale
  the cursor indicator with the zoom level when `scale-with-zoom` is enabled.
  Zoom key binds keep working while the screenshot UI is open, acting on the
  screenshot selection output.
- **Lock Screen**: Zoom persists across the lock screen. The zoom key binds
  work while locked, since zoom is an accessibility feature.
