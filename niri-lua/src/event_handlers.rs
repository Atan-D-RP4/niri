//! Event handler registration and management system for Niri Lua runtime.
//!
//! This module manages the lifecycle of event handlers - registration, storage,
//! execution, and cleanup. It provides error isolation so that handler failures
//! don't crash Niri.

use std::collections::HashMap;

use log::{debug, error, warn};
use mlua::prelude::*;

/// ID for tracking individual event handlers
pub type EventHandlerId = u64;

/// Represents a single registered event handler
#[derive(Clone)]
pub struct LuaEventHandler {
    /// Unique identifier for this handler
    pub id: EventHandlerId,
    /// The Lua function to call
    pub callback: LuaFunction,
    /// Whether this handler should only fire once
    pub once: bool,
}

/// Manages all registered event handlers
pub struct EventHandlers {
    /// Map of event names to their registered handlers
    handlers: HashMap<String, Vec<LuaEventHandler>>,
    /// Next handler ID to assign
    next_handler_id: EventHandlerId,
}

impl EventHandlers {
    /// Create a new empty event handler registry
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            next_handler_id: 1,
        }
    }

    /// Register a new event handler
    ///
    /// # Arguments
    /// * `event_type` - Name of the event (e.g., "window:open")
    /// * `callback` - Lua function to call when event fires
    /// * `once` - If true, handler is removed after first call
    ///
    /// # Returns
    /// The handler ID for later removal
    pub fn register_handler(
        &mut self,
        event_type: &str,
        callback: LuaFunction,
        once: bool,
    ) -> EventHandlerId {
        let handler_id = self.next_handler_id;
        self.next_handler_id += 1;

        let handler = LuaEventHandler {
            id: handler_id,
            callback,
            once,
        };

        self.handlers
            .entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(handler);

        debug!(
            "Registered event handler '{}' with ID {} (once={})",
            event_type, handler_id, once
        );

        handler_id
    }

    /// Unregister a specific event handler
    ///
    /// # Arguments
    /// * `event_type` - The event name
    /// * `handler_id` - The handler ID returned from register_handler
    ///
    /// # Returns
    /// True if handler was found and removed, false if not found
    pub fn unregister_handler(&mut self, event_type: &str, handler_id: EventHandlerId) -> bool {
        if let Some(handlers) = self.handlers.get_mut(event_type) {
            if let Some(pos) = handlers.iter().position(|h| h.id == handler_id) {
                handlers.remove(pos);
                debug!("Unregistered event handler '{}' with ID {}", event_type, handler_id);
                return true;
            }
        }

        warn!(
            "Attempted to unregister non-existent handler '{}' with ID {}",
            event_type, handler_id
        );
        false
    }

    /// Emit an event to all registered handlers
    ///
    /// This function calls all handlers for the specified event, with error
    /// isolation - if one handler errors, others still execute. One-time
    /// handlers are automatically removed after execution.
    ///
    /// # Arguments
    /// * `event_type` - The event name
    /// * `event_data` - Lua value containing event data
    ///
    /// # Returns
    /// LuaResult indicating if event emission succeeded at the Lua level
    pub fn emit_event(&mut self, event_type: &str, event_data: LuaValue) -> LuaResult<()> {
        if let Some(handlers_snapshot) = self.handlers.get(event_type).cloned() {
            // Clone handler list to avoid borrow issues when modifying during iteration
            let mut handlers_to_remove = Vec::new();

            for handler in handlers_snapshot {
                // Call handler with error isolation
                match handler.callback.call::<()>(event_data.clone()) {
                    Ok(_) => {
                        debug!(
                            "Handler {} executed successfully for event '{}'",
                            handler.id, event_type
                        );

                        // Mark one-time handlers for removal
                        if handler.once {
                            handlers_to_remove.push(handler.id);
                        }
                    }
                    Err(e) => {
                        // Log error but don't propagate - keep Niri running
                        error!(
                            "Error in event handler {} for '{}': {}",
                            handler.id, event_type, e
                        );
                    }
                }
            }

            // Remove one-time handlers that executed
            for handler_id in handlers_to_remove {
                self.unregister_handler(event_type, handler_id);
            }
        }

        Ok(())
    }

    /// Get the number of handlers for a specific event
    pub fn handler_count(&self, event_type: &str) -> usize {
        self.handlers
            .get(event_type)
            .map(|h| h.len())
            .unwrap_or(0)
    }

    /// Get the total number of registered handlers
    pub fn total_handlers(&self) -> usize {
        self.handlers.values().map(|h| h.len()).sum()
    }

    /// Remove all handlers for a specific event
    pub fn clear_event(&mut self, event_type: &str) {
        if self.handlers.remove(event_type).is_some() {
            debug!("Cleared all handlers for event '{}'", event_type);
        }
    }

    /// Remove all handlers for all events
    pub fn clear_all(&mut self) {
        self.handlers.clear();
        self.next_handler_id = 1;
        debug!("Cleared all event handlers");
    }

    /// Get a snapshot of all registered event types
    pub fn event_types(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }
}

impl Default for EventHandlers {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_empty() {
        let handlers = EventHandlers::new();
        assert_eq!(handlers.total_handlers(), 0);
        assert_eq!(handlers.handler_count("test"), 0);
    }

    #[test]
    fn test_register_handler() {
        let lua = Lua::new();
        let mut handlers = EventHandlers::new();

        let callback = lua.create_function(|_, ()| Ok(())).unwrap();
        let id1 = handlers.register_handler("test_event", callback.clone(), false);
        let id2 = handlers.register_handler("test_event", callback, false);

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(handlers.handler_count("test_event"), 2);
    }

    #[test]
    fn test_unregister_handler() {
        let lua = Lua::new();
        let mut handlers = EventHandlers::new();

        let callback = lua.create_function(|_, ()| Ok(())).unwrap();
        let id = handlers.register_handler("test_event", callback, false);

        assert_eq!(handlers.handler_count("test_event"), 1);
        assert!(handlers.unregister_handler("test_event", id));
        assert_eq!(handlers.handler_count("test_event"), 0);
    }

    #[test]
    fn test_unregister_nonexistent() {
        let mut handlers = EventHandlers::new();
        assert!(!handlers.unregister_handler("test_event", 999));
    }

    #[test]
    fn emit_calls_handlers() {
        let lua = Lua::new();
        let mut handlers = EventHandlers::new();

        // Create a Lua table to track if handler was called
        let called = lua.create_table().unwrap();
        called.set("value", false).unwrap();

        let called_clone = called.clone();
        let callback = lua
            .create_function(move |_, ()| {
                called_clone.set("value", true).unwrap();
                Ok(())
            })
            .unwrap();

        handlers.register_handler("test", callback, false);

        let data = lua.create_table().unwrap();
        handlers.emit_event("test", LuaValue::Table(data)).ok();

        let value: bool = called.get("value").unwrap();
        assert!(value);
    }

    #[test]
    fn test_once_handler_removal() {
        let lua = Lua::new();
        let mut handlers = EventHandlers::new();

        let callback = lua.create_function(|_, ()| Ok(())).unwrap();
        handlers.register_handler("once_event", callback, true);

        assert_eq!(handlers.handler_count("once_event"), 1);

        let data = lua.create_table().unwrap();
        handlers.emit_event("once_event", LuaValue::Table(data)).ok();

        // Handler should be removed after execution
        assert_eq!(handlers.handler_count("once_event"), 0);
    }

    #[test]
    fn test_clear_event() {
        let lua = Lua::new();
        let mut handlers = EventHandlers::new();

        let callback = lua.create_function(|_, ()| Ok(())).unwrap();
        handlers.register_handler("event1", callback.clone(), false);
        handlers.register_handler("event1", callback.clone(), false);
        handlers.register_handler("event2", callback, false);

        assert_eq!(handlers.handler_count("event1"), 2);
        assert_eq!(handlers.handler_count("event2"), 1);

        handlers.clear_event("event1");

        assert_eq!(handlers.handler_count("event1"), 0);
        assert_eq!(handlers.handler_count("event2"), 1);
    }

    #[test]
    fn test_clear_all() {
        let lua = Lua::new();
        let mut handlers = EventHandlers::new();

        let callback = lua.create_function(|_, ()| Ok(())).unwrap();
        handlers.register_handler("event1", callback.clone(), false);
        handlers.register_handler("event2", callback, false);

        assert_eq!(handlers.total_handlers(), 2);
        handlers.clear_all();
        assert_eq!(handlers.total_handlers(), 0);
    }

    #[test]
    fn test_event_types() {
        let lua = Lua::new();
        let mut handlers = EventHandlers::new();

        let callback = lua.create_function(|_, ()| Ok(())).unwrap();
        handlers.register_handler("event1", callback.clone(), false);
        handlers.register_handler("event2", callback, false);

        let mut types = handlers.event_types();
        types.sort();

        assert_eq!(types, vec!["event1".to_string(), "event2".to_string()]);
    }
}
