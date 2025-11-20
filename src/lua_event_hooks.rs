//! Lua event emission helpers for Niri core.
//!
//! This module provides helper functions to emit events from Niri's compositor logic
//! to the Lua runtime. Each function takes the necessary context and converts it into
//! event data for Lua handlers.

use log::debug;
use mlua::prelude::*;

use crate::niri::{Niri, State};

/// Emit a window:open event
///
/// Call this when a window is created and mapped to the layout
pub fn emit_window_open(state: &State, window_id: u32, window_title: &str) {
    if let Some(lua_runtime) = &state.niri.lua_runtime {
        if let Some(event_system) = &lua_runtime.event_system {
            let lua = lua_runtime.inner();
            match create_window_event_table(lua, window_id, window_title) {
                Ok(lua_value) => {
                    if let Err(e) = event_system.emit("window:open", lua_value) {
                        debug!("Failed to emit window:open event: {}", e);
                    }
                }
                Err(e) => {
                    debug!("Failed to create window:open data: {}", e);
                }
            }
        }
    }
}

/// Emit a window:close event
///
/// Call this when a window is destroyed
pub fn emit_window_close(state: &State, window_id: u32, window_title: &str) {
    if let Some(lua_runtime) = &state.niri.lua_runtime {
        if let Some(event_system) = &lua_runtime.event_system {
            let lua = lua_runtime.inner();
            match create_window_event_table(lua, window_id, window_title) {
                Ok(lua_value) => {
                    if let Err(e) = event_system.emit("window:close", lua_value) {
                        debug!("Failed to emit window:close event: {}", e);
                    }
                }
                Err(e) => {
                    debug!("Failed to create window:close data: {}", e);
                }
            }
        }
    }
}

/// Emit a window:focus event
///
/// Call this when a window receives focus
pub fn emit_window_focus(state: &State, window_id: u32, window_title: &str) {
    if let Some(lua_runtime) = &state.niri.lua_runtime {
        if let Some(event_system) = &lua_runtime.event_system {
            let lua = lua_runtime.inner();
            match create_window_event_table(lua, window_id, window_title) {
                Ok(lua_value) => {
                    if let Err(e) = event_system.emit("window:focus", lua_value) {
                        debug!("Failed to emit window:focus event: {}", e);
                    }
                }
                Err(e) => {
                    debug!("Failed to create window:focus data: {}", e);
                }
            }
        }
    }
}

/// Emit a window:blur event
///
/// Call this when a window loses focus
pub fn emit_window_blur(state: &State, window_id: u32, window_title: &str) {
    if let Some(lua_runtime) = &state.niri.lua_runtime {
        if let Some(event_system) = &lua_runtime.event_system {
            let lua = lua_runtime.inner();
            match create_window_event_table(lua, window_id, window_title) {
                Ok(lua_value) => {
                    if let Err(e) = event_system.emit("window:blur", lua_value) {
                        debug!("Failed to emit window:blur event: {}", e);
                    }
                }
                Err(e) => {
                    debug!("Failed to create window:blur data: {}", e);
                }
            }
        }
    }
}

/// Emit a workspace:activate event
///
/// Call this when a workspace becomes active
pub fn emit_workspace_activate(state: &State, workspace_name: &str, workspace_idx: u32) {
    if let Some(lua_runtime) = &state.niri.lua_runtime {
        if let Some(event_system) = &lua_runtime.event_system {
            let lua = lua_runtime.inner();
            match create_workspace_event_table(lua, workspace_name, workspace_idx, true) {
                Ok(lua_value) => {
                    if let Err(e) = event_system.emit("workspace:activate", lua_value) {
                        debug!("Failed to emit workspace:activate event: {}", e);
                    }
                }
                Err(e) => {
                    debug!("Failed to create workspace:activate data: {}", e);
                }
            }
        }
    }
}

/// Emit a workspace:deactivate event
///
/// Call this when a workspace becomes inactive
pub fn emit_workspace_deactivate(state: &State, workspace_name: &str, workspace_idx: u32) {
    if let Some(lua_runtime) = &state.niri.lua_runtime {
        if let Some(event_system) = &lua_runtime.event_system {
            let lua = lua_runtime.inner();
            match create_workspace_event_table(lua, workspace_name, workspace_idx, false) {
                Ok(lua_value) => {
                    if let Err(e) = event_system.emit("workspace:deactivate", lua_value) {
                        debug!("Failed to emit workspace:deactivate event: {}", e);
                    }
                }
                Err(e) => {
                    debug!("Failed to create workspace:deactivate data: {}", e);
                }
            }
        }
    }
}

/// Emit a monitor:connect event
///
/// Call this when a monitor is connected
pub fn emit_monitor_connect(niri: &Niri, monitor_name: &str, connector_name: &str) {
    if let Some(lua_runtime) = &niri.lua_runtime {
        if let Some(event_system) = &lua_runtime.event_system {
            let lua = lua_runtime.inner();
            match create_monitor_event_table(lua, monitor_name, connector_name, true) {
                Ok(lua_value) => {
                    if let Err(e) = event_system.emit("monitor:connect", lua_value) {
                        debug!("Failed to emit monitor:connect event: {}", e);
                    }
                }
                Err(e) => {
                    debug!("Failed to create monitor:connect data: {}", e);
                }
            }
        }
    }
}

/// Emit a monitor:disconnect event
///
/// Call this when a monitor is disconnected
pub fn emit_monitor_disconnect(niri: &Niri, monitor_name: &str, connector_name: &str) {
    if let Some(lua_runtime) = &niri.lua_runtime {
        if let Some(event_system) = &lua_runtime.event_system {
            let lua = lua_runtime.inner();
            match create_monitor_event_table(lua, monitor_name, connector_name, false) {
                Ok(lua_value) => {
                    if let Err(e) = event_system.emit("monitor:disconnect", lua_value) {
                        debug!("Failed to emit monitor:disconnect event: {}", e);
                    }
                }
                Err(e) => {
                    debug!("Failed to create monitor:disconnect data: {}", e);
                }
            }
        }
    }
}

/// Emit a layout:mode_changed event
///
/// Call this when tiling/floating mode changes
pub fn emit_layout_mode_changed(state: &State, is_floating: bool) {
    if let Some(lua_runtime) = &state.niri.lua_runtime {
        if let Some(event_system) = &lua_runtime.event_system {
            let lua = lua_runtime.inner();
            match create_layout_event_table(lua, if is_floating { "floating" } else { "tiling" }) {
                Ok(lua_value) => {
                    if let Err(e) = event_system.emit("layout:mode_changed", lua_value) {
                        debug!("Failed to emit layout:mode_changed event: {}", e);
                    }
                }
                Err(e) => {
                    debug!("Failed to create layout:mode_changed data: {}", e);
                }
            }
        }
    }
}

/// Emit a layout:window_added event
///
/// Call this when a window is added to the layout
pub fn emit_layout_window_added(state: &State, window_id: u32) {
    if let Some(lua_runtime) = &state.niri.lua_runtime {
        if let Some(event_system) = &lua_runtime.event_system {
            let lua = lua_runtime.inner();
            let event_type = format!("window_added:{}", window_id);
            match create_layout_event_table(lua, &event_type) {
                Ok(lua_value) => {
                    if let Err(e) = event_system.emit("layout:window_added", lua_value) {
                        debug!("Failed to emit layout:window_added event: {}", e);
                    }
                }
                Err(e) => {
                    debug!("Failed to create layout:window_added data: {}", e);
                }
            }
        }
    }
}

/// Emit a layout:window_removed event
///
/// Call this when a window is removed from the layout
pub fn emit_layout_window_removed(state: &State, window_id: u32) {
    if let Some(lua_runtime) = &state.niri.lua_runtime {
        if let Some(event_system) = &lua_runtime.event_system {
            let lua = lua_runtime.inner();
            let event_type = format!("window_removed:{}", window_id);
            match create_layout_event_table(lua, &event_type) {
                Ok(lua_value) => {
                    if let Err(e) = event_system.emit("layout:window_removed", lua_value) {
                        debug!("Failed to emit layout:window_removed event: {}", e);
                    }
                }
                Err(e) => {
                    debug!("Failed to create layout:window_removed data: {}", e);
                }
            }
        }
    }
}

// Helper functions to create Lua tables

fn create_window_event_table(lua: &Lua, id: u32, title: &str) -> LuaResult<LuaValue> {
    let table = lua.create_table()?;
    table.set("id", id)?;
    table.set("title", title)?;
    Ok(LuaValue::Table(table))
}

fn create_workspace_event_table(lua: &Lua, name: &str, index: u32, active: bool) -> LuaResult<LuaValue> {
    let table = lua.create_table()?;
    table.set("name", name)?;
    table.set("index", index)?;
    table.set("active", active)?;
    Ok(LuaValue::Table(table))
}

fn create_monitor_event_table(lua: &Lua, name: &str, connector: &str, connected: bool) -> LuaResult<LuaValue> {
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

