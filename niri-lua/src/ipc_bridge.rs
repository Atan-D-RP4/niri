//! Bridge between niri IPC types and Lua tables.
//!
//! This module provides conversion functions from niri's IPC types (Window, Workspace, Output,
//! etc.) into Lua tables that can be accessed from Lua scripts.

use mlua::{Lua, Result, Table, Value};
use niri_ipc::{Output, Window, Workspace};

/// Convert a Window to a Lua table.
///
/// Creates a table with the following structure:
/// ```lua
/// {
///     id = 123,
///     title = "Window Title",
///     app_id = "org.app.Name",
///     pid = 4567,
///     workspace_id = 1,
///     is_focused = true,
///     is_floating = false,
///     is_urgent = false,
///     layout = {
///         pos_in_scrolling_layout = { 1, 1 },  -- (column, tile)
///         tile_size = { 1920.0, 1080.0 },      -- (width, height)
///         window_size = { 1900, 1060 },        -- (width, height)
///         tile_pos_in_workspace_view = { 0.0, 0.0 },
///         window_offset_in_tile = { 10.0, 10.0 },
///     }
/// }
/// ```
pub fn window_to_lua(lua: &Lua, window: &Window) -> Result<Table> {
    let table = lua.create_table()?;

    table.set("id", window.id)?;
    table.set("title", window.title.clone())?;
    table.set("app_id", window.app_id.clone())?;
    table.set("pid", window.pid)?;
    table.set("workspace_id", window.workspace_id)?;
    table.set("is_focused", window.is_focused)?;
    table.set("is_floating", window.is_floating)?;
    table.set("is_urgent", window.is_urgent)?;

    // Convert layout
    let layout_table = lua.create_table()?;
    let layout = &window.layout;

    // pos_in_scrolling_layout is Option<(usize, usize)>
    if let Some((col, tile)) = layout.pos_in_scrolling_layout {
        let pos = lua.create_table()?;
        pos.set(1, col)?;
        pos.set(2, tile)?;
        layout_table.set("pos_in_scrolling_layout", pos)?;
    } else {
        layout_table.set("pos_in_scrolling_layout", Value::Nil)?;
    }

    // tile_size is (f64, f64)
    let tile_size = lua.create_table()?;
    tile_size.set(1, layout.tile_size.0)?;
    tile_size.set(2, layout.tile_size.1)?;
    layout_table.set("tile_size", tile_size)?;

    // window_size is (i32, i32)
    let window_size = lua.create_table()?;
    window_size.set(1, layout.window_size.0)?;
    window_size.set(2, layout.window_size.1)?;
    layout_table.set("window_size", window_size)?;

    // tile_pos_in_workspace_view is Option<(f64, f64)>
    if let Some((x, y)) = layout.tile_pos_in_workspace_view {
        let pos = lua.create_table()?;
        pos.set(1, x)?;
        pos.set(2, y)?;
        layout_table.set("tile_pos_in_workspace_view", pos)?;
    } else {
        layout_table.set("tile_pos_in_workspace_view", Value::Nil)?;
    }

    // window_offset_in_tile is (f64, f64)
    let offset = lua.create_table()?;
    offset.set(1, layout.window_offset_in_tile.0)?;
    offset.set(2, layout.window_offset_in_tile.1)?;
    layout_table.set("window_offset_in_tile", offset)?;

    table.set("layout", layout_table)?;

    Ok(table)
}

/// Convert a Workspace to a Lua table.
///
/// Creates a table with the following structure:
/// ```lua
/// {
///     id = 1,
///     idx = 1,
///     name = "Workspace Name",
///     output = "DP-1",
///     is_urgent = false,
///     is_active = true,
///     is_focused = true,
///     active_window_id = 123,
/// }
/// ```
pub fn workspace_to_lua(lua: &Lua, workspace: &Workspace) -> Result<Table> {
    let table = lua.create_table()?;

    table.set("id", workspace.id)?;
    table.set("idx", workspace.idx)?;
    table.set("name", workspace.name.clone())?;
    table.set("output", workspace.output.clone())?;
    table.set("is_urgent", workspace.is_urgent)?;
    table.set("is_active", workspace.is_active)?;
    table.set("is_focused", workspace.is_focused)?;
    table.set("active_window_id", workspace.active_window_id)?;

    Ok(table)
}

/// Convert an Output to a Lua table.
///
/// Creates a table with the following structure:
/// ```lua
/// {
///     name = "DP-1",
///     make = "Samsung",
///     model = "S27AG50",
///     serial = "SERIAL123",
///     physical_size = { 600, 340 },  -- (width_mm, height_mm)
///     current_mode = 1,
///     is_custom_mode = false,
///     vrr_supported = true,
///     vrr_enabled = false,
///     logical = {
///         x = 0,
///         y = 0,
///         width = 2560,
///         height = 1440,
///         scale = 1.0,
///         transform = "normal",
///     },
/// }
/// ```
pub fn output_to_lua(lua: &Lua, output: &Output) -> Result<Table> {
    let table = lua.create_table()?;

    table.set("name", output.name.clone())?;
    table.set("make", output.make.clone())?;
    table.set("model", output.model.clone())?;
    table.set("serial", output.serial.clone())?;

    // physical_size is Option<(u32, u32)>
    if let Some((width, height)) = output.physical_size {
        let size = lua.create_table()?;
        size.set(1, width)?;
        size.set(2, height)?;
        table.set("physical_size", size)?;
    } else {
        table.set("physical_size", Value::Nil)?;
    }

    table.set("current_mode", output.current_mode)?;
    table.set("is_custom_mode", output.is_custom_mode)?;
    table.set("vrr_supported", output.vrr_supported)?;
    table.set("vrr_enabled", output.vrr_enabled)?;

    // logical is Option<LogicalOutput>
    if let Some(logical) = &output.logical {
        let logical_table = lua.create_table()?;
        logical_table.set("x", logical.x)?;
        logical_table.set("y", logical.y)?;
        logical_table.set("width", logical.width)?;
        logical_table.set("height", logical.height)?;
        logical_table.set("scale", logical.scale)?;
        logical_table.set(
            "transform",
            format!("{:?}", logical.transform).to_lowercase(),
        )?;
        table.set("logical", logical_table)?;
    } else {
        table.set("logical", Value::Nil)?;
    }

    Ok(table)
}

/// Convert a Vec of Windows to a Lua array.
pub fn windows_to_lua(lua: &Lua, windows: &[Window]) -> Result<Table> {
    let table = lua.create_table()?;
    for (i, window) in windows.iter().enumerate() {
        let window_table = window_to_lua(lua, window)?;
        table.set(i + 1, window_table)?; // Lua arrays are 1-indexed
    }
    Ok(table)
}

/// Convert a Vec of Workspaces to a Lua array.
pub fn workspaces_to_lua(lua: &Lua, workspaces: &[Workspace]) -> Result<Table> {
    let table = lua.create_table()?;
    for (i, workspace) in workspaces.iter().enumerate() {
        let workspace_table = workspace_to_lua(lua, workspace)?;
        table.set(i + 1, workspace_table)?; // Lua arrays are 1-indexed
    }
    Ok(table)
}

#[cfg(test)]
mod tests {
    use niri_ipc::WindowLayout;

    use super::*;
    use crate::test_utils::{create_test_window, create_test_workspace};

    #[test]
    fn test_window_to_lua() {
        let lua = Lua::new();
        let window = Window {
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
        };

        let table = window_to_lua(&lua, &window).unwrap();

        // Verify basic fields
        assert_eq!(table.get::<u64>("id").unwrap(), 123);
        assert_eq!(table.get::<String>("title").unwrap(), "Test Window");
        assert!(table.get::<bool>("is_focused").unwrap());

        // Verify layout exists
        let layout: Table = table.get("layout").unwrap();
        let tile_size: Table = layout.get("tile_size").unwrap();
        assert_eq!(tile_size.get::<f64>(1).unwrap(), 1920.0);
        assert_eq!(tile_size.get::<f64>(2).unwrap(), 1080.0);
    }

    #[test]
    fn test_window_to_lua_minimal() {
        let lua = Lua::new();
        let window = Window {
            id: 1,
            title: None,
            app_id: None,
            pid: None,
            workspace_id: None,
            is_focused: false,
            is_floating: false,
            is_urgent: false,
            focus_timestamp: None,
            layout: WindowLayout {
                pos_in_scrolling_layout: None,
                tile_size: (0.0, 0.0),
                window_size: (0, 0),
                tile_pos_in_workspace_view: None,
                window_offset_in_tile: (0.0, 0.0),
            },
        };

        let table = window_to_lua(&lua, &window).unwrap();
        assert_eq!(table.get::<u64>("id").unwrap(), 1);
        assert!(!table.get::<bool>("is_focused").unwrap());
    }

    #[test]
    fn test_window_to_lua_floating() {
        let lua = Lua::new();
        let window = Window {
            id: 456,
            title: Some("Floating Window".to_string()),
            app_id: Some("org.test.FloatApp".to_string()),
            pid: Some(7890),
            workspace_id: Some(2),
            is_focused: false,
            is_floating: true,
            is_urgent: true,
            focus_timestamp: None,
            layout: WindowLayout {
                pos_in_scrolling_layout: None,
                tile_size: (800.0, 600.0),
                window_size: (800, 600),
                tile_pos_in_workspace_view: None,
                window_offset_in_tile: (0.0, 0.0),
            },
        };

        let table = window_to_lua(&lua, &window).unwrap();
        assert_eq!(table.get::<u64>("id").unwrap(), 456);
        assert!(table.get::<bool>("is_floating").unwrap());
        assert!(table.get::<bool>("is_urgent").unwrap());
    }

    #[test]
    fn test_workspace_to_lua() {
        let lua = Lua::new();
        let workspace = Workspace {
            id: 1,
            idx: 1,
            name: Some("Main".to_string()),
            output: Some("DP-1".to_string()),
            is_urgent: false,
            is_active: true,
            is_focused: true,
            active_window_id: Some(123),
        };

        let table = workspace_to_lua(&lua, &workspace).unwrap();

        assert_eq!(table.get::<u64>("id").unwrap(), 1);
        assert_eq!(table.get::<u8>("idx").unwrap(), 1);
        assert_eq!(table.get::<String>("name").unwrap(), "Main");
        assert!(table.get::<bool>("is_focused").unwrap());
    }

    #[test]
    fn test_workspace_to_lua_minimal() {
        let lua = Lua::new();
        let workspace = Workspace {
            id: 1,
            idx: 1,
            name: None,
            output: None,
            is_urgent: false,
            is_active: false,
            is_focused: false,
            active_window_id: None,
        };

        let table = workspace_to_lua(&lua, &workspace).unwrap();
        assert_eq!(table.get::<u64>("id").unwrap(), 1);
        assert!(!table.get::<bool>("is_active").unwrap());
    }

    #[test]
    fn test_workspace_to_lua_urgent() {
        let lua = Lua::new();
        let workspace = Workspace {
            id: 2,
            idx: 2,
            name: Some("Work".to_string()),
            output: Some("HDMI-1".to_string()),
            is_urgent: true,
            is_active: true,
            is_focused: false,
            active_window_id: Some(789),
        };

        let table = workspace_to_lua(&lua, &workspace).unwrap();
        assert_eq!(table.get::<u64>("id").unwrap(), 2);
        assert!(table.get::<bool>("is_urgent").unwrap());
    }

    // ========================================================================
    // output_to_lua tests
    // ========================================================================

    #[test]
    fn test_output_to_lua() {
        use niri_ipc::{LogicalOutput, Transform};

        let lua = Lua::new();
        let output = Output {
            name: "HDMI-1".to_string(),
            make: "Dell".to_string(),
            model: "U2415".to_string(),
            serial: Some("SN123456".to_string()),
            physical_size: Some((530, 300)),
            modes: vec![],
            current_mode: Some(0),
            is_custom_mode: false,
            vrr_supported: true,
            vrr_enabled: false,
            logical: Some(LogicalOutput {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                scale: 1.0,
                transform: Transform::Normal,
            }),
        };

        let table = output_to_lua(&lua, &output).unwrap();
        assert_eq!(table.get::<String>("name").unwrap(), "HDMI-1");
        assert_eq!(table.get::<String>("make").unwrap(), "Dell");
        assert_eq!(table.get::<String>("model").unwrap(), "U2415");
    }

    #[test]
    fn test_output_to_lua_no_logical() {
        use mlua::prelude::LuaValue;

        let lua = Lua::new();
        let output = Output {
            name: "DP-1".to_string(),
            make: "LG".to_string(),
            model: "27UP550".to_string(),
            serial: None,
            physical_size: None,
            modes: vec![],
            current_mode: None,
            is_custom_mode: false,
            vrr_supported: false,
            vrr_enabled: false,
            logical: None,
        };

        let table = output_to_lua(&lua, &output).unwrap();
        assert_eq!(table.get::<String>("name").unwrap(), "DP-1");
        let logical: LuaValue = table.get("logical").unwrap();
        assert!(matches!(logical, LuaValue::Nil));
    }

    // ========================================================================
    // windows_to_lua tests
    // ========================================================================

    #[test]
    fn test_windows_to_lua_empty() {
        let lua = Lua::new();
        let windows: Vec<Window> = vec![];

        let table = windows_to_lua(&lua, &windows).unwrap();
        assert_eq!(table.len().unwrap(), 0);
    }

    #[test]
    fn test_windows_to_lua_single() {
        let lua = Lua::new();
        let window = Window {
            id: 1,
            title: Some("Window 1".to_string()),
            app_id: None,
            pid: None,
            workspace_id: None,
            is_focused: false,
            is_floating: false,
            is_urgent: false,
            focus_timestamp: None,
            layout: WindowLayout {
                pos_in_scrolling_layout: None,
                tile_size: (0.0, 0.0),
                window_size: (0, 0),
                tile_pos_in_workspace_view: None,
                window_offset_in_tile: (0.0, 0.0),
            },
        };
        let windows = vec![window];

        let table = windows_to_lua(&lua, &windows).unwrap();
        assert_eq!(table.len().unwrap(), 1);

        let first_window: Table = table.get(1).unwrap();
        assert_eq!(first_window.get::<u64>("id").unwrap(), 1);
    }

    #[test]
    fn test_windows_to_lua_multiple() {
        let lua = Lua::new();
        let windows = vec![
            create_test_window(1),
            create_test_window(2),
            create_test_window(3),
        ];

        let table = windows_to_lua(&lua, &windows).unwrap();
        assert_eq!(table.len().unwrap(), 3);

        let second_window: Table = table.get(2).unwrap();
        assert_eq!(second_window.get::<u64>("id").unwrap(), 2);
    }

    // ========================================================================
    // workspaces_to_lua tests
    // ========================================================================

    #[test]
    fn test_workspaces_to_lua_empty() {
        let lua = Lua::new();
        let workspaces: Vec<Workspace> = vec![];

        let table = workspaces_to_lua(&lua, &workspaces).unwrap();
        assert_eq!(table.len().unwrap(), 0);
    }

    #[test]
    fn test_workspaces_to_lua_single() {
        let lua = Lua::new();
        let workspace = Workspace {
            id: 1,
            idx: 1,
            name: Some("Main".to_string()),
            output: Some("DP-1".to_string()),
            is_urgent: false,
            is_active: true,
            is_focused: true,
            active_window_id: None,
        };
        let workspaces = vec![workspace];

        let table = workspaces_to_lua(&lua, &workspaces).unwrap();
        assert_eq!(table.len().unwrap(), 1);

        let first_workspace: Table = table.get(1).unwrap();
        assert_eq!(first_workspace.get::<u64>("id").unwrap(), 1);
    }

    #[test]
    fn test_workspaces_to_lua_multiple() {
        let lua = Lua::new();
        let workspaces = vec![
            create_test_workspace(1),
            create_test_workspace(2),
            create_test_workspace(3),
            create_test_workspace(4),
        ];

        let table = workspaces_to_lua(&lua, &workspaces).unwrap();
        assert_eq!(table.len().unwrap(), 4);

        let third_workspace: Table = table.get(3).unwrap();
        assert_eq!(third_workspace.get::<u64>("id").unwrap(), 3);
    }

    // ========================================================================
}
