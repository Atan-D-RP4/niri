//! Event system for Niri Lua runtime.
//!
//! This module provides the public interface for emitting events from Niri core
//! to Lua event handlers registered via the `niri.events` proxy API.

use std::cell::RefCell;
use std::rc::Rc;

use mlua::prelude::*;

use crate::event_handlers::EventHandlers;

/// Shared wrapper around EventHandlers for Lua integration
pub type SharedEventHandlers = Rc<RefCell<EventHandlers>>;

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
    /// Callbacks are executed with timeout protection to prevent runaway
    /// scripts from freezing the compositor.
    ///
    /// # Arguments
    /// * `lua` - The Lua context (for timeout protection)
    /// * `event_type` - The event name (e.g., "window:open")
    /// * `event_data` - Lua value containing event data
    ///
    /// # Returns
    /// LuaResult indicating success or Lua error
    pub fn emit(&self, lua: &Lua, event_type: &str, event_data: LuaValue) -> LuaResult<()> {
        let mut h = self.handlers.borrow_mut();
        h.emit_event(lua, event_type, event_data)
    }

    /// Get statistics about registered handlers
    pub fn stats(&self) -> EventSystemStats {
        let h = self.handlers.borrow();
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
        let handlers = Rc::new(RefCell::new(EventHandlers::new()));
        let event_system = EventSystem::new(handlers);
        (lua, event_system)
    }

    #[test]
    fn test_event_system_new() {
        let (_lua, event_system) = create_test_system();
        let stats = event_system.stats();
        assert_eq!(stats.total_handlers, 0);
        assert_eq!(stats.event_types, 0);
    }

    #[test]
    fn test_event_system_emit_no_handlers() {
        let (lua, event_system) = create_test_system();
        let data = lua.create_table().unwrap();
        // Should not error even with no handlers
        let result = event_system.emit(&lua, "test_event", LuaValue::Table(data));
        assert!(result.is_ok());
    }

    #[test]
    fn test_event_system_stats() {
        let (_lua, event_system) = create_test_system();
        let stats = event_system.stats();
        assert_eq!(stats.total_handlers, 0);
        assert_eq!(stats.event_types, 0);
    }
}
