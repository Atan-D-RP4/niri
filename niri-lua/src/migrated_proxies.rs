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
//! - [x] InputConfigProxy - migrated from InputProxy
//! - [x] TouchpadConfigProxy - migrated from TouchpadProxy
//! - [x] KeyboardConfigProxy - migrated from KeyboardProxy
//! - [x] FocusRingConfigProxy - migrated from FocusRingProxy
//! - [x] BorderConfigProxy - migrated from BorderProxy
//! - [x] ShadowConfigProxy - migrated from ShadowProxy
//! - [x] OverviewWorkspaceShadowConfigProxy - migrated from OverviewWorkspaceShadowProxy
//! - [x] OverviewConfigProxy - migrated from OverviewProxy
//! - [x] RecentWindowsConfigProxy - migrated from RecentWindowsProxy
//!
//! Migration complete - all manual proxies have been migrated to derive macros.

use niri_config::animations::Kind;
use niri_config::input::{AccelProfile, ClickMethod, ScrollMethod, TapButtonMap, TrackLayout};
use niri_config::layout::CenterFocusedColumn;
use niri_config::{Color, FloatOrInt, Gradient, ShadowOffset, TabIndicatorPosition};
use niri_ipc::ColumnDisplay;
use niri_lua_derive::LuaConfigProxy;

// Re-export ConfigState for internal use
pub use crate::config_state::ConfigState;

// Note: The LuaConfigProxy derive macro generates `{StructName}Proxy` from `{StructName}`
// which is automatically public because the struct is public. No re-export needed.

// =============================================================================
// Group 1: Standalone Leaf Structs (no nested dependencies)
// =============================================================================

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
#[lua_proxy(crate = "crate", parent_path = "cursor", dirty = "Cursor", auto_detect)]
pub struct CursorConfig {
    /// Cursor size in pixels. Default is 24.
    pub xcursor_size: u8,

    /// Cursor theme name.
    pub xcursor_theme: String,

    /// Whether to hide cursor while typing.
    pub hide_when_typing: bool,

    /// Milliseconds of inactivity before hiding cursor.
    /// Set to `nil` to disable auto-hide.
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
#[lua_proxy(
    crate = "crate",
    parent_path = "clipboard",
    dirty = "Clipboard",
    auto_detect
)]
pub struct ClipboardConfig {
    /// Whether to disable primary selection (middle-click paste).
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
    dirty = "HotkeyOverlay",
    auto_detect
)]
pub struct HotkeyOverlayConfig {
    /// Whether to skip showing the overlay at startup.
    pub skip_at_startup: bool,

    /// Whether to hide keys that are not bound.
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
    dirty = "ConfigNotification",
    auto_detect
)]
pub struct ConfigNotificationConfig {
    /// Whether to disable notifications when config loading fails.
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
    dirty = "XwaylandSatellite",
    auto_detect
)]
pub struct XwaylandSatelliteConfig {
    /// Whether xwayland-satellite is disabled.
    pub off: bool,

    /// Path to the xwayland-satellite binary.
    pub path: String,
}

/// Proxy for debug configuration.
///
/// Contains various debug flags that affect compositor behavior.
/// These are primarily for development and troubleshooting.
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(crate = "crate", parent_path = "debug", dirty = "Debug", auto_detect)]
pub struct DebugConfig {
    /// Whether to enable DBus interfaces in non-session instances.
    pub dbus_interfaces_in_non_session_instances: bool,

    /// Whether to wait for frame completion before queueing the next frame.
    pub wait_for_frame_completion_before_queueing: bool,

    /// Whether to enable overlay planes.
    pub enable_overlay_planes: bool,

    /// Whether to disable the cursor plane.
    pub disable_cursor_plane: bool,

    /// Whether to disable direct scanout.
    pub disable_direct_scanout: bool,

    /// Whether to keep max BPC unchanged.
    pub keep_max_bpc_unchanged: bool,

    /// Whether to restrict primary scanout to matching format.
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
#[lua_proxy(
    crate = "crate",
    parent_path = "layout.struts",
    dirty = "Layout",
    auto_detect
)]
pub struct StrutsConfig {
    /// Left strut size in logical pixels.
    pub left: FloatOrInt<-65535, 65535>,

    /// Right strut size in logical pixels.
    pub right: FloatOrInt<-65535, 65535>,

    /// Top strut size in logical pixels.
    pub top: FloatOrInt<-65535, 65535>,

    /// Bottom strut size in logical pixels.
    pub bottom: FloatOrInt<-65535, 65535>,
}

/// Proxy for spawn-at-startup configuration collection.
///
/// Controls programs that should be launched automatically when the compositor starts.
///
/// ## Lua Usage
///
/// ```lua
/// -- Get the number of startup commands
/// local count = #config.spawn_at_startup
///
/// -- Access a specific startup command
/// local first = config.spawn_at_startup[1]
/// print(first.command[1])  -- First argument of the command
///
/// -- Add a new startup command
/// config.spawn_at_startup:append({ command = { "alacritty" } })
///
/// -- Remove a startup command
/// config.spawn_at_startup:remove(1)
///
/// -- Clear all startup commands
/// config.spawn_at_startup:clear()
///
/// -- Iterate over all startup commands
/// for i, cmd in ipairs(config.spawn_at_startup) do
///     print(i, cmd.command[1])
/// end
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(crate = "crate", is_root, dirty = "SpawnAtStartup")]
pub struct SpawnAtStartupConfig {
    /// Collection of commands to spawn at startup.
    #[lua_proxy(collection)]
    pub spawn_at_startup: Vec<niri_config::SpawnAtStartup>,
}

// =============================================================================
// Group 2: Keyboard dependencies (XkbConfig before KeyboardConfig)
// =============================================================================

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
    dirty = "Keyboard",
    auto_detect
)]
pub struct XkbConfig {
    /// XKB layout name(s).
    pub layout: String,

    /// XKB variant(s).
    pub variant: String,

    /// XKB model.
    pub model: String,

    /// XKB rules.
    pub rules: String,

    /// XKB options.
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
#[lua_proxy(
    crate = "crate",
    parent_path = "input.keyboard",
    dirty = "Keyboard",
    auto_detect
)]
pub struct KeyboardConfig {
    /// Key repeat delay in milliseconds.
    pub repeat_delay: u16,

    /// Key repeat rate (repeats per second).
    pub repeat_rate: u8,

    /// Whether to enable numlock on startup.
    pub numlock: bool,

    /// Keyboard layout tracking mode ("global" or "window").
    pub track_layout: TrackLayout,

    /// XKB keyboard configuration (layout, variant, options, etc).
    pub xkb: XkbConfig,
}

// =============================================================================
// Group 3: Input device leaves (BEFORE InputConfig)
// =============================================================================

/// Proxy for mouse input configuration.
///
/// ## Lua Usage
///
/// ```lua
/// config.input.mouse.natural_scroll = true
/// config.input.mouse.accel_speed = 0.5
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "input.mouse",
    dirty = "Input",
    auto_detect
)]
pub struct MouseConfig {
    /// Whether the mouse is disabled.
    pub off: bool,

    /// Whether natural scroll is enabled.
    pub natural_scroll: bool,

    /// Whether left-handed mode is enabled.
    pub left_handed: bool,

    /// Whether middle button emulation is enabled.
    pub middle_emulation: bool,

    /// Acceleration speed (-1.0 to 1.0).
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
#[lua_proxy(
    crate = "crate",
    parent_path = "input.trackpoint",
    dirty = "Input",
    auto_detect
)]
pub struct TrackpointConfig {
    /// Whether the trackpoint is disabled.
    pub off: bool,

    /// Whether natural scroll is enabled.
    pub natural_scroll: bool,

    /// Whether left-handed mode is enabled.
    pub left_handed: bool,

    /// Whether middle button emulation is enabled.
    pub middle_emulation: bool,

    /// Acceleration speed (-1.0 to 1.0).
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
#[lua_proxy(
    crate = "crate",
    parent_path = "input.touch",
    dirty = "Input",
    auto_detect
)]
pub struct TouchConfig {
    /// Whether touch input is disabled.
    pub off: bool,

    /// Whether natural scroll is enabled for touch.
    pub natural_scroll: bool,

    /// Output to map touch input to.
    pub map_to_output: Option<String>,
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
#[lua_proxy(
    crate = "crate",
    parent_path = "input.touchpad",
    dirty = "Input",
    auto_detect
)]
pub struct TouchpadConfig {
    /// Whether the touchpad is disabled.
    pub off: bool,

    /// Whether tap-to-click is enabled.
    pub tap: bool,

    /// Whether disable-while-typing is enabled.
    pub dwt: bool,

    /// Whether disable-while-trackpointing is enabled.
    pub dwtp: bool,

    /// Whether natural scroll is enabled.
    pub natural_scroll: bool,

    /// Whether left-handed mode is enabled.
    pub left_handed: bool,

    /// Whether middle button emulation is enabled.
    pub middle_emulation: bool,

    /// Whether drag lock is enabled.
    pub drag_lock: bool,

    /// Whether touchpad is disabled when external mouse is connected.
    pub disabled_on_external_mouse: bool,

    /// Acceleration speed (-1.0 to 1.0).
    pub accel_speed: FloatOrInt<-1, 1>,

    /// Scroll method configuration.
    ///
    /// Valid values: "no-scroll", "two-finger", "edge", "on-button-down", or nil.
    pub scroll_method: Option<ScrollMethod>,

    /// Click method configuration.
    ///
    /// Valid values: "button-areas", "clickfinger", or nil.
    pub click_method: Option<ClickMethod>,

    /// Tap button map configuration.
    ///
    /// Valid values: "left-right-middle", "left-middle-right", or nil.
    pub tap_button_map: Option<TapButtonMap>,

    /// Acceleration profile configuration.
    ///
    /// Valid values: "adaptive", "flat", or nil.
    pub accel_profile: Option<AccelProfile>,
}

/// Proxy for input configuration.
///
/// This proxy provides access to all input device settings, including keyboard,
/// touchpad, mouse, trackpoint, and touch configurations.
///
/// ## Lua Usage
///
/// ```lua
/// -- Top-level input settings
/// config.input.disable_power_key_handling = true
/// config.input.workspace_auto_back_and_forth = false
///
/// -- Nested device configurations
/// config.input.keyboard.repeat_rate = 25
/// config.input.touchpad.natural_scroll = true
/// config.input.mouse.accel_speed = 0.5
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(crate = "crate", parent_path = "input", dirty = "Input", auto_detect)]
pub struct InputConfig {
    /// Whether to disable power key handling.
    pub disable_power_key_handling: bool,

    /// Whether to enable workspace auto back-and-forth.
    pub workspace_auto_back_and_forth: bool,

    /// Keyboard configuration.
    pub keyboard: KeyboardConfig,

    /// Touchpad configuration.
    pub touchpad: TouchpadConfig,

    /// Mouse configuration.
    pub mouse: MouseConfig,

    /// Trackpoint configuration.
    pub trackpoint: TrackpointConfig,

    /// Touch configuration.
    pub touch: TouchConfig,
}

// =============================================================================
// Group 4: Gesture leaves (already correctly ordered BEFORE GesturesConfig)
// =============================================================================

/// Proxy for gesture DnD edge view scroll configuration.
///
/// Controls scroll behavior when dragging windows to screen edges.
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "gestures.dnd_edge_view_scroll",
    dirty = "Gestures",
    auto_detect
)]
pub struct DndEdgeViewScrollConfig {
    /// Width of the trigger zone at the edge.
    pub trigger_width: f64,

    /// Delay in milliseconds before scroll starts.
    pub delay_ms: u16,

    /// Maximum scroll speed.
    pub max_speed: f64,
}

/// Proxy for gesture DnD edge workspace switch configuration.
///
/// Controls workspace switching behavior when dragging windows to screen edges.
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "gestures.dnd_edge_workspace_switch",
    dirty = "Gestures",
    auto_detect
)]
pub struct DndEdgeWorkspaceSwitchConfig {
    /// Height of the trigger zone at the edge.
    pub trigger_height: f64,

    /// Delay in milliseconds before switch starts.
    pub delay_ms: u16,

    /// Maximum speed of workspace switching.
    pub max_speed: f64,
}

/// Proxy for hot corners configuration.
///
/// Controls hot corner behavior at screen edges.
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "gestures.hot_corners",
    dirty = "Gestures",
    auto_detect
)]
pub struct HotCornersConfig {
    /// Whether hot corners are disabled.
    pub off: bool,

    /// Whether top-left corner is active.
    pub top_left: bool,

    /// Whether top-right corner is active.
    pub top_right: bool,

    /// Whether bottom-left corner is active.
    pub bottom_left: bool,

    /// Whether bottom-right corner is active.
    pub bottom_right: bool,
}

/// Proxy for gestures configuration.
///
/// Controls gesture-related behavior including DnD edge scrolling,
/// workspace switching, and hot corners.
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "gestures",
    dirty = "Gestures",
    auto_detect
)]
pub struct GesturesConfig {
    /// DnD edge view scroll configuration.
    pub dnd_edge_view_scroll: DndEdgeViewScrollConfig,

    /// DnD edge workspace switch configuration.
    pub dnd_edge_workspace_switch: DndEdgeWorkspaceSwitchConfig,

    /// Hot corners configuration.
    pub hot_corners: HotCornersConfig,
}

// =============================================================================
// Group 5: MRU leaves (BEFORE RecentWindowsConfig)
// =============================================================================

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
    dirty = "RecentWindows",
    auto_detect
)]
pub struct MruPreviewsConfig {
    /// Maximum height of preview thumbnails.
    pub max_height: f64,

    /// Maximum scale factor for previews.
    pub max_scale: f64,
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
    dirty = "RecentWindows",
    auto_detect
)]
pub struct MruHighlightConfig {
    /// Color of highlight for active windows.
    pub active_color: Color,

    /// Color of highlight for urgent windows.
    pub urgent_color: Color,

    /// Padding around the highlight.
    pub padding: f64,

    /// Corner radius of the highlight.
    pub corner_radius: f64,
}

/// Proxy for recent windows configuration.
///
/// Controls the MRU (Most Recently Used) window switcher settings, including
/// whether it's enabled, delays, highlighting, and previews.
///
/// ## Lua Usage
///
/// ```lua
/// -- Disable recent windows
/// config.recent_windows.off = true
///
/// -- Enable recent windows
/// config.recent_windows.on = true
///
/// -- Set open delay
/// config.recent_windows.open_delay_ms = 200
///
/// -- Configure highlight
/// config.recent_windows.highlight.active_color = "#7fc8ff"
///
/// -- Configure previews
/// config.recent_windows.previews.max_height = 200.0
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "recent_windows",
    dirty = "RecentWindows",
    auto_detect
)]
pub struct RecentWindowsConfig {
    /// Whether recent windows is disabled (inverted from underlying `on` field).
    #[lua_proxy(inverted, name = "off", path = "on")]
    pub off: bool,

    /// Whether recent windows is enabled.
    #[lua_proxy(field, path = "on")]
    pub on: bool,

    /// Delay before opening in milliseconds.
    pub open_delay_ms: u16,

    /// Highlight configuration.
    pub highlight: MruHighlightConfig,

    /// Previews configuration.
    pub previews: MruPreviewsConfig,
}

// =============================================================================
// Group 6: Layout leaves (BEFORE LayoutConfig)
// =============================================================================

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
#[lua_proxy(
    crate = "crate",
    parent_path = "layout.insert_hint",
    dirty = "Layout",
    auto_detect
)]
pub struct InsertHintConfig {
    /// Whether the insert hint is disabled.
    pub off: bool,

    /// Color of the insert hint.
    pub color: Color,
}

/// Proxy for tab indicator configuration.
///
/// Controls how tab indicators are displayed for windows in a column with
/// multiple tabs (windows stacked as tabs).
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "layout.tab_indicator",
    dirty = "Layout",
    auto_detect
)]
pub struct TabIndicatorConfig {
    /// Whether tab indicator is off (disabled).
    pub off: bool,

    /// Hide the indicator when there's only a single tab.
    pub hide_when_single_tab: bool,

    /// Place the indicator within the column's visual bounds.
    pub place_within_column: bool,

    /// Gap between the tab indicator and the window edge.
    pub gap: f64,

    /// Width of the tab indicator in logical pixels.
    pub width: f64,

    /// Gaps between individual tab indicators.
    pub gaps_between_tabs: f64,

    /// Corner radius of the tab indicators.
    pub corner_radius: f64,

    /// Position of the tab indicator.
    ///
    /// Valid values: "left", "right", "top", "bottom".
    pub position: TabIndicatorPosition,

    /// Color for the active tab indicator.
    ///
    /// Use hex string like "#rrggbbaa" or nil to reset to default.
    pub active_color: Option<Color>,

    /// Color for inactive tab indicators.
    ///
    /// Use hex string like "#rrggbbaa" or nil to reset to default.
    pub inactive_color: Option<Color>,

    /// Color for urgent tab indicators.
    ///
    /// Use hex string like "#rrggbbaa" or nil to reset to default.
    pub urgent_color: Option<Color>,
}

/// Proxy for focus ring configuration.
///
/// Controls the visual styling of focus indicators around windows.
///
/// ## Lua Usage
///
/// ```lua
/// -- Disable focus ring
/// config.layout.focus_ring.off = true
///
/// -- Set focus ring width
/// config.layout.focus_ring.width = 4.0
///
/// -- Set colors for different states
/// config.layout.focus_ring.active_color = "#7fc8ff"
/// config.layout.focus_ring.inactive_color = "#505050"
/// config.layout.focus_ring.urgent_color = "#9b0000"
///
/// -- Set gradients (optional)
/// config.layout.focus_ring.active_gradient = {
///     from = "#ff0000",
///     to = "#00ff00",
///     angle = 45,
///     relative_to = "workspace-view"
/// }
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "layout.focus_ring",
    dirty = "Layout",
    auto_detect
)]
pub struct FocusRingConfig {
    /// Whether the focus ring is disabled.
    pub off: bool,

    /// Width of the focus ring in logical pixels.
    pub width: f64,

    /// Color of the focus ring for the active (focused) window.
    pub active_color: Color,

    /// Color of the focus ring for inactive windows.
    pub inactive_color: Color,

    /// Color of the focus ring for urgent windows.
    pub urgent_color: Color,

    /// Optional gradient for the active window's focus ring.
    #[lua_proxy(gradient)]
    pub active_gradient: Option<Gradient>,

    /// Optional gradient for inactive windows' focus ring.
    #[lua_proxy(gradient)]
    pub inactive_gradient: Option<Gradient>,

    /// Optional gradient for urgent windows' focus ring.
    #[lua_proxy(gradient)]
    pub urgent_gradient: Option<Gradient>,
}

/// Proxy for border configuration.
///
/// Controls window border styling including width, colors, and gradients.
///
/// ## Lua Usage
///
/// ```lua
/// -- Disable borders
/// config.layout.border.off = true
///
/// -- Set border width
/// config.layout.border.width = 4.0
///
/// -- Set colors for different states
/// config.layout.border.active_color = "#7fc8ff"
/// config.layout.border.inactive_color = "#505050"
/// config.layout.border.urgent_color = "#9b0000"
///
/// -- Set gradients (optional)
/// config.layout.border.active_gradient = {
///     from = "#ff0000",
///     to = "#00ff00",
///     angle = 45,
///     relative_to = "workspace-view"
/// }
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "layout.border",
    dirty = "Layout",
    auto_detect
)]
pub struct BorderConfig {
    /// Whether borders are disabled.
    pub off: bool,

    /// Border width in logical pixels.
    pub width: f64,

    /// Color for active window borders.
    pub active_color: Color,

    /// Color for inactive window borders.
    pub inactive_color: Color,

    /// Color for urgent window borders.
    pub urgent_color: Color,

    /// Gradient for active window borders.
    #[lua_proxy(gradient)]
    pub active_gradient: Option<Gradient>,

    /// Gradient for inactive window borders.
    #[lua_proxy(gradient)]
    pub inactive_gradient: Option<Gradient>,

    /// Gradient for urgent window borders.
    #[lua_proxy(gradient)]
    pub urgent_gradient: Option<Gradient>,
}

/// Proxy for shadow configuration.
///
/// Controls window shadow styling including softness, spread, offset and colors.
///
/// ## Lua Usage
///
/// ```lua
/// -- Enable shadows
/// config.layout.shadow.on = true
///
/// -- Set shadow properties
/// config.layout.shadow.softness = 30.0
/// config.layout.shadow.spread = 5.0
///
/// -- Set shadow offset
/// config.layout.shadow.offset = {x = 0, y = 5}
///
/// -- Set colors
/// config.layout.shadow.color = "#00000077"
/// config.layout.shadow.inactive_color = "#00000050"
///
/// -- Draw shadow behind window
/// config.layout.shadow.draw_behind_window = false
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "layout.shadow",
    dirty = "Layout",
    auto_detect
)]
pub struct ShadowConfig {
    /// Whether shadows are enabled.
    pub on: bool,

    /// Shadow softness (blur radius).
    pub softness: f64,

    /// Shadow spread distance.
    pub spread: f64,

    /// Whether to draw shadow behind the window content.
    pub draw_behind_window: bool,

    /// Shadow offset as {x, y} table.
    #[lua_proxy(offset)]
    pub offset: ShadowOffset,

    /// Shadow color (hex string).
    pub color: Color,

    /// Optional inactive shadow color.
    pub inactive_color: Option<Color>,
}

/// Proxy for layout configuration.
///
/// Controls overall window layout behavior including gaps, centering, and nested configurations
/// for focus rings, borders, shadows, and other visual elements.
///
/// ## Lua Usage
///
/// ```lua
/// -- Set workspace gaps
/// config.layout.gaps = 16
///
/// -- Center single column windows
/// config.layout.always_center_single_column = true
///
/// -- Add empty workspace above the first
/// config.layout.empty_workspace_above_first = true
///
/// -- Control focused column centering behavior
/// config.layout.center_focused_column = "on-overflow"  -- "never", "always", "on-overflow"
///
/// -- Set default column display mode
/// config.layout.default_column_display = "normal"  -- "normal", "tabbed"
///
/// -- Set background color
/// config.layout.background_color = "#181818"
///
/// -- Access nested configurations
/// config.layout.focus_ring.width = 4.0
/// config.layout.border.width = 2.0
/// config.layout.shadow.on = true
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(crate = "crate", parent_path = "layout", dirty = "Layout", auto_detect)]
pub struct LayoutConfig {
    /// Gap between windows in logical pixels.
    pub gaps: f64,

    /// Whether to center windows when there's only a single column.
    pub always_center_single_column: bool,

    /// Whether to add an empty workspace above the first workspace.
    pub empty_workspace_above_first: bool,

    /// When to center the focused column.
    ///
    /// Values: "never", "always", "on-overflow"
    pub center_focused_column: CenterFocusedColumn,

    /// Default column display mode for new windows.
    ///
    /// Values: "normal", "tabbed"
    pub default_column_display: ColumnDisplay,

    /// Background color of the workspace.
    pub background_color: Color,

    /// Focus ring configuration.
    pub focus_ring: FocusRingConfig,

    /// Border configuration.
    pub border: BorderConfig,

    /// Shadow configuration.
    pub shadow: ShadowConfig,

    /// Struts configuration.
    pub struts: StrutsConfig,

    /// Tab indicator configuration.
    pub tab_indicator: TabIndicatorConfig,

    /// Insert hint configuration.
    pub insert_hint: InsertHintConfig,
}

// =============================================================================
// Group 7: Overview leaves (BEFORE OverviewConfig)
// =============================================================================

/// Proxy for overview workspace shadow configuration.
///
/// Controls shadow styling for workspaces in the overview mode.
///
/// ## Lua Usage
///
/// ```lua
/// -- Disable workspace shadow in overview
/// config.overview.workspace_shadow.off = true
///
/// -- Set shadow properties
/// config.overview.workspace_shadow.softness = 40.0
/// config.overview.workspace_shadow.spread = 10.0
///
/// -- Set shadow offset
/// config.overview.workspace_shadow.offset = {x = 0, y = 10}
///
/// -- Set shadow color
/// config.overview.workspace_shadow.color = "#00000050"
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "overview.workspace_shadow",
    dirty = "Overview",
    auto_detect
)]
pub struct OverviewWorkspaceShadowConfig {
    /// Whether workspace shadow is disabled.
    pub off: bool,

    /// Shadow softness (blur radius).
    pub softness: f64,

    /// Shadow spread distance.
    pub spread: f64,

    /// Shadow offset as {x, y} table.
    #[lua_proxy(offset)]
    pub offset: ShadowOffset,

    /// Shadow color (hex string).
    pub color: Color,
}

/// Proxy for overview configuration.
///
/// Controls the overview mode settings including zoom level and backdrop color.
///
/// ## Lua Usage
///
/// ```lua
/// -- Set overview zoom level
/// config.overview.zoom = 0.5
///
/// -- Set backdrop color
/// config.overview.backdrop_color = "#00000080"
///
/// -- Configure workspace shadow
/// config.overview.workspace_shadow.off = false
/// config.overview.workspace_shadow.softness = 40.0
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "overview",
    dirty = "Overview",
    auto_detect
)]
pub struct OverviewConfig {
    /// Overview zoom level (0.0 to 1.0).
    pub zoom: f64,

    /// Backdrop color in overview mode.
    pub backdrop_color: Color,

    /// Workspace shadow configuration.
    pub workspace_shadow: OverviewWorkspaceShadowConfig,
}

// =============================================================================
// Group 8: Animation leaves (BEFORE AnimationsConfig)
// =============================================================================

/// Proxy for workspace switch animation.
///
/// Controls the animation when switching between workspaces.
///
/// ## Lua Usage
///
/// ```lua
/// -- Disable workspace switch animation
/// config.animations.workspace_switch.off = true
///
/// -- Configure spring animation
/// config.animations.workspace_switch.kind = {
///     spring = {
///         damping_ratio = 1.0,
///         stiffness = 800,
///         epsilon = 0.0001
///     }
/// }
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "animations.workspace_switch.0",
    dirty = "Animations",
    auto_detect
)]
pub struct WorkspaceSwitchAnimConfig {
    /// Whether this animation is disabled.
    pub off: bool,

    /// Animation curve parameters (spring or easing).
    #[lua_proxy(anim_kind)]
    pub kind: Kind,
}

/// Proxy for horizontal view movement animation.
///
/// Controls the animation when scrolling horizontally through columns.
///
/// ## Lua Usage
///
/// ```lua
/// -- Disable horizontal view animation
/// config.animations.horizontal_view_movement.off = true
///
/// -- Configure spring animation
/// config.animations.horizontal_view_movement.kind = {
///     spring = {
///         damping_ratio = 1.0,
///         stiffness = 800,
///         epsilon = 0.0001
///     }
/// }
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "animations.horizontal_view_movement.0",
    dirty = "Animations",
    auto_detect
)]
pub struct HorizontalViewMovementAnimConfig {
    /// Whether this animation is disabled.
    pub off: bool,

    /// Animation curve parameters (spring or easing).
    #[lua_proxy(anim_kind)]
    pub kind: Kind,
}

/// Proxy for window movement animation.
///
/// Controls the animation when windows move or resize within columns.
///
/// ## Lua Usage
///
/// ```lua
/// -- Disable window movement animation
/// config.animations.window_movement.off = true
///
/// -- Configure spring animation
/// config.animations.window_movement.kind = {
///     spring = {
///         damping_ratio = 1.0,
///         stiffness = 800,
///         epsilon = 0.0001
///     }
/// }
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "animations.window_movement.0",
    dirty = "Animations",
    auto_detect
)]
pub struct WindowMovementAnimConfig {
    /// Whether this animation is disabled.
    pub off: bool,

    /// Animation curve parameters (spring or easing).
    #[lua_proxy(anim_kind)]
    pub kind: Kind,
}

/// Proxy for config notification animation.
///
/// Controls the open/close animation for the configuration notification popup.
///
/// ## Lua Usage
///
/// ```lua
/// -- Disable config notification animation
/// config.animations.config_notification_open_close.off = true
///
/// -- Configure easing animation
/// config.animations.config_notification_open_close.kind = {
///     easing = {
///         duration_ms = 250,
///         curve = "ease-out-cubic"
///     }
/// }
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "animations.config_notification_open_close.0",
    dirty = "Animations",
    auto_detect
)]
pub struct ConfigNotificationAnimConfig {
    /// Whether this animation is disabled.
    pub off: bool,

    /// Animation curve parameters (spring or easing).
    #[lua_proxy(anim_kind)]
    pub kind: Kind,
}

/// Proxy for overview animation.
///
/// Controls the open/close animation for overview mode.
///
/// ## Lua Usage
///
/// ```lua
/// -- Disable overview animation
/// config.animations.overview_open_close.off = true
///
/// -- Configure easing animation
/// config.animations.overview_open_close.kind = {
///     easing = {
///         duration_ms = 200,
///         curve = "ease-out-expo"
///     }
/// }
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "animations.overview_open_close.0",
    dirty = "Animations",
    auto_detect
)]
pub struct OverviewAnimConfig {
    /// Whether this animation is disabled.
    pub off: bool,

    /// Animation curve parameters (spring or easing).
    #[lua_proxy(anim_kind)]
    pub kind: Kind,
}

/// Proxy for screenshot UI animation.
///
/// Controls the open animation for the screenshot UI.
///
/// ## Lua Usage
///
/// ```lua
/// -- Disable screenshot UI animation
/// config.animations.screenshot_ui_open.off = true
///
/// -- Configure easing animation
/// config.animations.screenshot_ui_open.kind = {
///     easing = {
///         duration_ms = 200,
///         curve = "ease-out-quad"
///     }
/// }
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "animations.screenshot_ui_open.0",
    dirty = "Animations",
    auto_detect
)]
pub struct ScreenshotUiAnimConfig {
    /// Whether this animation is disabled.
    pub off: bool,

    /// Animation curve parameters (spring or easing).
    #[lua_proxy(anim_kind)]
    pub kind: Kind,
}

/// Proxy for window open animation.
///
/// Controls the animation when windows open, with optional custom shader.
///
/// ## Lua Usage
///
/// ```lua
/// -- Disable window open animation
/// config.animations.window_open.off = true
///
/// -- Configure spring animation
/// config.animations.window_open.kind = {
///     spring = {
///         damping_ratio = 0.8,
///         stiffness = 600,
///         epsilon = 0.0001
///     }
/// }
///
/// -- Set custom shader (GLSL)
/// config.animations.window_open.custom_shader = [[
///     // Custom GLSL shader code
/// ]]
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "animations.window_open.anim",
    dirty = "Animations",
    auto_detect
)]
pub struct WindowOpenAnimConfig {
    /// Whether this animation is disabled.
    pub off: bool,

    /// Animation curve parameters (spring or easing).
    #[lua_proxy(anim_kind)]
    pub kind: Kind,

    /// Optional custom GLSL shader for this animation.
    #[lua_proxy(field, path = "../custom_shader")]
    pub custom_shader: Option<String>,
}

/// Proxy for window close animation.
///
/// Controls the animation when windows close, with optional custom shader.
///
/// ## Lua Usage
///
/// ```lua
/// -- Disable window close animation
/// config.animations.window_close.off = true
///
/// -- Configure easing animation
/// config.animations.window_close.kind = {
///     easing = {
///         duration_ms = 150,
///         curve = "ease-in-quad"
///     }
/// }
///
/// -- Set custom shader (GLSL)
/// config.animations.window_close.custom_shader = [[
///     // Custom GLSL shader code
/// ]]
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "animations.window_close.anim",
    dirty = "Animations",
    auto_detect
)]
pub struct WindowCloseAnimConfig {
    /// Whether this animation is disabled.
    pub off: bool,

    /// Animation curve parameters (spring or easing).
    #[lua_proxy(anim_kind)]
    pub kind: Kind,

    /// Optional custom GLSL shader for this animation.
    #[lua_proxy(field, path = "../custom_shader")]
    pub custom_shader: Option<String>,
}

/// Proxy for window resize animation.
///
/// Controls the animation when windows resize, with optional custom shader.
///
/// ## Lua Usage
///
/// ```lua
/// -- Disable window resize animation
/// config.animations.window_resize.off = true
///
/// -- Configure spring animation
/// config.animations.window_resize.kind = {
///     spring = {
///         damping_ratio = 1.0,
///         stiffness = 800,
///         epsilon = 0.0001
///     }
/// }
///
/// -- Set custom shader (GLSL)
/// config.animations.window_resize.custom_shader = [[
///     // Custom GLSL shader code
/// ]]
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "animations.window_resize.anim",
    dirty = "Animations",
    auto_detect
)]
pub struct WindowResizeAnimConfig {
    /// Whether this animation is disabled.
    pub off: bool,

    /// Animation curve parameters (spring or easing).
    #[lua_proxy(anim_kind)]
    pub kind: Kind,

    /// Optional custom GLSL shader for this animation.
    #[lua_proxy(field, path = "../custom_shader")]
    pub custom_shader: Option<String>,
}

/// Proxy for animations configuration container.
///
/// Provides access to all animation settings including global controls
/// and individual animation configurations.
///
/// ## Lua Usage
///
/// ```lua
/// -- Disable all animations
/// config.animations.off = true
///
/// -- Slow down animations (2.0 = half speed)
/// config.animations.slowdown = 2.0
///
/// -- Configure workspace switch animation
/// config.animations.workspace_switch.off = false
/// config.animations.workspace_switch.kind = {
///     spring = { damping_ratio = 1.0, stiffness = 800, epsilon = 0.0001 }
/// }
///
/// -- Configure window open animation with custom shader
/// config.animations.window_open.off = false
/// config.animations.window_open.custom_shader = [[
///     // Custom GLSL shader
/// ]]
/// ```
#[derive(Clone, LuaConfigProxy)]
#[lua_proxy(
    crate = "crate",
    parent_path = "animations",
    dirty = "Animations",
    auto_detect
)]
pub struct AnimationsConfig {
    /// Whether all animations are disabled.
    pub off: bool,

    /// Animation slowdown factor (1.0 = normal speed).
    pub slowdown: f64,

    /// Workspace switch animation settings.
    pub workspace_switch: WorkspaceSwitchAnimConfig,

    /// Horizontal view movement animation settings.
    pub horizontal_view_movement: HorizontalViewMovementAnimConfig,

    /// Window movement animation settings.
    pub window_movement: WindowMovementAnimConfig,

    /// Config notification open/close animation settings.
    pub config_notification_open_close: ConfigNotificationAnimConfig,

    /// Overview open/close animation settings.
    pub overview_open_close: OverviewAnimConfig,

    /// Screenshot UI open animation settings.
    pub screenshot_ui_open: ScreenshotUiAnimConfig,

    /// Window open animation settings.
    pub window_open: WindowOpenAnimConfig,

    /// Window close animation settings.
    pub window_close: WindowCloseAnimConfig,

    /// Window resize animation settings.
    pub window_resize: WindowResizeAnimConfig,
}
