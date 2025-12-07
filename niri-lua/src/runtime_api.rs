//! Runtime state API for querying compositor state from Lua scripts.
//!
//! This module provides the `niri.state` API that allows Lua scripts to query the current
//! compositor state, including windows, workspaces, and outputs.
//!
//! ## Architecture
//!
//! This module supports two modes of operation:
//!
//! ### 1. Event Handler Context (synchronous, no deadlock)
//! When called from within an event handler (e.g., `niri.events:on("window:open", ...)`),
//! we use pre-captured state snapshot stored in a thread-local. This avoids the deadlock
//! that would occur if we tried to use the idle callback pattern while the event loop
//! is blocked waiting for the Lua handler to complete.
//!
//! ### 2. Normal Context (async via idle callback)
//! When called from other contexts (e.g., REPL, timers), we use the event loop message
//! passing pattern like the IPC server:
//! - Lua calls a function like `niri.state.windows()`
//! - We create a channel and send a message to the event loop via `insert_idle()`
//! - The event loop handler runs on the main thread with access to State
//! - The handler collects the data and sends it back through the channel
//! - The Lua function blocks waiting for the response (from Lua's perspective)

use std::cell::RefCell;

use async_channel::{bounded, Sender};
use calloop::LoopHandle;
use mlua::{Lua, Result, Table, Value};
use niri_ipc::{Output, Window, Workspace};

use crate::ipc_bridge::{output_to_lua, window_to_lua, windows_to_lua, workspaces_to_lua};

// Thread-local storage for state snapshot during event handler execution.
//
// This allows `niri.state.*` functions to access pre-captured state data
// when called from within event handlers, avoiding the deadlock that would
// occur with the idle callback pattern.
thread_local! {
    static EVENT_CONTEXT_STATE: RefCell<Option<StateSnapshot>> = const { RefCell::new(None) };
}

/// A snapshot of compositor state captured before event handler execution.
///
/// This is used to provide state access within event handlers without
/// needing to query the event loop (which would deadlock).
#[derive(Clone, Default)]
pub struct StateSnapshot {
    pub windows: Vec<Window>,
    pub workspaces: Vec<Workspace>,
    pub outputs: Vec<Output>,
}

impl StateSnapshot {
    /// Create a new state snapshot from the compositor state.
    pub fn from_compositor_state<S: CompositorState>(state: &S) -> Self {
        Self {
            windows: state.get_windows(),
            workspaces: state.get_workspaces(),
            outputs: state.get_outputs(),
        }
    }

    /// Get the focused window from the snapshot.
    pub fn get_focused_window(&self) -> Option<&Window> {
        self.windows.iter().find(|w| w.is_focused)
    }
}

/// Set the event context state snapshot for the current thread.
///
/// This should be called before invoking Lua event handlers, and cleared
/// afterwards using `clear_event_context_state()`.
///
/// # Example
///
/// ```ignore
/// let snapshot = StateSnapshot::from_compositor_state(&state);
/// set_event_context_state(snapshot);
/// // ... call Lua event handlers ...
/// clear_event_context_state();
/// ```
pub fn set_event_context_state(snapshot: StateSnapshot) {
    EVENT_CONTEXT_STATE.with(|cell| {
        *cell.borrow_mut() = Some(snapshot);
    });
}

/// Clear the event context state snapshot for the current thread.
///
/// This should be called after Lua event handlers have completed.
pub fn clear_event_context_state() {
    EVENT_CONTEXT_STATE.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

/// Get a clone of the event context state snapshot, if available.
fn get_event_context_state() -> Option<StateSnapshot> {
    EVENT_CONTEXT_STATE.with(|cell| cell.borrow().clone())
}

/// Generic runtime API that can query state from the compositor.
///
/// The generic parameter `S` allows this to work with any State type that provides the necessary
/// accessors (e.g., `niri::State` from the main crate).
///
/// We use a generic here to avoid circular dependencies: niri-lua can't depend on niri, but niri
/// can depend on niri-lua.
pub struct RuntimeApi<S: 'static> {
    event_loop: LoopHandle<'static, S>,
}

impl<S> RuntimeApi<S> {
    /// Create a new RuntimeApi with access to the event loop.
    pub fn new(event_loop: LoopHandle<'static, S>) -> Self {
        Self { event_loop }
    }

    /// Query the event loop and wait for a response.
    ///
    /// This is a helper that creates a channel, inserts an idle callback into the event loop,
    /// and blocks waiting for the response.
    fn query<F, T>(&self, f: F) -> std::result::Result<T, String>
    where
        F: FnOnce(&mut S, Sender<T>) + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = bounded(1);

        self.event_loop.insert_idle(move |state| {
            f(state, tx);
        });

        // Block waiting for response from the event loop
        // This blocks the Lua thread but not the main event loop
        rx.recv_blocking()
            .map_err(|_| String::from("Failed to receive response from compositor"))
    }
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
}

/// Register the runtime state API in a Lua context.
///
/// This creates the `niri.state` table with the following functions:
/// - `windows()` - Returns an array of all window tables
/// - `focused_window()` - Returns the focused window table, or nil
/// - `workspaces()` - Returns an array of all workspace tables
/// - `outputs()` - Returns an array of all output tables
///
/// # Example
///
/// ```lua
/// local windows = niri.state.windows()
/// for i, win in ipairs(windows) do
///     print(win.id, win.title, win.app_id)
/// end
///
/// local focused = niri.state.focused_window()
/// if focused then
///     print("Focused:", focused.title)
/// end
/// ```
pub fn register_runtime_api<S>(lua: &Lua, api: RuntimeApi<S>) -> Result<()>
where
    S: CompositorState + 'static,
{
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

    // windows() -> array of window tables
    {
        let api = api.event_loop.clone();
        let windows_fn = lua.create_function(move |lua, ()| {
            // Check if we're in an event handler context with pre-captured state
            if let Some(snapshot) = get_event_context_state() {
                return windows_to_lua(lua, &snapshot.windows);
            }

            // Fall back to idle callback pattern for non-event contexts
            let runtime_api = RuntimeApi {
                event_loop: api.clone(),
            };
            let windows: Vec<Window> = runtime_api
                .query(|state, tx| {
                    let windows = state.get_windows();
                    if let Err(e) = tx.send_blocking(windows) {
                        log::warn!("Failed to send windows query result: {}", e);
                    }
                })
                .map_err(mlua::Error::external)?;

            windows_to_lua(lua, &windows)
        })?;
        state_table.set("windows", windows_fn)?;
    }

    // focused_window() -> window table or nil
    {
        let api = api.event_loop.clone();
        let focused_window_fn = lua.create_function(move |lua, ()| {
            // Check if we're in an event handler context with pre-captured state
            if let Some(snapshot) = get_event_context_state() {
                return match snapshot.get_focused_window() {
                    Some(win) => window_to_lua(lua, win).map(Value::Table),
                    None => Ok(Value::Nil),
                };
            }

            // Fall back to idle callback pattern for non-event contexts
            let runtime_api = RuntimeApi {
                event_loop: api.clone(),
            };
            let window = runtime_api
                .query(|state, tx| {
                    let window = state.get_focused_window();
                    if let Err(e) = tx.send_blocking(window) {
                        log::warn!("Failed to send focused_window query result: {}", e);
                    }
                })
                .map_err(mlua::Error::external)?;

            match window {
                Some(win) => window_to_lua(lua, &win).map(Value::Table),
                None => Ok(Value::Nil),
            }
        })?;
        state_table.set("focused_window", focused_window_fn)?;
    }

    // workspaces() -> array of workspace tables
    {
        let api = api.event_loop.clone();
        let workspaces_fn = lua.create_function(move |lua, ()| {
            // Check if we're in an event handler context with pre-captured state
            if let Some(snapshot) = get_event_context_state() {
                return workspaces_to_lua(lua, &snapshot.workspaces);
            }

            // Fall back to idle callback pattern for non-event contexts
            let runtime_api = RuntimeApi {
                event_loop: api.clone(),
            };
            let workspaces: Vec<Workspace> = runtime_api
                .query(|state, tx| {
                    let workspaces = state.get_workspaces();
                    if let Err(e) = tx.send_blocking(workspaces) {
                        log::warn!("Failed to send workspaces query result: {}", e);
                    }
                })
                .map_err(mlua::Error::external)?;

            workspaces_to_lua(lua, &workspaces)
        })?;
        state_table.set("workspaces", workspaces_fn)?;
    }

    // outputs() -> array of output tables
    {
        let api = api.event_loop;
        let outputs_fn = lua.create_function(move |lua, ()| {
            // Check if we're in an event handler context with pre-captured state
            if let Some(snapshot) = get_event_context_state() {
                let table = lua.create_table()?;
                for (i, output) in snapshot.outputs.iter().enumerate() {
                    let output_table = output_to_lua(lua, output)?;
                    table.set(i + 1, output_table)?;
                }
                return Ok(table);
            }

            // Fall back to idle callback pattern for non-event contexts
            let runtime_api = RuntimeApi {
                event_loop: api.clone(),
            };
            let outputs: Vec<Output> = runtime_api
                .query(|state, tx| {
                    let outputs = state.get_outputs();
                    if let Err(e) = tx.send_blocking(outputs) {
                        log::warn!("Failed to send outputs query result: {}", e);
                    }
                })
                .map_err(mlua::Error::external)?;

            // Convert Vec<Output> to Lua array
            let table = lua.create_table()?;
            for (i, output) in outputs.iter().enumerate() {
                let output_table = output_to_lua(lua, output)?;
                table.set(i + 1, output_table)?;
            }
            Ok(table)
        })?;
        state_table.set("outputs", outputs_fn)?;
    }

    // Set niri.state
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
        accepts_compositor_state(&MockState::default());
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
        let state = MockState::default();
        let trait_obj: &dyn CompositorState = &state;

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
            ..Default::default()
        };

        let state2 = MockState {
            windows: vec![
                make_window(2, "Win2", "app2", false),
                make_window(3, "Win3", "app3", true),
            ],
            ..Default::default()
        };

        assert_eq!(state1.get_windows().len(), 1);
        assert_eq!(state2.get_windows().len(), 2);
        assert_eq!(state1.get_focused_window().unwrap().id, 1);
        assert_eq!(state2.get_focused_window().unwrap().id, 3);
    }
}
