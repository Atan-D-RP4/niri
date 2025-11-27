//! Event data conversion from niri types to Lua tables.
//!
//! This module provides converters for event-specific data structures that are emitted
//! as Lua events. It builds on top of ipc_bridge for common types like Window, Workspace, Output.

use mlua::{Lua, Result, Table};
use niri_ipc::{Output, Window, Workspace};

use crate::ipc_bridge::{output_to_lua, window_to_lua, workspace_to_lua};

/// Window lifecycle event data.
#[derive(Debug, Clone)]
pub enum WindowEventData {
    /// Window has opened and is ready.
    Open { window: Window },
    /// Window is closing.
    Close { window: Window },
    /// Window gained focus.
    Focus { window: Window },
    /// Window lost focus.
    Blur { window: Window },
}

impl WindowEventData {
    /// Convert window event to Lua table.
    pub fn to_lua(&self, lua: &Lua) -> Result<Table> {
        let table = lua.create_table()?;

        let (event_type, window) = match self {
            WindowEventData::Open { window } => ("open", window),
            WindowEventData::Close { window } => ("close", window),
            WindowEventData::Focus { window } => ("focus", window),
            WindowEventData::Blur { window } => ("blur", window),
        };

        table.set("type", event_type)?;
        let window_table = window_to_lua(lua, window)?;
        table.set("window", window_table)?;

        Ok(table)
    }
}

/// Workspace lifecycle event data.
#[derive(Debug, Clone)]
pub enum WorkspaceEventData {
    /// Workspace is now active on its output.
    Activate {
        workspace: Workspace,
        output: Output,
    },
    /// Workspace is no longer active.
    Deactivate { workspace: Workspace },
}

impl WorkspaceEventData {
    /// Convert workspace event to Lua table.
    pub fn to_lua(&self, lua: &Lua) -> Result<Table> {
        let table = lua.create_table()?;

        match self {
            WorkspaceEventData::Activate { workspace, output } => {
                table.set("type", "activate")?;
                let workspace_table = workspace_to_lua(lua, workspace)?;
                table.set("workspace", workspace_table)?;
                let output_table = output_to_lua(lua, output)?;
                table.set("output", output_table)?;
            }
            WorkspaceEventData::Deactivate { workspace } => {
                table.set("type", "deactivate")?;
                let workspace_table = workspace_to_lua(lua, workspace)?;
                table.set("workspace", workspace_table)?;
            }
        }

        Ok(table)
    }
}

/// Monitor/Output lifecycle event data.
#[derive(Debug, Clone)]
pub enum MonitorEventData {
    /// Monitor/output connected and ready.
    Connect { output: Output },
    /// Monitor/output disconnected.
    Disconnect { output: Output },
}

impl MonitorEventData {
    /// Convert monitor event to Lua table.
    pub fn to_lua(&self, lua: &Lua) -> Result<Table> {
        let table = lua.create_table()?;

        let (event_type, output) = match self {
            MonitorEventData::Connect { output } => ("connect", output),
            MonitorEventData::Disconnect { output } => ("disconnect", output),
        };

        table.set("type", event_type)?;
        let output_table = output_to_lua(lua, output)?;
        table.set("output", output_table)?;

        Ok(table)
    }
}

/// Layout change event data.
#[derive(Debug, Clone)]
pub enum LayoutEventData {
    /// Layout mode changed (tiling <-> floating).
    ModeChanged { is_floating: bool },
    /// Window was added to layout.
    WindowAdded { window: Window },
    /// Window was removed from layout.
    WindowRemoved { window: Window },
}

impl LayoutEventData {
    /// Convert layout event to Lua table.
    pub fn to_lua(&self, lua: &Lua) -> Result<Table> {
        let table = lua.create_table()?;

        match self {
            LayoutEventData::ModeChanged { is_floating } => {
                table.set("type", "mode_changed")?;
                table.set("is_floating", *is_floating)?;
            }
            LayoutEventData::WindowAdded { window } => {
                table.set("type", "window_added")?;
                let window_table = window_to_lua(lua, window)?;
                table.set("window", window_table)?;
            }
            LayoutEventData::WindowRemoved { window } => {
                table.set("type", "window_removed")?;
                let window_table = window_to_lua(lua, window)?;
                table.set("window", window_table)?;
            }
        }

        Ok(table)
    }
}

/// Generic event data wrapper that can be emitted via the event system.
///
/// This enum allows the event system to handle different event types uniformly
/// while maintaining type-specific information.
#[derive(Debug, Clone)]
pub enum EventData {
    /// Window-related events
    Window(WindowEventData),
    /// Workspace-related events
    Workspace(WorkspaceEventData),
    /// Monitor/output-related events
    Monitor(MonitorEventData),
    /// Layout-related events
    Layout(LayoutEventData),
}

impl EventData {
    /// Get the event category (e.g., "window", "workspace").
    pub fn category(&self) -> &'static str {
        match self {
            EventData::Window(_) => "window",
            EventData::Workspace(_) => "workspace",
            EventData::Monitor(_) => "monitor",
            EventData::Layout(_) => "layout",
        }
    }

    /// Convert event data to Lua table.
    pub fn to_lua(&self, lua: &Lua) -> Result<Table> {
        match self {
            EventData::Window(w) => w.to_lua(lua),
            EventData::Workspace(w) => w.to_lua(lua),
            EventData::Monitor(m) => m.to_lua(lua),
            EventData::Layout(l) => l.to_lua(lua),
        }
    }
}

#[cfg(test)]
mod tests {
    use niri_ipc::WindowLayout;

    use super::*;

    fn create_test_window() -> Window {
        Window {
            id: 123,
            title: Some("Test Window".to_string()),
            app_id: Some("org.test.App".to_string()),
            pid: Some(4567),
            workspace_id: Some(1),
            is_focused: true,
            is_floating: false,
            is_urgent: false,
            focus_timestamp: None,
            layout: WindowLayout {
                pos_in_scrolling_layout: Some((1, 1)),
                tile_size: (1920.0, 1080.0),
                window_size: (1900, 1060),
                tile_pos_in_workspace_view: Some((0.0, 0.0)),
                window_offset_in_tile: (10.0, 10.0),
            },
        }
    }

    fn create_test_workspace() -> Workspace {
        Workspace {
            id: 1,
            idx: 1,
            name: Some("Main".to_string()),
            output: Some("DP-1".to_string()),
            is_urgent: false,
            is_active: true,
            is_focused: true,
            active_window_id: Some(123),
        }
    }

    fn create_test_output() -> Output {
        Output {
            name: "DP-1".to_string(),
            make: "Samsung".to_string(),
            model: "S27AG50".to_string(),
            serial: Some("SN123456".to_string()),
            physical_size: Some((600, 340)),
            modes: vec![],
            current_mode: None,
            is_custom_mode: false,
            vrr_supported: false,
            vrr_enabled: false,
            logical: None,
        }
    }

    #[test]
    fn test_window_open_event_to_lua() {
        let lua = Lua::new();
        let window = create_test_window();
        let event = WindowEventData::Open {
            window: window.clone(),
        };

        let table = event.to_lua(&lua).unwrap();
        assert_eq!(table.get::<String>("type").unwrap(), "open");

        let window_table: Table = table.get("window").unwrap();
        assert_eq!(window_table.get::<u64>("id").unwrap(), 123);
    }

    #[test]
    fn test_window_close_event_to_lua() {
        let lua = Lua::new();
        let window = create_test_window();
        let event = WindowEventData::Close { window };

        let table = event.to_lua(&lua).unwrap();
        assert_eq!(table.get::<String>("type").unwrap(), "close");
    }

    #[test]
    fn test_window_focus_event_to_lua() {
        let lua = Lua::new();
        let window = create_test_window();
        let event = WindowEventData::Focus { window };

        let table = event.to_lua(&lua).unwrap();
        assert_eq!(table.get::<String>("type").unwrap(), "focus");
    }

    #[test]
    fn test_window_blur_event_to_lua() {
        let lua = Lua::new();
        let window = create_test_window();
        let event = WindowEventData::Blur { window };

        let table = event.to_lua(&lua).unwrap();
        assert_eq!(table.get::<String>("type").unwrap(), "blur");
    }

    #[test]
    fn test_workspace_activate_event_to_lua() {
        let lua = Lua::new();
        let workspace = create_test_workspace();
        let output = create_test_output();
        let event = WorkspaceEventData::Activate { workspace, output };

        let table = event.to_lua(&lua).unwrap();
        assert_eq!(table.get::<String>("type").unwrap(), "activate");

        let workspace_table: Table = table.get("workspace").unwrap();
        assert_eq!(workspace_table.get::<u64>("id").unwrap(), 1);

        let output_table: Table = table.get("output").unwrap();
        assert_eq!(output_table.get::<String>("name").unwrap(), "DP-1");
    }

    #[test]
    fn test_workspace_deactivate_event_to_lua() {
        let lua = Lua::new();
        let workspace = create_test_workspace();
        let event = WorkspaceEventData::Deactivate { workspace };

        let table = event.to_lua(&lua).unwrap();
        assert_eq!(table.get::<String>("type").unwrap(), "deactivate");
    }

    #[test]
    fn test_monitor_connect_event_to_lua() {
        let lua = Lua::new();
        let output = create_test_output();
        let event = MonitorEventData::Connect {
            output: output.clone(),
        };

        let table = event.to_lua(&lua).unwrap();
        assert_eq!(table.get::<String>("type").unwrap(), "connect");

        let output_table: Table = table.get("output").unwrap();
        assert_eq!(output_table.get::<String>("name").unwrap(), "DP-1");
    }

    #[test]
    fn test_monitor_disconnect_event_to_lua() {
        let lua = Lua::new();
        let output = create_test_output();
        let event = MonitorEventData::Disconnect { output };

        let table = event.to_lua(&lua).unwrap();
        assert_eq!(table.get::<String>("type").unwrap(), "disconnect");
    }

    #[test]
    fn test_layout_mode_changed_event_to_lua() {
        let lua = Lua::new();
        let event = LayoutEventData::ModeChanged { is_floating: true };

        let table = event.to_lua(&lua).unwrap();
        assert_eq!(table.get::<String>("type").unwrap(), "mode_changed");
        assert_eq!(table.get::<bool>("is_floating").unwrap(), true);
    }

    #[test]
    fn test_generic_event_data_window() {
        let lua = Lua::new();
        let window = create_test_window();
        let event = EventData::Window(WindowEventData::Open { window });

        assert_eq!(event.category(), "window");

        let table = event.to_lua(&lua).unwrap();
        assert_eq!(table.get::<String>("type").unwrap(), "open");
    }

    #[test]
    fn test_generic_event_data_workspace() {
        let lua = Lua::new();
        let workspace = create_test_workspace();
        let output = create_test_output();
        let event = EventData::Workspace(WorkspaceEventData::Activate { workspace, output });

        assert_eq!(event.category(), "workspace");

        let table = event.to_lua(&lua).unwrap();
        assert_eq!(table.get::<String>("type").unwrap(), "activate");
    }

    #[test]
    fn test_generic_event_data_monitor() {
        let lua = Lua::new();
        let output = create_test_output();
        let event = EventData::Monitor(MonitorEventData::Connect { output });

        assert_eq!(event.category(), "monitor");

        let table = event.to_lua(&lua).unwrap();
        assert_eq!(table.get::<String>("type").unwrap(), "connect");
    }

    #[test]
    fn test_generic_event_data_layout() {
        let lua = Lua::new();
        let event = EventData::Layout(LayoutEventData::ModeChanged { is_floating: false });

        assert_eq!(event.category(), "layout");

        let table = event.to_lua(&lua).unwrap();
        assert_eq!(table.get::<String>("type").unwrap(), "mode_changed");
    }
}
