//! Event emitter system for Niri Lua runtime.
//!
//! This module provides an event-driven architecture for Niri plugins,
//! allowing registration and dispatching of events.
//!
//! # Events
//!
//! Events are dispatched through the `niri.events` namespace:
//!
//! ```lua
//! -- Register event handler
//! niri.events.on("window:open", function(event)
//!   print("Window opened: " .. event.window.title)
//! end)
//!
//! -- Register one-time handler
//! niri.events.once("workspace:enter", function(event)
//!   print("Entered workspace (fires only once)")
//! end)
//!
//! -- Remove handler
//! local handler_id = niri.events.on("action", function(event)
//!   -- handler code
//! end)
//! niri.events.off("action", handler_id)
//! ```

use std::collections::HashMap;

use log::{debug, error, warn};
use mlua::prelude::*;

/// Event handler ID
pub type HandlerId = u64;

/// Event handler function type
pub type EventHandler = LuaFunction;

/// Event entry in the registry
#[derive(Clone)]
struct EventEntry {
    handler_id: HandlerId,
    handler: EventHandler,
    once: bool,
}

/// Event emitter for Niri
pub struct EventEmitter {
    handlers: HashMap<String, Vec<EventEntry>>,
    next_handler_id: HandlerId,
}

impl EventEmitter {
    /// Create a new event emitter
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            next_handler_id: 1,
        }
    }

    /// Register an event handler
    pub fn on(&mut self, event_name: &str, handler: EventHandler) -> HandlerId {
        let handler_id = self.next_handler_id;
        self.next_handler_id += 1;

        let entry = EventEntry {
            handler_id,
            handler,
            once: false,
        };

        self.handlers
            .entry(event_name.to_string())
            .or_insert_with(Vec::new)
            .push(entry);

        debug!(
            "Registered event handler for '{}' with ID {}",
            event_name, handler_id
        );
        handler_id
    }

    /// Register a one-time event handler
    pub fn once(&mut self, event_name: &str, handler: EventHandler) -> HandlerId {
        let handler_id = self.next_handler_id;
        self.next_handler_id += 1;

        let entry = EventEntry {
            handler_id,
            handler,
            once: true,
        };

        self.handlers
            .entry(event_name.to_string())
            .or_insert_with(Vec::new)
            .push(entry);

        debug!(
            "Registered one-time event handler for '{}' with ID {}",
            event_name, handler_id
        );
        handler_id
    }

    /// Unregister an event handler by ID
    pub fn off(&mut self, event_name: &str, handler_id: HandlerId) -> bool {
        if let Some(entries) = self.handlers.get_mut(event_name) {
            if let Some(pos) = entries.iter().position(|e| e.handler_id == handler_id) {
                entries.remove(pos);
                debug!(
                    "Unregistered event handler for '{}' with ID {}",
                    event_name, handler_id
                );
                return true;
            }
        }

        warn!(
            "Handler with ID {} not found for event '{}'",
            handler_id, event_name
        );
        false
    }

    /// Emit an event to all registered handlers
    pub fn emit(&mut self, event_name: &str, event_data: LuaValue) -> LuaResult<()> {
        if let Some(entries) = self.handlers.get(event_name).cloned() {
            for entry in entries {
                match entry.handler.call::<()>(event_data.clone()) {
                    Ok(_) => {
                        debug!("Handler executed for event '{}'", event_name);
                    }
                    Err(e) => {
                        error!("Error in event handler for '{}': {}", event_name, e);
                        // Continue with other handlers even if one fails
                    }
                }

                // Remove one-time handlers after execution
                if entry.once {
                    self.off(event_name, entry.handler_id);
                }
            }
        }

        Ok(())
    }

    /// Clear all handlers for an event
    pub fn clear_event(&mut self, event_name: &str) {
        if self.handlers.remove(event_name).is_some() {
            debug!("Cleared all handlers for event '{}'", event_name);
        }
    }

    /// Clear all handlers for all events
    pub fn clear_all(&mut self) {
        self.handlers.clear();
        self.next_handler_id = 1;
        debug!("Cleared all event handlers");
    }

    /// Get number of handlers for an event
    pub fn handler_count(&self, event_name: &str) -> usize {
        self.handlers
            .get(event_name)
            .map(|entries| entries.len())
            .unwrap_or(0)
    }

    /// Get total number of handlers
    pub fn total_handlers(&self) -> usize {
        self.handlers.values().map(|entries| entries.len()).sum()
    }

    /// Register event emitter API to Lua
    pub fn register_to_lua(lua: &Lua) -> LuaResult<()> {
        let niri_table: LuaTable = lua.globals().get("niri")?;

        // Create events namespace
        let events = lua.create_table()?;

        // Use thread-local storage or a RefCell for mutable access
        // For now, we'll use a global table to store handlers
        let event_handlers = lua.create_table()?;
        lua.globals()
            .set("__niri_event_handlers", event_handlers.clone())?;

        let next_handler_id = lua.create_table()?;
        next_handler_id.set("value", 1_u64)?;
        lua.globals()
            .set("__niri_next_handler_id", next_handler_id)?;

        // Register on() function
        events.set(
            "on",
            lua.create_function(|lua, (event_name, handler): (String, LuaFunction)| {
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
            })?,
        )?;

        // Register once() function
        events.set(
            "once",
            lua.create_function(|lua, (event_name, handler): (String, LuaFunction)| {
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
            })?,
        )?;

        // Register off() function
        events.set(
            "off",
            lua.create_function(|lua, (event_name, handler_id): (String, u64)| {
                let handlers: LuaTable = lua.globals().get("__niri_event_handlers")?;
                if let Ok(LuaValue::Table(event_handlers)) = handlers.get(event_name.as_str()) {
                    event_handlers.set(handler_id, LuaValue::Nil)?;
                }
                Ok(())
            })?,
        )?;

        // Register emit() function
        events.set(
            "emit",
            lua.create_function(|lua, (event_name, data): (String, LuaValue)| {
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
            })?,
        )?;

        niri_table.set("events", events)?;

        debug!("Registered event emitter API to Lua");
        Ok(())
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_emitter_creation() {
        let emitter = EventEmitter::new();
        assert_eq!(emitter.total_handlers(), 0);
    }

    #[test]
    fn handler_registration() {
        let lua = Lua::new();
        let mut emitter = EventEmitter::new();

        let handler = lua.create_function(|_, ()| Ok(())).unwrap();
        let id1 = emitter.on("test_event", handler.clone());
        let id2 = emitter.on("test_event", handler.clone());

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(emitter.handler_count("test_event"), 2);
    }

    #[test]
    fn one_time_handler() {
        let lua = Lua::new();
        let mut emitter = EventEmitter::new();

        let handler = lua.create_function(|_, ()| Ok(())).unwrap();
        emitter.once("once_event", handler);

        assert_eq!(emitter.handler_count("once_event"), 1);
    }

    #[test]
    fn handler_removal() {
        let lua = Lua::new();
        let mut emitter = EventEmitter::new();

        let handler = lua.create_function(|_, ()| Ok(())).unwrap();
        let id = emitter.on("test_event", handler);

        assert_eq!(emitter.handler_count("test_event"), 1);
        assert!(emitter.off("test_event", id));
        assert_eq!(emitter.handler_count("test_event"), 0);
    }

    #[test]
    fn clear_event() {
        let lua = Lua::new();
        let mut emitter = EventEmitter::new();

        let handler = lua.create_function(|_, ()| Ok(())).unwrap();
        emitter.on("test_event", handler.clone());
        emitter.on("test_event", handler);

        assert_eq!(emitter.handler_count("test_event"), 2);
        emitter.clear_event("test_event");
        assert_eq!(emitter.handler_count("test_event"), 0);
    }

    #[test]
    fn clear_all_handlers() {
        let lua = Lua::new();
        let mut emitter = EventEmitter::new();

        let handler = lua.create_function(|_, ()| Ok(())).unwrap();
        emitter.on("event1", handler.clone());
        emitter.on("event2", handler);

        assert_eq!(emitter.total_handlers(), 2);
        emitter.clear_all();
        assert_eq!(emitter.total_handlers(), 0);
    }

    #[test]
    fn register_to_lua() {
        let lua = Lua::new();

        // Create niri namespace
        let niri = lua.create_table().unwrap();
        lua.globals().set("niri", niri).unwrap();

        // Register event emitter
        EventEmitter::register_to_lua(&lua).unwrap();

        // Check that events namespace was created
        let niri: LuaTable = lua.globals().get("niri").unwrap();
        let events: LuaTable = niri.get("events").unwrap();

        // Check that required functions exist
        let _on: LuaFunction = events.get("on").unwrap();
        let _off: LuaFunction = events.get("off").unwrap();
        let _once: LuaFunction = events.get("once").unwrap();
        let _emit: LuaFunction = events.get("emit").unwrap();
    }
}
