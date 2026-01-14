//! Callback registry for managing Lua function callbacks with numeric IDs.
//!
//! This module provides a registry for storing Lua functions with unique
//! numeric IDs. It is designed to support the Process API's callback system.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use mlua::prelude::*;

/// Registry for managing Lua function callbacks with numeric IDs.
pub struct CallbackRegistry {
    next_id: u64,
    callbacks: HashMap<u64, LuaRegistryKey>,
}

impl CallbackRegistry {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            callbacks: HashMap::new(),
        }
    }

    /// Register a Lua function and return its unique ID.
    pub fn register(&mut self, lua: &Lua, func: LuaFunction) -> LuaResult<u64> {
        let id = self.next_id;
        self.next_id += 1;
        let key = lua.create_registry_value(func)?;
        self.callbacks.insert(id, key);
        Ok(id)
    }

    /// Unregister a callback by ID and return its registry key.
    pub fn unregister(&mut self, id: u64) -> Option<LuaRegistryKey> {
        self.callbacks.remove(&id)
    }

    /// Get a Lua function by its ID.
    pub fn get(&self, lua: &Lua, id: u64) -> LuaResult<Option<LuaFunction>> {
        match self.callbacks.get(&id) {
            Some(key) => {
                let func: LuaFunction = lua.registry_value(key)?;
                Ok(Some(func))
            }
            None => Ok(None),
        }
    }

    /// Clear all callbacks and return their registry keys for cleanup.
    pub fn clear(&mut self) -> Vec<LuaRegistryKey> {
        self.callbacks.drain().map(|(_, key)| key).collect()
    }
}

impl Default for CallbackRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedCallbackRegistry = Rc<RefCell<CallbackRegistry>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_callback_registry_new() {
        let registry = CallbackRegistry::new();
        assert_eq!(registry.next_id, 1);
        assert!(registry.callbacks.is_empty());
    }

    #[test]
    fn test_callback_registry_register() {
        let lua = Lua::new();
        let mut registry = CallbackRegistry::new();

        let func: LuaFunction = lua.create_function(|_, ()| Ok(42)).unwrap();
        let id = registry.register(&lua, func).unwrap();
        assert_eq!(id, 1);
        assert_eq!(registry.callbacks.len(), 1);
        assert!(registry.callbacks.contains_key(&1));
    }

    #[test]
    fn test_callback_registry_get() {
        let lua = Lua::new();
        let mut registry = CallbackRegistry::new();

        let func: LuaFunction = lua
            .create_function(|_, ()| Ok("hello".to_string()))
            .unwrap();
        let id = registry.register(&lua, func).unwrap();

        let retrieved = registry.get(&lua, id).unwrap().unwrap();
        let result: String = retrieved.call(()).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_callback_registry_get_nonexistent() {
        let lua = Lua::new();
        let registry = CallbackRegistry::new();

        let result = registry.get(&lua, 999).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_callback_registry_unregister() {
        let lua = Lua::new();
        let mut registry = CallbackRegistry::new();

        let func: LuaFunction = lua.create_function(|_, ()| Ok(true)).unwrap();
        let id = registry.register(&lua, func).unwrap();
        assert!(registry.callbacks.contains_key(&id));

        let key = registry.unregister(id);
        assert!(key.is_some());
        assert!(!registry.callbacks.contains_key(&id));

        let retrieved = registry.get(&lua, id).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_callback_registry_unregister_nonexistent() {
        let mut registry = CallbackRegistry::new();
        let key = registry.unregister(999);
        assert!(key.is_none());
    }

    #[test]
    fn test_callback_registry_clear() {
        let lua = Lua::new();
        let mut registry = CallbackRegistry::new();

        for i in 0..3 {
            let func: LuaFunction = lua.create_function(move |_, ()| Ok(i)).unwrap();
            registry.register(&lua, func).unwrap();
        }
        assert_eq!(registry.callbacks.len(), 3);

        let keys = registry.clear();
        assert_eq!(keys.len(), 3);
        assert!(registry.callbacks.is_empty());
    }

    #[test]
    fn test_callback_registry_unique_ids() {
        let lua = Lua::new();
        let mut registry = CallbackRegistry::new();

        let mut ids = Vec::new();
        for _ in 0..5 {
            let func: LuaFunction = lua.create_function(|_, ()| Ok(())).unwrap();
            let id = registry.register(&lua, func).unwrap();
            ids.push(id);
        }

        let mut sorted_ids = ids.clone();
        sorted_ids.sort();
        sorted_ids.dedup();
        assert_eq!(sorted_ids.len(), ids.len());
        assert_eq!(ids, vec![1, 2, 3, 4, 5]);
    }
}
