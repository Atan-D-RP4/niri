//! Events proxy system for the new `niri.events` namespace.
//!
//! This module implements Phase R4 of the API refactor, providing a cleaner event API:
//! - `niri.events:on(event_name, callback)` - Register persistent handler
//! - `niri.events:once(event_name, callback)` - Register one-time handler
//! - `niri.events:off(event_name, handler_id)` - Remove handler
//! - `niri.events:emit(event_name, data)` - Emit custom events
//!
//! This follows the same pattern as `niri.config:*` from Phase R1/R2 for consistency.

use log::debug;
use mlua::prelude::*;

use crate::event_handlers::EventHandlerId;
use crate::event_system::SharedEventHandlers;

/// Proxy for the `niri.events` namespace.
///
/// This struct implements `UserData` to provide the new event API methods.
/// It wraps the existing `SharedEventHandlers` to reuse the event handler registry.
#[derive(Clone)]
pub struct EventsProxy {
    /// Reference to the shared event handlers
    handlers: SharedEventHandlers,
}

impl EventsProxy {
    /// Create a new events proxy
    pub fn new(handlers: SharedEventHandlers) -> Self {
        Self { handlers }
    }
}

impl LuaUserData for EventsProxy {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // niri.events:on(event_name, callback) -> handler_id
        // Register a persistent event handler that fires on every matching event
        methods.add_method(
            "on",
            |_lua, this, (event_type, callback): (String, LuaFunction)| {
                let mut h = this.handlers.lock();
                let handler_id = h.register_handler(&event_type, callback, false);
                debug!(
                    "events:on('{}') registered handler {}",
                    event_type, handler_id
                );
                Ok(handler_id)
            },
        );

        // niri.events:once(event_name, callback) -> handler_id
        // Register a one-time event handler that fires only once
        methods.add_method(
            "once",
            |_lua, this, (event_type, callback): (String, LuaFunction)| {
                let mut h = this.handlers.lock();
                let handler_id = h.register_handler(&event_type, callback, true);
                debug!(
                    "events:once('{}') registered handler {}",
                    event_type, handler_id
                );
                Ok(handler_id)
            },
        );

        // niri.events:off(event_name, handler_id)
        // Remove a previously registered event handler
        methods.add_method(
            "off",
            |_lua, this, (event_type, handler_id): (String, EventHandlerId)| {
                let mut h = this.handlers.lock();
                let removed = h.unregister_handler(&event_type, handler_id);
                debug!(
                    "events:off('{}', {}) -> removed={}",
                    event_type, handler_id, removed
                );
                Ok(removed)
            },
        );

        // niri.events:emit(event_name, data)
        // Emit a custom event to all registered handlers
        methods.add_method(
            "emit",
            |lua, this, (event_type, data): (String, LuaValue)| {
                // Only allow custom events (user-defined events should have a custom: prefix or
                // similar) For now, we allow any event to be emitted for
                // flexibility
                debug!("events:emit('{}') triggered", event_type);

                let mut h = this.handlers.lock();

                // Convert the data to a table if it isn't already, wrapping primitives
                let event_data = match &data {
                    LuaValue::Table(_) => data.clone(),
                    LuaValue::Nil => {
                        // Create an empty table for nil data
                        let table = lua.create_table()?;
                        LuaValue::Table(table)
                    }
                    _ => {
                        // Wrap primitive values in a table with a "value" key
                        let table = lua.create_table()?;
                        table.set("value", data.clone())?;
                        LuaValue::Table(table)
                    }
                };

                h.emit_event(&event_type, event_data)?;
                Ok(())
            },
        );

        // niri.events:list(event_name?)
        // List registered handler IDs for an event, or all events if no name given
        methods.add_method("list", |lua, this, event_type: Option<String>| {
            let h = this.handlers.lock();
            let result = lua.create_table()?;

            if let Some(event) = event_type {
                // Return handler count for specific event
                let count = h.handler_count(&event);
                result.set("event", event)?;
                result.set("count", count)?;
            } else {
                // Return all event types
                let types = h.event_types();
                let events_table = lua.create_table()?;
                for (i, event) in types.iter().enumerate() {
                    let info = lua.create_table()?;
                    info.set("name", event.as_str())?;
                    info.set("count", h.handler_count(event))?;
                    events_table.set(i + 1, info)?;
                }
                result.set("events", events_table)?;
                result.set("total", h.total_handlers())?;
            }

            Ok(result)
        });

        // niri.events:clear(event_name?)
        // Clear handlers for a specific event, or all handlers if no name given
        methods.add_method("clear", |_lua, this, event_type: Option<String>| {
            let mut h = this.handlers.lock();

            if let Some(event) = event_type {
                debug!("events:clear('{}') clearing handlers", event);
                h.clear_event(&event);
            } else {
                debug!("events:clear() clearing all handlers");
                h.clear_all();
            }

            Ok(())
        });
    }
}

/// Register the events proxy to the Lua runtime.
///
/// This creates the `niri.events` table as a userdata with methods.
/// If the `niri` table doesn't exist, it will be created.
///
/// # Arguments
/// * `lua` - The Lua runtime
/// * `handlers` - Shared event handlers storage
///
/// # Returns
/// LuaResult indicating success or Lua error
pub fn register_events_proxy(lua: &Lua, handlers: SharedEventHandlers) -> LuaResult<()> {
    let globals = lua.globals();

    // Get or create the niri table
    let niri_table: LuaTable = match globals.get::<LuaValue>("niri")? {
        LuaValue::Table(t) => t,
        LuaValue::Nil => {
            let t = lua.create_table()?;
            globals.set("niri", t.clone())?;
            t
        }
        _ => return Err(LuaError::external("niri global is not a table")),
    };

    let proxy = EventsProxy::new(handlers);
    niri_table.set("events", proxy)?;

    debug!("Registered events proxy to niri.events");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parking_lot::Mutex;

    use super::*;
    use crate::event_handlers::EventHandlers;

    fn create_test_env() -> (Lua, SharedEventHandlers) {
        let lua = Lua::new();

        // Create niri namespace
        let niri = lua.create_table().unwrap();
        lua.globals().set("niri", niri).unwrap();

        let handlers = Arc::new(Mutex::new(EventHandlers::new()));
        register_events_proxy(&lua, handlers.clone()).unwrap();

        (lua, handlers)
    }

    #[test]
    fn test_events_proxy_creation() {
        let (lua, _handlers) = create_test_env();

        // Verify niri.events exists
        let result: LuaResult<LuaValue> = lua.load("return niri.events").eval();
        assert!(result.is_ok());
        assert!(!matches!(result.unwrap(), LuaValue::Nil));
    }

    #[test]
    fn test_events_on_method() {
        let (lua, handlers) = create_test_env();

        // Register a handler using the new API
        let result: LuaResult<EventHandlerId> = lua
            .load(
                r#"
            return niri.events:on("test:event", function(data)
                -- handler code
            end)
        "#,
            )
            .eval();

        assert!(result.is_ok());
        let handler_id = result.unwrap();
        assert_eq!(handler_id, 1);

        // Verify handler was registered
        let h = handlers.lock();
        assert_eq!(h.handler_count("test:event"), 1);
    }

    #[test]
    fn test_events_once_method() {
        let (lua, handlers) = create_test_env();

        // Register a one-time handler
        let result: LuaResult<EventHandlerId> = lua
            .load(
                r#"
            return niri.events:once("test:event", function(data)
                -- handler code
            end)
        "#,
            )
            .eval();

        assert!(result.is_ok());
        let handler_id = result.unwrap();
        assert_eq!(handler_id, 1);

        // Verify handler was registered
        let h = handlers.lock();
        assert_eq!(h.handler_count("test:event"), 1);
    }

    #[test]
    fn test_events_off_method() {
        let (lua, handlers) = create_test_env();

        // Register and then remove a handler
        lua.load(
            r#"
            local id = niri.events:on("test:event", function() end)
            niri.events:off("test:event", id)
        "#,
        )
        .exec()
        .unwrap();

        // Verify handler was removed
        let h = handlers.lock();
        assert_eq!(h.handler_count("test:event"), 0);
    }

    #[test]
    fn test_events_emit_method() {
        let (lua, _handlers) = create_test_env();

        // Create a flag to track if handler was called
        lua.load(
            r#"
            _test_called = false
            niri.events:on("custom:event", function(data)
                _test_called = true
            end)
            niri.events:emit("custom:event", {})
        "#,
        )
        .exec()
        .unwrap();

        // Verify handler was called
        let called: bool = lua.globals().get("_test_called").unwrap();
        assert!(called);
    }

    #[test]
    fn test_events_emit_with_data() {
        let (lua, _handlers) = create_test_env();

        // Test that emit passes data to handlers
        lua.load(
            r#"
            _test_value = nil
            niri.events:on("custom:event", function(data)
                _test_value = data.message
            end)
            niri.events:emit("custom:event", { message = "hello" })
        "#,
        )
        .exec()
        .unwrap();

        let value: String = lua.globals().get("_test_value").unwrap();
        assert_eq!(value, "hello");
    }

    #[test]
    fn test_events_list_method() {
        let (lua, _handlers) = create_test_env();

        // Register some handlers and list them
        lua.load(
            r#"
            niri.events:on("event1", function() end)
            niri.events:on("event1", function() end)
            niri.events:on("event2", function() end)
        "#,
        )
        .exec()
        .unwrap();

        // List all events
        let total: i64 = lua
            .load(
                r#"
            local info = niri.events:list()
            return info.total
        "#,
            )
            .eval()
            .unwrap();

        assert_eq!(total, 3);

        // List specific event
        let count: i64 = lua
            .load(
                r#"
            local info = niri.events:list("event1")
            return info.count
        "#,
            )
            .eval()
            .unwrap();

        assert_eq!(count, 2);
    }

    #[test]
    fn test_events_clear_specific() {
        let (lua, handlers) = create_test_env();

        // Register handlers on multiple events
        lua.load(
            r#"
            niri.events:on("event1", function() end)
            niri.events:on("event2", function() end)
            niri.events:clear("event1")
        "#,
        )
        .exec()
        .unwrap();

        let h = handlers.lock();
        assert_eq!(h.handler_count("event1"), 0);
        assert_eq!(h.handler_count("event2"), 1);
    }

    #[test]
    fn test_events_clear_all() {
        let (lua, handlers) = create_test_env();

        // Register handlers on multiple events
        lua.load(
            r#"
            niri.events:on("event1", function() end)
            niri.events:on("event2", function() end)
            niri.events:clear()
        "#,
        )
        .exec()
        .unwrap();

        let h = handlers.lock();
        assert_eq!(h.total_handlers(), 0);
    }

    #[test]
    fn test_once_handler_fires_only_once() {
        let (lua, _handlers) = create_test_env();

        lua.load(
            r#"
            _test_count = 0
            niri.events:once("test:event", function()
                _test_count = _test_count + 1
            end)
            niri.events:emit("test:event", {})
            niri.events:emit("test:event", {})
            niri.events:emit("test:event", {})
        "#,
        )
        .exec()
        .unwrap();

        let count: i64 = lua.globals().get("_test_count").unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_multiple_handlers_same_event() {
        let (lua, _handlers) = create_test_env();

        lua.load(
            r#"
            _test_sum = 0
            niri.events:on("test:event", function() _test_sum = _test_sum + 1 end)
            niri.events:on("test:event", function() _test_sum = _test_sum + 10 end)
            niri.events:on("test:event", function() _test_sum = _test_sum + 100 end)
            niri.events:emit("test:event", {})
        "#,
        )
        .exec()
        .unwrap();

        let sum: i64 = lua.globals().get("_test_sum").unwrap();
        assert_eq!(sum, 111);
    }

    #[test]
    fn test_emit_with_primitive_value() {
        let (lua, _handlers) = create_test_env();

        // When emitting a primitive, it should be wrapped in a table
        lua.load(
            r#"
            _test_value = nil
            niri.events:on("test:event", function(data)
                _test_value = data.value
            end)
            niri.events:emit("test:event", 42)
        "#,
        )
        .exec()
        .unwrap();

        let value: i64 = lua.globals().get("_test_value").unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_emit_with_nil() {
        let (lua, _handlers) = create_test_env();

        // Emitting nil should pass an empty table
        lua.load(
            r#"
            _test_called = false
            niri.events:on("test:event", function(data)
                _test_called = true
            end)
            niri.events:emit("test:event", nil)
        "#,
        )
        .exec()
        .unwrap();

        let called: bool = lua.globals().get("_test_called").unwrap();
        assert!(called);
    }
}
