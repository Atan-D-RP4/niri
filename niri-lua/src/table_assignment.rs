use mlua::ObjectLike;

/// Generic table assignment that works with any proxy UserData.
/// Reuses proxy's existing __newindex setters - no per-field dispatch needed.
pub fn apply_table_to_proxy(proxy_ud: &mlua::AnyUserData, table: mlua::Table) -> mlua::Result<()> {
    for pair in table.pairs::<String, mlua::Value>() {
        let (key, value) = pair?;
        if let mlua::Value::Table(nested) = &value {
            // Get nested proxy via __index, then recurse
            match proxy_ud.get::<mlua::AnyUserData>(key.clone()) {
                Ok(nested_proxy) => {
                    apply_table_to_proxy(&nested_proxy, nested.clone())?;
                }
                Err(e) => {
                    log::warn!("{}: failed to get nested proxy: {}", key, e);
                }
            }
        } else {
            // Use proxy's existing __newindex setter
            if let Err(e) = proxy_ud.set(key.clone(), value) {
                log::warn!("{}: {}", key, e);
                // Continue processing other fields (warn-but-continue)
            }
        }
    }
    Ok(())
}
