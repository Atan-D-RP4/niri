//! Event system registration and dispatch for Niri Lua runtime.
//!
//! This module registers the Lua event API (`niri.on`, `niri.once`, `niri.off`)
//! and provides the public interface for emitting events from Niri core.

use std::sync::Arc;

use log::debug;
use mlua::prelude::*;

use crate::event_handlers::{EventHandlers, EventHandlerId};

/// Thread-safe wrapper around EventHandlers for Lua integration
pub type SharedEventHandlers = Arc<parking_lot::Mutex<EventHandlers>>;

/// Register the event API to Lua runtime
///
/// This creates the `niri.on()`, `niri.once()`, and `niri.off()` functions
/// in the Lua runtime, allowing users to register event handlers.
///
/// # Arguments
/// * `lua` - The Lua runtime
/// * `handlers` - Shared event handlers storage
///
/// # Returns
/// LuaResult indicating success or Lua error
pub fn register_event_api_to_lua(lua: &Lua, handlers: SharedEventHandlers) -> LuaResult<()> {
    let niri_table: LuaTable = lua.globals().get("niri")?;

    // Register niri.on(event_type, callback) - persistent handler
    let handlers_on = handlers.clone();
    niri_table.set(
        "on",
        lua.create_function(move |_, (event_type, callback): (String, LuaFunction)| {
            let mut h = handlers_on.lock();
            let handler_id = h.register_handler(&event_type, callback, false);
            Ok(handler_id)
        })?,
    )?;

    // Register niri.once(event_type, callback) - one-time handler
    let handlers_once = handlers.clone();
    niri_table.set(
        "once",
        lua.create_function(move |_, (event_type, callback): (String, LuaFunction)| {
            let mut h = handlers_once.lock();
            let handler_id = h.register_handler(&event_type, callback, true);
            Ok(handler_id)
        })?,
    )?;

    // Register niri.off(event_type, handler_id) - remove handler
    let handlers_off = handlers.clone();
    niri_table.set(
        "off",
        lua.create_function(move |_, (event_type, handler_id): (String, EventHandlerId)| {
            let mut h = handlers_off.lock();
            h.unregister_handler(&event_type, handler_id);
            Ok(())
        })?,
    )?;

    debug!("Registered event API to Lua");
    Ok(())
}

/// Public interface for emitting events from Niri core
pub struct EventSystem {
    handlers: SharedEventHandlers,
}

impl EventSystem {
    /// Create a new event system with shared handlers
    pub fn new(handlers: SharedEventHandlers) -> Self {
        Self { handlers }
    }

    /// Emit an event to all registered handlers
    ///
    /// # Arguments
    /// * `event_type` - The event name (e.g., "window:open")
    /// * `event_data` - Lua value containing event data
    ///
    /// # Returns
    /// LuaResult indicating success or Lua error
    pub fn emit(&self, event_type: &str, event_data: LuaValue) -> LuaResult<()> {
        let mut h = self.handlers.lock();
        h.emit_event(event_type, event_data)
    }

    /// Get statistics about registered handlers
    pub fn stats(&self) -> EventSystemStats {
        let h = self.handlers.lock();
        EventSystemStats {
            total_handlers: h.total_handlers(),
            event_types: h.event_types().len(),
        }
    }
}

/// Statistics about the event system
pub struct EventSystemStats {
    pub total_handlers: usize,
    pub event_types: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_system() -> (Lua, EventSystem) {
        let lua = Lua::new();

        // Create niri namespace
        let niri = lua.create_table().unwrap();
        lua.globals().set("niri", niri).unwrap();

        let handlers = Arc::new(parking_lot::Mutex::new(EventHandlers::new()));
        register_event_api_to_lua(&lua, handlers.clone()).unwrap();

        let event_system = EventSystem::new(handlers);
        (lua, event_system)
    }

    #[test]
    fn test_register_event_api() {
        let (lua, _) = create_test_system();

        let niri: LuaTable = lua.globals().get("niri").unwrap();
        let _on: LuaFunction = niri.get("on").unwrap();
        let _once: LuaFunction = niri.get("once").unwrap();
        let _off: LuaFunction = niri.get("off").unwrap();
    }

    #[test]
    fn test_lua_on_handler() {
        let (lua, event_system) = create_test_system();

        let called = lua.create_table().unwrap();
        called.set("value", false).unwrap();

        let called_clone = called.clone();
        let niri: LuaTable = lua.globals().get("niri").unwrap();
        let on_fn: LuaFunction = niri.get("on").unwrap();

        let callback = lua
            .create_function(move |_, ()| {
                called_clone.set("value", true).unwrap();
                Ok(())
            })
            .unwrap();

        let _: EventHandlerId = on_fn.call(("test_event", callback)).unwrap();

        let data = lua.create_table().unwrap();
        event_system.emit("test_event", LuaValue::Table(data)).ok();

        let value: bool = called.get("value").unwrap();
        assert!(value);
    }

    #[test]
    fn test_lua_once_handler() {
        let (lua, event_system) = create_test_system();

        let call_count = lua.create_table().unwrap();
        call_count.set("value", 0).unwrap();

        let count_clone = call_count.clone();
        let niri: LuaTable = lua.globals().get("niri").unwrap();
        let once_fn: LuaFunction = niri.get("once").unwrap();

        let callback = lua
            .create_function(move |_, ()| {
                let val: i32 = count_clone.get("value").unwrap();
                count_clone.set("value", val + 1).unwrap();
                Ok(())
            })
            .unwrap();

        let _: EventHandlerId = once_fn.call(("once_event", callback)).unwrap();

        // Emit twice
        let data = lua.create_table().unwrap();
        event_system.emit("once_event", LuaValue::Table(data.clone())).ok();
        event_system.emit("once_event", LuaValue::Table(data)).ok();

        // Should only be called once
        let count: i32 = call_count.get("value").unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_lua_off_handler() {
        let (lua, event_system) = create_test_system();

        let called = lua.create_table().unwrap();
        called.set("value", false).unwrap();

        let called_clone = called.clone();
        let niri: LuaTable = lua.globals().get("niri").unwrap();
        let on_fn: LuaFunction = niri.get("on").unwrap();
        let off_fn: LuaFunction = niri.get("off").unwrap();

        let callback = lua
            .create_function(move |_, ()| {
                called_clone.set("value", true).unwrap();
                Ok(())
            })
            .unwrap();

        let handler_id: EventHandlerId = on_fn.call(("test_event", callback)).unwrap();

        // Unregister the handler
        let _: () = off_fn.call(("test_event", handler_id)).unwrap();

        // Emit event - should not call the handler
        let data = lua.create_table().unwrap();
        event_system.emit("test_event", LuaValue::Table(data)).ok();

        let value: bool = called.get("value").unwrap();
        assert!(!value);
    }

    #[test]
    fn test_event_system_stats() {
        let (lua, event_system) = create_test_system();

        let niri: LuaTable = lua.globals().get("niri").unwrap();
        let on_fn: LuaFunction = niri.get("on").unwrap();

        let callback = lua.create_function(|_, ()| Ok(())).unwrap();

        let _: EventHandlerId = on_fn.call(("event1", callback.clone())).unwrap();
        let _: EventHandlerId = on_fn.call(("event1", callback.clone())).unwrap();
        let _: EventHandlerId = on_fn.call(("event2", callback)).unwrap();

        let stats = event_system.stats();
        assert_eq!(stats.total_handlers, 3);
        assert_eq!(stats.event_types, 2);
    }
}
