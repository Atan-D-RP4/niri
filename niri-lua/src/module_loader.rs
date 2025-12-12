//! Niri Lua module search paths.
//!
//! Provides XDG-compliant search paths for user Lua modules.
//! These paths can be used when implementing custom require behavior.
//!
//! # Search Paths
//!
//! - `$XDG_CONFIG_HOME/niri/lua/` (defaults to `~/.config/niri/lua/`)
//! - `$XDG_DATA_HOME/niri/lua/` (defaults to `~/.local/share/niri/lua/`)
//! - `/usr/share/niri/lua/`

use std::env;
use std::path::PathBuf;

/// Get the default niri Lua search paths following XDG Base Directory spec.
///
/// Returns paths in priority order (user config first, system last).
pub fn default_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // XDG_CONFIG_HOME or ~/.config (highest priority)
    let config_home = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));

    if let Some(config_dir) = config_home {
        paths.push(config_dir.join("niri/lua"));
    }

    // XDG_DATA_HOME or ~/.local/share
    let data_home = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));

    if let Some(data_dir) = data_home {
        paths.push(data_dir.join("niri/lua"));
    }

    // System directory
    paths.push(PathBuf::from("/usr/share/niri/lua"));

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_paths_not_empty() {
        let paths = default_paths();
        // Should have at least one path (system path always exists)
        assert!(!paths.is_empty());
        // System path is always last
        assert_eq!(paths.last().unwrap(), &PathBuf::from("/usr/share/niri/lua"));
    }

    #[test]
    fn default_paths_includes_system() {
        let paths = default_paths();
        assert!(paths.contains(&PathBuf::from("/usr/share/niri/lua")));
    }
}
