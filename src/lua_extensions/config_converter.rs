//! Converts Lua configuration values to Niri Config structures.
//!
//! This module provides utilities for extracting configuration values from a Lua runtime
//! and applying them to Niri's Config struct.

use super::LuaRuntime;
use niri_config::Config;
use anyhow::Result;
use log::{debug, info, warn};

/// Attempts to extract and apply Lua configuration values to the given Config.
///
/// This function looks for specific configuration values in the Lua runtime's global scope
/// and applies them to the provided Config struct. Unknown or invalid values are logged
/// and skipped rather than causing errors.
///
/// # Arguments
///
/// * `runtime` - The Lua runtime to extract configuration from
/// * `config` - The Config struct to apply values to (modified in place)
///
/// # Example
///
/// ```ignore
/// let runtime = LuaRuntime::new()?;
/// runtime.load_file("niri.lua")?;
/// let mut config = Config::default();
/// apply_lua_config(&runtime, &mut config)?;
/// ```
pub fn apply_lua_config(runtime: &LuaRuntime, config: &mut Config) -> Result<()> {
    debug!("=== Applying Lua configuration to Config ===");

    // Try to extract simple boolean settings
    debug!("Checking for prefer_no_csd in Lua globals");
    if runtime.has_global("prefer_no_csd") {
        info!("✓ Found prefer_no_csd in Lua globals");
        match runtime.get_global_bool_opt("prefer_no_csd") {
            Ok(Some(prefer_no_csd)) => {
                info!("✓ Applying prefer_no_csd: {} → {} (changed: {})", 
                    config.prefer_no_csd, prefer_no_csd, config.prefer_no_csd != prefer_no_csd);
                config.prefer_no_csd = prefer_no_csd;
            }
            Ok(None) => {
                warn!("⚠ prefer_no_csd was nil in Lua");
            }
            Err(e) => {
                warn!("✗ Error getting prefer_no_csd: {}", e);
            }
        }
    } else {
        debug!("ℹ prefer_no_csd not found in Lua globals");
    }

    // Additional configuration options can be added here as they're implemented
    // Examples:
    // - Screen lock settings
    // - Animation settings
    // - Cursor settings
    // - etc.

    debug!("=== Lua configuration application completed ===");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_lua_config_empty() {
        let runtime = LuaRuntime::new().unwrap();
        let mut config = Config::default();
        let result = apply_lua_config(&runtime, &mut config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_apply_lua_config_with_values() {
        let runtime = LuaRuntime::new().unwrap();
        runtime
            .load_string("prefer_no_csd = false")
            .expect("Failed to load Lua code");

        let mut config = Config::default();
        let original_value = config.prefer_no_csd;
        
        apply_lua_config(&runtime, &mut config).expect("Failed to apply config");
        
        assert_eq!(config.prefer_no_csd, false);
        assert_ne!(config.prefer_no_csd, original_value);
    }
}
