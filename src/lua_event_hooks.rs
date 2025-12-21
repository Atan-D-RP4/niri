//! Lua event emission helpers for Niri core.
//!
//! This module provides helper functions to emit events from Niri's compositor logic
//! to the Lua runtime. Each function takes the necessary context and converts it into
//! event data for Lua handlers.

use log::debug;
use mlua::prelude::*;
use niri_lua::{clear_event_context_state, set_event_context_state, StateSnapshot};

use crate::niri::{Niri, State};

/// Helper to emit an event with state context.
///
/// This sets up the state snapshot in thread-local storage before emitting the event,
/// allowing `niri.state.*` functions to work inside event handlers without deadlocking.
fn emit_with_state_context<F>(state: &State, event_name: &str, create_data: F)
where
    F: FnOnce(&Lua) -> LuaResult<LuaValue>,
{
    if let Some(lua_runtime) = &state.niri.lua_runtime {
        if let Some(event_system) = &lua_runtime.event_system {
            let lua = lua_runtime.inner();

            // Capture state snapshot for use inside event handlers
            let snapshot = StateSnapshot::from_compositor_state(state);
            set_event_context_state(snapshot);

            // Create event data and emit (with timeout protection via lua parameter)
            let result = create_data(lua)
                .and_then(|lua_value| event_system.emit(lua, event_name, lua_value));

            // Always clear the context, even on error
            clear_event_context_state();

            if let Err(e) = result {
                debug!("Failed to emit {} event: {}", event_name, e);
            }
        }
    }
}

/// Helper to emit an event with state context (for Niri-only context).
fn emit_with_niri_context<F>(niri: &Niri, event_name: &str, create_data: F)
where
    F: FnOnce(&Lua) -> LuaResult<LuaValue>,
{
    if let Some(lua_runtime) = &niri.lua_runtime {
        if let Some(event_system) = &lua_runtime.event_system {
            let lua = lua_runtime.inner();

            // For Niri-only context, we create an empty snapshot since we don't have full State
            // This is acceptable for monitor events which don't typically need window state
            let snapshot = StateSnapshot::default();
            set_event_context_state(snapshot);

            // Create event data and emit (with timeout protection via lua parameter)
            let result = create_data(lua)
                .and_then(|lua_value| event_system.emit(lua, event_name, lua_value));

            // Always clear the context, even on error
            clear_event_context_state();

            if let Err(e) = result {
                debug!("Failed to emit {} event: {}", event_name, e);
            }
        }
    }
}

/// Emit a window:open event
///
/// Call this when a window is created and mapped to the layout
pub fn emit_window_open(state: &State, window_id: u32, window_title: &str) {
    emit_with_state_context(state, "window:open", |lua| {
        create_window_event_table(lua, window_id, window_title)
    });
}

/// Emit a window:open event with full window data including app_id
///
/// Call this when a window is created and mapped to the layout
pub fn emit_window_open_full(state: &State, window_id: u32, window_title: &str, app_id: &str) {
    emit_with_state_context(state, "window:open", |lua| {
        let table = lua.create_table()?;
        table.set("id", window_id)?;
        table.set("title", window_title)?;
        table.set("app_id", app_id)?;
        Ok(LuaValue::Table(table))
    });
}

/// Emit a window:close event
///
/// Call this when a window is destroyed
pub fn emit_window_close(state: &State, window_id: u32, window_title: &str) {
    emit_with_state_context(state, "window:close", |lua| {
        create_window_event_table(lua, window_id, window_title)
    });
}

/// Emit a window:close event with full window data including app_id
///
/// Call this when a window is destroyed
pub fn emit_window_close_full(state: &State, window_id: u32, window_title: &str, app_id: &str) {
    emit_with_state_context(state, "window:close", |lua| {
        let table = lua.create_table()?;
        table.set("id", window_id)?;
        table.set("title", window_title)?;
        table.set("app_id", app_id)?;
        Ok(LuaValue::Table(table))
    });
}

/// Emit a window:focus event
///
/// Call this when a window receives focus
pub fn emit_window_focus(state: &State, window_id: u32, window_title: &str) {
    emit_with_state_context(state, "window:focus", |lua| {
        create_window_event_table(lua, window_id, window_title)
    });
}

/// Emit a window:blur event
///
/// Call this when a window loses focus
pub fn emit_window_blur(state: &State, window_id: u32, window_title: &str) {
    emit_with_state_context(state, "window:blur", |lua| {
        create_window_event_table(lua, window_id, window_title)
    });
}

/// Emit a workspace:activate event
///
/// Call this when a workspace becomes active
pub fn emit_workspace_activate(state: &State, workspace_name: &str, workspace_idx: u32) {
    tracing::debug!(
        "Emitting workspace:activate event: name={:?}, idx={}",
        workspace_name,
        workspace_idx
    );
    emit_with_state_context(state, "workspace:activate", |lua| {
        create_workspace_event_table(lua, workspace_name, workspace_idx, true)
    });
}

/// Emit a workspace:deactivate event
///
/// Call this when a workspace becomes inactive
pub fn emit_workspace_deactivate(state: &State, workspace_name: &str, workspace_idx: u32) {
    emit_with_state_context(state, "workspace:deactivate", |lua| {
        create_workspace_event_table(lua, workspace_name, workspace_idx, false)
    });
}

/// Emit a monitor:connect event
///
/// Call this when a monitor is connected
pub fn emit_monitor_connect(niri: &Niri, monitor_name: &str, connector_name: &str) {
    emit_with_niri_context(niri, "monitor:connect", |lua| {
        create_monitor_event_table(lua, monitor_name, connector_name, true)
    });
}

/// Emit a monitor:disconnect event
///
/// Call this when a monitor is disconnected
pub fn emit_monitor_disconnect(niri: &Niri, monitor_name: &str, connector_name: &str) {
    emit_with_niri_context(niri, "monitor:disconnect", |lua| {
        create_monitor_event_table(lua, monitor_name, connector_name, false)
    });
}

/// Emit a layout:mode_changed event
///
/// Call this when tiling/floating mode changes
pub fn emit_layout_mode_changed(state: &State, is_floating: bool) {
    emit_with_state_context(state, "layout:mode_changed", |lua| {
        create_layout_event_table(lua, if is_floating { "floating" } else { "tiling" })
    });
}

/// Emit a layout:window_added event
///
/// Call this when a window is added to the layout
pub fn emit_layout_window_added(state: &State, window_id: u32) {
    let event_type = format!("window_added:{}", window_id);
    emit_with_state_context(state, "layout:window_added", |lua| {
        create_layout_event_table(lua, &event_type)
    });
}

/// Emit a layout:window_removed event
///
/// Call this when a window is removed from the layout
pub fn emit_layout_window_removed(state: &State, window_id: u32) {
    let event_type = format!("window_removed:{}", window_id);
    emit_with_state_context(state, "layout:window_removed", |lua| {
        create_layout_event_table(lua, &event_type)
    });
}

// Helper functions to create Lua tables

fn create_window_event_table(lua: &Lua, id: u32, title: &str) -> LuaResult<LuaValue> {
    let table = lua.create_table()?;
    table.set("id", id)?;
    table.set("title", title)?;
    Ok(LuaValue::Table(table))
}

fn create_workspace_event_table(
    lua: &Lua,
    name: &str,
    index: u32,
    active: bool,
) -> LuaResult<LuaValue> {
    let table = lua.create_table()?;
    table.set("name", name)?;
    table.set("index", index)?;
    table.set("active", active)?;
    Ok(LuaValue::Table(table))
}

fn create_monitor_event_table(
    lua: &Lua,
    name: &str,
    connector: &str,
    connected: bool,
) -> LuaResult<LuaValue> {
    let table = lua.create_table()?;
    table.set("name", name)?;
    table.set("connector", connector)?;
    table.set("connected", connected)?;
    Ok(LuaValue::Table(table))
}

fn create_layout_event_table(lua: &Lua, mode: &str) -> LuaResult<LuaValue> {
    let table = lua.create_table()?;
    table.set("mode", mode)?;
    Ok(LuaValue::Table(table))
}

// ===== Phase R6: New Events =====

/// Emit a window:title_changed event
///
/// Call this when a window's title changes
pub fn emit_window_title_changed(state: &State, window_id: u32, new_title: &str) {
    emit_with_state_context(state, "window:title_changed", |lua| {
        create_title_changed_table(lua, window_id, new_title)
    });
}

/// Emit a window:app_id_changed event
///
/// Call this when a window's app_id changes
pub fn emit_window_app_id_changed(state: &State, window_id: u32, new_app_id: &str) {
    emit_with_state_context(state, "window:app_id_changed", |lua| {
        create_app_id_changed_table(lua, window_id, new_app_id)
    });
}

/// Emit a window:fullscreen event
///
/// Call this when a window enters or exits fullscreen
pub fn emit_window_fullscreen(
    state: &State,
    window_id: u32,
    window_title: &str,
    is_fullscreen: bool,
) {
    emit_with_state_context(state, "window:fullscreen", |lua| {
        create_fullscreen_event_table(lua, window_id, window_title, is_fullscreen)
    });
}

/// Emit a window:move event
///
/// Call this when a window moves to a different workspace/monitor
pub fn emit_window_move(
    state: &State,
    window_id: u32,
    window_title: &str,
    from_workspace: Option<&str>,
    to_workspace: &str,
    from_output: Option<&str>,
    to_output: &str,
) {
    emit_with_state_context(state, "window:move", |lua| {
        create_window_move_table(
            lua,
            window_id,
            window_title,
            from_workspace,
            to_workspace,
            from_output,
            to_output,
        )
    });
}

/// Emit a workspace:create event
///
/// Call this when a new workspace is created
pub fn emit_workspace_create(
    state: &State,
    workspace_name: &str,
    workspace_idx: u32,
    output: &str,
) {
    emit_with_state_context(state, "workspace:create", |lua| {
        create_workspace_lifecycle_table(lua, workspace_name, workspace_idx, output)
    });
}

/// Emit a workspace:destroy event
///
/// Call this when a workspace is destroyed
pub fn emit_workspace_destroy(
    state: &State,
    workspace_name: &str,
    workspace_idx: u32,
    output: &str,
) {
    emit_with_state_context(state, "workspace:destroy", |lua| {
        create_workspace_lifecycle_table(lua, workspace_name, workspace_idx, output)
    });
}

/// Emit a workspace:rename event
///
/// Call this when a workspace is renamed
pub fn emit_workspace_rename(
    state: &State,
    workspace_idx: u32,
    old_name: Option<&str>,
    new_name: Option<&str>,
    output: &str,
) {
    emit_with_state_context(state, "workspace:rename", |lua| {
        create_workspace_rename_table(lua, workspace_idx, old_name, new_name, output)
    });
}

/// Emit a config:reload event
///
/// Call this when the configuration is reloaded
pub fn emit_config_reload(state: &State, success: bool) {
    emit_with_state_context(state, "config:reload", |lua| {
        create_config_reload_table(lua, success)
    });
}

/// Emit an overview:open event
///
/// Call this when the overview is opened
pub fn emit_overview_open(state: &State) {
    emit_with_state_context(state, "overview:open", |lua| {
        create_overview_event_table(lua, true)
    });
}

/// Emit an overview:close event
///
/// Call this when the overview is closed
pub fn emit_overview_close(state: &State) {
    emit_with_state_context(state, "overview:close", |lua| {
        create_overview_event_table(lua, false)
    });
}

// Helper functions for new events

fn create_title_changed_table(lua: &Lua, id: u32, new_title: &str) -> LuaResult<LuaValue> {
    let table = lua.create_table()?;
    table.set("id", id)?;
    table.set("title", new_title)?;
    Ok(LuaValue::Table(table))
}

fn create_app_id_changed_table(lua: &Lua, id: u32, new_app_id: &str) -> LuaResult<LuaValue> {
    let table = lua.create_table()?;
    table.set("id", id)?;
    table.set("app_id", new_app_id)?;
    Ok(LuaValue::Table(table))
}

fn create_fullscreen_event_table(
    lua: &Lua,
    id: u32,
    title: &str,
    is_fullscreen: bool,
) -> LuaResult<LuaValue> {
    let table = lua.create_table()?;
    table.set("id", id)?;
    table.set("title", title)?;
    table.set("is_fullscreen", is_fullscreen)?;
    Ok(LuaValue::Table(table))
}

fn create_window_move_table(
    lua: &Lua,
    id: u32,
    title: &str,
    from_workspace: Option<&str>,
    to_workspace: &str,
    from_output: Option<&str>,
    to_output: &str,
) -> LuaResult<LuaValue> {
    let table = lua.create_table()?;
    table.set("id", id)?;
    table.set("title", title)?;
    if let Some(from_ws) = from_workspace {
        table.set("from_workspace", from_ws)?;
    }
    table.set("to_workspace", to_workspace)?;
    if let Some(from_out) = from_output {
        table.set("from_output", from_out)?;
    }
    table.set("to_output", to_output)?;
    Ok(LuaValue::Table(table))
}

fn create_workspace_lifecycle_table(
    lua: &Lua,
    name: &str,
    index: u32,
    output: &str,
) -> LuaResult<LuaValue> {
    let table = lua.create_table()?;
    table.set("name", name)?;
    table.set("index", index)?;
    table.set("output", output)?;
    Ok(LuaValue::Table(table))
}

fn create_workspace_rename_table(
    lua: &Lua,
    index: u32,
    old_name: Option<&str>,
    new_name: Option<&str>,
    output: &str,
) -> LuaResult<LuaValue> {
    let table = lua.create_table()?;
    table.set("index", index)?;
    if let Some(old) = old_name {
        table.set("old_name", old)?;
    }
    if let Some(new) = new_name {
        table.set("new_name", new)?;
    }
    table.set("output", output)?;
    Ok(LuaValue::Table(table))
}

fn create_config_reload_table(lua: &Lua, success: bool) -> LuaResult<LuaValue> {
    let table = lua.create_table()?;
    table.set("success", success)?;
    Ok(LuaValue::Table(table))
}

fn create_overview_event_table(lua: &Lua, is_open: bool) -> LuaResult<LuaValue> {
    let table = lua.create_table()?;
    table.set("is_open", is_open)?;
    Ok(LuaValue::Table(table))
}

/// Emit a window:resize event
///
/// Call this when a window's size changes
pub fn emit_window_resize(
    state: &State,
    window_id: u32,
    window_title: &str,
    width: i32,
    height: i32,
) {
    emit_with_state_context(state, "window:resize", |lua| {
        let table = lua.create_table()?;
        table.set("id", window_id)?;
        table.set("title", window_title)?;
        table.set("width", width)?;
        table.set("height", height)?;
        Ok(LuaValue::Table(table))
    });
}

/// Emit a window:maximize event
///
/// Call this when a window is maximized or unmaximized
pub fn emit_window_maximize(state: &State, window_id: u32, window_title: &str, is_maximized: bool) {
    emit_with_state_context(state, "window:maximize", |lua| {
        let table = lua.create_table()?;
        table.set("id", window_id)?;
        table.set("title", window_title)?;
        table.set("is_maximized", is_maximized)?;
        Ok(LuaValue::Table(table))
    });
}

/// Emit an output:mode_change event
///
/// Call this when an output's mode (resolution, refresh rate) changes
pub fn emit_output_mode_change(
    niri: &Niri,
    output_name: &str,
    width: i32,
    height: i32,
    refresh_rate: Option<f64>,
) {
    emit_with_niri_context(niri, "output:mode_change", |lua| {
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

/// Emit an idle:start event
///
/// Call this when the system becomes idle
pub fn emit_idle_start(state: &State) {
    emit_with_state_context(state, "idle:start", |lua| {
        let table = lua.create_table()?;
        table.set("is_idle", true)?;
        Ok(LuaValue::Table(table))
    });
}

/// Emit an idle:end event
///
/// Call this when the system becomes active after idle
pub fn emit_idle_end(state: &State) {
    emit_with_state_context(state, "idle:end", |lua| {
        let table = lua.create_table()?;
        table.set("is_idle", false)?;
        Ok(LuaValue::Table(table))
    });
}

/// Emit a lock:activate event
///
/// Call this when the screen is locked
pub fn emit_lock_activate_niri(niri: &Niri) {
    emit_with_niri_context(niri, "lock:activate", |lua| {
        let table = lua.create_table()?;
        table.set("is_locked", true)?;
        Ok(LuaValue::Table(table))
    });
}

/// Emit a lock:deactivate event
///
/// Call this when the screen is unlocked
pub fn emit_lock_deactivate_niri(niri: &Niri) {
    emit_with_niri_context(niri, "lock:deactivate", |lua| {
        let table = lua.create_table()?;
        table.set("is_locked", false)?;
        Ok(LuaValue::Table(table))
    });
}

/// Emit a startup event
///
/// Call this when the compositor finishes initializing
pub fn emit_startup(state: &State) {
    emit_with_state_context(state, "startup", |lua| {
        let table = lua.create_table()?;
        Ok(LuaValue::Table(table))
    });
}

/// Emit a shutdown event
///
/// Call this when the compositor is about to shut down
pub fn emit_shutdown(state: &State) {
    emit_with_state_context(state, "shutdown", |lua| {
        let table = lua.create_table()?;
        Ok(LuaValue::Table(table))
    });
}

/// Emit a key:press event
///
/// Call this when a key is pressed (for hotkey monitoring)
pub fn emit_key_press(state: &State, key_name: &str, modifiers: &str, consumed: bool) {
    emit_with_state_context(state, "key:press", |lua| {
        let table = lua.create_table()?;
        table.set("key", key_name)?;
        table.set("modifiers", modifiers)?;
        table.set("consumed", consumed)?;
        Ok(LuaValue::Table(table))
    });
}

/// Emit a key:release event
///
/// Call this when a key is released
pub fn emit_key_release(state: &State, key_name: &str, modifiers: &str) {
    emit_with_state_context(state, "key:release", |lua| {
        let table = lua.create_table()?;
        table.set("key", key_name)?;
        table.set("modifiers", modifiers)?;
        Ok(LuaValue::Table(table))
    });
}

// ===== Phase 2: Extension Trait for cleaner event emission =====

/// Extension trait for emitting Lua events from State.
///
/// This trait provides method-style event emission, making call sites cleaner
/// and reducing the need to import individual emit_* functions.
///
/// # Example
/// ```ignore
/// use crate::lua_event_hooks::StateLuaEvents;
///
/// // Instead of:
/// lua_event_hooks::emit_window_focus(state, id, title);
///
/// // You can write:
/// state.emit_window_focus(id, title);
/// ```
pub trait StateLuaEvents {
    // Window events
    fn emit_window_open(&self, window_id: u32, window_title: &str);
    fn emit_window_open_full(&self, window_id: u32, window_title: &str, app_id: &str);
    fn emit_window_close(&self, window_id: u32, window_title: &str);
    fn emit_window_close_full(&self, window_id: u32, window_title: &str, app_id: &str);
    fn emit_window_focus(&self, window_id: u32, window_title: &str);
    fn emit_window_blur(&self, window_id: u32, window_title: &str);
    fn emit_window_title_changed(&self, window_id: u32, new_title: &str);
    fn emit_window_app_id_changed(&self, window_id: u32, new_app_id: &str);
    fn emit_window_fullscreen(&self, window_id: u32, window_title: &str, is_fullscreen: bool);
    fn emit_window_maximize(&self, window_id: u32, window_title: &str, is_maximized: bool);
    fn emit_window_resize(&self, window_id: u32, window_title: &str, width: i32, height: i32);
    fn emit_window_move(
        &self,
        window_id: u32,
        window_title: &str,
        from_workspace: Option<&str>,
        to_workspace: &str,
        from_output: Option<&str>,
        to_output: &str,
    );

    // Workspace events
    fn emit_workspace_activate(&self, workspace_name: &str, workspace_idx: u32);
    fn emit_workspace_deactivate(&self, workspace_name: &str, workspace_idx: u32);
    fn emit_workspace_create(&self, workspace_name: &str, workspace_idx: u32, output: &str);
    fn emit_workspace_destroy(&self, workspace_name: &str, workspace_idx: u32, output: &str);
    fn emit_workspace_rename(
        &self,
        workspace_idx: u32,
        old_name: Option<&str>,
        new_name: Option<&str>,
        output: &str,
    );

    // Layout events
    fn emit_layout_mode_changed(&self, is_floating: bool);
    fn emit_layout_window_added(&self, window_id: u32);
    fn emit_layout_window_removed(&self, window_id: u32);

    // Config events
    fn emit_config_reload(&self, success: bool);

    // Overview events
    fn emit_overview_open(&self);
    fn emit_overview_close(&self);

    // Lifecycle events
    fn emit_startup(&self);
    fn emit_shutdown(&self);
}

impl StateLuaEvents for State {
    fn emit_window_open(&self, window_id: u32, window_title: &str) {
        emit_window_open(self, window_id, window_title);
    }

    fn emit_window_open_full(&self, window_id: u32, window_title: &str, app_id: &str) {
        emit_window_open_full(self, window_id, window_title, app_id);
    }

    fn emit_window_close(&self, window_id: u32, window_title: &str) {
        emit_window_close(self, window_id, window_title);
    }

    fn emit_window_close_full(&self, window_id: u32, window_title: &str, app_id: &str) {
        emit_window_close_full(self, window_id, window_title, app_id);
    }

    fn emit_window_focus(&self, window_id: u32, window_title: &str) {
        emit_window_focus(self, window_id, window_title);
    }

    fn emit_window_blur(&self, window_id: u32, window_title: &str) {
        emit_window_blur(self, window_id, window_title);
    }

    fn emit_window_title_changed(&self, window_id: u32, new_title: &str) {
        emit_window_title_changed(self, window_id, new_title);
    }

    fn emit_window_app_id_changed(&self, window_id: u32, new_app_id: &str) {
        emit_window_app_id_changed(self, window_id, new_app_id);
    }

    fn emit_window_fullscreen(&self, window_id: u32, window_title: &str, is_fullscreen: bool) {
        emit_window_fullscreen(self, window_id, window_title, is_fullscreen);
    }

    fn emit_window_maximize(&self, window_id: u32, window_title: &str, is_maximized: bool) {
        emit_window_maximize(self, window_id, window_title, is_maximized);
    }

    fn emit_window_resize(&self, window_id: u32, window_title: &str, width: i32, height: i32) {
        emit_window_resize(self, window_id, window_title, width, height);
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
        emit_window_move(
            self,
            window_id,
            window_title,
            from_workspace,
            to_workspace,
            from_output,
            to_output,
        );
    }

    fn emit_workspace_activate(&self, workspace_name: &str, workspace_idx: u32) {
        emit_workspace_activate(self, workspace_name, workspace_idx);
    }

    fn emit_workspace_deactivate(&self, workspace_name: &str, workspace_idx: u32) {
        emit_workspace_deactivate(self, workspace_name, workspace_idx);
    }

    fn emit_workspace_create(&self, workspace_name: &str, workspace_idx: u32, output: &str) {
        emit_workspace_create(self, workspace_name, workspace_idx, output);
    }

    fn emit_workspace_destroy(&self, workspace_name: &str, workspace_idx: u32, output: &str) {
        emit_workspace_destroy(self, workspace_name, workspace_idx, output);
    }

    fn emit_workspace_rename(
        &self,
        workspace_idx: u32,
        old_name: Option<&str>,
        new_name: Option<&str>,
        output: &str,
    ) {
        emit_workspace_rename(self, workspace_idx, old_name, new_name, output);
    }

    fn emit_layout_mode_changed(&self, is_floating: bool) {
        emit_layout_mode_changed(self, is_floating);
    }

    fn emit_layout_window_added(&self, window_id: u32) {
        emit_layout_window_added(self, window_id);
    }

    fn emit_layout_window_removed(&self, window_id: u32) {
        emit_layout_window_removed(self, window_id);
    }

    fn emit_config_reload(&self, success: bool) {
        emit_config_reload(self, success);
    }

    fn emit_overview_open(&self) {
        emit_overview_open(self);
    }

    fn emit_overview_close(&self) {
        emit_overview_close(self);
    }

    fn emit_startup(&self) {
        emit_startup(self);
    }

    fn emit_shutdown(&self) {
        emit_shutdown(self);
    }
}

/// Extension trait for emitting Lua events from Niri (when full State is unavailable).
///
/// Some events are emitted from contexts where only `Niri` is available (e.g., monitor
/// connect/disconnect during backend initialization). This trait handles those cases.
pub trait NiriLuaEvents {
    fn emit_monitor_connect(&self, monitor_name: &str, connector_name: &str);
    fn emit_monitor_disconnect(&self, monitor_name: &str, connector_name: &str);
    fn emit_output_mode_change(
        &self,
        output_name: &str,
        width: i32,
        height: i32,
        refresh_rate: Option<f64>,
    );
    fn emit_lock_activate(&self);
    fn emit_lock_deactivate(&self);
}

impl NiriLuaEvents for Niri {
    fn emit_monitor_connect(&self, monitor_name: &str, connector_name: &str) {
        emit_monitor_connect(self, monitor_name, connector_name);
    }

    fn emit_monitor_disconnect(&self, monitor_name: &str, connector_name: &str) {
        emit_monitor_disconnect(self, monitor_name, connector_name);
    }

    fn emit_output_mode_change(
        &self,
        output_name: &str,
        width: i32,
        height: i32,
        refresh_rate: Option<f64>,
    ) {
        emit_output_mode_change(self, output_name, width, height, refresh_rate);
    }

    fn emit_lock_activate(&self) {
        emit_lock_activate_niri(self);
    }

    fn emit_lock_deactivate(&self) {
        emit_lock_deactivate_niri(self);
    }
}
