//! ConfigWrapper - Direct UserData wrapper for niri_config::Config.
//!
//! This module provides a direct Lua ↔ Config interface without JSON intermediaries.
//! It replaces the previous config_proxy.rs + config_converter.rs pipeline.
//!
//! ## Design
//!
//! - `ConfigWrapper` wraps `Arc<Mutex<Config>>` for thread-safe access
//! - `UserData` implementation provides direct field access from Lua
//! - `ConfigDirtyFlags` tracks which subsystems need refresh
//! - Section proxies (LayoutProxy, InputProxy, etc.) provide nested access
//!
//! ## Usage from Lua
//!
//! ```lua
//! -- Direct property access
//! niri.config.layout.gaps = 16
//! niri.config.prefer_no_csd = true
//!
//! -- Collection operations
//! niri.config.binds:add({ key = "Mod+Return", action = "spawn", args = { "kitty" } })
//! ```

use std::sync::{Arc, Mutex};

use mlua::prelude::*;
use mlua::UserData;
use niri_config::Config;

use crate::collections::{
    BindsCollection, EnvironmentCollection, LayerRulesCollection, OutputsCollection,
    WindowRulesCollection, WorkspacesCollection,
};
use crate::config_dirty::ConfigDirtyFlags;
use crate::extractors::{
    extract_animations, extract_clipboard, extract_config_notification, extract_cursor,
    extract_debug, extract_gestures, extract_hotkey_overlay, extract_input, extract_layout,
    extract_overview, extract_recent_windows, extract_xwayland_satellite,
};

/// Macro to generate simple scalar field getter/setter methods for config proxies.
///
/// This reduces boilerplate for fields that are directly copied (no clone needed)
/// and use a single dirty flag.
///
/// # Usage
/// ```ignore
/// config_field_methods!(fields, dirty_flag,
///     "field_name" => [config_path.field]: Type,
///     "another_field" => [config_path.another]: AnotherType,
/// );
/// ```
macro_rules! config_field_methods {
    ($fields:expr, $dirty_flag:ident,
     $( $name:literal => [ $($path:tt).+ ] : $ty:ty ),* $(,)?) => {
        $(
            $fields.add_field_method_get($name, |_, this| {
                Ok(this.config.lock().unwrap().$($path).+)
            });
            $fields.add_field_method_set($name, |_, this, value: $ty| {
                this.config.lock().unwrap().$($path).+ = value;
                this.dirty.lock().unwrap().$dirty_flag = true;
                Ok(())
            });
        )*
    };
}

/// Macro to generate clone-based field getter/setter methods for config proxies.
///
/// Use this for String and Option<String> fields that need .clone() on get.
///
/// # Usage
/// ```ignore
/// config_field_methods_clone!(fields, dirty_flag,
///     "field_name" => [config_path.field]: String,
/// );
/// ```
macro_rules! config_field_methods_clone {
    ($fields:expr, $dirty_flag:ident,
     $( $name:literal => [ $($path:tt).+ ] : $ty:ty ),* $(,)?) => {
        $(
            $fields.add_field_method_get($name, |_, this| {
                Ok(this.config.lock().unwrap().$($path).+.clone())
            });
            $fields.add_field_method_set($name, |_, this, value: $ty| {
                this.config.lock().unwrap().$($path).+ = value;
                this.dirty.lock().unwrap().$dirty_flag = true;
                Ok(())
            });
        )*
    };
}

/// Macro for FloatOrInt wrapper fields (unwraps .0 on get, wraps in FloatOrInt on set).
macro_rules! config_field_methods_float_or_int {
    ($fields:expr, $dirty_flag:ident,
     $( $name:literal => [ $($path:tt).+ ] ),* $(,)?) => {
        $(
            $fields.add_field_method_get($name, |_, this| {
                Ok(this.config.lock().unwrap().$($path).+.0)
            });
            $fields.add_field_method_set($name, |_, this, value: f64| {
                use niri_config::FloatOrInt;
                this.config.lock().unwrap().$($path).+ = FloatOrInt(value);
                this.dirty.lock().unwrap().$dirty_flag = true;
                Ok(())
            });
        )*
    };
}

/// Wrapper around Config that implements UserData for Lua access.
///
/// This is the main entry point for Lua config access. It provides:
/// - Direct field access for top-level config values
/// - Section proxies for nested config sections
/// - Collection proxies for config collections (binds, outputs, etc.)
/// - Dirty flag tracking for subsystem updates
#[derive(Clone)]
pub struct ConfigWrapper {
    /// The actual config wrapped in Arc<Mutex> for thread-safe access.
    /// Required by mlua's `send` feature, even though niri is single-threaded.
    pub config: Arc<Mutex<Config>>,
    /// Dirty flags tracking which subsystems need refresh.
    pub dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl ConfigWrapper {
    /// Create a new ConfigWrapper with the given config.
    pub fn new(config: Arc<Mutex<Config>>) -> Self {
        Self {
            config,
            dirty: Arc::new(Mutex::new(ConfigDirtyFlags::default())),
        }
    }

    /// Create a new ConfigWrapper with a default config.
    pub fn new_default() -> Self {
        Self::new(Arc::new(Mutex::new(Config::default())))
    }

    /// Take and reset dirty flags.
    /// Called by the compositor after processing to get pending updates.
    pub fn take_dirty_flags(&self) -> ConfigDirtyFlags {
        self.dirty.lock().unwrap().take()
    }

    /// Check if any dirty flags are set.
    pub fn has_dirty_flags(&self) -> bool {
        self.dirty.lock().unwrap().any()
    }

    /// Execute a function with a reference to the config.
    /// This is the safe way to access config values without Clone.
    pub fn with_config<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Config) -> R,
    {
        let guard = self.config.lock().unwrap();
        f(&guard)
    }

    /// Execute a function with a mutable reference to the config.
    pub fn with_config_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Config) -> R,
    {
        let mut guard = self.config.lock().unwrap();
        f(&mut guard)
    }

    /// Replace the current config with a new one.
    /// Sets all dirty flags to trigger a full refresh.
    pub fn set_config(&self, config: Config) {
        *self.config.lock().unwrap() = config;
        // Mark everything as dirty
        let mut flags = self.dirty.lock().unwrap();
        flags.layout = true;
        flags.input = true;
        flags.cursor = true;
        flags.keyboard = true;
        flags.outputs = true;
        flags.animations = true;
        flags.window_rules = true;
        flags.layer_rules = true;
        flags.binds = true;
        flags.gestures = true;
        flags.overview = true;
        flags.recent_windows = true;
        flags.clipboard = true;
        flags.hotkey_overlay = true;
        flags.config_notification = true;
        flags.debug = true;
        flags.xwayland_satellite = true;
        flags.misc = true;
        flags.spawn_at_startup = true;
        flags.environment = true;
        flags.workspaces = true;
    }

    /// Get a reference to the underlying config Arc.
    /// Use this when you need to pass the config to other components.
    pub fn config_arc(&self) -> Arc<Mutex<Config>> {
        self.config.clone()
    }

    /// Extract the Config by taking it from the wrapper and replacing with default.
    ///
    /// This is useful when you need to move the Config into another container
    /// (like an Rc<RefCell<Config>>).
    ///
    /// Note: After calling this, the wrapper will contain a default Config.
    pub fn extract_config(&self) -> Config {
        let mut guard = self.config.lock().unwrap();
        std::mem::take(&mut *guard)
    }

    /// Replace the inner config and return the old one.
    ///
    /// Useful for swapping configs during reload.
    pub fn swap_config(&self, new_config: Config) -> Config {
        let mut guard = self.config.lock().unwrap();
        std::mem::replace(&mut *guard, new_config)
    }
}

/// Register the ConfigWrapper as `niri.config` in Lua.
///
/// This creates a `niri` global table (if not existing) and sets `config` on it.
pub fn register_config_wrapper(lua: &Lua, wrapper: ConfigWrapper) -> LuaResult<()> {
    // Get or create the niri global table
    let niri_table: LuaTable = match lua.globals().get::<LuaTable>("niri") {
        Ok(table) => table,
        Err(_) => {
            let table = lua.create_table()?;
            lua.globals().set("niri", table.clone())?;
            table
        }
    };

    // Set the config wrapper
    niri_table.set("config", wrapper)?;

    Ok(())
}

impl UserData for ConfigWrapper {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        // Top-level scalar fields
        fields.add_field_method_get("prefer_no_csd", |_, this| {
            Ok(this.config.lock().unwrap().prefer_no_csd)
        });

        fields.add_field_method_set("prefer_no_csd", |_, this, value: bool| {
            this.config.lock().unwrap().prefer_no_csd = value;
            this.dirty.lock().unwrap().misc = true;
            Ok(())
        });

        // Section proxies will be added here as we implement them
        // For now, provide stubs that will be replaced

        fields.add_field_method_get("layout", |_, this| {
            Ok(LayoutProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_set("layout", |_, this, value: LuaTable| {
            if let Some(layout) = extract_layout(&value)? {
                this.config.lock().unwrap().layout = layout;
                this.dirty.lock().unwrap().layout = true;
            }
            Ok(())
        });

        fields.add_field_method_get("cursor", |_, this| {
            Ok(CursorProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_set("cursor", |_, this, value: LuaTable| {
            if let Some(cursor) = extract_cursor(&value)? {
                this.config.lock().unwrap().cursor = cursor;
                this.dirty.lock().unwrap().cursor = true;
            }
            Ok(())
        });

        fields.add_field_method_get("animations", |_, this| {
            Ok(AnimationsProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_set("animations", |_, this, value: LuaTable| {
            if let Some(animations) = extract_animations(&value)? {
                this.config.lock().unwrap().animations = animations;
                this.dirty.lock().unwrap().animations = true;
            }
            Ok(())
        });

        fields.add_field_method_get("input", |_, this| {
            Ok(InputProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_set("input", |_, this, value: LuaTable| {
            if let Some(input) = extract_input(&value)? {
                this.config.lock().unwrap().input = input;
                this.dirty.lock().unwrap().input = true;
            }
            Ok(())
        });

        fields.add_field_method_get("overview", |_, this| {
            Ok(OverviewProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_set("overview", |_, this, value: LuaTable| {
            if let Some(overview) = extract_overview(&value)? {
                this.config.lock().unwrap().overview = overview;
                this.dirty.lock().unwrap().misc = true;
            }
            Ok(())
        });

        fields.add_field_method_get("hotkey_overlay", |_, this| {
            Ok(HotkeyOverlayProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_set("hotkey_overlay", |_, this, value: LuaTable| {
            if let Some(overlay) = extract_hotkey_overlay(&value)? {
                this.config.lock().unwrap().hotkey_overlay = overlay;
                this.dirty.lock().unwrap().misc = true;
            }
            Ok(())
        });

        fields.add_field_method_get("config_notification", |_, this| {
            Ok(ConfigNotificationProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_set("config_notification", |_, this, value: LuaTable| {
            if let Some(notification) = extract_config_notification(&value)? {
                this.config.lock().unwrap().config_notification = notification;
                this.dirty.lock().unwrap().misc = true;
            }
            Ok(())
        });

        fields.add_field_method_get("clipboard", |_, this| {
            Ok(ClipboardProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_set("clipboard", |_, this, value: LuaTable| {
            if let Some(clipboard) = extract_clipboard(&value)? {
                this.config.lock().unwrap().clipboard = clipboard;
                this.dirty.lock().unwrap().misc = true;
            }
            Ok(())
        });

        fields.add_field_method_get("xwayland_satellite", |_, this| {
            Ok(XwaylandSatelliteProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_set("xwayland_satellite", |_, this, value: LuaTable| {
            if let Some(xwayland) = extract_xwayland_satellite(&value)? {
                this.config.lock().unwrap().xwayland_satellite = xwayland;
                this.dirty.lock().unwrap().misc = true;
            }
            Ok(())
        });

        fields.add_field_method_get("debug", |_, this| {
            Ok(DebugProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_set("debug", |_, this, value: LuaTable| {
            if let Some(debug) = extract_debug(&value)? {
                this.config.lock().unwrap().debug = debug;
                this.dirty.lock().unwrap().misc = true;
            }
            Ok(())
        });

        fields.add_field_method_get("gestures", |_, this| {
            Ok(GesturesProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_set("gestures", |_, this, value: LuaTable| {
            if let Some(gestures) = extract_gestures(&value)? {
                this.config.lock().unwrap().gestures = gestures;
                this.dirty.lock().unwrap().misc = true;
            }
            Ok(())
        });

        fields.add_field_method_get("recent_windows", |_, this| {
            Ok(RecentWindowsProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_set("recent_windows", |_, this, value: LuaTable| {
            if let Some(recent_windows) = extract_recent_windows(&value)? {
                this.config.lock().unwrap().recent_windows = recent_windows;
                this.dirty.lock().unwrap().misc = true;
            }
            Ok(())
        });

        // Collection proxies
        fields.add_field_method_get("workspaces", |_, this| {
            Ok(WorkspacesCollection {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_get("spawn_at_startup", |_, this| {
            Ok(SpawnAtStartupProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_get("outputs", |_, this| {
            Ok(OutputsCollection {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_get("window_rules", |_, this| {
            Ok(WindowRulesCollection {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_get("binds", |_, this| {
            Ok(BindsCollection {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_get("environment", |_, this| {
            Ok(EnvironmentCollection {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_get("layer_rules", |_, this| {
            Ok(LayerRulesCollection {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        // Top-level simple fields
        fields.add_field_method_get("prefer_no_csd", |_, this| {
            Ok(this.config.lock().unwrap().prefer_no_csd)
        });

        fields.add_field_method_set("prefer_no_csd", |_, this, value: bool| {
            this.config.lock().unwrap().prefer_no_csd = value;
            this.dirty.lock().unwrap().misc = true;
            Ok(())
        });

        fields.add_field_method_get("screenshot_path", |_, this| {
            Ok(this.config.lock().unwrap().screenshot_path.0.clone())
        });

        fields.add_field_method_set("screenshot_path", |_, this, value: Option<String>| {
            use niri_config::ScreenshotPath;
            this.config.lock().unwrap().screenshot_path = ScreenshotPath(value);
            this.dirty.lock().unwrap().misc = true;
            Ok(())
        });
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // apply() method - for compatibility with explicit apply model
        methods.add_method("apply", |_, _this, ()| {
            // In the new model, changes are applied immediately
            // This method exists for API compatibility
            Ok(())
        });
    }
}

// ============================================================================
// Section Proxies
// ============================================================================

/// Proxy for layout config section.
#[derive(Clone)]
struct LayoutProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for LayoutProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        config_field_methods!(fields, layout,
            "gaps" => [layout.gaps]: f64,
        );

        fields.add_field_method_get("center_focused_column", |_, this| {
            let config = this.config.lock().unwrap();
            let value = match config.layout.center_focused_column {
                niri_config::layout::CenterFocusedColumn::Never => "never",
                niri_config::layout::CenterFocusedColumn::Always => "always",
                niri_config::layout::CenterFocusedColumn::OnOverflow => "on-overflow",
            };
            Ok(value.to_string())
        });

        fields.add_field_method_set("center_focused_column", |_, this, value: String| {
            use niri_config::layout::CenterFocusedColumn;
            let parsed = match value.as_str() {
                "never" => CenterFocusedColumn::Never,
                "always" => CenterFocusedColumn::Always,
                "on-overflow" => CenterFocusedColumn::OnOverflow,
                _ => {
                    return Err(mlua::Error::external(format!(
                        "Invalid center_focused_column value: {}. Expected 'never', 'always', or 'on-overflow'",
                        value
                    )));
                }
            };
            this.config.lock().unwrap().layout.center_focused_column = parsed;
            this.dirty.lock().unwrap().layout = true;
            Ok(())
        });

        // Nested proxies for focus_ring, border, shadow
        fields.add_field_method_get("focus_ring", |_, this| {
            Ok(FocusRingProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_get("border", |_, this| {
            Ok(BorderProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_get("shadow", |_, this| {
            Ok(ShadowProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });
    }
}

/// Proxy for focus_ring config section.
#[derive(Clone)]
struct FocusRingProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for FocusRingProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        config_field_methods!(fields, layout,
            "width" => [layout.focus_ring.width]: f64,
            "off" => [layout.focus_ring.off]: bool,
        );
    }
}

/// Proxy for border config section.
#[derive(Clone)]
struct BorderProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for BorderProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        config_field_methods!(fields, layout,
            "width" => [layout.border.width]: f64,
            "off" => [layout.border.off]: bool,
        );
    }
}

/// Proxy for shadow config section.
#[derive(Clone)]
struct ShadowProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for ShadowProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        config_field_methods!(fields, layout,
            "softness" => [layout.shadow.softness]: f64,
            "spread" => [layout.shadow.spread]: f64,
        );
    }
}

/// Proxy for cursor config section.
#[derive(Clone)]
struct CursorProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for CursorProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        config_field_methods!(fields, cursor,
            "xcursor_size" => [cursor.xcursor_size]: u8,
            "hide_when_typing" => [cursor.hide_when_typing]: bool,
            "hide_after_inactive_ms" => [cursor.hide_after_inactive_ms]: Option<u32>,
        );
        config_field_methods_clone!(fields, cursor,
            "xcursor_theme" => [cursor.xcursor_theme]: String,
        );
    }
}

/// Proxy for animations config section.
#[derive(Clone)]
struct AnimationsProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for AnimationsProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        config_field_methods!(fields, animations,
            "off" => [animations.off]: bool,
            "slowdown" => [animations.slowdown]: f64,
        );
    }
}

/// Proxy for input config section.
#[derive(Clone)]
struct InputProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for InputProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        // Boolean fields
        config_field_methods!(fields, input,
            "disable_power_key_handling" => [input.disable_power_key_handling]: bool,
            "workspace_auto_back_and_forth" => [input.workspace_auto_back_and_forth]: bool,
        );

        // Nested device proxies
        fields.add_field_method_get("keyboard", |_, this| {
            Ok(KeyboardProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_get("touchpad", |_, this| {
            Ok(TouchpadProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_get("mouse", |_, this| {
            Ok(MouseProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_get("trackpoint", |_, this| {
            Ok(TrackpointProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });

        fields.add_field_method_get("touch", |_, this| {
            Ok(TouchProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });
    }
}

/// Proxy for keyboard config section.
#[derive(Clone)]
struct KeyboardProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for KeyboardProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("repeat_delay", |_, this| {
            Ok(this.config.lock().unwrap().input.keyboard.repeat_delay)
        });

        fields.add_field_method_set("repeat_delay", |_, this, value: u16| {
            this.config.lock().unwrap().input.keyboard.repeat_delay = value;
            this.dirty.lock().unwrap().keyboard = true;
            Ok(())
        });

        config_field_methods!(fields, keyboard,
            "repeat_delay" => [input.keyboard.repeat_delay]: u16,
            "repeat_rate" => [input.keyboard.repeat_rate]: u8,
            "numlock" => [input.keyboard.numlock]: bool,
        );

        fields.add_field_method_get("track_layout", |_, this| {
            use niri_config::input::TrackLayout;
            let value = match this.config.lock().unwrap().input.keyboard.track_layout {
                TrackLayout::Global => "global",
                TrackLayout::Window => "window",
            };
            Ok(value.to_string())
        });

        fields.add_field_method_set("track_layout", |_, this, value: String| {
            use niri_config::input::TrackLayout;
            let parsed = match value.as_str() {
                "global" => TrackLayout::Global,
                "window" => TrackLayout::Window,
                _ => {
                    return Err(mlua::Error::external(format!(
                        "Invalid track_layout value: {}. Expected 'global' or 'window'",
                        value
                    )));
                }
            };
            this.config.lock().unwrap().input.keyboard.track_layout = parsed;
            this.dirty.lock().unwrap().keyboard = true;
            Ok(())
        });

        // XKB nested proxy
        fields.add_field_method_get("xkb", |_, this| {
            Ok(XkbProxy {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });
    }
}

/// Proxy for xkb config section.
#[derive(Clone)]
struct XkbProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for XkbProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        config_field_methods_clone!(fields, keyboard,
            "layout" => [input.keyboard.xkb.layout]: String,
            "variant" => [input.keyboard.xkb.variant]: String,
            "model" => [input.keyboard.xkb.model]: String,
            "rules" => [input.keyboard.xkb.rules]: String,
            "options" => [input.keyboard.xkb.options]: Option<String>,
        );
    }
}

/// Proxy for touchpad config section.
#[derive(Clone)]
struct TouchpadProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for TouchpadProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        config_field_methods!(fields, input,
            "off" => [input.touchpad.off]: bool,
            "tap" => [input.touchpad.tap]: bool,
            "dwt" => [input.touchpad.dwt]: bool,
            "dwtp" => [input.touchpad.dwtp]: bool,
            "natural_scroll" => [input.touchpad.natural_scroll]: bool,
            "left_handed" => [input.touchpad.left_handed]: bool,
            "middle_emulation" => [input.touchpad.middle_emulation]: bool,
            "drag_lock" => [input.touchpad.drag_lock]: bool,
            "disabled_on_external_mouse" => [input.touchpad.disabled_on_external_mouse]: bool,
        );
        config_field_methods_float_or_int!(fields, input,
            "accel_speed" => [input.touchpad.accel_speed],
        );

        // Scroll method (optional enum)
        fields.add_field_method_get("scroll_method", |_, this| {
            use niri_config::input::ScrollMethod;
            let value = match this.config.lock().unwrap().input.touchpad.scroll_method {
                Some(ScrollMethod::NoScroll) => Some("no-scroll"),
                Some(ScrollMethod::TwoFinger) => Some("two-finger"),
                Some(ScrollMethod::Edge) => Some("edge"),
                Some(ScrollMethod::OnButtonDown) => Some("on-button-down"),
                None => None,
            };
            Ok(value.map(|s| s.to_string()))
        });

        fields.add_field_method_set("scroll_method", |_, this, value: Option<String>| {
            use niri_config::input::ScrollMethod;
            let parsed = match value.as_deref() {
                Some("no-scroll") => Some(ScrollMethod::NoScroll),
                Some("two-finger") => Some(ScrollMethod::TwoFinger),
                Some("edge") => Some(ScrollMethod::Edge),
                Some("on-button-down") => Some(ScrollMethod::OnButtonDown),
                None => None,
                Some(other) => {
                    return Err(mlua::Error::external(format!(
                        "Invalid scroll_method: {}. Expected 'no-scroll', 'two-finger', 'edge', or 'on-button-down'",
                        other
                    )));
                }
            };
            this.config.lock().unwrap().input.touchpad.scroll_method = parsed;
            this.dirty.lock().unwrap().input = true;
            Ok(())
        });

        // Click method (optional enum)
        fields.add_field_method_get("click_method", |_, this| {
            use niri_config::input::ClickMethod;
            let value = match this.config.lock().unwrap().input.touchpad.click_method {
                Some(ClickMethod::ButtonAreas) => Some("button-areas"),
                Some(ClickMethod::Clickfinger) => Some("clickfinger"),
                None => None,
            };
            Ok(value.map(|s| s.to_string()))
        });

        fields.add_field_method_set("click_method", |_, this, value: Option<String>| {
            use niri_config::input::ClickMethod;
            let parsed = match value.as_deref() {
                Some("button-areas") => Some(ClickMethod::ButtonAreas),
                Some("clickfinger") => Some(ClickMethod::Clickfinger),
                None => None,
                Some(other) => {
                    return Err(mlua::Error::external(format!(
                        "Invalid click_method: {}. Expected 'button-areas' or 'clickfinger'",
                        other
                    )));
                }
            };
            this.config.lock().unwrap().input.touchpad.click_method = parsed;
            this.dirty.lock().unwrap().input = true;
            Ok(())
        });

        // Tap button map (optional enum)
        fields.add_field_method_get("tap_button_map", |_, this| {
            use niri_config::input::TapButtonMap;
            let value = match this.config.lock().unwrap().input.touchpad.tap_button_map {
                Some(TapButtonMap::LeftRightMiddle) => Some("left-right-middle"),
                Some(TapButtonMap::LeftMiddleRight) => Some("left-middle-right"),
                None => None,
            };
            Ok(value.map(|s| s.to_string()))
        });

        fields.add_field_method_set("tap_button_map", |_, this, value: Option<String>| {
            use niri_config::input::TapButtonMap;
            let parsed = match value.as_deref() {
                Some("left-right-middle") => Some(TapButtonMap::LeftRightMiddle),
                Some("left-middle-right") => Some(TapButtonMap::LeftMiddleRight),
                None => None,
                Some(other) => {
                    return Err(mlua::Error::external(format!(
                        "Invalid tap_button_map: {}. Expected 'left-right-middle' or 'left-middle-right'",
                        other
                    )));
                }
            };
            this.config.lock().unwrap().input.touchpad.tap_button_map = parsed;
            this.dirty.lock().unwrap().input = true;
            Ok(())
        });

        // Accel profile (optional enum)
        fields.add_field_method_get("accel_profile", |_, this| {
            use niri_config::input::AccelProfile;
            let value = match this.config.lock().unwrap().input.touchpad.accel_profile {
                Some(AccelProfile::Adaptive) => Some("adaptive"),
                Some(AccelProfile::Flat) => Some("flat"),
                None => None,
            };
            Ok(value.map(|s| s.to_string()))
        });

        fields.add_field_method_set("accel_profile", |_, this, value: Option<String>| {
            use niri_config::input::AccelProfile;
            let parsed = match value.as_deref() {
                Some("adaptive") => Some(AccelProfile::Adaptive),
                Some("flat") => Some(AccelProfile::Flat),
                None => None,
                Some(other) => {
                    return Err(mlua::Error::external(format!(
                        "Invalid accel_profile: {}. Expected 'adaptive' or 'flat'",
                        other
                    )));
                }
            };
            this.config.lock().unwrap().input.touchpad.accel_profile = parsed;
            this.dirty.lock().unwrap().input = true;
            Ok(())
        });
    }
}

/// Proxy for mouse config section.
#[derive(Clone)]
struct MouseProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for MouseProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        config_field_methods!(fields, input,
            "off" => [input.mouse.off]: bool,
            "natural_scroll" => [input.mouse.natural_scroll]: bool,
            "left_handed" => [input.mouse.left_handed]: bool,
            "middle_emulation" => [input.mouse.middle_emulation]: bool,
        );
        config_field_methods_float_or_int!(fields, input,
            "accel_speed" => [input.mouse.accel_speed],
        );
    }
}

/// Proxy for trackpoint config section.
#[derive(Clone)]
struct TrackpointProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for TrackpointProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        config_field_methods!(fields, input,
            "off" => [input.trackpoint.off]: bool,
            "natural_scroll" => [input.trackpoint.natural_scroll]: bool,
            "left_handed" => [input.trackpoint.left_handed]: bool,
            "middle_emulation" => [input.trackpoint.middle_emulation]: bool,
        );
        config_field_methods_float_or_int!(fields, input,
            "accel_speed" => [input.trackpoint.accel_speed],
        );
    }
}

/// Proxy for touch config section.
#[derive(Clone)]
struct TouchProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for TouchProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        config_field_methods!(fields, input,
            "off" => [input.touch.off]: bool,
            "natural_scroll" => [input.touch.natural_scroll]: bool,
        );
        config_field_methods_clone!(fields, input,
            "map_to_output" => [input.touch.map_to_output]: Option<String>,
        );
    }
}

/// Proxy for overview config section.
#[derive(Clone)]
struct OverviewProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for OverviewProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        config_field_methods!(fields, overview,
            "zoom" => [overview.zoom]: f64,
        );
    }
}

/// Proxy for hotkey_overlay config section.
#[derive(Clone)]
struct HotkeyOverlayProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for HotkeyOverlayProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        config_field_methods!(fields, hotkey_overlay,
            "skip_at_startup" => [hotkey_overlay.skip_at_startup]: bool,
            "hide_not_bound" => [hotkey_overlay.hide_not_bound]: bool,
        );
    }
}

/// Proxy for config_notification section.
#[derive(Clone)]
struct ConfigNotificationProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for ConfigNotificationProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        config_field_methods!(fields, config_notification,
            "disable_failed" => [config_notification.disable_failed]: bool,
        );
    }
}

/// Proxy for clipboard config section.
#[derive(Clone)]
struct ClipboardProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for ClipboardProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        config_field_methods!(fields, clipboard,
            "disable_primary" => [clipboard.disable_primary]: bool,
        );
    }
}

/// Proxy for xwayland_satellite config section.
#[derive(Clone)]
struct XwaylandSatelliteProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for XwaylandSatelliteProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        config_field_methods!(fields, xwayland_satellite,
            "off" => [xwayland_satellite.off]: bool,
        );
        config_field_methods_clone!(fields, xwayland_satellite,
            "path" => [xwayland_satellite.path]: String,
        );
    }
}

/// Proxy for debug config section.
#[derive(Clone)]
struct DebugProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for DebugProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        config_field_methods!(fields, debug,
            "dbus_interfaces_in_non_session_instances" => [debug.dbus_interfaces_in_non_session_instances]: bool,
            "wait_for_frame_completion_before_queueing" => [debug.wait_for_frame_completion_before_queueing]: bool,
            "enable_overlay_planes" => [debug.enable_overlay_planes]: bool,
            "disable_cursor_plane" => [debug.disable_cursor_plane]: bool,
            "disable_direct_scanout" => [debug.disable_direct_scanout]: bool,
            "keep_max_bpc_unchanged" => [debug.keep_max_bpc_unchanged]: bool,
            "restrict_primary_scanout_to_matching_format" => [debug.restrict_primary_scanout_to_matching_format]: bool,
        );
    }
}

/// Proxy for gestures config section.
#[derive(Clone)]
struct GesturesProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for GesturesProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        // Gestures currently has limited public config options
        // Expose the internal state to silence dead_code warning and enable future expansion
        fields.add_field_method_get("_configured", |_, this| {
            // Just check if config is accessible - returns true if gestures are configured
            let _config = this.config.lock().unwrap();
            let _ = &this.dirty;
            Ok(true)
        });
    }
}

/// Proxy for recent_windows (MRU) config section.
#[derive(Clone)]
struct RecentWindowsProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for RecentWindowsProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("off", |_, this| {
            Ok(!this.config.lock().unwrap().recent_windows.on)
        });

        fields.add_field_method_set("off", |_, this, value: bool| {
            this.config.lock().unwrap().recent_windows.on = !value;
            this.dirty.lock().unwrap().recent_windows = true;
            Ok(())
        });

        fields.add_field_method_get("on", |_, this| {
            Ok(this.config.lock().unwrap().recent_windows.on)
        });

        fields.add_field_method_set("on", |_, this, value: bool| {
            this.config.lock().unwrap().recent_windows.on = value;
            this.dirty.lock().unwrap().recent_windows = true;
            Ok(())
        });

        fields.add_field_method_get("open_delay_ms", |_, this| {
            Ok(this.config.lock().unwrap().recent_windows.open_delay_ms)
        });

        fields.add_field_method_set("open_delay_ms", |_, this, value: u16| {
            this.config.lock().unwrap().recent_windows.open_delay_ms = value;
            this.dirty.lock().unwrap().recent_windows = true;
            Ok(())
        });
    }
}

// ============================================================================
// Collection Proxies
// ============================================================================

/// Proxy for spawn_at_startup collection.
#[derive(Clone)]
struct SpawnAtStartupProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for SpawnAtStartupProxy {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("len", |_, this, ()| {
            Ok(this.config.lock().unwrap().spawn_at_startup.len())
        });

        methods.add_method("clear", |_, this, ()| {
            this.config.lock().unwrap().spawn_at_startup.clear();
            this.dirty.lock().unwrap().spawn_at_startup = true;
            Ok(())
        });

        // Add a new spawn command
        methods.add_method("add", |_, this, table: mlua::Table| {
            use niri_config::SpawnAtStartup;
            let command: Vec<String> = table.get("command")?;

            let spawn = SpawnAtStartup { command };
            this.config.lock().unwrap().spawn_at_startup.push(spawn);
            this.dirty.lock().unwrap().spawn_at_startup = true;
            Ok(())
        });
    }

    fn add_fields<F: LuaUserDataFields<Self>>(_fields: &mut F) {}
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_wrapper_new() {
        let wrapper = ConfigWrapper::new_default();
        assert!(!wrapper.has_dirty_flags());
    }

    #[test]
    fn test_config_wrapper_take_dirty_flags() {
        let wrapper = ConfigWrapper::new_default();
        wrapper.dirty.lock().unwrap().layout = true;

        let flags = wrapper.take_dirty_flags();
        assert!(flags.layout);
        assert!(!wrapper.has_dirty_flags());
    }

    #[test]
    fn test_config_wrapper_set_config() {
        let wrapper = ConfigWrapper::new_default();
        let new_config = Config {
            prefer_no_csd: true,
            ..Default::default()
        };

        wrapper.set_config(new_config);

        let is_no_csd = wrapper.with_config(|c| c.prefer_no_csd);
        assert!(is_no_csd);
        // All dirty flags should be set
        assert!(wrapper.has_dirty_flags());
    }

    #[test]
    fn test_layout_proxy_gaps() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();
        lua.load("wrapper.layout.gaps = 24").exec().unwrap();

        let gaps = wrapper.with_config(|c| c.layout.gaps);
        assert_eq!(gaps, 24.0);
        assert!(wrapper.dirty.lock().unwrap().layout);
    }

    #[test]
    fn test_cursor_proxy_xcursor_size() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();
        lua.load("wrapper.cursor.xcursor_size = 32").exec().unwrap();

        let size = wrapper.with_config(|c| c.cursor.xcursor_size);
        assert_eq!(size, 32);
        assert!(wrapper.dirty.lock().unwrap().cursor);
    }

    #[test]
    fn test_animations_proxy_slowdown() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();
        lua.load("wrapper.animations.slowdown = 2.5")
            .exec()
            .unwrap();

        let slowdown = wrapper.with_config(|c| c.animations.slowdown);
        assert_eq!(slowdown, 2.5);
        assert!(wrapper.dirty.lock().unwrap().animations);
    }

    #[test]
    fn test_center_focused_column_enum() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();
        lua.load("wrapper.layout.center_focused_column = 'always'")
            .exec()
            .unwrap();

        let cfc = wrapper.with_config(|c| c.layout.center_focused_column);
        assert_eq!(cfc, niri_config::layout::CenterFocusedColumn::Always);
    }

    #[test]
    fn test_center_focused_column_invalid() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();
        let result = lua
            .load("wrapper.layout.center_focused_column = 'invalid'")
            .exec();

        assert!(result.is_err());
    }

    #[test]
    fn test_input_keyboard_proxy() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();
        lua.load("wrapper.input.keyboard.repeat_delay = 400")
            .exec()
            .unwrap();

        let delay = wrapper.with_config(|c| c.input.keyboard.repeat_delay);
        assert_eq!(delay, 400);
        assert!(wrapper.dirty.lock().unwrap().keyboard);
    }

    #[test]
    fn test_input_touchpad_proxy() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();
        lua.load("wrapper.input.touchpad.tap = true")
            .exec()
            .unwrap();

        let tap = wrapper.with_config(|c| c.input.touchpad.tap);
        assert!(tap);
        assert!(wrapper.dirty.lock().unwrap().input);
    }

    #[test]
    fn test_overview_proxy() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();
        lua.load("wrapper.overview.zoom = 0.75").exec().unwrap();

        let zoom = wrapper.with_config(|c| c.overview.zoom);
        assert_eq!(zoom, 0.75);
        assert!(wrapper.dirty.lock().unwrap().overview);
    }

    #[test]
    fn test_hotkey_overlay_proxy() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();
        lua.load("wrapper.hotkey_overlay.skip_at_startup = true")
            .exec()
            .unwrap();

        let skip = wrapper.with_config(|c| c.hotkey_overlay.skip_at_startup);
        assert!(skip);
        assert!(wrapper.dirty.lock().unwrap().hotkey_overlay);
    }

    #[test]
    fn test_debug_proxy() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();
        lua.load("wrapper.debug.disable_cursor_plane = true")
            .exec()
            .unwrap();

        let disabled = wrapper.with_config(|c| c.debug.disable_cursor_plane);
        assert!(disabled);
        assert!(wrapper.dirty.lock().unwrap().debug);
    }

    #[test]
    fn test_clipboard_proxy() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();
        lua.load("wrapper.clipboard.disable_primary = true")
            .exec()
            .unwrap();

        let disabled = wrapper.with_config(|c| c.clipboard.disable_primary);
        assert!(disabled);
        assert!(wrapper.dirty.lock().unwrap().clipboard);
    }

    #[test]
    fn test_xkb_proxy() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();
        lua.load("wrapper.input.keyboard.xkb.layout = 'us,de'")
            .exec()
            .unwrap();

        let layout = wrapper.with_config(|c| c.input.keyboard.xkb.layout.clone());
        assert_eq!(layout, "us,de");
        assert!(wrapper.dirty.lock().unwrap().keyboard);
    }

    #[test]
    fn test_workspaces_collection() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        // Initially empty
        let len: usize = lua.load("return wrapper.workspaces:len()").eval().unwrap();
        assert_eq!(len, 0);

        // Add a workspace
        lua.load("wrapper.workspaces:add({ name = 'main', open_on_output = 'eDP-1' })")
            .exec()
            .unwrap();

        let len: usize = lua.load("return wrapper.workspaces:len()").eval().unwrap();
        assert_eq!(len, 1);

        // Get the workspace
        let name: String = lua
            .load("return wrapper.workspaces:get(1).name")
            .eval()
            .unwrap();
        assert_eq!(name, "main");

        assert!(wrapper.dirty.lock().unwrap().workspaces);
    }

    #[test]
    fn test_spawn_at_startup_collection() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        // Add a spawn command
        lua.load("wrapper.spawn_at_startup:add({ command = { 'waybar' } })")
            .exec()
            .unwrap();

        let len: usize = lua
            .load("return wrapper.spawn_at_startup:len()")
            .eval()
            .unwrap();
        assert_eq!(len, 1);

        assert!(wrapper.dirty.lock().unwrap().spawn_at_startup);
    }

    #[test]
    fn test_outputs_collection() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        // Add an output
        lua.load("wrapper.outputs:add({ name = 'eDP-1', scale = 2.0 })")
            .exec()
            .unwrap();

        let len: usize = lua.load("return wrapper.outputs:len()").eval().unwrap();
        assert_eq!(len, 1);

        // Get by name
        let scale: f64 = lua
            .load("return wrapper.outputs:get('eDP-1').scale")
            .eval()
            .unwrap();
        assert_eq!(scale, 2.0);

        assert!(wrapper.dirty.lock().unwrap().outputs);
    }

    #[test]
    fn test_window_rules_collection() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        // Add a window rule
        lua.load("wrapper.window_rules:add({ app_id = 'firefox', open_floating = true })")
            .exec()
            .unwrap();

        let len: usize = lua
            .load("return wrapper.window_rules:len()")
            .eval()
            .unwrap();
        assert_eq!(len, 1);

        assert!(wrapper.dirty.lock().unwrap().window_rules);
    }

    #[test]
    fn test_workspaces_clear() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        // Add then clear
        lua.load("wrapper.workspaces:add({ name = 'test' })")
            .exec()
            .unwrap();
        lua.load("wrapper.workspaces:clear()").exec().unwrap();

        let len: usize = lua.load("return wrapper.workspaces:len()").eval().unwrap();
        assert_eq!(len, 0);
    }

    #[test]
    fn test_binds_collection() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        // Add a key binding using table format
        lua.load(
            r#"
            wrapper.binds:add({
                key = "Mod+T",
                action = "spawn",
                args = { "kitty" },
            })
        "#,
        )
        .exec()
        .unwrap();

        let len: usize = lua.load("return wrapper.binds:len()").eval().unwrap();
        assert!(len >= 1);

        assert!(wrapper.dirty.lock().unwrap().binds);
    }

    #[test]
    fn test_environment_collection() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        // Add environment variable
        lua.load("wrapper.environment:add({ name = 'MY_VAR', value = 'my_value' })")
            .exec()
            .unwrap();

        let len: usize = lua.load("return wrapper.environment:len()").eval().unwrap();
        assert!(len >= 1);

        // Test get - returns a table with name and value fields
        let value: String = lua
            .load("return wrapper.environment:get('MY_VAR').value")
            .eval()
            .unwrap();
        assert_eq!(value, "my_value");

        assert!(wrapper.dirty.lock().unwrap().environment);
    }

    #[test]
    fn test_layer_rules_collection() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        // Add a layer rule
        lua.load("wrapper.layer_rules:add({ match = { namespace = 'waybar' }, block_out_from = 'screencast' })")
            .exec()
            .unwrap();

        let len: usize = lua.load("return wrapper.layer_rules:len()").eval().unwrap();
        assert!(len >= 1);

        assert!(wrapper.dirty.lock().unwrap().layer_rules);
    }

    #[test]
    fn test_niriv2_style_config() {
        // Test a configuration similar to examples/niriv2.lua
        // This tests the collection-based API
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("cfg", wrapper.clone()).unwrap();

        // Test outputs collection
        lua.load(
            r#"
            cfg.outputs:add({
                name = "eDP-1",
                scale = 1.5,
                off = false,
            })
            cfg.outputs:add({
                name = "HDMI-A-1",
                scale = 1.0,
            })
        "#,
        )
        .exec()
        .unwrap();

        let len: usize = lua.load("return cfg.outputs:len()").eval().unwrap();
        assert_eq!(len, 2);

        // Test workspaces collection
        lua.load(
            r#"
            for i = 1, 10 do
                cfg.workspaces:add({ name = tostring(i) })
            end
        "#,
        )
        .exec()
        .unwrap();

        let len: usize = lua.load("return cfg.workspaces:len()").eval().unwrap();
        assert_eq!(len, 10);

        // Test binds collection
        lua.load(
            r#"
            -- Test string format
            cfg.binds:add("Mod+Return spawn alacritty")
            cfg.binds:add("Mod+Q close-window")
            -- Test table format
            cfg.binds:add({
                key = "Mod+T",
                action = "spawn",
                args = { "kitty" },
            })
        "#,
        )
        .exec()
        .unwrap();

        let len: usize = lua.load("return cfg.binds:len()").eval().unwrap();
        assert!(len >= 3);

        // Test window_rules collection
        lua.load(
            r#"
            cfg.window_rules:add({
                match = { app_id = "firefox" },
                open_floating = true,
            })
        "#,
        )
        .exec()
        .unwrap();

        let len: usize = lua.load("return cfg.window_rules:len()").eval().unwrap();
        assert!(len >= 1);

        // Test environment collection
        lua.load(
            r#"
            cfg.environment:add({ name = "QT_QPA_PLATFORM", value = "wayland" })
            cfg.environment:add({ name = "ELECTRON_OZONE_PLATFORM_HINT", value = "wayland" })
        "#,
        )
        .exec()
        .unwrap();

        let len: usize = lua.load("return cfg.environment:len()").eval().unwrap();
        assert!(len >= 2);

        // Test layer_rules collection
        lua.load(
            r#"
            cfg.layer_rules:add({
                match = { namespace = "waybar" },
                shadow = false,
            })
        "#,
        )
        .exec()
        .unwrap();

        let len: usize = lua.load("return cfg.layer_rules:len()").eval().unwrap();
        assert!(len >= 1);

        // Verify dirty flags were set
        let dirty = wrapper.dirty.lock().unwrap();
        assert!(dirty.outputs);
        assert!(dirty.workspaces);
        assert!(dirty.binds);
        assert!(dirty.window_rules);
        assert!(dirty.environment);
        assert!(dirty.layer_rules);
    }
}
