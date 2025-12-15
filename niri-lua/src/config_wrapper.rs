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
use crate::config_state::ConfigState;
use crate::extractors::{
    extract_animations, extract_clipboard, extract_config_notification, extract_cursor,
    extract_debug, extract_gestures, extract_hotkey_overlay, extract_input, extract_layout,
    extract_overview, extract_recent_windows, extract_xwayland_satellite,
};
use crate::migrated_proxies::{
    AnimationsConfigProxy, ClipboardConfigProxy, ConfigNotificationConfigProxy, CursorConfigProxy,
    DebugConfigProxy, GesturesConfigProxy, HotkeyOverlayConfigProxy, InputConfigProxy,
    LayoutConfigProxy, OverviewConfigProxy, RecentWindowsConfigProxy,
    SpawnAtStartupConfigProxy, XwaylandSatelliteConfigProxy,
};

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
            let state = ConfigState::new(this.config.clone(), this.dirty.clone());
            Ok(LayoutConfigProxy::new(state))
        });

        fields.add_field_method_set("layout", |_, this, value: LuaTable| {
            if let Some(layout) = extract_layout(&value)? {
                this.config.lock().unwrap().layout = layout;
                this.dirty.lock().unwrap().layout = true;
            }
            Ok(())
        });

        fields.add_field_method_get("cursor", |_, this| {
            let state = ConfigState::new(this.config.clone(), this.dirty.clone());
            Ok(CursorConfigProxy::new(state))
        });

        fields.add_field_method_set("cursor", |_, this, value: LuaTable| {
            if let Some(cursor) = extract_cursor(&value)? {
                this.config.lock().unwrap().cursor = cursor;
                this.dirty.lock().unwrap().cursor = true;
            }
            Ok(())
        });

        fields.add_field_method_get("animations", |_, this| {
            let state = ConfigState::new(this.config.clone(), this.dirty.clone());
            Ok(AnimationsConfigProxy::new(state))
        });

        fields.add_field_method_set("animations", |_, this, value: LuaTable| {
            if let Some(animations) = extract_animations(&value)? {
                this.config.lock().unwrap().animations = animations;
                this.dirty.lock().unwrap().animations = true;
            }
            Ok(())
        });

        fields.add_field_method_get("input", |_, this| {
            let state = ConfigState::new(this.config.clone(), this.dirty.clone());
            Ok(InputConfigProxy::new(state))
        });

        fields.add_field_method_set("input", |_, this, value: LuaTable| {
            if let Some(input) = extract_input(&value)? {
                this.config.lock().unwrap().input = input;
                this.dirty.lock().unwrap().input = true;
            }
            Ok(())
        });

        fields.add_field_method_get("overview", |_, this| {
            let state = ConfigState::new(this.config.clone(), this.dirty.clone());
            Ok(OverviewConfigProxy::new(state))
        });

        fields.add_field_method_set("overview", |_, this, value: LuaTable| {
            if let Some(overview) = extract_overview(&value)? {
                this.config.lock().unwrap().overview = overview;
                this.dirty.lock().unwrap().misc = true;
            }
            Ok(())
        });

        fields.add_field_method_get("hotkey_overlay", |_, this| {
            let state = ConfigState::new(this.config.clone(), this.dirty.clone());
            Ok(HotkeyOverlayConfigProxy::new(state))
        });

        fields.add_field_method_set("hotkey_overlay", |_, this, value: LuaTable| {
            if let Some(overlay) = extract_hotkey_overlay(&value)? {
                this.config.lock().unwrap().hotkey_overlay = overlay;
                this.dirty.lock().unwrap().misc = true;
            }
            Ok(())
        });

        fields.add_field_method_get("config_notification", |_, this| {
            let state = ConfigState::new(this.config.clone(), this.dirty.clone());
            Ok(ConfigNotificationConfigProxy::new(state))
        });

        fields.add_field_method_set("config_notification", |_, this, value: LuaTable| {
            if let Some(notification) = extract_config_notification(&value)? {
                this.config.lock().unwrap().config_notification = notification;
                this.dirty.lock().unwrap().misc = true;
            }
            Ok(())
        });

        fields.add_field_method_get("clipboard", |_, this| {
            let state = ConfigState::new(this.config.clone(), this.dirty.clone());
            Ok(ClipboardConfigProxy::new(state))
        });

        fields.add_field_method_set("clipboard", |_, this, value: LuaTable| {
            if let Some(clipboard) = extract_clipboard(&value)? {
                this.config.lock().unwrap().clipboard = clipboard;
                this.dirty.lock().unwrap().misc = true;
            }
            Ok(())
        });

        fields.add_field_method_get("xwayland_satellite", |_, this| {
            let state = ConfigState::new(this.config.clone(), this.dirty.clone());
            Ok(XwaylandSatelliteConfigProxy::new(state))
        });

        fields.add_field_method_set("xwayland_satellite", |_, this, value: LuaTable| {
            if let Some(xwayland) = extract_xwayland_satellite(&value)? {
                this.config.lock().unwrap().xwayland_satellite = xwayland;
                this.dirty.lock().unwrap().misc = true;
            }
            Ok(())
        });

        fields.add_field_method_get("debug", |_, this| {
            let state = ConfigState::new(this.config.clone(), this.dirty.clone());
            Ok(DebugConfigProxy::new(state))
        });

        fields.add_field_method_set("debug", |_, this, value: LuaTable| {
            if let Some(debug) = extract_debug(&value)? {
                this.config.lock().unwrap().debug = debug;
                this.dirty.lock().unwrap().misc = true;
            }
            Ok(())
        });

        fields.add_field_method_get("gestures", |_, this| {
            let state = ConfigState::new(this.config.clone(), this.dirty.clone());
            Ok(GesturesConfigProxy::new(state))
        });

        fields.add_field_method_set("gestures", |_, this, value: LuaTable| {
            if let Some(gestures) = extract_gestures(&value)? {
                this.config.lock().unwrap().gestures = gestures;
                this.dirty.lock().unwrap().misc = true;
            }
            Ok(())
        });

        fields.add_field_method_get("recent_windows", |_, this| {
            let state = ConfigState::new(this.config.clone(), this.dirty.clone());
            Ok(RecentWindowsConfigProxy::new(state))
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
            let state = ConfigState::new(this.config.clone(), this.dirty.clone());
            let proxy = SpawnAtStartupConfigProxy::new(state);
            // Return the inner collection directly for better UX
            Ok(proxy.get_spawn_at_startup())
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

        // update() method - batch update multiple config properties in a single lock
        methods.add_method("update", |_, this, table: LuaTable| {
            // Single lock for all updates
            let mut config = this.config.lock().unwrap();
            let mut dirty = this.dirty.lock().unwrap();

            // Handle top-level scalars
            if let Ok(prefer_no_csd) = table.get::<bool>("prefer_no_csd") {
                config.prefer_no_csd = prefer_no_csd;
                dirty.misc = true;
            }

            if let Ok(screenshot_path) = table.get::<String>("screenshot_path") {
                config.screenshot_path = niri_config::ScreenshotPath(Some(screenshot_path));
                dirty.misc = true;
            }

            // Handle layout section
            if let Ok(layout_table) = table.get::<LuaTable>("layout") {
                if let Ok(gaps) = layout_table.get::<f64>("gaps") {
                    config.layout.gaps = gaps;
                    dirty.layout = true;
                }

                if let Ok(always_center_single_column) =
                    layout_table.get::<bool>("always_center_single_column")
                {
                    config.layout.always_center_single_column = always_center_single_column;
                    dirty.layout = true;
                }
            }

            // Handle cursor section
            if let Ok(cursor_table) = table.get::<LuaTable>("cursor") {
                if let Ok(xcursor_size) = cursor_table.get::<u8>("xcursor_size") {
                    config.cursor.xcursor_size = xcursor_size;
                    dirty.cursor = true;
                }

                if let Ok(xcursor_theme) = cursor_table.get::<String>("xcursor_theme") {
                    config.cursor.xcursor_theme = xcursor_theme;
                    dirty.cursor = true;
                }
            }

            // Handle animations section
            if let Ok(animations_table) = table.get::<LuaTable>("animations") {
                if let Ok(off) = animations_table.get::<bool>("off") {
                    config.animations.off = off;
                    dirty.animations = true;
                }

                if let Ok(slowdown) = animations_table.get::<f64>("slowdown") {
                    config.animations.slowdown = slowdown;
                    dirty.animations = true;
                }
            }

            Ok(())
        });
    }
}

// ============================================================================
// Section Proxies
// ============================================================================

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
        lua.load("wrapper.spawn_at_startup:append({ command = { 'waybar' } })")
            .exec()
            .unwrap();

        let len: usize = lua.load("return #wrapper.spawn_at_startup").eval().unwrap();
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

    // ========================================================================
    // SNAPSHOT TESTS - Config Transformation Patterns
    // ========================================================================

    #[test]
    fn snapshot_layout_gaps_transformation() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();
        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        // Set gaps from Lua
        lua.load("wrapper.layout.gaps = 32").exec().unwrap();

        let gaps = wrapper.with_config(|c| c.layout.gaps);
        let dirty = wrapper.dirty.lock().unwrap().layout;

        insta::assert_debug_snapshot!("layout_gaps_transformation", (gaps, dirty));
    }

    #[test]
    fn snapshot_cursor_config_transformation() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();
        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        lua.load(
            r#"
            wrapper.cursor.xcursor_theme = "Adwaita"
            wrapper.cursor.xcursor_size = 24
            wrapper.cursor.hide_when_typing = true
        "#,
        )
        .exec()
        .unwrap();

        let (theme, size, hide) = wrapper.with_config(|c| {
            (
                c.cursor.xcursor_theme.clone(),
                c.cursor.xcursor_size,
                c.cursor.hide_when_typing,
            )
        });

        insta::assert_debug_snapshot!("cursor_config_transformation", (theme, size, hide));
    }

    #[test]
    fn snapshot_keyboard_xkb_transformation() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();
        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        lua.load(
            r#"
            wrapper.input.keyboard.xkb.layout = "us,de,fr"
            wrapper.input.keyboard.xkb.variant = "dvorak"
            wrapper.input.keyboard.xkb.options = "grp:alt_shift_toggle,caps:escape"
        "#,
        )
        .exec()
        .unwrap();

        let (layout, variant, options) = wrapper.with_config(|c| {
            (
                c.input.keyboard.xkb.layout.clone(),
                c.input.keyboard.xkb.variant.clone(),
                c.input.keyboard.xkb.options.clone(),
            )
        });

        insta::assert_debug_snapshot!("keyboard_xkb_transformation", (layout, variant, options));
    }

    #[test]
    fn snapshot_animations_slowdown_transformation() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();
        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        lua.load("wrapper.animations.slowdown = 3.0")
            .exec()
            .unwrap();

        let slowdown = wrapper.with_config(|c| c.animations.slowdown);
        let off = wrapper.with_config(|c| c.animations.off);

        insta::assert_debug_snapshot!("animations_slowdown_transformation", (slowdown, off));
    }

    #[test]
    fn snapshot_center_focused_column_enum_transformation() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();
        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        // Test all valid enum values
        lua.load("wrapper.layout.center_focused_column = 'never'")
            .exec()
            .unwrap();
        let never_value = wrapper.with_config(|c| format!("{:?}", c.layout.center_focused_column));

        lua.load("wrapper.layout.center_focused_column = 'always'")
            .exec()
            .unwrap();
        let always_value = wrapper.with_config(|c| format!("{:?}", c.layout.center_focused_column));

        lua.load("wrapper.layout.center_focused_column = 'on-overflow'")
            .exec()
            .unwrap();
        let overflow_value =
            wrapper.with_config(|c| format!("{:?}", c.layout.center_focused_column));

        insta::assert_debug_snapshot!(
            "center_focused_column_enum_values",
            (never_value, always_value, overflow_value)
        );
    }

    #[test]
    fn snapshot_touchpad_config_transformation() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();
        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        lua.load(
            r#"
            wrapper.input.touchpad.tap = true
            wrapper.input.touchpad.natural_scroll = true
            wrapper.input.touchpad.accel_speed = 0.3
        "#,
        )
        .exec()
        .unwrap();

        let (tap, natural_scroll, accel_speed) = wrapper.with_config(|c| {
            (
                c.input.touchpad.tap,
                c.input.touchpad.natural_scroll,
                c.input.touchpad.accel_speed.0,
            )
        });

        insta::assert_debug_snapshot!(
            "touchpad_config_transformation",
            (tap, natural_scroll, accel_speed)
        );
    }

    #[test]
    fn snapshot_workspace_collection_operations() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();
        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        // Add multiple workspaces
        lua.load(
            r#"
            wrapper.workspaces:add({ name = "main" })
            wrapper.workspaces:add({ name = "web" })
            wrapper.workspaces:add({ name = "dev" })
        "#,
        )
        .exec()
        .unwrap();

        let len: usize = lua.load("return wrapper.workspaces:len()").eval().unwrap();
        let first_name: String = lua
            .load("return wrapper.workspaces:get(1).name")
            .eval()
            .unwrap();
        let third_name: String = lua
            .load("return wrapper.workspaces:get(3).name")
            .eval()
            .unwrap();

        insta::assert_debug_snapshot!(
            "workspace_collection_operations",
            (len, first_name, third_name)
        );
    }

    #[test]
    fn snapshot_output_collection_with_scale() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();
        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        lua.load(
            r#"
            wrapper.outputs:add({
                name = "eDP-1",
                scale = 2.0,
                off = false,
            })
            wrapper.outputs:add({
                name = "HDMI-A-1",
                scale = 1.5,
            })
        "#,
        )
        .exec()
        .unwrap();

        let len: usize = lua.load("return wrapper.outputs:len()").eval().unwrap();
        let first_scale: f64 = lua
            .load("return wrapper.outputs:get('eDP-1').scale")
            .eval()
            .unwrap();
        let second_scale: f64 = lua
            .load("return wrapper.outputs:get('HDMI-A-1').scale")
            .eval()
            .unwrap();

        insta::assert_debug_snapshot!(
            "output_collection_with_scale",
            (len, first_scale, second_scale)
        );
    }

    #[test]
    fn snapshot_window_rule_transformation() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();
        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        lua.load(
            r#"
            wrapper.window_rules:add({
                match = { app_id = "firefox" },
                open_floating = true,
            })
        "#,
        )
        .exec()
        .unwrap();

        let len: usize = lua
            .load("return wrapper.window_rules:len()")
            .eval()
            .unwrap();
        let dirty = wrapper.dirty.lock().unwrap().window_rules;

        insta::assert_debug_snapshot!("window_rule_transformation", (len, dirty));
    }

    #[test]
    fn snapshot_binds_collection_string_format() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();
        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        // Add binds using string format
        lua.load(
            r#"
            wrapper.binds:add("Mod+Return spawn alacritty")
            wrapper.binds:add("Mod+Q close-window")
        "#,
        )
        .exec()
        .unwrap();

        let len: usize = lua.load("return wrapper.binds:len()").eval().unwrap();
        let dirty = wrapper.dirty.lock().unwrap().binds;

        insta::assert_debug_snapshot!("binds_collection_string_format", (len, dirty));
    }

    #[test]
    fn snapshot_binds_collection_table_format() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();
        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        // Add binds using table format
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
        insta::assert_debug_snapshot!("binds_collection_table_format", len);
    }

    #[test]
    fn snapshot_environment_variable_transformation() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();
        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        lua.load(
            r#"
            wrapper.environment:add({ name = "QT_QPA_PLATFORM", value = "wayland" })
            wrapper.environment:add({ name = "MOZ_ENABLE_WAYLAND", value = "1" })
        "#,
        )
        .exec()
        .unwrap();

        let len: usize = lua.load("return wrapper.environment:len()").eval().unwrap();
        let first_value: String = lua
            .load("return wrapper.environment:get('QT_QPA_PLATFORM').value")
            .eval()
            .unwrap();

        insta::assert_debug_snapshot!("environment_variable_transformation", (len, first_value));
    }

    #[test]
    fn snapshot_dirty_flags_after_multiple_changes() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();
        lua.globals().set("wrapper", wrapper.clone()).unwrap();

        // Make changes to different subsystems
        lua.load(
            r#"
            wrapper.layout.gaps = 16
            wrapper.cursor.xcursor_size = 32
            wrapper.input.touchpad.tap = true
            wrapper.animations.slowdown = 2.0
        "#,
        )
        .exec()
        .unwrap();

        let dirty = wrapper.dirty.lock().unwrap();
        let flags = (
            dirty.layout,
            dirty.cursor,
            dirty.input,
            dirty.animations,
            dirty.keyboard,
            dirty.binds,
        );

        insta::assert_debug_snapshot!("dirty_flags_after_multiple_changes", flags);
    }

    // ========================================================================
    // SNAPSHOT TESTS - Default Config Structure and Values
    // ========================================================================

    #[test]
    fn snapshot_default_layout_config() {
        let wrapper = ConfigWrapper::new_default();
        let layout = wrapper.with_config(|c| format!("{:#?}", c.layout));
        insta::assert_debug_snapshot!("config_wrapper_default_layout", layout);
    }

    #[test]
    fn snapshot_default_input_config() {
        let wrapper = ConfigWrapper::new_default();
        let input = wrapper.with_config(|c| format!("{:#?}", c.input));
        insta::assert_debug_snapshot!("config_wrapper_default_input", input);
    }

    #[test]
    fn snapshot_default_animations_config() {
        let wrapper = ConfigWrapper::new_default();
        let animations = wrapper.with_config(|c| format!("{:#?}", c.animations));
        insta::assert_debug_snapshot!("config_wrapper_default_animations", animations);
    }

    #[test]
    fn snapshot_default_cursor_config() {
        let wrapper = ConfigWrapper::new_default();
        let cursor = wrapper.with_config(|c| format!("{:#?}", c.cursor));
        insta::assert_debug_snapshot!("config_wrapper_default_cursor", cursor);
    }

    #[test]
    fn snapshot_default_overview_config() {
        let wrapper = ConfigWrapper::new_default();
        let overview = wrapper.with_config(|c| format!("{:#?}", c.overview));
        insta::assert_debug_snapshot!("config_wrapper_default_overview", overview);
    }

    #[test]
    fn snapshot_default_gestures_config() {
        let wrapper = ConfigWrapper::new_default();
        let gestures = wrapper.with_config(|c| format!("{:#?}", c.gestures));
        insta::assert_debug_snapshot!("config_wrapper_default_gestures", gestures);
    }

    #[test]
    fn snapshot_default_debug_config() {
        let wrapper = ConfigWrapper::new_default();
        let debug = wrapper.with_config(|c| format!("{:#?}", c.debug));
        insta::assert_debug_snapshot!("config_wrapper_default_debug", debug);
    }

    #[test]
    fn snapshot_default_top_level_config() {
        let wrapper = ConfigWrapper::new_default();
        let top_level = wrapper.with_config(|c| (c.prefer_no_csd, c.screenshot_path.0.clone()));
        insta::assert_debug_snapshot!("config_wrapper_default_top_level", top_level);
    }

    // ========================================================================
    // BATCH UPDATE TESTS
    // ========================================================================

    #[test]
    fn test_batch_update_single_lock() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();
        lua.load(
            r#"
            wrapper:update({
                prefer_no_csd = true,
                layout = { gaps = 16, always_center_single_column = true },
                cursor = { xcursor_size = 32, xcursor_theme = "Adwaita" },
                animations = { off = false, slowdown = 2.5 }
            })
        "#,
        )
        .exec()
        .unwrap();

        // Verify all values were set
        wrapper.with_config(|c| {
            assert!(c.prefer_no_csd);
            assert_eq!(c.layout.gaps, 16.0);
            assert!(c.layout.always_center_single_column);
            assert_eq!(c.cursor.xcursor_size, 32);
            assert_eq!(c.cursor.xcursor_theme, "Adwaita");
            assert!(!c.animations.off);
            assert_eq!(c.animations.slowdown, 2.5);
        });

        // Verify correct dirty flags were set
        let dirty = wrapper.dirty.lock().unwrap();
        assert!(dirty.misc);
        assert!(dirty.layout);
        assert!(dirty.cursor);
        assert!(dirty.animations);
    }

    #[test]
    fn test_batch_update_partial_sections() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();
        lua.load(
            r#"
            wrapper:update({
                layout = { gaps = 24 },
                animations = { slowdown = 1.5 }
            })
        "#,
        )
        .exec()
        .unwrap();

        // Verify only specified fields were changed
        wrapper.with_config(|c| {
            assert_eq!(c.layout.gaps, 24.0);
            // Other layout fields should remain at defaults
            assert!(!c.layout.always_center_single_column);
            assert_eq!(c.animations.slowdown, 1.5);
        });
    }

    #[test]
    fn test_batch_update_screenshot_path() {
        let wrapper = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper", wrapper.clone()).unwrap();
        lua.load(
            r#"
            wrapper:update({
                screenshot_path = "/home/user/screenshots"
            })
        "#,
        )
        .exec()
        .unwrap();

        wrapper.with_config(|c| {
            assert_eq!(
                c.screenshot_path.0,
                Some("/home/user/screenshots".to_string())
            );
        });

        assert!(wrapper.dirty.lock().unwrap().misc);
    }

    #[test]
    fn test_batch_update_vs_individual() {
        // This test demonstrates that batch update achieves the same result
        // but with fewer lock acquisitions
        let wrapper1 = ConfigWrapper::new_default();
        let wrapper2 = ConfigWrapper::new_default();
        let lua = mlua::Lua::new();

        lua.globals().set("wrapper1", wrapper1.clone()).unwrap();
        lua.globals().set("wrapper2", wrapper2.clone()).unwrap();

        // Individual updates (multiple locks)
        lua.load(
            r#"
            wrapper1.layout.gaps = 16
            wrapper1.cursor.xcursor_size = 32
            wrapper1.animations.slowdown = 2.0
        "#,
        )
        .exec()
        .unwrap();

        // Batch update (single lock)
        lua.load(
            r#"
            wrapper2:update({
                layout = { gaps = 16 },
                cursor = { xcursor_size = 32 },
                animations = { slowdown = 2.0 }
            })
        "#,
        )
        .exec()
        .unwrap();

        // Both should have the same final state
        let result1 =
            wrapper1.with_config(|c| (c.layout.gaps, c.cursor.xcursor_size, c.animations.slowdown));
        let result2 =
            wrapper2.with_config(|c| (c.layout.gaps, c.cursor.xcursor_size, c.animations.slowdown));

        assert_eq!(result1, result2);
    }
}
