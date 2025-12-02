//! Event system for Niri Lua runtime.
//!
//! This module provides the public interface for emitting events from Niri core
//! to Lua event handlers registered via the `niri.events` proxy API.

use std::sync::Arc;

use mlua::prelude::*;

use crate::event_handlers::EventHandlers;

/// Thread-safe wrapper around EventHandlers for Lua integration
pub type SharedEventHandlers = Arc<parking_lot::Mutex<EventHandlers>>;

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
        let handlers = Arc::new(parking_lot::Mutex::new(EventHandlers::new()));
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
        let result = event_system.emit("test_event", LuaValue::Table(data));
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
