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

            // Create event data and emit
            let result = create_data(lua).and_then(|lua_value| {
                event_system.emit(event_name, lua_value)
            });

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

            // Create event data and emit
            let result = create_data(lua).and_then(|lua_value| {
                event_system.emit(event_name, lua_value)
            });

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

/// Emit a window:close event
///
/// Call this when a window is destroyed
pub fn emit_window_close(state: &State, window_id: u32, window_title: &str) {
    emit_with_state_context(state, "window:close", |lua| {
        create_window_event_table(lua, window_id, window_title)
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
pub fn emit_window_fullscreen(state: &State, window_id: u32, window_title: &str, is_fullscreen: bool) {
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
pub fn emit_workspace_create(state: &State, workspace_name: &str, workspace_idx: u32, output: &str) {
    emit_with_state_context(state, "workspace:create", |lua| {
        create_workspace_lifecycle_table(lua, workspace_name, workspace_idx, output)
    });
}

/// Emit a workspace:destroy event
///
/// Call this when a workspace is destroyed
pub fn emit_workspace_destroy(state: &State, workspace_name: &str, workspace_idx: u32, output: &str) {
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
