use niri::lua_extensions::{LuaConfig, apply_lua_config};
use niri_config::Config;

/* test_keybinds.lua content:
-- Simple test keybindings
binds = {
    {
        key = "Super+A",
        action = "spawn",
        args = { "alacritty" }
    },
    {
        key = "Super+Q",
        action = "close-window",
        args = {}
    }
}

niri.log("Test binds table loaded with " .. #binds .. " keybindings")
*/

fn main() {
    // Test loading from test file
    match LuaConfig::from_file("/tmp/test_keybinds.lua") {
        Ok(lua_config) => {
            println!("✓ Lua config loaded successfully");
            let runtime = lua_config.runtime();

            // Try to get keybindings
            match runtime.get_keybindings() {
                Ok(bindings) => {
                    println!("✓ Found {} keybindings", bindings.len());
                    for (i, (key, action, args)) in bindings.iter().enumerate() {
                        println!("  Binding {}: key='{}', action='{}', args={:?}", i+1, key, action, args);
                    }
                }
                Err(e) => {
                    println!("✗ Error getting keybindings: {}", e);
                }
            }

            // Try applying to config
            let mut config = Config::default();
            match apply_lua_config(runtime, &mut config) {
                Ok(_) => {
                    println!("✓ Applied Lua config to Config");
                    println!("  Config now has {} keybindings", config.binds.0.len());
                }
                Err(e) => {
                    println!("✗ Error applying config: {}", e);
                }
            }
        }
        Err(e) => {
            println!("✗ Failed to load Lua config: {}", e);
        }
    }
}
