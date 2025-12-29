use mlua::prelude::*;
use niri_lua_traits::{extract_string_opt, FromLuaTable};

use crate::ScreenshotPath;

impl FromLuaTable for ScreenshotPath {
    fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
        if let Ok(false) = table.get::<bool>("path") {
            return Ok(Some(ScreenshotPath(None)));
        }
        let path = extract_string_opt(table, "path")?;
        Ok(Some(ScreenshotPath(path)))
    }
}
