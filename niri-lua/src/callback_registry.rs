//! Callback registry for managing Lua function callbacks with numeric IDs.
//!
//! This module provides a thread-safe registry for storing Lua functions with unique
//! numeric IDs. It is designed to support the Process API's callback system.
//!
//! # Thread Safety
//!
//! Registrations and destruction of LuaRegistryKey must only happen on the main thread
//! where the Lua state is valid. The registry itself is thread-safe for concurrent access.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use mlua::prelude::*;

/// Registry for managing Lua function callbacks with numeric IDs.
pub struct CallbackRegistry {
    /// Next available callback ID.
    next_id: AtomicU64,
    /// Map of callback IDs to registry keys.
    callbacks: Mutex<HashMap<u64, LuaRegistryKey>>,
}

impl CallbackRegistry {
    /// Create a new callback registry.
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            callbacks: Mutex::new(HashMap::new()),
        }
    }

    /// Register a Lua function and return its unique ID.
    ///
    /// The function is stored in the Lua registry and can be retrieved later
    /// using the returned ID. This must be called on the main thread.
    pub fn register(&self, lua: &Lua, func: LuaFunction) -> LuaResult<u64> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let key = lua.create_registry_value(func)?;
        let mut callbacks = self.callbacks.lock().unwrap();
        callbacks.insert(id, key);
        Ok(id)
    }

    /// Unregister a callback by ID and return its registry key.
    ///
    /// This removes the callback from the registry and returns the registry key
    /// so it can be properly cleaned up. This must be called on the main thread.
    pub fn unregister(&self, id: u64) -> Option<LuaRegistryKey> {
        let mut callbacks = self.callbacks.lock().unwrap();
        callbacks.remove(&id)
    }

    /// Get a Lua function by its ID.
    ///
    /// Returns the function if it exists in the registry, or None if not found.
    /// This must be called on the main thread.
    pub fn get(&self, lua: &Lua, id: u64) -> LuaResult<Option<LuaFunction>> {
        let callbacks = self.callbacks.lock().unwrap();
        match callbacks.get(&id) {
            Some(key) => {
                let func: LuaFunction = lua.registry_value(key)?;
                Ok(Some(func))
            }
            None => Ok(None),
        }
    }

    /// Clear all callbacks and return their registry keys for cleanup.
    ///
    /// This removes all callbacks from the registry and returns their registry keys
    /// so they can be properly cleaned up. This must be called on the main thread.
    pub fn clear(&self) -> Vec<LuaRegistryKey> {
        let mut callbacks = self.callbacks.lock().unwrap();
        callbacks.drain().map(|(_, key)| key).collect()
    }
}

impl Default for CallbackRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared callback registry for use across threads.
pub type SharedCallbackRegistry = Arc<CallbackRegistry>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_callback_registry_new() {
        let registry = CallbackRegistry::new();
        assert_eq!(registry.next_id.load(Ordering::SeqCst), 1);
        assert!(registry.callbacks.lock().unwrap().is_empty());
    }

    #[test]
    fn test_callback_registry_register() {
        let lua = Lua::new();
        let registry = CallbackRegistry::new();

        // Create a simple Lua function
        let func: LuaFunction = lua.create_function(|_, ()| Ok(42)).unwrap();

        // Register it
        let id = registry.register(&lua, func).unwrap();
        assert_eq!(id, 1);

        // Check it was stored
        let callbacks = registry.callbacks.lock().unwrap();
        assert_eq!(callbacks.len(), 1);
        assert!(callbacks.contains_key(&1));
    }

    #[test]
    fn test_callback_registry_get() {
        let lua = Lua::new();
        let registry = CallbackRegistry::new();

        // Create and register a function
        let func: LuaFunction = lua
            .create_function(|_, ()| Ok("hello".to_string()))
            .unwrap();
        let id = registry.register(&lua, func).unwrap();

        // Get it back
        let retrieved = registry.get(&lua, id).unwrap().unwrap();
        let result: String = retrieved.call(()).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_callback_registry_get_nonexistent() {
        let lua = Lua::new();
        let registry = CallbackRegistry::new();

        // Try to get a non-existent ID
        let result = registry.get(&lua, 999).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_callback_registry_unregister() {
        let lua = Lua::new();
        let registry = CallbackRegistry::new();

        // Create and register a function
        let func: LuaFunction = lua.create_function(|_, ()| Ok(true)).unwrap();
        let id = registry.register(&lua, func).unwrap();

        // Verify it's registered
        assert!(registry.callbacks.lock().unwrap().contains_key(&id));

        // Unregister it
        let key = registry.unregister(id);
        assert!(key.is_some());

        // Verify it's gone
        assert!(!registry.callbacks.lock().unwrap().contains_key(&id));

        // Try to get it back - should be None
        let retrieved = registry.get(&lua, id).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_callback_registry_unregister_nonexistent() {
        let registry = CallbackRegistry::new();

        // Try to unregister a non-existent ID
        let key = registry.unregister(999);
        assert!(key.is_none());
    }

    #[test]
    fn test_callback_registry_clear() {
        let lua = Lua::new();
        let registry = CallbackRegistry::new();

        // Register multiple functions
        for i in 0..3 {
            let func: LuaFunction = lua.create_function(move |_, ()| Ok(i)).unwrap();
            registry.register(&lua, func).unwrap();
        }

        // Verify they were registered
        assert_eq!(registry.callbacks.lock().unwrap().len(), 3);

        // Clear them
        let keys = registry.clear();
        assert_eq!(keys.len(), 3);

        // Verify registry is empty
        assert!(registry.callbacks.lock().unwrap().is_empty());
    }

    #[test]
    fn test_callback_registry_unique_ids() {
        let lua = Lua::new();
        let registry = CallbackRegistry::new();

        // Register multiple functions
        let mut ids = Vec::new();
        for _ in 0..5 {
            let func: LuaFunction = lua.create_function(|_, ()| Ok(())).unwrap();
            let id = registry.register(&lua, func).unwrap();
            ids.push(id);
        }

        // All IDs should be unique
        let mut sorted_ids = ids.clone();
        sorted_ids.sort();
        sorted_ids.dedup();
        assert_eq!(sorted_ids.len(), ids.len());

        // IDs should be sequential starting from 1
        assert_eq!(ids, vec![1, 2, 3, 4, 5]);
    }
}
