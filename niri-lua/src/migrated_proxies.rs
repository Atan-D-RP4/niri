//! Migrated proxy implementations using derive macros.
//!
//! This module contains proxy implementations that use the `LuaConfigProxy` derive macro
//! instead of manual implementations. As migration progresses, proxies are moved from
//! `config_wrapper.rs` to this module.
//!
//! ## Migration Status
//!
//! - [x] CursorConfigProxy - migrated from CursorProxy
//! - [x] ClipboardConfigProxy - migrated from ClipboardProxy
//! - [x] HotkeyOverlayConfigProxy - migrated from HotkeyOverlayProxy
//! - [x] ConfigNotificationConfigProxy - migrated from ConfigNotificationProxy
//! - [x] XwaylandSatelliteConfigProxy - migrated from XwaylandSatelliteProxy
//! - [x] TouchpadConfigProxy - migrated from TouchpadProxy
//! - [x] KeyboardConfigProxy - migrated from KeyboardProxy
//! - [ ] Other proxies - pending

use niri_config::input::{AccelProfile, ClickMethod, ScrollMethod, TapButtonMap, TrackLayout};
use niri_config::{Color, FloatOrInt};
use niri_lua_derive::LuaConfigProxy;

// Re-export ConfigState for internal use
pub use crate::config_state::ConfigState;

// Note: The LuaConfigProxy derive macro generates `{StructName}Proxy` from `{StructName}`
// which is automatically public because the struct is public. No re-export needed.

/// Proxy for cursor configuration.
///
/// This proxy provides access to cursor settings like size, theme,
/// and hide behavior.
///
/// ## Lua Usage
///
/// ```lua
/// -- Get cursor size
/// local size = config.cursor.xcursor_size
///
/// -- Set cursor theme
/// config.cursor.xcursor_theme = "Adwaita"
///
/// -- Configure auto-hide
/// config.cursor.hide_when_typing = true
/// config.cursor.hide_after_inactive_ms = 5000
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(crate = "crate", parent_path = "cursor", dirty = "Cursor")]
pub struct CursorConfig {
    /// Cursor size in pixels. Default is 24.
    #[lua_proxy(field)]
    pub xcursor_size: u8,

    /// Cursor theme name.
    #[lua_proxy(field)]
    pub xcursor_theme: String,

    /// Whether to hide cursor while typing.
    #[lua_proxy(field)]
    pub hide_when_typing: bool,

    /// Milliseconds of inactivity before hiding cursor.
    /// Set to `nil` to disable auto-hide.
    #[lua_proxy(field)]
    pub hide_after_inactive_ms: Option<u32>,
}

/// Proxy for clipboard configuration.
///
/// ## Lua Usage
///
/// ```lua
/// -- Disable primary selection
/// config.clipboard.disable_primary = true
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(crate = "crate", parent_path = "clipboard", dirty = "Clipboard")]
pub struct ClipboardConfig {
    /// Whether to disable primary selection (middle-click paste).
    #[lua_proxy(field)]
    pub disable_primary: bool,
}

/// Proxy for hotkey overlay configuration.
///
/// ## Lua Usage
///
/// ```lua
/// -- Skip showing overlay at startup
/// config.hotkey_overlay.skip_at_startup = true
///
/// -- Hide unbound keys
/// config.hotkey_overlay.hide_not_bound = true
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "hotkey_overlay",
    dirty = "HotkeyOverlay"
)]
pub struct HotkeyOverlayConfig {
    /// Whether to skip showing the overlay at startup.
    #[lua_proxy(field)]
    pub skip_at_startup: bool,

    /// Whether to hide keys that are not bound.
    #[lua_proxy(field)]
    pub hide_not_bound: bool,
}

/// Proxy for config notification settings.
///
/// ## Lua Usage
///
/// ```lua
/// -- Disable error notifications for config failures
/// config.config_notification.disable_failed = true
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "config_notification",
    dirty = "ConfigNotification"
)]
pub struct ConfigNotificationConfig {
    /// Whether to disable notifications when config loading fails.
    #[lua_proxy(field)]
    pub disable_failed: bool,
}

/// Proxy for xwayland-satellite configuration.
///
/// ## Lua Usage
///
/// ```lua
/// -- Disable xwayland-satellite
/// config.xwayland_satellite.off = true
///
/// -- Set custom path
/// config.xwayland_satellite.path = "/usr/bin/xwayland-satellite"
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "xwayland_satellite",
    dirty = "XwaylandSatellite"
)]
pub struct XwaylandSatelliteConfig {
    /// Whether xwayland-satellite is disabled.
    #[lua_proxy(field)]
    pub off: bool,

    /// Path to the xwayland-satellite binary.
    #[lua_proxy(field)]
    pub path: String,
}

/// Proxy for debug configuration.
///
/// Contains various debug flags that affect compositor behavior.
/// These are primarily for development and troubleshooting.
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(crate = "crate", parent_path = "debug", dirty = "Debug")]
pub struct DebugConfig {
    /// Whether to enable DBus interfaces in non-session instances.
    #[lua_proxy(field)]
    pub dbus_interfaces_in_non_session_instances: bool,

    /// Whether to wait for frame completion before queueing the next frame.
    #[lua_proxy(field)]
    pub wait_for_frame_completion_before_queueing: bool,

    /// Whether to enable overlay planes.
    #[lua_proxy(field)]
    pub enable_overlay_planes: bool,

    /// Whether to disable the cursor plane.
    #[lua_proxy(field)]
    pub disable_cursor_plane: bool,

    /// Whether to disable direct scanout.
    #[lua_proxy(field)]
    pub disable_direct_scanout: bool,

    /// Whether to keep max BPC unchanged.
    #[lua_proxy(field)]
    pub keep_max_bpc_unchanged: bool,

    /// Whether to restrict primary scanout to matching format.
    #[lua_proxy(field)]
    pub restrict_primary_scanout_to_matching_format: bool,
}

/// Proxy for layout struts configuration.
///
/// Struts define reserved areas on the edges of the screen that windows
/// cannot occupy, useful for panels or docks.
///
/// ## Lua Usage
///
/// ```lua
/// config.layout.struts.left = 50
/// config.layout.struts.right = 0
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(crate = "crate", parent_path = "layout.struts", dirty = "Layout")]
pub struct StrutsConfig {
    /// Left strut size in logical pixels.
    #[lua_proxy(field)]
    pub left: FloatOrInt<-65535, 65535>,

    /// Right strut size in logical pixels.
    #[lua_proxy(field)]
    pub right: FloatOrInt<-65535, 65535>,

    /// Top strut size in logical pixels.
    #[lua_proxy(field)]
    pub top: FloatOrInt<-65535, 65535>,

    /// Bottom strut size in logical pixels.
    #[lua_proxy(field)]
    pub bottom: FloatOrInt<-65535, 65535>,
}

/// Proxy for XKB keyboard configuration.
///
/// ## Lua Usage
///
/// ```lua
/// config.input.keyboard.xkb.layout = "us,ru"
/// config.input.keyboard.xkb.options = "grp:alt_shift_toggle"
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "input.keyboard.xkb",
    dirty = "Keyboard"
)]
pub struct XkbConfig {
    /// XKB layout name(s).
    #[lua_proxy(field)]
    pub layout: String,

    /// XKB variant(s).
    #[lua_proxy(field)]
    pub variant: String,

    /// XKB model.
    #[lua_proxy(field)]
    pub model: String,

    /// XKB rules.
    #[lua_proxy(field)]
    pub rules: String,

    /// XKB options.
    #[lua_proxy(field)]
    pub options: Option<String>,
}

/// Proxy for keyboard input configuration.
///
/// ## Lua Usage
///
/// ```lua
/// config.input.keyboard.repeat_delay = 600
/// config.input.keyboard.repeat_rate = 25
/// config.input.keyboard.numlock = true
/// config.input.keyboard.track_layout = "global"  -- or "window"
/// config.input.keyboard.xkb.layout = "us,ru"
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(crate = "crate", parent_path = "input.keyboard", dirty = "Keyboard")]
pub struct KeyboardConfig {
    /// Key repeat delay in milliseconds.
    #[lua_proxy(field)]
    pub repeat_delay: u16,

    /// Key repeat rate (repeats per second).
    #[lua_proxy(field)]
    pub repeat_rate: u8,

    /// Whether to enable numlock on startup.
    #[lua_proxy(field)]
    pub numlock: bool,

    /// Keyboard layout tracking mode ("global" or "window").
    #[lua_proxy(field)]
    pub track_layout: TrackLayout,

    /// XKB keyboard configuration (layout, variant, options, etc).
    #[lua_proxy(nested)]
    pub xkb: XkbConfig,
}

/// Proxy for mouse input configuration.
///
/// ## Lua Usage
///
/// ```lua
/// config.input.mouse.natural_scroll = true
/// config.input.mouse.accel_speed = 0.5
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(crate = "crate", parent_path = "input.mouse", dirty = "Input")]
pub struct MouseConfig {
    /// Whether the mouse is disabled.
    #[lua_proxy(field)]
    pub off: bool,

    /// Whether natural scroll is enabled.
    #[lua_proxy(field)]
    pub natural_scroll: bool,

    /// Whether left-handed mode is enabled.
    #[lua_proxy(field)]
    pub left_handed: bool,

    /// Whether middle button emulation is enabled.
    #[lua_proxy(field)]
    pub middle_emulation: bool,

    /// Acceleration speed (-1.0 to 1.0).
    #[lua_proxy(field)]
    pub accel_speed: FloatOrInt<-1, 1>,
}

/// Proxy for trackpoint input configuration.
///
/// ## Lua Usage
///
/// ```lua
/// config.input.trackpoint.natural_scroll = true
/// config.input.trackpoint.accel_speed = 0.5
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(crate = "crate", parent_path = "input.trackpoint", dirty = "Input")]
pub struct TrackpointConfig {
    /// Whether the trackpoint is disabled.
    #[lua_proxy(field)]
    pub off: bool,

    /// Whether natural scroll is enabled.
    #[lua_proxy(field)]
    pub natural_scroll: bool,

    /// Whether left-handed mode is enabled.
    #[lua_proxy(field)]
    pub left_handed: bool,

    /// Whether middle button emulation is enabled.
    #[lua_proxy(field)]
    pub middle_emulation: bool,

    /// Acceleration speed (-1.0 to 1.0).
    #[lua_proxy(field)]
    pub accel_speed: FloatOrInt<-1, 1>,
}

/// Proxy for touch input configuration.
///
/// ## Lua Usage
///
/// ```lua
/// config.input.touch.off = true
/// config.input.touch.natural_scroll = true
/// config.input.touch.map_to_output = "eDP-1"
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(crate = "crate", parent_path = "input.touch", dirty = "Input")]
pub struct TouchConfig {
    /// Whether touch input is disabled.
    #[lua_proxy(field)]
    pub off: bool,

    /// Whether natural scroll is enabled for touch.
    #[lua_proxy(field)]
    pub natural_scroll: bool,

    /// Output to map touch input to.
    #[lua_proxy(field)]
    pub map_to_output: Option<String>,
}

/// Proxy for gesture DnD edge view scroll configuration.
///
/// Controls scroll behavior when dragging windows to screen edges.
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "gestures.dnd_edge_view_scroll",
    dirty = "Gestures"
)]
pub struct DndEdgeViewScrollConfig {
    /// Width of the trigger zone at the edge.
    #[lua_proxy(field)]
    pub trigger_width: f64,

    /// Delay in milliseconds before scroll starts.
    #[lua_proxy(field)]
    pub delay_ms: u16,

    /// Maximum scroll speed.
    #[lua_proxy(field)]
    pub max_speed: f64,
}

/// Proxy for gesture DnD edge workspace switch configuration.
///
/// Controls workspace switching behavior when dragging windows to screen edges.
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "gestures.dnd_edge_workspace_switch",
    dirty = "Gestures"
)]
pub struct DndEdgeWorkspaceSwitchConfig {
    /// Height of the trigger zone at the edge.
    #[lua_proxy(field)]
    pub trigger_height: f64,

    /// Delay in milliseconds before switch starts.
    #[lua_proxy(field)]
    pub delay_ms: u16,

    /// Maximum speed of workspace switching.
    #[lua_proxy(field)]
    pub max_speed: f64,
}

/// Proxy for hot corners configuration.
///
/// Controls hot corner behavior at screen edges.
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "gestures.hot_corners",
    dirty = "Gestures"
)]
pub struct HotCornersConfig {
    /// Whether hot corners are disabled.
    #[lua_proxy(field)]
    pub off: bool,

    /// Whether top-left corner is active.
    #[lua_proxy(field)]
    pub top_left: bool,

    /// Whether top-right corner is active.
    #[lua_proxy(field)]
    pub top_right: bool,

    /// Whether bottom-left corner is active.
    #[lua_proxy(field)]
    pub bottom_left: bool,

    /// Whether bottom-right corner is active.
    #[lua_proxy(field)]
    pub bottom_right: bool,
}

/// Proxy for MRU previews configuration.
///
/// Controls the preview thumbnails in the recent windows UI.
///
/// ## Lua Usage
///
/// ```lua
/// config.recent_windows.previews.max_height = 200.0
/// config.recent_windows.previews.max_scale = 0.2
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "recent_windows.previews",
    dirty = "RecentWindows"
)]
pub struct MruPreviewsConfig {
    /// Maximum height of preview thumbnails.
    #[lua_proxy(field)]
    pub max_height: f64,

    /// Maximum scale factor for previews.
    #[lua_proxy(field)]
    pub max_scale: f64,
}

/// Proxy for layout insert hint configuration.
///
/// Controls the visual hint shown when inserting windows into the layout.
///
/// ## Lua Usage
///
/// ```lua
/// -- Disable insert hint
/// config.layout.insert_hint.off = true
///
/// -- Set insert hint color
/// config.layout.insert_hint.color = "#ff0000"
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(crate = "crate", parent_path = "layout.insert_hint", dirty = "Layout")]
pub struct InsertHintConfig {
    /// Whether the insert hint is disabled.
    #[lua_proxy(field)]
    pub off: bool,

    /// Color of the insert hint.
    #[lua_proxy(field)]
    pub color: Color,
}

/// Proxy for MRU highlight configuration.
///
/// Controls the highlight styling in the recent windows UI.
///
/// ## Lua Usage
///
/// ```lua
/// -- Set active window highlight color
/// config.recent_windows.highlight.active_color = "#00ff00"
///
/// -- Set urgent window highlight color
/// config.recent_windows.highlight.urgent_color = "#ff0000"
///
/// -- Set highlight padding and corner radius
/// config.recent_windows.highlight.padding = 4.0
/// config.recent_windows.highlight.corner_radius = 8.0
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "recent_windows.highlight",
    dirty = "RecentWindows"
)]
pub struct MruHighlightConfig {
    /// Color of highlight for active windows.
    #[lua_proxy(field)]
    pub active_color: Color,

    /// Color of highlight for urgent windows.
    #[lua_proxy(field)]
    pub urgent_color: Color,

    /// Padding around the highlight.
    #[lua_proxy(field)]
    pub padding: f64,

    /// Corner radius of the highlight.
    #[lua_proxy(field)]
    pub corner_radius: f64,
}

/// Proxy for touchpad input configuration.
///
/// ## Lua Usage
///
/// ```lua
/// -- Disable touchpad
/// config.input.touchpad.off = true
///
/// -- Enable tap-to-click
/// config.input.touchpad.tap = true
///
/// -- Enable natural scrolling
/// config.input.touchpad.natural_scroll = true
///
/// -- Set acceleration speed
/// config.input.touchpad.accel_speed = 0.5
///
/// -- Configure scroll method
/// config.input.touchpad.scroll_method = "two-finger"
///
/// -- Configure click method
/// config.input.touchpad.click_method = "clickfinger"
///
/// -- Configure tap button map
/// config.input.touchpad.tap_button_map = "left-right-middle"
///
/// -- Configure acceleration profile
/// config.input.touchpad.accel_profile = "adaptive"
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(crate = "crate", parent_path = "input.touchpad", dirty = "Input")]
pub struct TouchpadConfig {
    /// Whether the touchpad is disabled.
    #[lua_proxy(field)]
    pub off: bool,

    /// Whether tap-to-click is enabled.
    #[lua_proxy(field)]
    pub tap: bool,

    /// Whether disable-while-typing is enabled.
    #[lua_proxy(field)]
    pub dwt: bool,

    /// Whether disable-while-trackpointing is enabled.
    #[lua_proxy(field)]
    pub dwtp: bool,

    /// Whether natural scroll is enabled.
    #[lua_proxy(field)]
    pub natural_scroll: bool,

    /// Whether left-handed mode is enabled.
    #[lua_proxy(field)]
    pub left_handed: bool,

    /// Whether middle button emulation is enabled.
    #[lua_proxy(field)]
    pub middle_emulation: bool,

    /// Whether drag lock is enabled.
    #[lua_proxy(field)]
    pub drag_lock: bool,

    /// Whether touchpad is disabled when external mouse is connected.
    #[lua_proxy(field)]
    pub disabled_on_external_mouse: bool,

    /// Acceleration speed (-1.0 to 1.0).
    #[lua_proxy(field)]
    pub accel_speed: FloatOrInt<-1, 1>,

    /// Scroll method configuration.
    ///
    /// Valid values: "no-scroll", "two-finger", "edge", "on-button-down", or nil.
    #[lua_proxy(field)]
    pub scroll_method: Option<ScrollMethod>,

    /// Click method configuration.
    ///
    /// Valid values: "button-areas", "clickfinger", or nil.
    #[lua_proxy(field)]
    pub click_method: Option<ClickMethod>,

    /// Tap button map configuration.
    ///
    /// Valid values: "left-right-middle", "left-middle-right", or nil.
    #[lua_proxy(field)]
    pub tap_button_map: Option<TapButtonMap>,

    /// Acceleration profile configuration.
    ///
    /// Valid values: "adaptive", "flat", or nil.
    #[lua_proxy(field)]
    pub accel_profile: Option<AccelProfile>,
}
