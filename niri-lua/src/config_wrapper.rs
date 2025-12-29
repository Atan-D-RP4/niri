use std::sync::{Arc, Mutex};

use mlua::prelude::*;
use mlua::UserData;
use niri_config::{Config, ScreenshotPath};

use crate::collections::{
    BindsCollection, EnvironmentCollection, LayerRulesCollection, OutputsCollection,
    WindowRulesCollection, WorkspacesCollection,
};
use crate::config_dirty::ConfigDirtyFlags;
use crate::config_proxies::{
    AnimationsConfigProxy, ClipboardConfigProxy, ConfigNotificationConfigProxy, CursorConfigProxy,
    DebugConfigProxy, GesturesConfigProxy, HotkeyOverlayConfigProxy, InputConfigProxy,
    LayoutConfigProxy, OverviewConfigProxy, RecentWindowsConfigProxy, SpawnAtStartupConfigProxy,
    XwaylandSatelliteConfigProxy,
};
use crate::config_state::ConfigState;
use crate::extractors::{
    extract_animations, extract_clipboard, extract_config_notification, extract_cursor,
    extract_debug, extract_gestures, extract_hotkey_overlay, extract_input, extract_layout,
    extract_overview, extract_recent_windows, extract_xwayland_satellite,
};

macro_rules! proxy_field {
    ($fields:expr, $name:literal, $config_field:ident, $dirty_flag:ident, $proxy:ident, $extractor:ident) => {
        $fields.add_field_method_get($name, |_, this| {
            let state = ConfigState::new(this.config.clone(), this.dirty.clone());
            Ok($proxy::new(state))
        });

        $fields.add_field_method_set($name, |_, this, value: LuaTable| {
            if let Some(v) = $extractor(&value)? {
                this.config.lock().unwrap().$config_field = v;
                this.dirty.lock().unwrap().$dirty_flag = true;
            }
            Ok(())
        });
    };
}

macro_rules! collection_field {
    ($fields:expr, $name:literal, $collection:ident) => {
        $fields.add_field_method_get($name, |_, this| {
            Ok($collection {
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });
    };
}

macro_rules! scalar_field {
    ($fields:expr, $name:literal, $config_field:ident, $dirty_flag:ident, $type:ty) => {
        $fields.add_field_method_get($name, |_, this| {
            Ok(this.config.lock().unwrap().$config_field)
        });

        $fields.add_field_method_set($name, |_, this, value: $type| {
            this.config.lock().unwrap().$config_field = value;
            this.dirty.lock().unwrap().$dirty_flag = true;
            Ok(())
        });
    };
}

macro_rules! wrapper_field {
    ($fields:expr, $name:literal, $config_field:ident, $dirty_flag:ident, $wrapper:ident, $inner:ty) => {
        $fields.add_field_method_get($name, |_, this| {
            Ok(this.config.lock().unwrap().$config_field.0.clone())
        });

        $fields.add_field_method_set($name, |_, this, value: $inner| {
            this.config.lock().unwrap().$config_field = $wrapper(value);
            this.dirty.lock().unwrap().$dirty_flag = true;
            Ok(())
        });
    };
}

#[derive(Default, Debug)]
pub struct DirtyFlags {
    pub layout: bool,
    pub cursor: bool,
    pub animations: bool,
    pub input: bool,
    pub misc: bool,
    pub outputs: bool,
    pub window_rules: bool,
    pub layer_rules: bool,
    pub binds: bool,
    pub workspaces: bool,
    pub environment: bool,
    pub spawn_at_startup: bool,
}

impl DirtyFlags {
    pub fn any(&self) -> bool {
        self.layout
            || self.cursor
            || self.animations
            || self.input
            || self.misc
            || self.outputs
            || self.window_rules
            || self.layer_rules
            || self.binds
            || self.workspaces
            || self.environment
            || self.spawn_at_startup
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn summary(&self) -> Vec<&'static str> {
        let mut result = Vec::new();
        if self.layout {
            result.push("layout");
        }
        if self.cursor {
            result.push("cursor");
        }
        if self.animations {
            result.push("animations");
        }
        if self.input {
            result.push("input");
        }
        if self.misc {
            result.push("misc");
        }
        if self.outputs {
            result.push("outputs");
        }
        if self.window_rules {
            result.push("window_rules");
        }
        if self.layer_rules {
            result.push("layer_rules");
        }
        if self.binds {
            result.push("binds");
        }
        if self.workspaces {
            result.push("workspaces");
        }
        if self.environment {
            result.push("environment");
        }
        if self.spawn_at_startup {
            result.push("spawn_at_startup");
        }
        result
    }
}

#[derive(Clone)]
pub struct ConfigWrapper {
    pub config: Arc<Mutex<Config>>,
    pub dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl ConfigWrapper {
    pub fn new(config: Arc<Mutex<Config>>) -> Self {
        Self {
            config,
            dirty: Arc::new(Mutex::new(ConfigDirtyFlags::default())),
        }
    }

    pub fn new_default() -> Self {
        Self::new(Arc::new(Mutex::new(Config::default())))
    }

    pub fn take_dirty_flags(&self) -> ConfigDirtyFlags {
        self.dirty.lock().unwrap().take()
    }

    pub fn has_dirty_flags(&self) -> bool {
        self.dirty.lock().unwrap().any()
    }

    pub fn get_config(&self) -> Arc<Mutex<Config>> {
        self.config.clone()
    }

    pub fn extract_config(&self) -> Config {
        let mut guard = self.config.lock().unwrap();
        std::mem::take(&mut *guard)
    }

    pub fn swap_config(&self, new_config: Config) -> Config {
        let mut guard = self.config.lock().unwrap();
        std::mem::replace(&mut *guard, new_config)
    }
}

impl UserData for ConfigWrapper {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        // Proxy fields: getter returns proxy, setter uses extractor
        proxy_field!(
            fields,
            "layout",
            layout,
            layout,
            LayoutConfigProxy,
            extract_layout
        );
        proxy_field!(
            fields,
            "cursor",
            cursor,
            cursor,
            CursorConfigProxy,
            extract_cursor
        );
        proxy_field!(
            fields,
            "animations",
            animations,
            animations,
            AnimationsConfigProxy,
            extract_animations
        );
        proxy_field!(
            fields,
            "input",
            input,
            input,
            InputConfigProxy,
            extract_input
        );
        proxy_field!(
            fields,
            "overview",
            overview,
            misc,
            OverviewConfigProxy,
            extract_overview
        );
        proxy_field!(
            fields,
            "hotkey_overlay",
            hotkey_overlay,
            misc,
            HotkeyOverlayConfigProxy,
            extract_hotkey_overlay
        );
        proxy_field!(
            fields,
            "config_notification",
            config_notification,
            misc,
            ConfigNotificationConfigProxy,
            extract_config_notification
        );
        proxy_field!(
            fields,
            "clipboard",
            clipboard,
            misc,
            ClipboardConfigProxy,
            extract_clipboard
        );
        proxy_field!(
            fields,
            "xwayland_satellite",
            xwayland_satellite,
            misc,
            XwaylandSatelliteConfigProxy,
            extract_xwayland_satellite
        );
        proxy_field!(
            fields,
            "debug",
            debug,
            misc,
            DebugConfigProxy,
            extract_debug
        );
        proxy_field!(
            fields,
            "gestures",
            gestures,
            misc,
            GesturesConfigProxy,
            extract_gestures
        );
        proxy_field!(
            fields,
            "recent_windows",
            recent_windows,
            misc,
            RecentWindowsConfigProxy,
            extract_recent_windows
        );

        // Collection fields: read-only access to collections
        collection_field!(fields, "workspaces", WorkspacesCollection);
        collection_field!(fields, "outputs", OutputsCollection);
        collection_field!(fields, "window_rules", WindowRulesCollection);
        collection_field!(fields, "binds", BindsCollection);
        collection_field!(fields, "environment", EnvironmentCollection);
        collection_field!(fields, "layer_rules", LayerRulesCollection);

        // spawn_at_startup: special case - returns inner collection from proxy
        fields.add_field_method_get("spawn_at_startup", |_, this| {
            let state = ConfigState::new(this.config.clone(), this.dirty.clone());
            let proxy = SpawnAtStartupConfigProxy::new(state);
            Ok(proxy.get_spawn_at_startup())
        });

        // Scalar fields
        scalar_field!(fields, "prefer_no_csd", prefer_no_csd, misc, bool);

        // Wrapper fields (newtypes)
        wrapper_field!(
            fields,
            "screenshot_path",
            screenshot_path,
            misc,
            ScreenshotPath,
            Option<String>
        );
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("apply", |_, _this, ()| Ok(()));
    }
}

pub fn register_config_wrapper(lua: &Lua, wrapper: ConfigWrapper) -> LuaResult<()> {
    let globals = lua.globals();
    let niri: LuaTable = globals.get("niri")?;
    niri.set("config", wrapper)?;
    Ok(())
}
