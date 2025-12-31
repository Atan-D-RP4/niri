//! Runtime state API for querying compositor state from Lua scripts.
//!
//! This module provides the `niri.state` API that allows Lua scripts to query the current
//! compositor state, including windows, workspaces, outputs, cursor position, and focus mode.
//!
//! ## Architecture
//!
//! State queries use `StateHandle` stored in Lua's `app_data`. The `StateHandle` is shared
//! with the compositor and provides live access to windows, workspaces, outputs, cursor
//! position, focus mode, and reserved space.

use mlua::{Lua, Result, Table, Value};

use crate::ipc_bridge::{output_to_lua, window_to_lua, windows_to_lua, workspaces_to_lua};
use crate::state_handle::StateHandle;

fn get_state_handle(lua: &Lua) -> mlua::Result<StateHandle> {
    lua.app_data_ref::<StateHandle>()
        .map(|h| h.clone())
        .ok_or_else(|| mlua::Error::external("StateHandle not available"))
}

/// Register the runtime state API in a Lua context.
///
/// Creates the `niri.state` table with functions for querying compositor state.
/// All functions use StateHandle from app_data for live state access.
pub fn register_runtime_api(lua: &Lua) -> Result<()> {
    let niri: Table = match lua.globals().get("niri")? {
        Value::Table(t) => t,
        _ => {
            let t = lua.create_table()?;
            lua.globals().set("niri", t.clone())?;
            t
        }
    };

    let state_table = lua.create_table()?;

    state_table.set(
        "windows",
        lua.create_function(|lua, ()| {
            let handle = get_state_handle(lua)?;
            let windows = handle.windows();
            windows_to_lua(lua, &windows)
        })?,
    )?;

    state_table.set(
        "focused_window",
        lua.create_function(|lua, ()| {
            let handle = get_state_handle(lua)?;
            match handle.focused_window() {
                Some(w) => Ok(Some(window_to_lua(lua, &w)?)),
                None => Ok(None),
            }
        })?,
    )?;

    state_table.set(
        "workspaces",
        lua.create_function(|lua, ()| {
            let handle = get_state_handle(lua)?;
            let workspaces = handle.workspaces();
            workspaces_to_lua(lua, &workspaces)
        })?,
    )?;

    state_table.set(
        "outputs",
        lua.create_function(|lua, ()| {
            let handle = get_state_handle(lua)?;
            let outputs = handle.outputs();
            let result: Vec<Table> = outputs
                .iter()
                .map(|o| output_to_lua(lua, o))
                .collect::<Result<_>>()?;
            Ok(result)
        })?,
    )?;

    state_table.set(
        "keyboard_layouts",
        lua.create_function(|lua, ()| {
            let handle = get_state_handle(lua)?;
            match handle.keyboard_layouts() {
                Some(kl) => {
                    let t = lua.create_table()?;
                    let names_table = lua.create_table()?;
                    for (i, name) in kl.names.iter().enumerate() {
                        names_table.set(i + 1, name.as_str())?;
                    }
                    t.set("names", names_table)?;
                    t.set("current_idx", kl.current_idx)?;
                    Ok(Some(t))
                }
                None => Ok(None),
            }
        })?,
    )?;

    state_table.set(
        "cursor_position",
        lua.create_function(|lua, ()| {
            let handle = get_state_handle(lua)?;
            match handle.cursor_position() {
                Some(pos) => {
                    let t = lua.create_table()?;
                    t.set("x", pos.x)?;
                    t.set("y", pos.y)?;
                    t.set("output", pos.output)?;
                    Ok(Some(t))
                }
                None => Ok(None),
            }
        })?,
    )?;

    state_table.set(
        "focus_mode",
        lua.create_function(|lua, ()| {
            let handle = get_state_handle(lua)?;
            let mode_str = match handle.focus_mode() {
                FocusMode::Normal => "normal",
                FocusMode::Overview => "overview",
                FocusMode::LayerShell => "layer_shell",
                FocusMode::Locked => "locked",
            };
            Ok(mode_str)
        })?,
    )?;

    state_table.set(
        "reserved_space",
        lua.create_function(|lua, output_name: String| {
            let handle = get_state_handle(lua)?;
            let reserved = handle.reserved_space(&output_name);
            let t = lua.create_table()?;
            t.set("top", reserved.top)?;
            t.set("bottom", reserved.bottom)?;
            t.set("left", reserved.left)?;
            t.set("right", reserved.right)?;
            Ok(t)
        })?,
    )?;

    niri.set("state", state_table)?;

    Ok(())
}

use crate::state_handle::FocusMode;

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use niri_ipc::state::EventStreamState;

    use super::*;
    use crate::StateHandle;

    fn create_test_lua() -> Lua {
        let lua = Lua::new();
        let handle = StateHandle::new(Rc::new(RefCell::new(EventStreamState::default())));
        lua.set_app_data(handle);
        register_runtime_api(&lua).unwrap();
        lua
    }

    #[test]
    fn test_state_api_available() {
        let lua = create_test_lua();
        let result: Table = lua.load("return niri.state").eval().unwrap();
        assert!(result.get::<mlua::Function>("windows").is_ok());
        assert!(result.get::<mlua::Function>("workspaces").is_ok());
        assert!(result.get::<mlua::Function>("outputs").is_ok());
        assert!(result.get::<mlua::Function>("focused_window").is_ok());
        assert!(result.get::<mlua::Function>("keyboard_layouts").is_ok());
        assert!(result.get::<mlua::Function>("cursor_position").is_ok());
        assert!(result.get::<mlua::Function>("focus_mode").is_ok());
        assert!(result.get::<mlua::Function>("reserved_space").is_ok());
    }

    #[test]
    fn test_windows_returns_empty() {
        let lua = create_test_lua();
        let result: Table = lua.load("return niri.state.windows()").eval().unwrap();
        assert_eq!(result.len().unwrap(), 0);
    }

    #[test]
    fn test_focus_mode_default() {
        let lua = create_test_lua();
        let result: String = lua.load("return niri.state.focus_mode()").eval().unwrap();
        assert_eq!(result, "normal");
    }
}
