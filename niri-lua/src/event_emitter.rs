//! Event emitter system for Niri Lua runtime.
//!
//! This module provides an event-driven architecture for Niri plugins,
//! allowing registration and dispatching of events via Lua global tables.
//!
//! # Events
//!
//! Events are dispatched through the `niri.events` namespace:
//!
//! ```lua
//! -- Register event handler
//! niri.events:on("window:open", function(event)
//!   print("Window opened: " .. event.window.title)
//! end)
//!
//! -- Register one-time handler
//! niri.events:once("workspace:enter", function(event)
//!   print("Entered workspace (fires only once)")
//! end)
//!
//! -- Remove handler
//! local handler_id = niri.events:on("action", function(event)
//!   -- handler code
//! end)
//! niri.events:off("action", handler_id)
//! ```

use log::{debug, error};
use mlua::prelude::*;

/// Register event emitter API to Lua.
///
/// This uses Lua global tables to store handlers, which is simpler and avoids
/// the need for `Send + Sync` on `LuaFunction`. Handlers are stored in:
/// - `__niri_event_handlers`: table mapping event names to handler tables
/// - `__niri_next_handler_id`: counter for unique handler IDs
/// - `__niri_once_handlers`: set of handler IDs that should only fire once
pub fn register_to_lua(lua: &Lua) -> LuaResult<()> {
    let niri_table: LuaTable = lua.globals().get("niri")?;

    // Create events namespace
    let events = lua.create_table()?;

    // Use global tables to store handlers (simpler than Arc<Mutex<...>>)
    let event_handlers = lua.create_table()?;
    lua.globals()
        .set("__niri_event_handlers", event_handlers.clone())?;

    let next_handler_id = lua.create_table()?;
    next_handler_id.set("value", 1_u64)?;
    lua.globals()
        .set("__niri_next_handler_id", next_handler_id)?;

    // Register on() function
    // Note: Using method syntax (niri.events:on) passes self as first arg
    events.set(
        "on",
        lua.create_function(
            |lua, (_self_table, event_name, handler): (LuaTable, String, LuaFunction)| {
                let handlers: LuaTable = lua.globals().get("__niri_event_handlers")?;
                let next_id_table: LuaTable = lua.globals().get("__niri_next_handler_id")?;

                let handler_id: u64 = next_id_table.get("value")?;
                next_id_table.set("value", handler_id + 1)?;

                // Get or create the event's handler list
                let event_handlers: LuaTable = match handlers.get(event_name.as_str()) {
                    Ok(LuaValue::Table(t)) => t,
                    _ => lua.create_table()?,
                };

                // Add handler to list
                event_handlers.set(handler_id, handler)?;
                handlers.set(event_name.as_str(), event_handlers)?;

                Ok(handler_id)
            },
        )?,
    )?;

    // Register once() function
    // Note: Using method syntax (niri.events:once) passes self as first arg
    events.set(
        "once",
        lua.create_function(
            |lua, (_self_table, event_name, handler): (LuaTable, String, LuaFunction)| {
                let handlers: LuaTable = lua.globals().get("__niri_event_handlers")?;
                let next_id_table: LuaTable = lua.globals().get("__niri_next_handler_id")?;

                let handler_id: u64 = next_id_table.get("value")?;
                next_id_table.set("value", handler_id + 1)?;

                // Mark this handler as "once"
                let once_handlers: LuaTable = match lua.globals().get("__niri_once_handlers") {
                    Ok(LuaValue::Table(t)) => t,
                    _ => lua.create_table()?,
                };
                once_handlers.set(handler_id, true)?;
                lua.globals().set("__niri_once_handlers", once_handlers)?;

                // Get or create the event's handler list
                let event_handlers: LuaTable = match handlers.get(event_name.as_str()) {
                    Ok(LuaValue::Table(t)) => t,
                    _ => lua.create_table()?,
                };

                // Add handler to list
                event_handlers.set(handler_id, handler)?;
                handlers.set(event_name.as_str(), event_handlers)?;

                Ok(handler_id)
            },
        )?,
    )?;

    // Register off() function
    // Note: Using method syntax (niri.events:off) passes self as first arg
    events.set(
        "off",
        lua.create_function(
            |lua, (_self_table, event_name, handler_id): (LuaTable, String, u64)| {
                let handlers: LuaTable = lua.globals().get("__niri_event_handlers")?;
                if let Ok(LuaValue::Table(event_handlers)) = handlers.get(event_name.as_str()) {
                    event_handlers.set(handler_id, LuaValue::Nil)?;
                }
                Ok(())
            },
        )?,
    )?;

    // Register emit() function
    // Note: Using method syntax (niri.events:emit) passes self as first arg
    events.set(
        "emit",
        lua.create_function(
            |lua, (_self_table, event_name, data): (LuaTable, String, LuaValue)| {
                let handlers: LuaTable = lua.globals().get("__niri_event_handlers")?;
                let once_handlers: LuaTable = match lua.globals().get("__niri_once_handlers") {
                    Ok(LuaValue::Table(t)) => t,
                    _ => lua.create_table()?,
                };

                if let Ok(LuaValue::Table(event_handlers)) = handlers.get(event_name.as_str()) {
                    for pair in event_handlers.pairs::<u64, LuaValue>() {
                        let (id, value) = pair?;
                        if let LuaValue::Function(handler) = value {
                            // Execute the handler
                            match handler.call::<()>(data.clone()) {
                                Ok(_) => {
                                    debug!("Handler executed for event '{}'", event_name);
                                }
                                Err(e) => {
                                    error!("Error in event handler for '{}': {}", event_name, e);
                                }
                            }

                            // Check if this was a once handler
                            if once_handlers.get(id).unwrap_or(false) {
                                event_handlers.set(id, LuaValue::Nil)?;
                                once_handlers.set(id, LuaValue::Nil)?;
                            }
                        }
                    }
                }

                Ok(())
            },
        )?,
    )?;

    niri_table.set("events", events)?;

    debug!("Registered event emitter API to Lua");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_to_lua() {
        let lua = Lua::new();

        // Create niri namespace
        let niri = lua.create_table().unwrap();
        lua.globals().set("niri", niri).unwrap();

        // Register event emitter
        register_to_lua(&lua).unwrap();

        // Check that events namespace was created
        let niri: LuaTable = lua.globals().get("niri").unwrap();
        let events: LuaTable = niri.get("events").unwrap();

        // Check that required functions exist
        let _on: LuaFunction = events.get("on").unwrap();
        let _off: LuaFunction = events.get("off").unwrap();
        let _once: LuaFunction = events.get("once").unwrap();
        let _emit: LuaFunction = events.get("emit").unwrap();
    }

    #[test]
    fn test_event_registration_and_emit() {
        let lua = Lua::new();

        // Create niri namespace and register
        let niri = lua.create_table().unwrap();
        lua.globals().set("niri", niri).unwrap();
        register_to_lua(&lua).unwrap();

        // Test event registration and emission via Lua
        lua.load(
            r#"
            __test_count = 0
            local id = niri.events:on("test", function(data)
                __test_count = __test_count + 1
            end)
            niri.events:emit("test", {})
            niri.events:emit("test", {})
            "#,
        )
        .exec()
        .unwrap();

        let count: i32 = lua.globals().get("__test_count").unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_once_handler() {
        let lua = Lua::new();

        // Create niri namespace and register
        let niri = lua.create_table().unwrap();
        lua.globals().set("niri", niri).unwrap();
        register_to_lua(&lua).unwrap();

        // Test once handler fires only once
        lua.load(
            r#"
            __once_count = 0
            niri.events:once("test", function(data)
                __once_count = __once_count + 1
            end)
            niri.events:emit("test", {})
            niri.events:emit("test", {})
            "#,
        )
        .exec()
        .unwrap();

        let count: i32 = lua.globals().get("__once_count").unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_off_handler() {
        let lua = Lua::new();

        // Create niri namespace and register
        let niri = lua.create_table().unwrap();
        lua.globals().set("niri", niri).unwrap();
        register_to_lua(&lua).unwrap();

        // Test handler removal
        lua.load(
            r#"
            __off_count = 0
            local id = niri.events:on("test", function(data)
                __off_count = __off_count + 1
            end)
            niri.events:emit("test", {})
            niri.events:off("test", id)
            niri.events:emit("test", {})
            "#,
        )
        .exec()
        .unwrap();

        let count: i32 = lua.globals().get("__off_count").unwrap();
        assert_eq!(count, 1);
    }
}
