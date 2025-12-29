//! Runtime state API for querying compositor state from Lua scripts.
//!
//! This module provides the `niri.state` API that allows Lua scripts to query the current
//! compositor state, including windows, workspaces, and outputs.
//!
//! ## Architecture
//!
//! All state queries use `lua.scope()` to create non-static userdata that directly borrows
//! `&State`. This provides live state access and Rust's borrow checker ensures callbacks
//! cannot retain references beyond the scope.
//!
//! State queries are only available within a scoped context (event handlers, timers with
//! `insert_idle`, IPC execution). Calling them outside a scoped context returns an error.

use std::cell::Cell;

use mlua::{Lua, Result, Table, Value};
use niri_ipc::{KeyboardLayouts, Output, Window, Workspace};

use crate::ipc_bridge::{output_to_lua, window_to_lua, windows_to_lua, workspaces_to_lua};

// Thread-local storage for state snapshot during event handler execution.

// This allows `niri.state.*` functions to access pre-captured state data
// when called from within event handlers, avoiding the deadlock that would
// occur with the idle callback pattern.
pub const SCOPED_STATE_GLOBAL_KEY: &str = "__niri_scoped_state";

// Thread-local storage for scoped state active flag.
thread_local! {
    static SCOPED_STATE_ACTIVE: Cell<bool> = const { Cell::new(false) };
}

/// Set whether scoped state is currently active.
///
/// This is used by query functions to determine if they should use the
/// scoped state table or fall back to other methods.
pub fn set_scoped_state_active(active: bool) {
    SCOPED_STATE_ACTIVE.with(|cell| cell.set(active));
}

/// Check if scoped state is currently active.
pub fn is_scoped_state_active() -> bool {
    SCOPED_STATE_ACTIVE.with(|cell| cell.get())
}

fn get_scoped_state_table(lua: &Lua) -> mlua::Result<Table> {
    if !is_scoped_state_active() {
        return Err(mlua::Error::external(
            "state queries require scoped state context",
        ));
    }
    lua.globals().get::<Table>(SCOPED_STATE_GLOBAL_KEY)
}

/// Cursor position in global compositor coordinates.
#[derive(Clone, Debug)]
pub struct CursorPosition {
    pub x: f64,
    pub y: f64,
    pub output: String,
}

/// Reserved space from layer-shell exclusive zones.
#[derive(Clone, Debug, Default)]
pub struct ReservedSpace {
    pub top: i32,
    pub bottom: i32,
    pub left: i32,
    pub right: i32,
}

/// Current focus mode of the compositor.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum FocusMode {
    #[default]
    Normal,
    Overview,
    LayerShell,
    Locked,
}

/// Execute a callback with scoped state access.
///
/// This sets up the scoped state context so that `niri.state.*` functions
/// can access live compositor state. Use this for timer callbacks, scheduled
/// callbacks, and IPC Lua execution.
pub fn with_scoped_state<S, F, R>(lua: &Lua, state: &S, f: F) -> R
where
    S: CompositorState,
    F: FnOnce() -> R,
{
    let result = lua.scope(|scope| {
        let scoped_state_table = lua.create_table().unwrap();

        let windows = state.get_windows();
        let windows_data = windows_to_lua(lua, &windows).unwrap();
        scoped_state_table
            .set(
                "windows",
                scope
                    .create_function(move |_, ()| -> Result<Table> { Ok(windows_data.clone()) })
                    .unwrap(),
            )
            .unwrap();

        let focused = windows.iter().find(|w| w.is_focused).cloned();
        let focused_data = focused.as_ref().map(|w| window_to_lua(lua, w).unwrap());
        scoped_state_table
            .set(
                "focused_window",
                scope
                    .create_function(move |_, ()| -> Result<Option<Table>> {
                        Ok(focused_data.clone())
                    })
                    .unwrap(),
            )
            .unwrap();

        let workspaces = state.get_workspaces();
        let workspaces_data = workspaces_to_lua(lua, &workspaces).unwrap();
        scoped_state_table
            .set(
                "workspaces",
                scope
                    .create_function(move |_, ()| -> Result<Table> { Ok(workspaces_data.clone()) })
                    .unwrap(),
            )
            .unwrap();

        let outputs = state.get_outputs();
        let outputs_data: Vec<Table> = outputs
            .iter()
            .map(|o| output_to_lua(lua, o).unwrap())
            .collect();
        scoped_state_table
            .set(
                "outputs",
                scope
                    .create_function(move |_, ()| -> Result<Vec<Table>> {
                        Ok(outputs_data.clone())
                    })
                    .unwrap(),
            )
            .unwrap();

        let keyboard_layouts = state.get_keyboard_layouts();
        let keyboard_layouts_data = keyboard_layouts.as_ref().map(|kl| {
            let t = lua.create_table().unwrap();
            let names_table = lua.create_table().unwrap();
            for (i, name) in kl.names.iter().enumerate() {
                names_table.set(i + 1, name.as_str()).unwrap();
            }
            t.set("names", names_table).unwrap();
            t.set("current_idx", kl.current_idx).unwrap();
            t
        });
        scoped_state_table
            .set(
                "keyboard_layouts",
                scope
                    .create_function(move |_, ()| -> Result<Option<Table>> {
                        Ok(keyboard_layouts_data.clone())
                    })
                    .unwrap(),
            )
            .unwrap();

        let cursor_position = state.get_cursor_position();
        let cursor_position_data = cursor_position.as_ref().map(|pos| {
            let t = lua.create_table().unwrap();
            t.set("x", pos.x).unwrap();
            t.set("y", pos.y).unwrap();
            t.set("output", pos.output.clone()).unwrap();
            t
        });
        scoped_state_table
            .set(
                "cursor_position",
                scope
                    .create_function(move |_, ()| -> Result<Option<Table>> {
                        Ok(cursor_position_data.clone())
                    })
                    .unwrap(),
            )
            .unwrap();

        let focus_mode = state.get_focus_mode();
        let focus_mode_str = match focus_mode {
            FocusMode::Normal => "normal",
            FocusMode::Overview => "overview",
            FocusMode::LayerShell => "layer_shell",
            FocusMode::Locked => "locked",
        };
        scoped_state_table
            .set(
                "focus_mode",
                scope
                    .create_function(move |_, ()| -> Result<&'static str> { Ok(focus_mode_str) })
                    .unwrap(),
            )
            .unwrap();

        scoped_state_table
            .set(
                "reserved_space",
                scope
                    .create_function(move |lua, output_name: String| -> Result<Table> {
                        let reserved = state.get_reserved_space(&output_name);
                        let t = lua.create_table()?;
                        t.set("top", reserved.top)?;
                        t.set("bottom", reserved.bottom)?;
                        t.set("left", reserved.left)?;
                        t.set("right", reserved.right)?;
                        Ok(t)
                    })
                    .unwrap(),
            )
            .unwrap();

        lua.globals()
            .set(SCOPED_STATE_GLOBAL_KEY, scoped_state_table)
            .unwrap();
        set_scoped_state_active(true);

        let result = f();

        set_scoped_state_active(false);
        lua.globals()
            .set(SCOPED_STATE_GLOBAL_KEY, Value::Nil)
            .unwrap();

        Ok(result)
    });

    result.unwrap()
}

/// Trait for accessing compositor state.
///
/// This trait must be implemented by the main State type to allow the RuntimeApi to query it.
/// It provides a safe, well-defined interface for accessing compositor state.
pub trait CompositorState {
    /// Get all windows in the compositor.
    fn get_windows(&self) -> Vec<Window>;

    /// Get the currently focused window, if any.
    fn get_focused_window(&self) -> Option<Window>;

    /// Get all workspaces in the compositor.
    fn get_workspaces(&self) -> Vec<Workspace>;

    /// Get all outputs (monitors) in the compositor.
    fn get_outputs(&self) -> Vec<Output>;

    /// Get the keyboard layouts configuration.
    fn get_keyboard_layouts(&self) -> Option<KeyboardLayouts>;

    /// Get the current cursor position, if available.
    fn get_cursor_position(&self) -> Option<CursorPosition>;

    /// Get reserved space (exclusive zones) for an output.
    fn get_reserved_space(&self, output_name: &str) -> ReservedSpace;

    /// Get the current focus mode.
    fn get_focus_mode(&self) -> FocusMode;
}

/// Register the runtime state API in a Lua context.
///
/// This creates the `niri.state` table with functions for querying compositor state.
/// All functions require a scoped state context (event handlers, timers, IPC execution).
pub fn register_runtime_api(lua: &Lua) -> Result<()> {
    // Get or create the niri table
    let niri: Table = match lua.globals().get("niri")? {
        Value::Table(t) => t,
        _ => {
            let t = lua.create_table()?;
            lua.globals().set("niri", t.clone())?;
            t
        }
    };

    // Create the state table
    let state_table = lua.create_table()?;

    {
        let windows_fn = lua.create_function(|lua, ()| -> mlua::Result<mlua::Value> {
            let state_table = get_scoped_state_table(lua)?;
            let scoped_fn = state_table.get::<mlua::Function>("windows")?;
            scoped_fn.call(())
        })?;
        state_table.set("windows", windows_fn)?;
    }

    {
        let focused_window_fn = lua.create_function(|lua, ()| -> mlua::Result<mlua::Value> {
            let state_table = get_scoped_state_table(lua)?;
            let scoped_fn = state_table.get::<mlua::Function>("focused_window")?;
            scoped_fn.call(())
        })?;
        state_table.set("focused_window", focused_window_fn)?;
    }

    {
        let workspaces_fn = lua.create_function(|lua, ()| -> mlua::Result<mlua::Value> {
            let state_table = get_scoped_state_table(lua)?;
            let scoped_fn = state_table.get::<mlua::Function>("workspaces")?;
            scoped_fn.call(())
        })?;
        state_table.set("workspaces", workspaces_fn)?;
    }

    {
        let outputs_fn = lua.create_function(|lua, ()| -> mlua::Result<mlua::Value> {
            let state_table = get_scoped_state_table(lua)?;
            let scoped_fn = state_table.get::<mlua::Function>("outputs")?;
            scoped_fn.call(())
        })?;
        state_table.set("outputs", outputs_fn)?;
    }

    {
        let keyboard_layouts_fn = lua.create_function(|lua, ()| -> mlua::Result<mlua::Value> {
            let state_table = get_scoped_state_table(lua)?;
            let scoped_fn = state_table.get::<mlua::Function>("keyboard_layouts")?;
            scoped_fn.call(())
        })?;
        state_table.set("keyboard_layouts", keyboard_layouts_fn)?;
    }

    {
        let cursor_position_fn = lua.create_function(|lua, ()| -> mlua::Result<mlua::Value> {
            let state_table = get_scoped_state_table(lua)?;
            let scoped_fn = state_table.get::<mlua::Function>("cursor_position")?;
            scoped_fn.call(())
        })?;
        state_table.set("cursor_position", cursor_position_fn)?;
    }

    {
        let reserved_space_fn =
            lua.create_function(|lua, output_name: String| -> mlua::Result<mlua::Value> {
                let state_table = get_scoped_state_table(lua)?;
                let scoped_fn = state_table.get::<mlua::Function>("reserved_space")?;
                scoped_fn.call(output_name)
            })?;
        state_table.set("reserved_space", reserved_space_fn)?;
    }

    {
        let focus_mode_fn = lua.create_function(|lua, ()| -> mlua::Result<mlua::Value> {
            let state_table = get_scoped_state_table(lua)?;
            let scoped_fn = state_table.get::<mlua::Function>("focus_mode")?;
            scoped_fn.call(())
        })?;
        state_table.set("focus_mode", focus_mode_fn)?;
    }

    niri.set("state", state_table)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use niri_ipc::{LogicalOutput, Mode, Timestamp, Transform, WindowLayout};

    use super::*;

    // ========================================================================
    // Test Fixtures
    // ========================================================================

    /// Mock state for testing CompositorState trait implementation.
    #[derive(Clone, Default)]
    struct MockState {
        windows: Vec<Window>,
        workspaces: Vec<Workspace>,
        outputs: Vec<Output>,
        keyboard_layouts: Option<KeyboardLayouts>,
        #[allow(dead_code)]
        cursor_position: Option<CursorPosition>,
        #[allow(dead_code)]
        focus_mode: FocusMode,
    }

    impl CompositorState for MockState {
        fn get_windows(&self) -> Vec<Window> {
            self.windows.clone()
        }

        fn get_focused_window(&self) -> Option<Window> {
            self.windows.iter().find(|w| w.is_focused).cloned()
        }

        fn get_workspaces(&self) -> Vec<Workspace> {
            self.workspaces.clone()
        }

        fn get_outputs(&self) -> Vec<Output> {
            self.outputs.clone()
        }

        fn get_keyboard_layouts(&self) -> Option<KeyboardLayouts> {
            self.keyboard_layouts.clone()
        }

        fn get_cursor_position(&self) -> Option<CursorPosition> {
            // Mock implementation - return None for simplicity
            None
        }

        fn get_reserved_space(&self, _output_name: &str) -> ReservedSpace {
            // Mock implementation - return zeros
            ReservedSpace::default()
        }

        fn get_focus_mode(&self) -> FocusMode {
            // Mock implementation - return Normal
            FocusMode::Normal
        }
    }

    /// Create a test window with the given properties.
    fn make_window(id: u64, title: &str, app_id: &str, is_focused: bool) -> Window {
        Window {
            id,
            title: Some(title.to_string()),
            app_id: Some(app_id.to_string()),
            pid: Some(1000 + id as i32),
            workspace_id: Some(1),
            is_focused,
            is_floating: false,
            is_urgent: false,
            focus_timestamp: if is_focused {
                Some(Timestamp {
                    secs: 1234,
                    nanos: 0,
                })
            } else {
                None
            },
            layout: WindowLayout {
                pos_in_scrolling_layout: Some((1, 1)),
                tile_size: (800.0, 600.0),
                window_size: (800, 600),
                tile_pos_in_workspace_view: Some((0.0, 0.0)),
                window_offset_in_tile: (0.0, 0.0),
            },
        }
    }

    /// Create a test workspace with the given properties.
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

    /// Create a test output with the given properties.
    fn make_output(name: &str, is_enabled: bool) -> Output {
        Output {
            name: name.to_string(),
            make: "Test Make".to_string(),
            model: "Test Model".to_string(),
            serial: Some("12345".to_string()),
            physical_size: Some((600, 340)),
            modes: vec![Mode {
                width: 1920,
                height: 1080,
                refresh_rate: 60000,
                is_preferred: true,
            }],
            current_mode: if is_enabled { Some(0) } else { None },
            is_custom_mode: false,
            vrr_supported: false,
            vrr_enabled: false,
            logical: if is_enabled {
                Some(LogicalOutput {
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                    scale: 1.0,
                    transform: Transform::Normal,
                })
            } else {
                None
            },
        }
    }

    // ========================================================================
    // RuntimeApi Construction Tests
    // ========================================================================

    #[test]
    fn runtime_api_type_system() {
        // Verify RuntimeApi can be constructed with generic CompositorState types.
        // Full event loop testing is integration-level.
        let _ = std::mem::size_of::<RuntimeApi<MockState>>();

        // Verify generic constraint works with any CompositorState impl
        fn accepts_compositor_state<S: CompositorState + 'static>(_state: &S) {
            let _ = std::mem::size_of::<RuntimeApi<S>>();
        }
        let mock_state = MockState {
            cursor_position: None,
            focus_mode: FocusMode::Normal,
            ..Default::default()
        };
        accepts_compositor_state(&mock_state);
    }

    // ========================================================================
    // Empty State Tests
    // ========================================================================

    #[test]
    fn empty_state_returns_empty_collections() {
        let state = MockState::default();

        assert!(state.get_windows().is_empty());
        assert!(state.get_focused_window().is_none());
        assert!(state.get_workspaces().is_empty());
        assert!(state.get_outputs().is_empty());
    }

    #[test]
    fn empty_state_trait_object() {
        let mock_state = MockState {
            cursor_position: None,
            focus_mode: FocusMode::Normal,
            ..Default::default()
        };
        let trait_obj: &dyn CompositorState = &mock_state;

        assert!(trait_obj.get_windows().is_empty());
        assert!(trait_obj.get_focused_window().is_none());
        assert!(trait_obj.get_workspaces().is_empty());
        assert!(trait_obj.get_outputs().is_empty());
    }

    // ========================================================================
    // Populated State Tests - Windows
    // ========================================================================

    #[test]
    fn windows_returns_all_windows() {
        let state = MockState {
            windows: vec![
                make_window(1, "Firefox", "firefox", false),
                make_window(2, "Terminal", "kitty", true),
                make_window(3, "Editor", "code", false),
            ],
            ..Default::default()
        };

        let windows = state.get_windows();
        assert_eq!(windows.len(), 3);
        assert_eq!(windows[0].id, 1);
        assert_eq!(windows[0].title.as_deref(), Some("Firefox"));
        assert_eq!(windows[1].id, 2);
        assert_eq!(windows[2].id, 3);
    }

    #[test]
    fn focused_window_returns_focused() {
        let state = MockState {
            windows: vec![
                make_window(1, "Firefox", "firefox", false),
                make_window(2, "Terminal", "kitty", true), // focused
                make_window(3, "Editor", "code", false),
            ],
            ..Default::default()
        };

        let focused = state.get_focused_window();
        assert!(focused.is_some());
        let focused = focused.unwrap();
        assert_eq!(focused.id, 2);
        assert_eq!(focused.title.as_deref(), Some("Terminal"));
        assert!(focused.is_focused);
    }

    #[test]
    fn focused_window_none_when_no_focus() {
        let state = MockState {
            windows: vec![
                make_window(1, "Firefox", "firefox", false),
                make_window(2, "Terminal", "kitty", false), // none focused
            ],
            ..Default::default()
        };

        assert!(state.get_focused_window().is_none());
    }

    #[test]
    fn focused_window_first_match_when_multiple_focused() {
        // Edge case: multiple windows marked focused (shouldn't happen, but test the behavior)
        let state = MockState {
            windows: vec![
                make_window(1, "Firefox", "firefox", true),
                make_window(2, "Terminal", "kitty", true),
            ],
            ..Default::default()
        };

        let focused = state.get_focused_window();
        assert!(focused.is_some());
        assert_eq!(focused.unwrap().id, 1); // First match
    }

    // ========================================================================
    // Populated State Tests - Workspaces
    // ========================================================================

    #[test]
    fn workspaces_returns_all_workspaces() {
        let state = MockState {
            workspaces: vec![
                make_workspace(1, 1, Some("main"), true),
                make_workspace(2, 2, Some("dev"), false),
                make_workspace(3, 3, None, false), // unnamed workspace
            ],
            ..Default::default()
        };

        let workspaces = state.get_workspaces();
        assert_eq!(workspaces.len(), 3);
        assert_eq!(workspaces[0].name.as_deref(), Some("main"));
        assert!(workspaces[0].is_active);
        assert_eq!(workspaces[1].name.as_deref(), Some("dev"));
        assert!(!workspaces[1].is_active);
        assert!(workspaces[2].name.is_none());
    }

    // ========================================================================
    // Populated State Tests - Outputs
    // ========================================================================

    #[test]
    fn outputs_returns_all_outputs() {
        let state = MockState {
            outputs: vec![
                make_output("DP-1", true),
                make_output("HDMI-1", true),
                make_output("eDP-1", false), // disabled
            ],
            ..Default::default()
        };

        let outputs = state.get_outputs();
        assert_eq!(outputs.len(), 3);
        assert_eq!(outputs[0].name, "DP-1");
        assert!(outputs[0].logical.is_some());
        assert_eq!(outputs[2].name, "eDP-1");
        assert!(outputs[2].logical.is_none()); // disabled = no logical
    }

    // ========================================================================
    // State Independence and Cloning Tests
    // ========================================================================

    #[test]
    fn state_cloning_preserves_data() {
        let state1 = MockState {
            windows: vec![make_window(1, "Test", "test", true)],
            workspaces: vec![make_workspace(1, 1, Some("ws"), true)],
            outputs: vec![make_output("DP-1", true)],
            keyboard_layouts: None,
            cursor_position: None,
            focus_mode: FocusMode::Normal,
        };

        let state2 = state1.clone();

        assert_eq!(state2.get_windows().len(), 1);
        assert_eq!(state2.get_workspaces().len(), 1);
        assert_eq!(state2.get_outputs().len(), 1);
        assert_eq!(state2.get_windows()[0].id, state1.get_windows()[0].id);
    }

    #[test]
    fn multiple_state_instances_independent() {
        let state1 = MockState {
            windows: vec![make_window(1, "Win1", "app1", true)],
            cursor_position: None,
            focus_mode: FocusMode::Normal,
            ..Default::default()
        };

        let state2 = MockState {
            windows: vec![
                make_window(2, "Win2", "app2", false),
                make_window(3, "Win3", "app3", true),
            ],
            cursor_position: None,
            focus_mode: FocusMode::Normal,
            ..Default::default()
        };

        assert_eq!(state1.get_windows().len(), 1);
        assert_eq!(state2.get_windows().len(), 2);
        assert_eq!(state1.get_focused_window().unwrap().id, 1);
        assert_eq!(state2.get_focused_window().unwrap().id, 3);
    }
}
