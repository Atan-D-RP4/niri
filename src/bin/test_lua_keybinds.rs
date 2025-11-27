use niri::lua_extensions::{apply_lua_config, LuaConfig};
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
    // Get file path from command line or use default
    let file_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/test_keybinds.lua".to_string());

    println!("Loading Lua config from: {}", file_path);

    // Test loading from test file
    match LuaConfig::from_file(&file_path) {
        Ok(lua_config) => {
            println!("✓ Lua config loaded successfully");
            let runtime = lua_config.runtime();

            // Try to get keybindings
            match runtime.get_keybindings() {
                Ok(bindings) => {
                    println!("✓ Found {} keybindings", bindings.len());
                    for (i, (key, action, args)) in bindings.iter().enumerate() {
                        println!(
                            "  Binding {}: key='{}', action='{}', args={:?}",
                            i + 1,
                            key,
                            action,
                            args
                        );
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
                    println!(
                        "  Config now has {} startup commands",
                        config.spawn_at_startup.len()
                    );
                    for (i, cmd) in config.spawn_at_startup.iter().enumerate() {
                        println!("    Startup {}: {:?}", i + 1, cmd.command);
                    }

                    // Show recent_windows configuration
                    println!("\n  Recent Windows Configuration:");
                    println!("    on: {}", config.recent_windows.on);
                    println!("    open_delay_ms: {}", config.recent_windows.open_delay_ms);
                    println!(
                        "    highlight.padding: {}",
                        config.recent_windows.highlight.padding
                    );
                    println!(
                        "    highlight.corner_radius: {}",
                        config.recent_windows.highlight.corner_radius
                    );
                    println!(
                        "    previews.max_height: {}",
                        config.recent_windows.previews.max_height
                    );
                    println!(
                        "    previews.max_scale: {}",
                        config.recent_windows.previews.max_scale
                    );

                    // Show overview configuration
                    println!("\n  Overview Configuration:");
                    println!("    zoom: {}", config.overview.zoom);
                    println!(
                        "    workspace_shadow.off: {}",
                        config.overview.workspace_shadow.off
                    );
                    println!(
                        "    workspace_shadow.softness: {}",
                        config.overview.workspace_shadow.softness
                    );
                    println!(
                        "    workspace_shadow.spread: {}",
                        config.overview.workspace_shadow.spread
                    );
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
