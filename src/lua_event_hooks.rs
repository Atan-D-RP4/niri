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
use niri_lua::{set_scoped_state_active, CompositorState, SCOPED_STATE_GLOBAL_KEY};

use crate::niri::{Niri, State};

// ============================================================================
// Internal helpers for event emission
// ============================================================================

/// Helper to emit an event with live scoped state access via lua.scope().
///
/// This uses mlua's scope API to create non-static userdata that directly borrows `&State`.
/// Event handlers get live state access (not a stale snapshot), and Rust's borrow checker
/// ensures callbacks cannot retain references beyond the scope.
///
/// Benefits over snapshot-based approach:
/// - Live data: queries return current state, not pre-captured snapshot
/// - No cloning: avoids Vec cloning overhead for windows/workspaces/outputs
/// - Lifetime safety: Rust enforces callbacks can't store references
fn emit_with_scoped_state<F>(state: &State, event_name: &str, create_data: F)
where
    F: FnOnce(&Lua) -> LuaResult<LuaValue>,
{
    if let Some(lua_runtime) = &state.niri.lua_runtime {
        if let Some(event_system) = &lua_runtime.event_system {
            let lua = lua_runtime.inner();

            let result = lua.scope(|scope| {
                let state_table = lua.create_table()?;

                let windows_fn = scope.create_function(|lua, ()| {
                    let windows = state.get_windows();
                    let result = lua.create_table()?;
                    for (i, w) in windows.iter().enumerate() {
                        let t = lua.create_table()?;
                        t.set("id", w.id)?;
                        t.set("title", w.title.clone())?;
                        t.set("app_id", w.app_id.clone())?;
                        t.set("workspace_id", w.workspace_id)?;
                        t.set("is_focused", w.is_focused)?;
                        result.set(i + 1, t)?;
                    }
                    Ok(result)
                })?;
                state_table.set("windows", windows_fn)?;

                let focused_window_fn =
                    scope.create_function(|lua, ()| match state.get_focused_window() {
                        Some(w) => {
                            let t = lua.create_table()?;
                            t.set("id", w.id)?;
                            t.set("title", w.title.clone())?;
                            t.set("app_id", w.app_id.clone())?;
                            t.set("workspace_id", w.workspace_id)?;
                            t.set("is_focused", w.is_focused)?;
                            Ok(LuaValue::Table(t))
                        }
                        None => Ok(LuaValue::Nil),
                    })?;
                state_table.set("focused_window", focused_window_fn)?;

                let workspaces_fn = scope.create_function(|lua, ()| {
                    let workspaces = state.get_workspaces();
                    let result = lua.create_table()?;
                    for (i, ws) in workspaces.iter().enumerate() {
                        let t = lua.create_table()?;
                        t.set("id", ws.id)?;
                        t.set("idx", ws.idx)?;
                        t.set("name", ws.name.clone())?;
                        t.set("output", ws.output.clone())?;
                        t.set("is_active", ws.is_active)?;
                        t.set("is_focused", ws.is_focused)?;
                        result.set(i + 1, t)?;
                    }
                    Ok(result)
                })?;
                state_table.set("workspaces", workspaces_fn)?;

                let outputs_fn = scope.create_function(|lua, ()| {
                    let outputs = state.get_outputs();
                    let result = lua.create_table()?;
                    for (i, out) in outputs.iter().enumerate() {
                        let t = lua.create_table()?;
                        t.set("name", out.name.clone())?;
                        t.set("make", out.make.clone())?;
                        t.set("model", out.model.clone())?;
                        if let Some(logical) = &out.logical {
                            t.set("width", logical.width)?;
                            t.set("height", logical.height)?;
                            t.set("scale", logical.scale)?;
                        }
                        result.set(i + 1, t)?;
                    }
                    Ok(result)
                })?;
                state_table.set("outputs", outputs_fn)?;

                let keyboard_layouts_fn =
                    scope.create_function(|lua, ()| match state.get_keyboard_layouts() {
                        Some(layouts) => {
                            let t = lua.create_table()?;
                            let names_table = lua.create_table()?;
                            for (i, name) in layouts.names.iter().enumerate() {
                                names_table.set(i + 1, name.as_str())?;
                            }
                            t.set("names", names_table)?;
                            t.set("current_idx", layouts.current_idx)?;
                            Ok(LuaValue::Table(t))
                        }
                        None => Ok(LuaValue::Nil),
                    })?;
                state_table.set("keyboard_layouts", keyboard_layouts_fn)?;

                let cursor_position_fn =
                    scope.create_function(|lua, ()| match state.get_cursor_position() {
                        Some(pos) => {
                            let t = lua.create_table()?;
                            t.set("x", pos.x)?;
                            t.set("y", pos.y)?;
                            t.set("output", pos.output.clone())?;
                            Ok(LuaValue::Table(t))
                        }
                        None => Ok(LuaValue::Nil),
                    })?;
                state_table.set("cursor_position", cursor_position_fn)?;

                let focus_mode_fn = scope.create_function(|_, ()| {
                    let mode = state.get_focus_mode();
                    Ok(match mode {
                        niri_lua::FocusMode::Normal => "normal",
                        niri_lua::FocusMode::Overview => "overview",
                        niri_lua::FocusMode::LayerShell => "layer_shell",
                        niri_lua::FocusMode::Locked => "locked",
                    })
                })?;
                state_table.set("focus_mode", focus_mode_fn)?;

                let reserved_space_fn = scope.create_function(|lua, output_name: String| {
                    let reserved = state.get_reserved_space(&output_name);
                    let t = lua.create_table()?;
                    t.set("top", reserved.top)?;
                    t.set("bottom", reserved.bottom)?;
                    t.set("left", reserved.left)?;
                    t.set("right", reserved.right)?;
                    Ok(t)
                })?;
                state_table.set("reserved_space", reserved_space_fn)?;

                lua.globals().set(SCOPED_STATE_GLOBAL_KEY, state_table)?;
                set_scoped_state_active(true);

                let event_result = create_data(lua)
                    .and_then(|lua_value| event_system.emit(lua, event_name, lua_value));

                set_scoped_state_active(false);
                lua.globals().set(SCOPED_STATE_GLOBAL_KEY, LuaValue::Nil)?;

                event_result
            });

            if let Err(e) = result {
                debug!("Failed to emit {} event: {}", event_name, e);
            }
        }
    }
}

/// Helper to emit an event with Niri-only context (when full State is unavailable).
/// Uses scoped state with empty accessors since we don't have full compositor state.
fn emit_with_niri_context<F>(niri: &Niri, event_name: &str, create_data: F)
where
    F: FnOnce(&Lua) -> LuaResult<LuaValue>,
{
    if let Some(lua_runtime) = &niri.lua_runtime {
        if let Some(event_system) = &lua_runtime.event_system {
            let lua = lua_runtime.inner();

            let result = lua.scope(|scope| {
                let state_table = lua.create_table()?;

                let empty_list_fn = scope.create_function(|lua, ()| lua.create_table())?;
                let nil_fn = scope.create_function(|_, ()| Ok(LuaValue::Nil))?;

                state_table.set("windows", empty_list_fn.clone())?;
                state_table.set("workspaces", empty_list_fn.clone())?;
                state_table.set("outputs", empty_list_fn)?;
                state_table.set("focused_window", nil_fn)?;

                lua.globals().set(SCOPED_STATE_GLOBAL_KEY, state_table)?;
                set_scoped_state_active(true);

                let event_result = create_data(lua)
                    .and_then(|lua_value| event_system.emit(lua, event_name, lua_value));

                set_scoped_state_active(false);
                lua.globals().set(SCOPED_STATE_GLOBAL_KEY, LuaValue::Nil)?;

                event_result
            });

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
        emit_with_scoped_state(self, "window:open", |lua| {
            create_window_table(lua, window_id, window_title)
        });
    }

    fn emit_window_close(&self, window_id: u32, window_title: &str) {
        emit_with_scoped_state(self, "window:close", |lua| {
            create_window_table(lua, window_id, window_title)
        });
    }

    fn emit_window_focus(&self, window_id: u32, window_title: &str) {
        emit_with_scoped_state(self, "window:focus", |lua| {
            create_window_table(lua, window_id, window_title)
        });
    }

    fn emit_window_blur(&self, window_id: u32, window_title: &str) {
        emit_with_scoped_state(self, "window:blur", |lua| {
            create_window_table(lua, window_id, window_title)
        });
    }

    fn emit_window_title_changed(&self, window_id: u32, new_title: &str) {
        emit_with_scoped_state(self, "window:title_changed", |lua| {
            create_window_table(lua, window_id, new_title)
        });
    }

    fn emit_window_app_id_changed(&self, window_id: u32, new_app_id: &str) {
        emit_with_scoped_state(self, "window:app_id_changed", |lua| {
            let table = lua.create_table()?;
            table.set("id", window_id)?;
            table.set("app_id", new_app_id)?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_window_fullscreen(&self, window_id: u32, window_title: &str, is_fullscreen: bool) {
        emit_with_scoped_state(self, "window:fullscreen", |lua| {
            let table = lua.create_table()?;
            table.set("id", window_id)?;
            table.set("title", window_title)?;
            table.set("is_fullscreen", is_fullscreen)?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_window_maximize(&self, window_id: u32, window_title: &str, is_maximized: bool) {
        emit_with_scoped_state(self, "window:maximize", |lua| {
            let table = lua.create_table()?;
            table.set("id", window_id)?;
            table.set("title", window_title)?;
            table.set("is_maximized", is_maximized)?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_window_resize(&self, window_id: u32, window_title: &str, width: i32, height: i32) {
        emit_with_scoped_state(self, "window:resize", |lua| {
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
        emit_with_scoped_state(self, "window:move", |lua| {
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
        emit_with_scoped_state(self, "workspace:activate", |lua| {
            create_workspace_table(lua, workspace_name, workspace_idx)
        });
    }

    fn emit_workspace_deactivate(&self, workspace_name: &str, workspace_idx: u32) {
        emit_with_scoped_state(self, "workspace:deactivate", |lua| {
            create_workspace_table(lua, workspace_name, workspace_idx)
        });
    }

    fn emit_workspace_create(&self, workspace_name: &str, workspace_idx: u32, output: &str) {
        emit_with_scoped_state(self, "workspace:create", |lua| {
            let table = lua.create_table()?;
            table.set("name", workspace_name)?;
            table.set("idx", workspace_idx)?;
            table.set("output", output)?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_workspace_destroy(&self, workspace_name: &str, workspace_idx: u32, output: &str) {
        emit_with_scoped_state(self, "workspace:destroy", |lua| {
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
        emit_with_scoped_state(self, "workspace:rename", |lua| {
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
        emit_with_scoped_state(self, "layout:mode_changed", |lua| {
            let table = lua.create_table()?;
            table.set("mode", if is_floating { "floating" } else { "tiling" })?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_layout_window_added(&self, window_id: u32) {
        emit_with_scoped_state(self, "layout:window_added", |lua| {
            let table = lua.create_table()?;
            table.set("id", window_id)?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_layout_window_removed(&self, window_id: u32) {
        emit_with_scoped_state(self, "layout:window_removed", |lua| {
            let table = lua.create_table()?;
            table.set("id", window_id)?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_config_reload(&self, success: bool) {
        emit_with_scoped_state(self, "config:reload", |lua| {
            let table = lua.create_table()?;
            table.set("success", success)?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_overview_open(&self) {
        emit_with_scoped_state(self, "overview:open", create_empty_table);
    }

    fn emit_overview_close(&self) {
        emit_with_scoped_state(self, "overview:close", create_empty_table);
    }

    fn emit_idle_start(&self) {
        emit_with_scoped_state(self, "idle:start", create_empty_table);
    }

    fn emit_idle_end(&self) {
        emit_with_scoped_state(self, "idle:end", create_empty_table);
    }

    fn emit_startup(&self) {
        emit_with_scoped_state(self, "startup", create_empty_table);
    }

    fn emit_shutdown(&self) {
        emit_with_scoped_state(self, "shutdown", create_empty_table);
    }

    fn emit_key_press(&self, key_name: &str, modifiers: &str, consumed: bool) {
        emit_with_scoped_state(self, "key:press", |lua| {
            let table = lua.create_table()?;
            table.set("key", key_name)?;
            table.set("modifiers", modifiers)?;
            table.set("consumed", consumed)?;
            Ok(LuaValue::Table(table))
        });
    }

    fn emit_key_release(&self, key_name: &str, modifiers: &str) {
        emit_with_scoped_state(self, "key:release", |lua| {
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
