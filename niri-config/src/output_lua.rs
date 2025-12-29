use mlua::prelude::*;
use niri_ipc::{ConfiguredMode, HSyncPolarity, Transform, VSyncPolarity};
use niri_lua_traits::{
    extract_bool_opt, extract_float_opt, extract_int_opt, extract_string_opt, extract_table_opt,
    FromLuaTable,
};

use crate::output::{Mode, Modeline, Output, Position, Vrr};
use crate::FloatOrInt;

fn parse_output_transform(s: &str) -> Option<Transform> {
    match s {
        "normal" => Some(Transform::Normal),
        "90" => Some(Transform::_90),
        "180" => Some(Transform::_180),
        "270" => Some(Transform::_270),
        "flipped" => Some(Transform::Flipped),
        "flipped-90" => Some(Transform::Flipped90),
        "flipped-180" => Some(Transform::Flipped180),
        "flipped-270" => Some(Transform::Flipped270),
        _ => None,
    }
}

fn parse_hsync_polarity(s: &str) -> Option<HSyncPolarity> {
    match s {
        "+hsync" => Some(HSyncPolarity::PHSync),
        "-hsync" => Some(HSyncPolarity::NHSync),
        _ => None,
    }
}

fn parse_vsync_polarity(s: &str) -> Option<VSyncPolarity> {
    match s {
        "+vsync" => Some(VSyncPolarity::PVSync),
        "-vsync" => Some(VSyncPolarity::NVSync),
        _ => None,
    }
}

impl FromLuaTable for Mode {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        let width = extract_int_opt(table, "width")?.unwrap_or(0) as u16;
        let height = extract_int_opt(table, "height")?.unwrap_or(0) as u16;
        let refresh = extract_float_opt(table, "refresh")?;
        let custom = extract_bool_opt(table, "custom")?.unwrap_or(false);
        Ok(Some(Mode {
            custom,
            mode: ConfiguredMode {
                width,
                height,
                refresh,
            },
        }))
    }
}

impl FromLuaTable for Modeline {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        Ok(Some(Modeline {
            clock: extract_float_opt(table, "clock")?.unwrap_or(0.0),
            hdisplay: extract_int_opt(table, "hdisplay")?.unwrap_or(0) as u16,
            hsync_start: extract_int_opt(table, "hsync_start")?.unwrap_or(0) as u16,
            hsync_end: extract_int_opt(table, "hsync_end")?.unwrap_or(0) as u16,
            htotal: extract_int_opt(table, "htotal")?.unwrap_or(0) as u16,
            vdisplay: extract_int_opt(table, "vdisplay")?.unwrap_or(0) as u16,
            vsync_start: extract_int_opt(table, "vsync_start")?.unwrap_or(0) as u16,
            vsync_end: extract_int_opt(table, "vsync_end")?.unwrap_or(0) as u16,
            vtotal: extract_int_opt(table, "vtotal")?.unwrap_or(0) as u16,
            hsync_polarity: extract_string_opt(table, "hsync_polarity")?
                .and_then(|s| parse_hsync_polarity(&s))
                .unwrap_or(HSyncPolarity::PHSync),
            vsync_polarity: extract_string_opt(table, "vsync_polarity")?
                .and_then(|s| parse_vsync_polarity(&s))
                .unwrap_or(VSyncPolarity::PVSync),
        }))
    }
}

impl FromLuaTable for Position {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        Ok(Some(Position {
            x: extract_int_opt(table, "x")?.unwrap_or(0) as i32,
            y: extract_int_opt(table, "y")?.unwrap_or(0) as i32,
        }))
    }
}

impl FromLuaTable for Output {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        let mut result = Output::default();

        if let Some(v) = extract_bool_opt(table, "off")? {
            result.off = v;
        }
        if let Some(v) = extract_string_opt(table, "name")? {
            result.name = v;
        }
        if let Some(v) = extract_float_opt(table, "scale")? {
            result.scale = Some(FloatOrInt(v));
        }
        if let Some(s) = extract_string_opt(table, "transform")? {
            if let Some(t) = parse_output_transform(&s) {
                result.transform = t;
            }
        }
        if let Some(t) = extract_table_opt(table, "position")? {
            result.position = Position::from_lua_table(&t)?;
        }
        if let Some(t) = extract_table_opt(table, "mode")? {
            result.mode = Mode::from_lua_table(&t)?;
        }
        if let Some(t) = extract_table_opt(table, "modeline")? {
            result.modeline = Modeline::from_lua_table(&t)?;
        }
        if let Some(v) = extract_bool_opt(table, "variable-refresh-rate")? {
            result.variable_refresh_rate = Some(Vrr { on_demand: !v });
        }
        if let Some(s) = extract_string_opt(table, "variable-refresh-rate")? {
            result.variable_refresh_rate = match s.as_str() {
                "on" | "true" => Some(Vrr { on_demand: false }),
                "on-demand" => Some(Vrr { on_demand: true }),
                _ => None,
            };
        }

        Ok(Some(result))
    }
}
