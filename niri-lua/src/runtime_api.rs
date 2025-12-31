//! Runtime state API for querying compositor state from Lua scripts.
//!
//! This module provides the `niri.state` API that allows Lua scripts to query the current
//! compositor state, including windows, workspaces, outputs, cursor position, and focus mode.
//!
//! ## Query API
//!
//! All collection queries support optional filter tables:
//!
//! ```lua
//! -- Get all windows
//! niri.state.windows()
//!
//! -- Filter by ID (O(1) lookup)
//! niri.state.windows({ id = 123 })
//!
//! -- Filter by focused state
//! niri.state.windows({ focused = true })
//!
//! -- Filter by app_id pattern (glob matching)
//! niri.state.windows({ app_id = "firefox*" })
//!
//! -- Filter by workspace name (exact match only)
//! niri.state.workspaces({ name = "dev" })
//! -- Filter by workspace output (exact match only)
//! niri.state.workspaces({ output = "HDMI-A-1" })
//!
//! -- Filter by output name (exact match only)
//! niri.state.outputs({ name = "DP-1" })
//!
//! -- Combine filters (AND logic)
//! niri.state.windows({ app_id = "kitty", focused = true })
//! ```
//!
//! ### Matching Behavior
//!
//! - **app_id, title**: Support glob patterns (`*`, `?`)
//! - **name, output (workspaces)**: Exact string match only
//! - **id**: Exact numeric match with O(1) lookup
//! - **focused**: Boolean filter (exact match)

use mlua::{Lua, Result, Table, Value};

use crate::ipc_bridge::{output_to_lua, window_to_lua, workspace_to_lua};
use crate::state_handle::{FocusMode, StateHandle};

fn get_state_handle(lua: &Lua) -> mlua::Result<StateHandle> {
    lua.app_data_ref::<StateHandle>()
        .map(|h| h.clone())
        .ok_or_else(|| mlua::Error::external("StateHandle not available"))
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let mut pattern_chars = pattern.chars().peekable();
    let mut text_chars = text.chars().peekable();

    while let Some(p) = pattern_chars.next() {
        match p {
            '*' => {
                if pattern_chars.peek().is_none() {
                    return true;
                }
                let remaining_pattern: String = pattern_chars.collect();
                for i in 0..=text_chars.clone().count() {
                    let remaining_text: String = text_chars.clone().skip(i).collect();
                    if glob_match(&remaining_pattern, &remaining_text) {
                        return true;
                    }
                }
                return false;
            }
            '?' => {
                if text_chars.next().is_none() {
                    return false;
                }
            }
            c => {
                if text_chars.next() != Some(c) {
                    return false;
                }
            }
        }
    }
    text_chars.next().is_none()
}

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

    // niri.state.windows(filter?) -> array of windows
    // filter: { id?, focused?, app_id?, title?, workspace_id? }
    state_table.set(
        "windows",
        lua.create_function(|lua, filter: Option<Table>| {
            let handle = get_state_handle(lua)?;

            // Fast path: lookup by ID
            if let Some(ref f) = filter {
                if let Ok(id) = f.get::<u64>("id") {
                    return match handle.window(id) {
                        Some(w) => Ok(vec![window_to_lua(lua, &w)?]),
                        None => Ok(vec![]),
                    };
                }
            }

            let windows = handle.windows();

            let filtered: Vec<_> = windows
                .into_iter()
                .filter(|w| {
                    if let Some(ref f) = filter {
                        // focused filter - use contains_key to avoid nil->false coercion
                        if f.contains_key("focused").unwrap_or(false) {
                            if let Ok(focused) = f.get::<bool>("focused") {
                                if w.is_focused != focused {
                                    return false;
                                }
                            }
                        }
                        // app_id filter (glob)
                        if let Ok(pattern) = f.get::<String>("app_id") {
                            let app_id = w.app_id.as_deref().unwrap_or("");
                            if !glob_match(&pattern, app_id) {
                                return false;
                            }
                        }
                        // title filter (glob)
                        if let Ok(pattern) = f.get::<String>("title") {
                            let title = w.title.as_deref().unwrap_or("");
                            if !glob_match(&pattern, title) {
                                return false;
                            }
                        }
                        // workspace_id filter
                        if let Ok(ws_id) = f.get::<u64>("workspace_id") {
                            if w.workspace_id != Some(ws_id) {
                                return false;
                            }
                        }
                    }
                    true
                })
                .collect();

            filtered
                .iter()
                .map(|w| window_to_lua(lua, w))
                .collect::<Result<Vec<_>>>()
        })?,
    )?;

    // niri.state.workspaces(filter?) -> array of workspaces
    // filter: { id?, name?, index?, active?, output? }
    state_table.set(
        "workspaces",
        lua.create_function(|lua, filter: Option<Table>| {
            let handle = get_state_handle(lua)?;

            // Fast path: lookup by ID
            if let Some(ref f) = filter {
                if let Ok(id) = f.get::<u64>("id") {
                    return match handle.workspace_by_id(id) {
                        Some(ws) => Ok(vec![workspace_to_lua(lua, &ws)?]),
                        None => Ok(vec![]),
                    };
                }
            }

            let workspaces = handle.workspaces();

            let filtered: Vec<_> = workspaces
                .into_iter()
                .filter(|ws| {
                    if let Some(ref f) = filter {
                        // name filter (exact match)
                        if let Ok(name) = f.get::<String>("name") {
                            if ws.name.as_deref() != Some(name.as_str()) {
                                return false;
                            }
                        }
                        // index filter
                        if let Ok(idx) = f.get::<u8>("index") {
                            if ws.idx != idx {
                                return false;
                            }
                        }
                        // active filter
                        if let Ok(Some(active)) = f.get::<Option<bool>>("active") {
                            if ws.is_active != active {
                                return false;
                            }
                        }
                        // focused filter
                        if let Ok(Some(focused)) = f.get::<Option<bool>>("focused") {
                            if ws.is_focused != focused {
                                return false;
                            }
                        }
                        if let Ok(output) = f.get::<String>("output") {
                            let ws_output = ws.output.as_deref().unwrap_or("");
                            if output != ws_output {
                                return false;
                            }
                        }
                    }
                    true
                })
                .collect();

            filtered
                .iter()
                .map(|ws| workspace_to_lua(lua, ws))
                .collect::<Result<Vec<_>>>()
        })?,
    )?;

    // niri.state.outputs(filter?) -> array of outputs
    // filter: { name?, focused? }
    state_table.set(
        "outputs",
        lua.create_function(|lua, filter: Option<Table>| {
            let handle = get_state_handle(lua)?;

            // Fast path: lookup by name
            if let Some(ref f) = filter {
                if let Ok(name) = f.get::<String>("name") {
                    // Exact match for single output lookup
                    if !name.contains('*') && !name.contains('?') {
                        return match handle.output_by_name(&name) {
                            Some(o) => Ok(vec![output_to_lua(lua, &o)?]),
                            None => Ok(vec![]),
                        };
                    }
                }
            }

            let outputs = handle.outputs();

            let filtered: Vec<_> = outputs
                .into_iter()
                .filter(|o| {
                    if let Some(ref f) = filter {
                        // name filter (exact match)
                        if let Ok(name) = f.get::<String>("name") {
                            if o.name != name {
                                return false;
                            }
                        }
                    }
                    true
                })
                .collect();

            filtered
                .iter()
                .map(|o| output_to_lua(lua, o))
                .collect::<Result<Vec<_>>>()
        })?,
    )?;

    // Convenience: niri.state.focused_window() -> window or nil
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

    // Convenience: niri.state.focused_workspace() -> workspace or nil
    state_table.set(
        "focused_workspace",
        lua.create_function(|lua, ()| {
            let handle = get_state_handle(lua)?;
            let workspaces = handle.workspaces();
            match workspaces.into_iter().find(|ws| ws.is_focused) {
                Some(ws) => Ok(Some(workspace_to_lua(lua, &ws)?)),
                None => Ok(None),
            }
        })?,
    )?;

    // Convenience: niri.state.focused_output() -> output or nil
    state_table.set(
        "focused_output",
        lua.create_function(|lua, ()| {
            let handle = get_state_handle(lua)?;
            // Get focused workspace's output
            let workspaces = handle.workspaces();
            let focused_ws = workspaces.into_iter().find(|ws| ws.is_focused);
            match focused_ws.and_then(|ws| ws.output) {
                Some(output_name) => match handle.output_by_name(&output_name) {
                    Some(o) => Ok(Some(output_to_lua(lua, &o)?)),
                    None => Ok(None),
                },
                None => Ok(None),
            }
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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use niri_ipc::state::EventStreamState;
    use niri_ipc::{Timestamp, Window, WindowLayout, Workspace};

    use super::*;
    use crate::StateHandle;

    fn make_window(id: u64, app_id: &str, title: &str, is_focused: bool) -> Window {
        Window {
            id,
            title: Some(title.to_string()),
            app_id: Some(app_id.to_string()),
            pid: Some(id as i32),
            workspace_id: Some(1),
            is_focused,
            is_floating: false,
            is_urgent: false,
            layout: WindowLayout {
                pos_in_scrolling_layout: Some((1, 1)),
                tile_size: (800.0, 600.0),
                window_size: (800, 600),
                tile_pos_in_workspace_view: Some((0.0, 0.0)),
                window_offset_in_tile: (0.0, 0.0),
            },
            focus_timestamp: Some(Timestamp { secs: 1, nanos: 0 }),
        }
    }

    fn make_workspace(id: u64, idx: u8, name: Option<&str>, is_active: bool) -> Workspace {
        Workspace {
            id,
            idx,
            name: name.map(|s| s.to_string()),
            output: Some("DP-1".to_string()),
            is_urgent: false,
            is_active,
            is_focused: is_active,
            active_window_id: None,
        }
    }

    fn create_test_lua() -> Lua {
        let lua = Lua::new();
        let mut state = EventStreamState::default();

        state
            .windows
            .windows
            .insert(1, make_window(1, "firefox", "Mozilla Firefox", false));
        state
            .windows
            .windows
            .insert(2, make_window(2, "kitty", "Terminal", true));
        state
            .windows
            .windows
            .insert(3, make_window(3, "firefox", "Settings", false));

        state
            .workspaces
            .workspaces
            .insert(1, make_workspace(1, 1, Some("main"), true));
        state
            .workspaces
            .workspaces
            .insert(2, make_workspace(2, 2, Some("dev"), false));

        let handle = StateHandle::new(Rc::new(RefCell::new(state)));
        handle.set_outputs(vec![niri_ipc::Output {
            name: "DP-1".to_string(),
            make: "Test".to_string(),
            model: "Monitor".to_string(),
            serial: None,
            physical_size: None,
            modes: vec![],
            current_mode: None,
            is_custom_mode: false,
            vrr_supported: false,
            vrr_enabled: false,
            logical: None,
        }]);
        lua.set_app_data(handle);
        register_runtime_api(&lua).unwrap();
        lua
    }

    #[test]
    fn test_windows_no_filter() {
        let lua = create_test_lua();
        let result: Vec<Table> = lua.load("return niri.state.windows()").eval().unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_windows_filter_by_id() {
        let lua = create_test_lua();
        let result: Vec<Table> = lua
            .load("return niri.state.windows({ id = 2 })")
            .eval()
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get::<u64>("id").unwrap(), 2);
    }

    #[test]
    fn test_windows_filter_by_focused() {
        let lua = create_test_lua();
        let result: Vec<Table> = lua
            .load("return niri.state.windows({ focused = true })")
            .eval()
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get::<String>("app_id").unwrap(), "kitty");
    }

    #[test]
    fn test_windows_filter_by_app_id_glob() {
        let lua = create_test_lua();
        let result: Vec<Table> = lua
            .load("return niri.state.windows({ app_id = 'fire*' })")
            .eval()
            .unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_windows_filter_combined() {
        let lua = create_test_lua();
        let result: Vec<Table> = lua
            .load("return niri.state.windows({ app_id = 'firefox', title = '*Settings*' })")
            .eval()
            .unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_workspaces_no_filter() {
        let lua = create_test_lua();
        let result: Vec<Table> = lua.load("return niri.state.workspaces()").eval().unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_workspaces_filter_by_name() {
        let lua = create_test_lua();
        let result: Vec<Table> = lua
            .load("return niri.state.workspaces({ name = 'dev' })")
            .eval()
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get::<String>("name").unwrap(), "dev");
    }

    #[test]
    fn test_outputs_filter_by_name() {
        let lua = create_test_lua();
        let result: Vec<Table> = lua
            .load("return niri.state.outputs({ name = 'DP-1' })")
            .eval()
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get::<String>("name").unwrap(), "DP-1");

        let result: Vec<Table> = lua
            .load("return niri.state.outputs({ name = 'DP' })")
            .eval()
            .unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_workspaces_filter_by_active() {
        let lua = create_test_lua();
        let result: Vec<Table> = lua
            .load("return niri.state.workspaces({ active = true })")
            .eval()
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get::<String>("name").unwrap(), "main");
    }

    #[test]
    fn test_focused_window() {
        let lua = create_test_lua();
        let result: Table = lua
            .load("return niri.state.focused_window()")
            .eval()
            .unwrap();
        assert_eq!(result.get::<String>("app_id").unwrap(), "kitty");
    }

    #[test]
    fn test_focused_workspace() {
        let lua = create_test_lua();
        let result: Table = lua
            .load("return niri.state.focused_workspace()")
            .eval()
            .unwrap();
        assert_eq!(result.get::<String>("name").unwrap(), "main");
    }

    #[test]
    fn test_glob_matching() {
        assert!(glob_match("fire*", "firefox"));
        assert!(glob_match("*fox", "firefox"));
        assert!(glob_match("*re*", "firefox"));
        assert!(glob_match("fire???", "firefox"));
        assert!(!glob_match("fire", "firefox"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("", ""));
        assert!(!glob_match("", "something"));
    }

    #[test]
    fn test_state_api_available() {
        let lua = create_test_lua();
        let result: Table = lua.load("return niri.state").eval().unwrap();
        assert!(result.get::<mlua::Function>("windows").is_ok());
        assert!(result.get::<mlua::Function>("workspaces").is_ok());
        assert!(result.get::<mlua::Function>("outputs").is_ok());
        assert!(result.get::<mlua::Function>("focused_window").is_ok());
        assert!(result.get::<mlua::Function>("focused_workspace").is_ok());
        assert!(result.get::<mlua::Function>("focused_output").is_ok());
        assert!(result.get::<mlua::Function>("keyboard_layouts").is_ok());
        assert!(result.get::<mlua::Function>("cursor_position").is_ok());
        assert!(result.get::<mlua::Function>("focus_mode").is_ok());
        assert!(result.get::<mlua::Function>("reserved_space").is_ok());
    }

    #[test]
    fn test_focus_mode_default() {
        let lua = create_test_lua();
        let result: String = lua.load("return niri.state.focus_mode()").eval().unwrap();
        assert_eq!(result, "normal");
    }
}
