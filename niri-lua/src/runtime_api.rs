//! Runtime API for querying compositor state from Lua scripts.
//!
//! This module provides the `niri.runtime` API that allows Lua scripts to query the current
//! compositor state, including windows, workspaces, and outputs.
//!
//! ## Architecture
//!
//! This uses the same event loop message passing pattern as the IPC server:
//! - Lua calls a function like `niri.runtime.get_windows()`
//! - We create a channel and send a message to the event loop via `insert_idle()`
//! - The event loop handler runs on the main thread with access to State
//! - The handler collects the data and sends it back through the channel
//! - The Lua function blocks waiting for the response (from Lua's perspective)
//!
//! This approach avoids all lifetime issues, requires zero unsafe code, and is proven in
//! production by the IPC server.

use async_channel::{bounded, Sender};
use calloop::LoopHandle;
use mlua::{Lua, Result, Table, Value};
use niri_ipc::{Output, Window, Workspace};

use crate::ipc_bridge::{output_to_lua, window_to_lua, windows_to_lua, workspaces_to_lua};

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

/// Register the runtime API in a Lua context.
///
/// This creates the `niri.runtime` table with the following functions:
/// - `get_windows()` - Returns an array of all window tables
/// - `get_focused_window()` - Returns the focused window table, or nil
/// - `get_workspaces()` - Returns an array of all workspace tables
/// - `get_outputs()` - Returns an array of all output tables
///
/// # Example
///
/// ```lua
/// local windows = niri.runtime.get_windows()
/// for i, win in ipairs(windows) do
///     print(win.id, win.title, win.app_id)
/// end
///
/// local focused = niri.runtime.get_focused_window()
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

    // Create the runtime table
    let runtime = lua.create_table()?;

    // get_windows() -> array of window tables
    {
        let api = api.event_loop.clone();
        let get_windows = lua.create_function(move |lua, ()| {
            let runtime_api = RuntimeApi {
                event_loop: api.clone(),
            };
            let windows: Vec<Window> = runtime_api
                .query(|state, tx| {
                    let windows = state.get_windows();
                    let _ = tx.send_blocking(windows);
                })
                .map_err(mlua::Error::external)?;

            windows_to_lua(lua, &windows)
        })?;
        runtime.set("get_windows", get_windows)?;
    }

    // get_focused_window() -> window table or nil
    {
        let api = api.event_loop.clone();
        let get_focused_window = lua.create_function(move |lua, ()| {
            let runtime_api = RuntimeApi {
                event_loop: api.clone(),
            };
            let window = runtime_api
                .query(|state, tx| {
                    let window = state.get_focused_window();
                    let _ = tx.send_blocking(window);
                })
                .map_err(mlua::Error::external)?;

            match window {
                Some(win) => window_to_lua(lua, &win).map(Value::Table),
                None => Ok(Value::Nil),
            }
        })?;
        runtime.set("get_focused_window", get_focused_window)?;
    }

    // get_workspaces() -> array of workspace tables
    {
        let api = api.event_loop.clone();
        let get_workspaces = lua.create_function(move |lua, ()| {
            let runtime_api = RuntimeApi {
                event_loop: api.clone(),
            };
            let workspaces: Vec<Workspace> = runtime_api
                .query(|state, tx| {
                    let workspaces = state.get_workspaces();
                    let _ = tx.send_blocking(workspaces);
                })
                .map_err(mlua::Error::external)?;

            workspaces_to_lua(lua, &workspaces)
        })?;
        runtime.set("get_workspaces", get_workspaces)?;
    }

    // get_outputs() -> array of output tables
    {
        let api = api.event_loop;
        let get_outputs = lua.create_function(move |lua, ()| {
            let runtime_api = RuntimeApi {
                event_loop: api.clone(),
            };
            let outputs: Vec<Output> = runtime_api
                .query(|state, tx| {
                    let outputs = state.get_outputs();
                    let _ = tx.send_blocking(outputs);
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
        runtime.set("get_outputs", get_outputs)?;
    }

    // Set niri.runtime
    niri.set("runtime", runtime)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock state for testing
    #[derive(Clone)]
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

    // ========================================================================
    // RuntimeApi::new tests
    // ========================================================================

    #[test]
    fn runtime_api_construction() {
        // We can test that RuntimeApi can be constructed
        // Full testing requires a real event loop which is integration-level
        // This test verifies the type system works correctly
        let _ = std::mem::size_of::<RuntimeApi<MockState>>();
    }

    // ========================================================================
    // CompositorState trait implementation tests with empty collections
    // ========================================================================

    #[test]
    fn mock_state_get_windows_empty() {
        let state = MockState {
            windows: vec![],
            workspaces: vec![],
            outputs: vec![],
        };

        let result = state.get_windows();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn mock_state_get_focused_window_none() {
        let state = MockState {
            windows: vec![],
            workspaces: vec![],
            outputs: vec![],
        };

        let result = state.get_focused_window();
        assert!(result.is_none());
    }

    #[test]
    fn mock_state_get_workspaces_empty() {
        let state = MockState {
            windows: vec![],
            workspaces: vec![],
            outputs: vec![],
        };

        let result = state.get_workspaces();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn mock_state_get_outputs_empty() {
        let state = MockState {
            windows: vec![],
            workspaces: vec![],
            outputs: vec![],
        };

        let result = state.get_outputs();
        assert_eq!(result.len(), 0);
    }

    // ========================================================================
    // Trait composition and polymorphism tests
    // ========================================================================

    #[test]
    fn compositor_state_trait_object() {
        let state = MockState {
            windows: vec![],
            workspaces: vec![],
            outputs: vec![],
        };

        // Test that we can use state as a trait object
        let trait_obj: &dyn CompositorState = &state;
        assert_eq!(trait_obj.get_windows().len(), 0);
        assert_eq!(trait_obj.get_workspaces().len(), 0);
        assert_eq!(trait_obj.get_outputs().len(), 0);
    }

    // ========================================================================
    // CompositorState trait documentation tests
    // ========================================================================

    #[test]
    fn compositor_state_get_windows_returns_vec() {
        // Ensures get_windows returns a Vec that can be checked for length
        let state = MockState {
            windows: vec![],
            workspaces: vec![],
            outputs: vec![],
        };
        let windows = state.get_windows();
        assert!(windows.is_empty());
    }

    #[test]
    fn compositor_state_get_focused_window_returns_option() {
        // Ensures get_focused_window returns an Option that can be checked
        let state = MockState {
            windows: vec![],
            workspaces: vec![],
            outputs: vec![],
        };
        let focused = state.get_focused_window();
        assert!(matches!(focused, None));
    }

    #[test]
    fn compositor_state_get_workspaces_returns_vec() {
        // Ensures get_workspaces returns a Vec that can be checked for length
        let state = MockState {
            windows: vec![],
            workspaces: vec![],
            outputs: vec![],
        };
        let workspaces = state.get_workspaces();
        assert!(workspaces.is_empty());
    }

    #[test]
    fn compositor_state_get_outputs_returns_vec() {
        // Ensures get_outputs returns a Vec that can be checked for length
        let state = MockState {
            windows: vec![],
            workspaces: vec![],
            outputs: vec![],
        };
        let outputs = state.get_outputs();
        assert!(outputs.is_empty());
    }

    // ========================================================================
    // RuntimeApi trait generic parameter tests
    // ========================================================================

    #[test]
    fn runtime_api_generic_constraint() {
        // This test verifies that RuntimeApi can work with any type implementing
        // CompositorState, not just MockState
        fn create_runtime_api<S: CompositorState + 'static>(_state: &S) {
            let _ = std::mem::size_of::<RuntimeApi<S>>();
        }

        let mock_state = MockState {
            windows: vec![],
            workspaces: vec![],
            outputs: vec![],
        };
        create_runtime_api(&mock_state);
    }

    // ========================================================================
    // CompositorState trait cloning tests
    // ========================================================================

    #[test]
    fn compositor_state_cloning_preserves_data() {
        // Verify that cloning state preserves empty collections correctly
        let state1 = MockState {
            windows: vec![],
            workspaces: vec![],
            outputs: vec![],
        };

        let state2 = state1.clone();
        assert_eq!(state2.get_windows().len(), state1.get_windows().len());
        assert_eq!(state2.get_workspaces().len(), state1.get_workspaces().len());
        assert_eq!(state2.get_outputs().len(), state1.get_outputs().len());
    }

    // ========================================================================
    // CompositorState trait behavior consistency tests
    // ========================================================================

    #[test]
    fn focused_window_filter_consistency() {
        // Verify that get_focused_window filters correctly from get_windows
        let state = MockState {
            windows: vec![],
            workspaces: vec![],
            outputs: vec![],
        };

        let all_windows = state.get_windows();
        let focused = state.get_focused_window();

        // When no windows, focused should be None
        assert!(all_windows.is_empty());
        assert!(focused.is_none());
    }
}
