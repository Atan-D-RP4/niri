//! Lua event emission helpers for Niri core.
//!
//! This module provides extension traits to emit events from Niri's compositor logic
//! to the Lua runtime. Events are emitted via trait methods on `State` and `Niri`.
//!
//! # Usage
//!
//! ```ignore
//! use crate::lua_event_hooks::StateLuaEvents;
//!
//! // Emit events using method syntax:
//! state.emit_window_open(window_id, "Window Title");
//! state.emit_workspace_activate("workspace-name", 1);
//! ```

use log::debug;
use mlua::prelude::*;

use crate::niri::{Niri, State};

/// Simplified event emission that uses StateHandle from app_data.
///
/// Since StateHandle is always available via lua.app_data(), event handlers can
/// access state through niri.state.* without needing scoped state setup.
fn emit_event<F>(state: &State, event_name: &str, create_data: F)
where
    F: FnOnce(&Lua) -> LuaResult<LuaValue>,
{
    if let Some(lua_runtime) = &state.niri.lua_runtime {
        if let Some(event_system) = &lua_runtime.event_system {
            let lua = lua_runtime.inner();
            let result = create_data(lua).and_then(|data| event_system.emit(lua, event_name, data));
            if let Err(e) = result {
                debug!("Failed to emit {} event: {}", event_name, e);
            }
        }
    }
}

/// Emit an event with Niri-only context (when full State is unavailable).
/// Uses StateHandle from app_data for state access.
fn emit_with_niri_context<F>(niri: &Niri, event_name: &str, create_data: F)
where
    F: FnOnce(&Lua) -> LuaResult<LuaValue>,
{
    if let Some(lua_runtime) = &niri.lua_runtime {
        if let Some(event_system) = &lua_runtime.event_system {
            let lua = lua_runtime.inner();
            let result = create_data(lua).and_then(|data| event_system.emit(lua, event_name, data));
            if let Err(e) = result {
                debug!("Failed to emit {} event: {}", event_name, e);
            }
        }
    }
}

// ============================================================================
// Helper functions to create Lua tables for event payloads
// ============================================================================

fn create_window_table(lua: &Lua, id: u32, title: &str) -> LuaResult<LuaValue> {
    let table = lua.create_table()?;
    table.set("id", id)?;
    table.set("title", title)?;
    Ok(LuaValue::Table(table))
}

fn create_workspace_table(lua: &Lua, name: &str, idx: u32) -> LuaResult<LuaValue> {
    let table = lua.create_table()?;
    table.set("name", name)?;
    table.set("idx", idx)?;
    Ok(LuaValue::Table(table))
}

fn create_monitor_table(lua: &Lua, name: &str, connector: &str) -> LuaResult<LuaValue> {
    let table = lua.create_table()?;
    table.set("name", name)?;
    table.set("connector", connector)?;
    Ok(LuaValue::Table(table))
}

fn create_empty_table(lua: &Lua) -> LuaResult<LuaValue> {
    Ok(LuaValue::Table(lua.create_table()?))
}

// ============================================================================
// Extension trait for State - primary event emission interface
// ============================================================================

/// Extension trait for emitting Lua events from State.
///
/// This trait provides method-style event emission, making call sites cleaner.
///
/// # Example
/// ```ignore
/// use crate::lua_event_hooks::StateLuaEvents;
///
/// state.emit_window_focus(window_id, "Window Title");
/// state.emit_workspace_activate("workspace", 1);
/// ```
pub trait StateLuaEvents {
    // ---- Window events ----

    /// Emit a window:open event when a window is created and mapped
    fn emit_window_open(&self, window_id: u32, window_title: &str);

    /// Emit a window:close event when a window is destroyed
    fn emit_window_close(&self, window_id: u32, window_title: &str);

    /// Emit a window:focus event when a window receives focus
    fn emit_window_focus(&self, window_id: u32, window_title: &str);

    /// Emit a window:blur event when a window loses focus
    fn emit_window_blur(&self, window_id: u32, window_title: &str);

    /// Emit a window:title_changed event when a window's title changes
    fn emit_window_title_changed(&self, window_id: u32, new_title: &str);

    /// Emit a window:app_id_changed event when a window's app_id changes
    fn emit_window_app_id_changed(&self, window_id: u32, new_app_id: &str);

    /// Emit a window:fullscreen event when a window enters or exits fullscreen
    fn emit_window_fullscreen(&self, window_id: u32, window_title: &str, is_fullscreen: bool);

    /// Emit a window:maximize event when a window is maximized or unmaximized
    fn emit_window_maximize(&self, window_id: u32, window_title: &str, is_maximized: bool);

    /// Emit a window:resize event when a window's size changes
    fn emit_window_resize(&self, window_id: u32, window_title: &str, width: i32, height: i32);

    /// Emit a window:move event when a window moves to a different workspace/monitor
    fn emit_window_move(
        &self,
        window_id: u32,
        window_title: &str,
        from_workspace: Option<&str>,
        to_workspace: &str,
        from_output: Option<&str>,
        to_output: &str,
    );

    // ---- Workspace events ----

    /// Emit a workspace:activate event when a workspace becomes active
    fn emit_workspace_activate(&self, workspace_name: &str, workspace_idx: u32);

    /// Emit a workspace:deactivate event when a workspace becomes inactive
    fn emit_workspace_deactivate(&self, workspace_name: &str, workspace_idx: u32);

    /// Emit a workspace:create event when a new workspace is created
    fn emit_workspace_create(&self, workspace_name: &str, workspace_idx: u32, output: &str);

    /// Emit a workspace:destroy event when a workspace is destroyed
    fn emit_workspace_destroy(&self, workspace_name: &str, workspace_idx: u32, output: &str);

    /// Emit a workspace:rename event when a workspace is renamed
    fn emit_workspace_rename(
        &self,
        workspace_idx: u32,
        old_name: Option<&str>,
        new_name: Option<&str>,
        output: &str,
    );

    // ---- Layout events ----

    /// Emit a layout:mode_changed event when tiling/floating mode changes
    fn emit_layout_mode_changed(&self, is_floating: bool);

    /// Emit a layout:window_added event when a window is added to the layout
    fn emit_layout_window_added(&self, window_id: u32);

    /// Emit a layout:window_removed event when a window is removed from the layout
    fn emit_layout_window_removed(&self, window_id: u32);

    // ---- Config events ----

    /// Emit a config:reload event when the configuration is reloaded
    fn emit_config_reload(&self, success: bool);

    // ---- Overview events ----

    /// Emit an overview:open event when the overview is opened
    fn emit_overview_open(&self);

    /// Emit an overview:close event when the overview is closed
    fn emit_overview_close(&self);

    // ---- Idle events ----

    /// Emit an idle:start event when the system becomes idle
    fn emit_idle_start(&self);

    /// Emit an idle:end event when the system becomes active after idle
    fn emit_idle_end(&self);

    // ---- Lifecycle events ----

    /// Emit a startup event when the compositor finishes initializing
    fn emit_startup(&self);

    /// Emit a shutdown event when the compositor is about to shut down
    fn emit_shutdown(&self);

    // ---- Input events ----

    /// Emit a key:press event when a key is pressed
    fn emit_key_press(&self, key_name: &str, modifiers: &str, consumed: bool);

    /// Emit a key:release event when a key is released
    fn emit_key_release(&self, key_name: &str, modifiers: &str);
}

impl StateLuaEvents for State {
    fn emit_window_open(&self, window_id: u32, window_title: &str) {
        emit_event(self, "window:open", |lua| {
            create_window_table(lua, window_id, window_title)
        });
    }

    fn emit_window_close(&self, window_id: u32, window_title: &str) {
        emit_event(self, "window:close", |lua| {
            create_window_table(lua, window_id, window_title)
        });
    }

    fn emit_window_focus(&self, window_id: u32, window_title: &str) {
        emit_event(self, "window:focus", |lua| {
            create_window_table(lua, window_id, window_title)
        });
    }

    fn emit_window_blur(&self, window_id: u32, window_title: &str) {
        emit_event(self, "window:blur", |lua| {
            create_window_table(lua, window_id, window_title)
        });
    }

    fn emit_window_title_changed(&self, window_id: u32, new_title: &str) {
        emit_event(self, "window:title_changed", |lua| {
            create_window_table(lua, window_id, new_title)
        });
    }

    fn emit_window_app_id_changed(&self, window_id: u32, new_app_id: &str) {
        emit_event(self, "window:app_id_changed", |lua| {
            let table = lua.create_table()?;
            table.set("id", window_id)?;
            table.set("app_id", new_app_id)?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_window_fullscreen(&self, window_id: u32, window_title: &str, is_fullscreen: bool) {
        emit_event(self, "window:fullscreen", |lua| {
            let table = lua.create_table()?;
            table.set("id", window_id)?;
            table.set("title", window_title)?;
            table.set("is_fullscreen", is_fullscreen)?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_window_maximize(&self, window_id: u32, window_title: &str, is_maximized: bool) {
        emit_event(self, "window:maximize", |lua| {
            let table = lua.create_table()?;
            table.set("id", window_id)?;
            table.set("title", window_title)?;
            table.set("is_maximized", is_maximized)?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_window_resize(&self, window_id: u32, window_title: &str, width: i32, height: i32) {
        emit_event(self, "window:resize", |lua| {
            let table = lua.create_table()?;
            table.set("id", window_id)?;
            table.set("title", window_title)?;
            table.set("width", width)?;
            table.set("height", height)?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_window_move(
        &self,
        window_id: u32,
        window_title: &str,
        from_workspace: Option<&str>,
        to_workspace: &str,
        from_output: Option<&str>,
        to_output: &str,
    ) {
        emit_event(self, "window:move", |lua| {
            let table = lua.create_table()?;
            table.set("id", window_id)?;
            table.set("title", window_title)?;
            if let Some(from_ws) = from_workspace {
                table.set("from_workspace", from_ws)?;
            }
            table.set("to_workspace", to_workspace)?;
            if let Some(from_out) = from_output {
                table.set("from_output", from_out)?;
            }
            table.set("to_output", to_output)?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_workspace_activate(&self, workspace_name: &str, workspace_idx: u32) {
        tracing::debug!(
            "Emitting workspace:activate event: name={:?}, idx={}",
            workspace_name,
            workspace_idx
        );
        emit_event(self, "workspace:activate", |lua| {
            create_workspace_table(lua, workspace_name, workspace_idx)
        });
    }

    fn emit_workspace_deactivate(&self, workspace_name: &str, workspace_idx: u32) {
        emit_event(self, "workspace:deactivate", |lua| {
            create_workspace_table(lua, workspace_name, workspace_idx)
        });
    }

    fn emit_workspace_create(&self, workspace_name: &str, workspace_idx: u32, output: &str) {
        emit_event(self, "workspace:create", |lua| {
            let table = lua.create_table()?;
            table.set("name", workspace_name)?;
            table.set("idx", workspace_idx)?;
            table.set("output", output)?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_workspace_destroy(&self, workspace_name: &str, workspace_idx: u32, output: &str) {
        emit_event(self, "workspace:destroy", |lua| {
            let table = lua.create_table()?;
            table.set("name", workspace_name)?;
            table.set("idx", workspace_idx)?;
            table.set("output", output)?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_workspace_rename(
        &self,
        workspace_idx: u32,
        old_name: Option<&str>,
        new_name: Option<&str>,
        output: &str,
    ) {
        emit_event(self, "workspace:rename", |lua| {
            let table = lua.create_table()?;
            table.set("idx", workspace_idx)?;
            if let Some(old) = old_name {
                table.set("old_name", old)?;
            }
            if let Some(new) = new_name {
                table.set("new_name", new)?;
            }
            table.set("output", output)?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_layout_mode_changed(&self, is_floating: bool) {
        emit_event(self, "layout:mode_changed", |lua| {
            let table = lua.create_table()?;
            table.set("mode", if is_floating { "floating" } else { "tiling" })?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_layout_window_added(&self, window_id: u32) {
        emit_event(self, "layout:window_added", |lua| {
            let table = lua.create_table()?;
            table.set("id", window_id)?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_layout_window_removed(&self, window_id: u32) {
        emit_event(self, "layout:window_removed", |lua| {
            let table = lua.create_table()?;
            table.set("id", window_id)?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_config_reload(&self, success: bool) {
        emit_event(self, "config:reload", |lua| {
            let table = lua.create_table()?;
            table.set("success", success)?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_overview_open(&self) {
        emit_event(self, "overview:open", create_empty_table);
    }

    fn emit_overview_close(&self) {
        emit_event(self, "overview:close", create_empty_table);
    }

    fn emit_idle_start(&self) {
        emit_event(self, "idle:start", create_empty_table);
    }

    fn emit_idle_end(&self) {
        emit_event(self, "idle:end", create_empty_table);
    }

    fn emit_startup(&self) {
        emit_event(self, "startup", create_empty_table);
    }

    fn emit_shutdown(&self) {
        emit_event(self, "shutdown", create_empty_table);
    }

    fn emit_key_press(&self, key_name: &str, modifiers: &str, consumed: bool) {
        emit_event(self, "key:press", |lua| {
            let table = lua.create_table()?;
            table.set("key", key_name)?;
            table.set("modifiers", modifiers)?;
            table.set("consumed", consumed)?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_key_release(&self, key_name: &str, modifiers: &str) {
        emit_event(self, "key:release", |lua| {
            let table = lua.create_table()?;
            table.set("key", key_name)?;
            table.set("modifiers", modifiers)?;
            Ok(LuaValue::Table(table))
        });
    }
}

// ============================================================================
// Extension trait for Niri - for events emitted when full State is unavailable
// ============================================================================

/// Extension trait for emitting Lua events from Niri.
///
/// Some events are emitted from contexts where only `Niri` is available (e.g., monitor
/// connect/disconnect during backend initialization). This trait handles those cases.
pub trait NiriLuaEvents {
    /// Emit a monitor:connect event when a monitor is connected
    fn emit_monitor_connect(&self, monitor_name: &str, connector_name: &str);

    /// Emit a monitor:disconnect event when a monitor is disconnected
    fn emit_monitor_disconnect(&self, monitor_name: &str, connector_name: &str);

    /// Emit an output:mode_change event when an output's mode changes
    fn emit_output_mode_change(
        &self,
        output_name: &str,
        width: i32,
        height: i32,
        refresh_rate: Option<f64>,
    );

    /// Emit a lock:activate event when the screen is locked
    fn emit_lock_activate(&self);

    /// Emit a lock:deactivate event when the screen is unlocked
    fn emit_lock_deactivate(&self);
}

impl NiriLuaEvents for Niri {
    fn emit_monitor_connect(&self, monitor_name: &str, connector_name: &str) {
        emit_with_niri_context(self, "monitor:connect", |lua| {
            create_monitor_table(lua, monitor_name, connector_name)
        });
    }

    fn emit_monitor_disconnect(&self, monitor_name: &str, connector_name: &str) {
        emit_with_niri_context(self, "monitor:disconnect", |lua| {
            create_monitor_table(lua, monitor_name, connector_name)
        });
    }

    fn emit_output_mode_change(
        &self,
        output_name: &str,
        width: i32,
        height: i32,
        refresh_rate: Option<f64>,
    ) {
        emit_with_niri_context(self, "output:mode_change", |lua| {
            let table = lua.create_table()?;
            table.set("output", output_name)?;
            table.set("width", width)?;
            table.set("height", height)?;
            if let Some(refresh) = refresh_rate {
                table.set("refresh_rate", refresh)?;
            }
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_lock_activate(&self) {
        emit_with_niri_context(self, "lock:activate", create_empty_table);
    }

    fn emit_lock_deactivate(&self) {
        emit_with_niri_context(self, "lock:deactivate", create_empty_table);
    }
}
