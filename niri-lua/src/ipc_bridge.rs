//! Bridge between niri IPC types and Lua tables.
//!
//! This module provides conversion functions from niri's IPC types (Window, Workspace, Output, etc.)
//! into Lua tables that can be accessed from Lua scripts.

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
        logical_table.set("transform", format!("{:?}", logical.transform).to_lowercase())?;
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
    use super::*;
    use niri_ipc::WindowLayout;

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
        assert_eq!(
            table.get::<String>("title").unwrap(),
            "Test Window"
        );
        assert_eq!(table.get::<bool>("is_focused").unwrap(), true);

        // Verify layout exists
        let layout: Table = table.get("layout").unwrap();
        let tile_size: Table = layout.get("tile_size").unwrap();
        assert_eq!(tile_size.get::<f64>(1).unwrap(), 1920.0);
        assert_eq!(tile_size.get::<f64>(2).unwrap(), 1080.0);
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
        assert_eq!(table.get::<bool>("is_focused").unwrap(), true);
    }
}
