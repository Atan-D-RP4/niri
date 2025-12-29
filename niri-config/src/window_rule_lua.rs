use mlua::prelude::*;
use niri_lua_traits::{
    extract_bool_opt, extract_float_opt, extract_int_opt, extract_string_opt, extract_table_opt,
    FromLuaTable,
};
use regex::Regex;

use crate::layout::PresetSize;
use crate::utils::RegexEq;
use crate::window_rule::{Match, WindowRule};

fn extract_regex(field: &str, table: &LuaTable) -> LuaResult<Option<Regex>> {
    if let Some(value) = extract_string_opt(table, field)? {
        let regex = Regex::new(&value)
            .map_err(|e| LuaError::external(format!("Invalid {field} regex: {e}")))?;
        Ok(Some(regex))
    } else {
        Ok(None)
    }
}

fn extract_window_match(table: &LuaTable) -> LuaResult<Option<Match>> {
    let app_id = extract_regex("app_id", table)?.map(RegexEq);
    let title = extract_regex("title", table)?.map(RegexEq);
    let is_active = extract_bool_opt(table, "is_active")?;
    let is_focused = extract_bool_opt(table, "is_focused")?;

    if app_id.is_none() && title.is_none() && is_active.is_none() && is_focused.is_none() {
        return Ok(None);
    }

    Ok(Some(Match {
        app_id,
        title,
        is_active,
        is_focused,
        ..Default::default()
    }))
}

fn extract_window_matches(table: &LuaTable, field: &str) -> LuaResult<Vec<Match>> {
    if let Some(array_table) = extract_table_opt(table, field)? {
        let mut matches = Vec::new();
        for i in 1..=array_table.len()? {
            if let Ok(match_table) = array_table.get::<LuaTable>(i) {
                if let Some(m) = extract_window_match(&match_table)? {
                    matches.push(m);
                }
            }
        }
        return Ok(matches);
    }

    Ok(Vec::new())
}

fn extract_size_change(table: &LuaTable) -> LuaResult<Option<PresetSize>> {
    if let Some(proportion) = extract_float_opt(table, "proportion")? {
        return Ok(Some(PresetSize::Proportion(proportion)));
    }
    if let Some(fixed) = extract_int_opt(table, "fixed")? {
        return Ok(Some(PresetSize::Fixed(fixed as i32)));
    }
    Ok(None)
}

impl FromLuaTable for WindowRule {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        let matches = extract_window_matches(table, "matches")?;
        let excludes = extract_window_matches(table, "excludes")?;
        let default_column_width =
            if let Some(size_table) = extract_table_opt(table, "default_column_width")? {
                extract_size_change(&size_table)?
                    .map(|size| crate::layout::DefaultPresetSize(Some(size)))
            } else {
                None
            };
        let open_on_output = extract_string_opt(table, "open_on_output")?;
        let open_on_workspace = extract_string_opt(table, "open_on_workspace")?;
        let open_maximized = extract_bool_opt(table, "open_maximized")?;
        let open_fullscreen = extract_bool_opt(table, "open_fullscreen")?;
        let open_floating = extract_bool_opt(table, "open_floating")?;
        let opacity = extract_float_opt(table, "opacity")?.map(|v| v as f32);

        if matches.is_empty()
            && excludes.is_empty()
            && default_column_width.is_none()
            && open_on_output.is_none()
            && open_on_workspace.is_none()
            && open_maximized.is_none()
            && open_fullscreen.is_none()
            && open_floating.is_none()
            && opacity.is_none()
        {
            return Ok(None);
        }

        Ok(Some(WindowRule {
            matches,
            excludes,
            default_column_width,
            default_window_height: None,
            open_on_output,
            open_on_workspace,
            open_maximized,
            open_maximized_to_edges: None,
            open_fullscreen,
            open_floating,
            open_focused: None,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            focus_ring: Default::default(),
            border: Default::default(),
            shadow: Default::default(),
            tab_indicator: Default::default(),
            draw_border_with_background: None,
            opacity,
            geometry_corner_radius: None,
            clip_to_geometry: None,
            baba_is_float: None,
            block_out_from: None,
            variable_refresh_rate: None,
            default_column_display: None,
            default_floating_position: None,
            scroll_factor: None,
            tiled_state: None,
        }))
    }
}
