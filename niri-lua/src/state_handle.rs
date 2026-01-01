use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use niri_ipc::state::EventStreamState;
use niri_ipc::{KeyboardLayouts, Output, Window, Workspace};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CursorPosition {
    pub x: f64,
    pub y: f64,
    pub output: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReservedSpace {
    pub top: i32,
    pub bottom: i32,
    pub left: i32,
    pub right: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FocusMode {
    #[default]
    Normal,
    Overview,
    LayerShell,
    Locked,
}

/// Always-live handle to compositor state.
/// Stored in Lua app_data, queries resolve through shared references.
#[derive(Clone)]
pub struct StateHandle {
    /// Windows, workspaces, keyboard layouts
    event_stream_state: Rc<RefCell<EventStreamState>>,

    /// Outputs/monitors - values only (OutputId keys not exposed to Lua)
    outputs: Rc<RefCell<Vec<Output>>>,

    /// Cursor position - updated on pointer motion
    cursor_position: Rc<RefCell<Option<CursorPosition>>>,

    /// Focus mode - updated on focus change
    focus_mode: Rc<RefCell<FocusMode>>,

    /// Reserved space per output - updated on layer shell changes
    reserved_spaces: Rc<RefCell<HashMap<String, ReservedSpace>>>,
}

impl StateHandle {
    /// Create new StateHandle. outputs_source is cloned into internal Vec on each query.
    pub fn new(event_stream_state: Rc<RefCell<EventStreamState>>) -> Self {
        Self {
            event_stream_state,
            outputs: Rc::new(RefCell::new(Vec::new())),
            cursor_position: Rc::new(RefCell::new(None)),
            focus_mode: Rc::new(RefCell::new(FocusMode::Normal)),
            reserved_spaces: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    // === Collection Queries (LIVE) ===

    /// Get all windows as Vec
    pub fn windows(&self) -> Vec<Window> {
        self.event_stream_state
            .try_borrow()
            .map(|state| state.windows.windows.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all workspaces as Vec
    pub fn workspaces(&self) -> Vec<Workspace> {
        self.event_stream_state
            .try_borrow()
            .map(|state| state.workspaces.workspaces.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all outputs as Vec
    pub fn outputs(&self) -> Vec<Output> {
        self.outputs.borrow().clone()
    }

    /// Get keyboard layouts
    pub fn keyboard_layouts(&self) -> Option<KeyboardLayouts> {
        self.event_stream_state
            .borrow()
            .keyboard_layouts
            .keyboard_layouts
            .clone()
    }

    // === Targeted Queries ===

    /// Get window by ID - O(1) HashMap lookup
    pub fn window(&self, id: u64) -> Option<Window> {
        self.event_stream_state
            .borrow()
            .windows
            .windows
            .get(&id)
            .cloned()
    }

    /// Get focused window
    pub fn focused_window(&self) -> Option<Window> {
        self.event_stream_state
            .borrow()
            .windows
            .windows
            .values()
            .find(|w| w.is_focused)
            .cloned()
    }

    /// Get workspace by ID - O(1)
    pub fn workspace_by_id(&self, id: u64) -> Option<Workspace> {
        self.event_stream_state
            .borrow()
            .workspaces
            .workspaces
            .get(&id)
            .cloned()
    }

    /// Get workspace by name - O(n)
    pub fn workspace_by_name(&self, name: &str) -> Option<Workspace> {
        self.event_stream_state
            .borrow()
            .workspaces
            .workspaces
            .values()
            .find(|ws| ws.name.as_deref() == Some(name))
            .cloned()
    }

    /// Get workspace by index - O(n)
    pub fn workspace_by_idx(&self, idx: u8) -> Option<Workspace> {
        self.event_stream_state
            .borrow()
            .workspaces
            .workspaces
            .values()
            .find(|ws| ws.idx == idx)
            .cloned()
    }

    /// Get output by name - O(n)
    pub fn output_by_name(&self, name: &str) -> Option<Output> {
        self.outputs
            .borrow()
            .iter()
            .find(|o| o.name == name)
            .cloned()
    }

    // === Compositor State Queries ===

    pub fn cursor_position(&self) -> Option<CursorPosition> {
        self.cursor_position.borrow().clone()
    }

    pub fn focus_mode(&self) -> FocusMode {
        *self.focus_mode.borrow()
    }

    pub fn reserved_space(&self, output_name: &str) -> ReservedSpace {
        self.reserved_spaces
            .borrow()
            .get(output_name)
            .cloned()
            .unwrap_or_default()
    }

    // === Update Methods (called by compositor) ===

    /// Set the outputs list (called when IpcOutputMap changes)
    pub fn set_outputs(&self, outputs: Vec<Output>) {
        *self.outputs.borrow_mut() = outputs;
    }

    pub fn set_cursor_position(&self, pos: Option<CursorPosition>) {
        *self.cursor_position.borrow_mut() = pos;
    }

    pub fn set_focus_mode(&self, mode: FocusMode) {
        *self.focus_mode.borrow_mut() = mode;
    }

    pub fn set_reserved_space(&self, output_name: &str, space: ReservedSpace) {
        self.reserved_spaces
            .borrow_mut()
            .insert(output_name.to_string(), space);
    }

    pub fn remove_reserved_space(&self, output_name: &str) {
        self.reserved_spaces.borrow_mut().remove(output_name);
    }
}

#[cfg(test)]
mod tests {
    use niri_ipc::{Timestamp, WindowLayout};

    use super::*;

    fn make_window(id: u64, is_focused: bool) -> Window {
        Window {
            id,
            title: Some(format!("Window {id}")),
            app_id: Some("app".to_string()),
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

    fn create_handle_with_state(state: EventStreamState) -> StateHandle {
        let shared = Rc::new(RefCell::new(state));
        StateHandle::new(shared)
    }

    #[test]
    fn test_statehandle_creation() {
        let handle = StateHandle::new(Rc::new(RefCell::new(EventStreamState::default())));
        assert!(handle.windows().is_empty());
        assert!(handle.workspaces().is_empty());
        assert!(handle.outputs().is_empty());
        assert!(handle.keyboard_layouts().is_none());
        assert!(handle.cursor_position().is_none());
        assert_eq!(handle.focus_mode(), FocusMode::Normal);
    }

    #[test]
    fn test_statehandle_clone_shares_state() {
        let shared = Rc::new(RefCell::new(EventStreamState::default()));
        let handle = StateHandle::new(shared.clone());
        let handle_clone = handle.clone();

        shared
            .borrow_mut()
            .windows
            .windows
            .insert(10, make_window(10, false));

        assert!(handle_clone.window(10).is_some());
        assert_eq!(handle.window(10).unwrap().id, 10);
    }

    #[test]
    fn test_windows_returns_all() {
        let mut state = EventStreamState::default();
        state.windows.windows.insert(1, make_window(1, false));
        state.windows.windows.insert(2, make_window(2, true));
        let handle = create_handle_with_state(state);

        let windows = handle.windows();
        assert_eq!(windows.len(), 2);
        assert!(windows.iter().any(|w| w.id == 1));
        assert!(windows.iter().any(|w| w.id == 2));
    }

    #[test]
    fn test_window_by_id_found() {
        let mut state = EventStreamState::default();
        state.windows.windows.insert(5, make_window(5, false));
        let handle = create_handle_with_state(state);

        let window = handle.window(5);
        assert!(window.is_some());
        assert_eq!(window.unwrap().id, 5);
    }

    #[test]
    fn test_window_by_id_not_found() {
        let handle = StateHandle::new(Rc::new(RefCell::new(EventStreamState::default())));
        assert!(handle.window(999).is_none());
    }

    #[test]
    fn test_workspace_by_id() {
        let mut state = EventStreamState::default();
        state
            .workspaces
            .workspaces
            .insert(1, make_workspace(1, 1, Some("main"), true));
        let handle = create_handle_with_state(state);

        let ws = handle.workspace_by_id(1);
        assert!(ws.is_some());
        assert_eq!(ws.unwrap().id, 1);
    }

    #[test]
    fn test_workspace_by_name() {
        let mut state = EventStreamState::default();
        state
            .workspaces
            .workspaces
            .insert(2, make_workspace(2, 2, Some("dev"), false));
        let handle = create_handle_with_state(state);

        let ws = handle.workspace_by_name("dev");
        assert!(ws.is_some());
        assert_eq!(ws.unwrap().id, 2);
    }

    #[test]
    fn test_workspace_by_idx() {
        let mut state = EventStreamState::default();
        state
            .workspaces
            .workspaces
            .insert(3, make_workspace(3, 3, None, false));
        let handle = create_handle_with_state(state);

        let ws = handle.workspace_by_idx(3);
        assert!(ws.is_some());
        assert_eq!(ws.unwrap().id, 3);
    }

    #[test]
    fn test_focused_window() {
        let mut state = EventStreamState::default();
        state.windows.windows.insert(1, make_window(1, false));
        state.windows.windows.insert(2, make_window(2, true));
        let handle = create_handle_with_state(state);

        let focused = handle.focused_window();
        assert!(focused.is_some());
        assert_eq!(focused.unwrap().id, 2);
    }

    #[test]
    fn test_cursor_position_update() {
        let handle = StateHandle::new(Rc::new(RefCell::new(EventStreamState::default())));
        let pos = CursorPosition {
            x: 1.0,
            y: 2.0,
            output: "DP-1".to_string(),
        };
        handle.set_cursor_position(Some(pos.clone()));

        assert_eq!(handle.cursor_position(), Some(pos));
    }

    #[test]
    fn test_focus_mode_update() {
        let handle = StateHandle::new(Rc::new(RefCell::new(EventStreamState::default())));
        handle.set_focus_mode(FocusMode::Overview);
        assert_eq!(handle.focus_mode(), FocusMode::Overview);
    }

    #[test]
    fn test_reserved_space_update() {
        let handle = StateHandle::new(Rc::new(RefCell::new(EventStreamState::default())));
        let space = ReservedSpace {
            top: 1,
            bottom: 2,
            left: 3,
            right: 4,
        };
        handle.set_reserved_space("DP-1", space.clone());

        assert_eq!(handle.reserved_space("DP-1"), space);

        handle.remove_reserved_space("DP-1");
        assert_eq!(handle.reserved_space("DP-1"), ReservedSpace::default());
    }
}
