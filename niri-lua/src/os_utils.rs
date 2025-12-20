//! OS utilities for Lua API (niri.os namespace)
//!
//! Provides operating system information functions for Lua configuration scripts.
//! These functions enable multi-machine configurations and environment variable access.

use std::fs;
use std::path::PathBuf;

use mlua::{Lua, Result, Table};

/// Register the `niri.os` namespace with OS utility functions.
///
/// # Functions
///
/// - `hostname()` - Returns the system hostname as a string
/// - `getenv(name)` - Returns the value of an environment variable or nil
/// - `username()` - Returns the current username as a string
/// - `home()` - Returns the user's home directory as a string
/// - `tmpdir()` - Returns the system temporary directory as a string
/// - `platform()` - Returns the OS name as a string
/// - `arch()` - Returns the CPU architecture as a string
pub fn register(lua: &Lua, niri: &Table) -> Result<()> {
    let os_table = lua.create_table()?;

    // niri.os.hostname() -> string
    // Returns the system hostname. Throws on invalid UTF-8 (rare).
    // On other system errors, returns empty string (per spec).
    os_table.set(
        "hostname",
        lua.create_function(|_, ()| {
            gethostname::gethostname().into_string().map_err(|_| {
                mlua::Error::runtime("niri.os.hostname: hostname contains invalid UTF-8")
            })
        })?,
    )?;

    // niri.os.getenv(name) -> string | nil
    // Returns the value of an environment variable, or nil if not set.
    // Returns empty string "" if the variable is set to empty (matches Neovim semantics).
    os_table.set(
        "getenv",
        lua.create_function(|_, name: String| Ok(std::env::var(&name).ok()))?,
    )?;

    // niri.os.setenv(name, value) -> ()
    // Sets an environment variable. If value is nil, removes the variable.
    // Note: Changes only affect the current process and its children.
    os_table.set(
        "setenv",
        lua.create_function(|_, (name, value): (String, Option<String>)| {
            match value {
                Some(v) => std::env::set_var(&name, &v),
                None => std::env::remove_var(&name),
            }
            Ok(())
        })?,
    )?;

    // niri.os.username() -> string
    // Returns the current username from $USER or $USERNAME, or "unknown" if unavailable.
    os_table.set(
        "username",
        lua.create_function(|_, ()| {
            Ok(std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or("unknown".to_string()))
        })?,
    )?;

    // niri.os.home() -> string
    // Returns the user's home directory, or "unknown" if unavailable.
    os_table.set(
        "home",
        lua.create_function(|_, ()| {
            Ok(dirs::home_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or("unknown".to_string()))
        })?,
    )?;

    // niri.os.tmpdir() -> string
    // Returns the system temporary directory.
    os_table.set(
        "tmpdir",
        lua.create_function(|_, ()| Ok(std::env::temp_dir().to_string_lossy().to_string()))?,
    )?;

    // niri.os.platform() -> string
    // Returns the OS name.
    os_table.set(
        "platform",
        lua.create_function(|_, ()| Ok(std::env::consts::OS.to_string()))?,
    )?;

    // niri.os.arch() -> string
    // Returns the CPU architecture.
    os_table.set(
        "arch",
        lua.create_function(|_, ()| Ok(std::env::consts::ARCH.to_string()))?,
    )?;

    // =========================================================================
    // XDG Base Directory Functions
    // =========================================================================

    // niri.os.xdg_config_home() -> string
    // Returns $XDG_CONFIG_HOME or ~/.config. Creates the directory if needed.
    os_table.set(
        "xdg_config_home",
        lua.create_function(|_, ()| {
            let path = dirs::config_dir()
                .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"));
            ensure_dir_exists(&path);
            Ok(path.to_string_lossy().to_string())
        })?,
    )?;

    // niri.os.xdg_data_home() -> string
    // Returns $XDG_DATA_HOME or ~/.local/share. Creates the directory if needed.
    os_table.set(
        "xdg_data_home",
        lua.create_function(|_, ()| {
            let path = dirs::data_dir()
                .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".local/share"));
            ensure_dir_exists(&path);
            Ok(path.to_string_lossy().to_string())
        })?,
    )?;

    // niri.os.xdg_cache_home() -> string
    // Returns $XDG_CACHE_HOME or ~/.cache. Creates the directory if needed.
    os_table.set(
        "xdg_cache_home",
        lua.create_function(|_, ()| {
            let path = dirs::cache_dir()
                .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".cache"));
            ensure_dir_exists(&path);
            Ok(path.to_string_lossy().to_string())
        })?,
    )?;

    // niri.os.xdg_state_home() -> string
    // Returns $XDG_STATE_HOME or ~/.local/state. Creates the directory if needed.
    os_table.set(
        "xdg_state_home",
        lua.create_function(|_, ()| {
            let path = dirs::state_dir()
                .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".local/state"));
            ensure_dir_exists(&path);
            Ok(path.to_string_lossy().to_string())
        })?,
    )?;

    // niri.os.xdg_runtime_dir() -> string | nil
    // Returns $XDG_RUNTIME_DIR or nil if not set. Does NOT create the directory.
    os_table.set(
        "xdg_runtime_dir",
        lua.create_function(|_, ()| {
            Ok(dirs::runtime_dir().map(|p| p.to_string_lossy().to_string()))
        })?,
    )?;

    // niri.os.xdg_data_dirs() -> string[]
    // Returns $XDG_DATA_DIRS split by ':' or defaults to ["/usr/local/share", "/usr/share"].
    os_table.set(
        "xdg_data_dirs",
        lua.create_function(|_, ()| Ok(get_xdg_data_dirs()))?,
    )?;

    // niri.os.xdg_config_dirs() -> string[]
    // Returns $XDG_CONFIG_DIRS split by ':' or defaults to ["/etc/xdg"].
    os_table.set(
        "xdg_config_dirs",
        lua.create_function(|_, ()| Ok(get_xdg_config_dirs()))?,
    )?;

    // =========================================================================
    // Niri-Specific Directory Functions
    // =========================================================================

    // niri.os.niri_config_dir() -> string
    // Returns the niri config directory ($XDG_CONFIG_HOME/niri or ~/.config/niri).
    // Creates the directory if it doesn't exist (best-effort).
    os_table.set(
        "niri_config_dir",
        lua.create_function(|_, ()| {
            let path = dirs::config_dir()
                .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"))
                .join("niri");
            ensure_dir_exists(&path);
            Ok(path.to_string_lossy().to_string())
        })?,
    )?;

    // niri.os.niri_data_dir() -> string
    // Returns the niri data directory ($XDG_DATA_HOME/niri or ~/.local/share/niri).
    // Creates the directory if it doesn't exist (best-effort).
    os_table.set(
        "niri_data_dir",
        lua.create_function(|_, ()| {
            let path = dirs::data_dir()
                .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".local/share"))
                .join("niri");
            ensure_dir_exists(&path);
            Ok(path.to_string_lossy().to_string())
        })?,
    )?;

    // niri.os.niri_cache_dir() -> string
    // Returns the niri cache directory ($XDG_CACHE_HOME/niri or ~/.cache/niri).
    // Creates the directory if it doesn't exist (best-effort).
    os_table.set(
        "niri_cache_dir",
        lua.create_function(|_, ()| {
            let path = dirs::cache_dir()
                .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".cache"))
                .join("niri");
            ensure_dir_exists(&path);
            Ok(path.to_string_lossy().to_string())
        })?,
    )?;

    // niri.os.niri_state_dir() -> string
    // Returns the niri state directory ($XDG_STATE_HOME/niri or ~/.local/state/niri).
    // Creates the directory if it doesn't exist (best-effort).
    os_table.set(
        "niri_state_dir",
        lua.create_function(|_, ()| {
            let path = dirs::state_dir()
                .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".local/state"))
                .join("niri");
            ensure_dir_exists(&path);
            Ok(path.to_string_lossy().to_string())
        })?,
    )?;

    niri.set("os", os_table)?;
    Ok(())
}

/// Ensure a directory exists, creating it if necessary.
/// Silently ignores errors (best-effort).
fn ensure_dir_exists(path: &PathBuf) {
    if !path.exists() {
        let _ = fs::create_dir_all(path);
    }
}

/// Get XDG_DATA_DIRS, split by ':', with defaults if unset or empty.
fn get_xdg_data_dirs() -> Vec<String> {
    match std::env::var("XDG_DATA_DIRS") {
        Ok(val) if !val.is_empty() => val.split(':').map(|s| s.to_string()).collect(),
        _ => vec!["/usr/local/share".to_string(), "/usr/share".to_string()],
    }
}

/// Get XDG_CONFIG_DIRS, split by ':', with defaults if unset or empty.
fn get_xdg_config_dirs() -> Vec<String> {
    match std::env::var("XDG_CONFIG_DIRS") {
        Ok(val) if !val.is_empty() => val.split(':').map(|s| s.to_string()).collect(),
        _ => vec!["/etc/xdg".to_string()],
    }
}

/// Helper function for testing hostname decoding with arbitrary OsString values.
/// This allows unit tests to inject invalid UTF-8 without depending on the host's actual hostname.
#[cfg(test)]
pub fn decode_hostname_for_test(
    hostname: std::ffi::OsString,
) -> std::result::Result<String, &'static str> {
    hostname
        .into_string()
        .map_err(|_| "niri.os.hostname: hostname contains invalid UTF-8")
}

#[cfg(test)]
mod tests {
    use mlua::{Function, Lua, Table};

    use super::*;

    #[test]
    fn hostname_returns_non_empty_string() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();

        let os: Table = niri.get("os").unwrap();
        let hostname_fn: Function = os.get("hostname").unwrap();
        let hostname: String = hostname_fn.call(()).unwrap();
        assert!(!hostname.is_empty());
    }

    #[test]
    fn getenv_returns_value_and_preserves_empty() {
        std::env::set_var("NIRI_TEST_ENV", "value");
        std::env::set_var("NIRI_TEST_EMPTY", "");

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let os: Table = niri.get("os").unwrap();
        let getenv: Function = os.get("getenv").unwrap();

        assert_eq!(
            getenv.call::<Option<String>>("NIRI_TEST_ENV").unwrap(),
            Some("value".into())
        );
        assert_eq!(
            getenv.call::<Option<String>>("NIRI_TEST_EMPTY").unwrap(),
            Some("".into())
        );

        std::env::remove_var("NIRI_TEST_ENV");
        std::env::remove_var("NIRI_TEST_EMPTY");
    }

    #[test]
    fn getenv_returns_none_for_unset() {
        std::env::remove_var("NIRI_TEST_UNSET");

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let os: Table = niri.get("os").unwrap();
        let getenv: Function = os.get("getenv").unwrap();
        let res: Option<String> = getenv.call("NIRI_TEST_UNSET").unwrap();
        assert!(res.is_none());
    }

    #[test]
    fn hostname_invalid_utf8_throws() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        // Construct an OsString containing invalid UTF-8 bytes
        let invalid = OsString::from_vec(vec![0xff, 0xff, 0xff]);

        match decode_hostname_for_test(invalid) {
            Err(e) => {
                assert!(e.contains("invalid UTF-8"));
            }
            Ok(_) => panic!("expected decode to fail on invalid UTF-8"),
        }
    }

    #[test]
    fn getenv_case_sensitivity() {
        // On Unix, environment variables are case-sensitive
        std::env::set_var("NIRI_CASE_TEST", "upper");
        std::env::remove_var("niri_case_test");

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let os: Table = niri.get("os").unwrap();
        let getenv: Function = os.get("getenv").unwrap();

        assert_eq!(
            getenv.call::<Option<String>>("NIRI_CASE_TEST").unwrap(),
            Some("upper".into())
        );
        // On Unix, lowercase should not find the uppercase variable
        assert!(getenv
            .call::<Option<String>>("niri_case_test")
            .unwrap()
            .is_none());

        std::env::remove_var("NIRI_CASE_TEST");
    }

    #[test]
    fn getenv_long_value() {
        // Test that long values are not truncated
        let long_value: String = "x".repeat(8192);
        std::env::set_var("NIRI_LONG_VAR", &long_value);

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let os: Table = niri.get("os").unwrap();
        let getenv: Function = os.get("getenv").unwrap();

        let result: Option<String> = getenv.call("NIRI_LONG_VAR").unwrap();
        assert_eq!(result.unwrap().len(), 8192);

        std::env::remove_var("NIRI_LONG_VAR");
    }

    #[test]
    fn username_returns_string() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();

        let os: Table = niri.get("os").unwrap();
        let username_fn: Function = os.get("username").unwrap();
        let username: String = username_fn.call(()).unwrap();
        assert!(!username.is_empty());
    }

    #[test]
    fn home_returns_string() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();

        let os: Table = niri.get("os").unwrap();
        let home_fn: Function = os.get("home").unwrap();
        let home: String = home_fn.call(()).unwrap();
        assert!(!home.is_empty());
    }

    #[test]
    fn tmpdir_returns_string() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();

        let os: Table = niri.get("os").unwrap();
        let tmpdir_fn: Function = os.get("tmpdir").unwrap();
        let tmpdir: String = tmpdir_fn.call(()).unwrap();
        assert!(!tmpdir.is_empty());
    }

    #[test]
    fn platform_returns_string() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();

        let os: Table = niri.get("os").unwrap();
        let platform_fn: Function = os.get("platform").unwrap();
        let platform: String = platform_fn.call(()).unwrap();
        assert!(!platform.is_empty());
    }

    #[test]
    fn arch_returns_string() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();

        let os: Table = niri.get("os").unwrap();
        let arch_fn: Function = os.get("arch").unwrap();
        let arch: String = arch_fn.call(()).unwrap();
        assert!(!arch.is_empty());
    }

    // =========================================================================
    // XDG Directory Function Tests
    // =========================================================================

    #[test]
    fn xdg_config_home_returns_path() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();

        let os: Table = niri.get("os").unwrap();
        let func: Function = os.get("xdg_config_home").unwrap();
        let path: String = func.call(()).unwrap();
        assert!(!path.is_empty());
        // Should contain "config" somewhere (either ~/.config or XDG_CONFIG_HOME)
        assert!(
            path.contains("config") || path.contains("Config"),
            "path should contain 'config': {}",
            path
        );
    }

    #[test]
    fn xdg_data_home_returns_path() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();

        let os: Table = niri.get("os").unwrap();
        let func: Function = os.get("xdg_data_home").unwrap();
        let path: String = func.call(()).unwrap();
        assert!(!path.is_empty());
    }

    #[test]
    fn xdg_cache_home_returns_path() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();

        let os: Table = niri.get("os").unwrap();
        let func: Function = os.get("xdg_cache_home").unwrap();
        let path: String = func.call(()).unwrap();
        assert!(!path.is_empty());
    }

    #[test]
    fn xdg_state_home_returns_path() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();

        let os: Table = niri.get("os").unwrap();
        let func: Function = os.get("xdg_state_home").unwrap();
        let path: String = func.call(()).unwrap();
        assert!(!path.is_empty());
    }

    #[test]
    fn xdg_runtime_dir_returns_option() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();

        let os: Table = niri.get("os").unwrap();
        let func: Function = os.get("xdg_runtime_dir").unwrap();
        // This may return Some or None depending on the environment
        let result: Option<String> = func.call(()).unwrap();
        // Just verify it doesn't error - value depends on environment
        if let Some(path) = result {
            assert!(!path.is_empty());
        }
    }

    #[test]
    fn xdg_data_dirs_returns_array() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();

        let os: Table = niri.get("os").unwrap();
        let func: Function = os.get("xdg_data_dirs").unwrap();
        let dirs: Vec<String> = func.call(()).unwrap();
        assert!(
            !dirs.is_empty(),
            "xdg_data_dirs should return at least one directory"
        );
    }

    #[test]
    fn xdg_data_dirs_env_behavior() {
        // Test env var behavior in a single test to avoid race conditions
        // with parallel test execution
        let old_val = std::env::var("XDG_DATA_DIRS").ok();

        // Test 1: Custom value
        std::env::set_var("XDG_DATA_DIRS", "/custom/path1:/custom/path2");
        let dirs = get_xdg_data_dirs();
        assert_eq!(dirs, vec!["/custom/path1", "/custom/path2"]);

        // Test 2: Empty value should return defaults
        std::env::set_var("XDG_DATA_DIRS", "");
        let dirs = get_xdg_data_dirs();
        assert_eq!(dirs, vec!["/usr/local/share", "/usr/share"]);

        // Test 3: Unset should return defaults
        std::env::remove_var("XDG_DATA_DIRS");
        let dirs = get_xdg_data_dirs();
        assert_eq!(dirs, vec!["/usr/local/share", "/usr/share"]);

        // Restore
        match old_val {
            Some(v) => std::env::set_var("XDG_DATA_DIRS", v),
            None => std::env::remove_var("XDG_DATA_DIRS"),
        }
    }

    #[test]
    fn xdg_config_dirs_returns_array() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();

        let os: Table = niri.get("os").unwrap();
        let func: Function = os.get("xdg_config_dirs").unwrap();
        let dirs: Vec<String> = func.call(()).unwrap();
        assert!(
            !dirs.is_empty(),
            "xdg_config_dirs should return at least one directory"
        );
    }

    #[test]
    fn xdg_config_dirs_env_behavior() {
        // Test env var behavior in a single test to avoid race conditions
        let old_val = std::env::var("XDG_CONFIG_DIRS").ok();

        // Test 1: Custom value
        std::env::set_var("XDG_CONFIG_DIRS", "/etc/custom:/opt/config");
        let dirs = get_xdg_config_dirs();
        assert_eq!(dirs, vec!["/etc/custom", "/opt/config"]);

        // Test 2: Empty value should return defaults
        std::env::set_var("XDG_CONFIG_DIRS", "");
        let dirs = get_xdg_config_dirs();
        assert_eq!(dirs, vec!["/etc/xdg"]);

        // Test 3: Unset should return defaults
        std::env::remove_var("XDG_CONFIG_DIRS");
        let dirs = get_xdg_config_dirs();
        assert_eq!(dirs, vec!["/etc/xdg"]);

        // Restore
        match old_val {
            Some(v) => std::env::set_var("XDG_CONFIG_DIRS", v),
            None => std::env::remove_var("XDG_CONFIG_DIRS"),
        }
    }

    // =========================================================================
    // Niri-Specific Directory Function Tests
    // =========================================================================

    #[test]
    fn niri_config_dir_returns_path_with_niri_suffix() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();

        let os: Table = niri.get("os").unwrap();
        let func: Function = os.get("niri_config_dir").unwrap();
        let path: String = func.call(()).unwrap();

        assert!(!path.is_empty());
        assert!(
            path.ends_with("/niri") || path.ends_with("\\niri"),
            "path should end with /niri: {}",
            path
        );
    }

    #[test]
    fn niri_data_dir_returns_path_with_niri_suffix() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();

        let os: Table = niri.get("os").unwrap();
        let func: Function = os.get("niri_data_dir").unwrap();
        let path: String = func.call(()).unwrap();

        assert!(!path.is_empty());
        assert!(
            path.ends_with("/niri") || path.ends_with("\\niri"),
            "path should end with /niri: {}",
            path
        );
    }

    #[test]
    fn niri_cache_dir_returns_path_with_niri_suffix() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();

        let os: Table = niri.get("os").unwrap();
        let func: Function = os.get("niri_cache_dir").unwrap();
        let path: String = func.call(()).unwrap();

        assert!(!path.is_empty());
        assert!(
            path.ends_with("/niri") || path.ends_with("\\niri"),
            "path should end with /niri: {}",
            path
        );
    }

    #[test]
    fn niri_state_dir_returns_path_with_niri_suffix() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();

        let os: Table = niri.get("os").unwrap();
        let func: Function = os.get("niri_state_dir").unwrap();
        let path: String = func.call(()).unwrap();

        assert!(!path.is_empty());
        assert!(
            path.ends_with("/niri") || path.ends_with("\\niri"),
            "path should end with /niri: {}",
            path
        );
    }

    #[test]
    fn niri_dirs_env_behavior_and_directory_creation() {
        // Test all niri-specific dirs in a single test to avoid race conditions
        // with parallel test execution (they all use XDG_* env vars)
        let tmpdir = tempfile::tempdir().unwrap();
        let tmp_path = tmpdir.path();

        // Save old values
        let old_config = std::env::var("XDG_CONFIG_HOME").ok();
        let old_data = std::env::var("XDG_DATA_HOME").ok();
        let old_cache = std::env::var("XDG_CACHE_HOME").ok();
        let old_state = std::env::var("XDG_STATE_HOME").ok();

        // Set custom XDG directories
        let config_base = tmp_path.join("config");
        let data_base = tmp_path.join("data");
        let cache_base = tmp_path.join("cache");
        let state_base = tmp_path.join("state");

        std::env::set_var("XDG_CONFIG_HOME", &config_base);
        std::env::set_var("XDG_DATA_HOME", &data_base);
        std::env::set_var("XDG_CACHE_HOME", &cache_base);
        std::env::set_var("XDG_STATE_HOME", &state_base);

        let lua = Lua::new();
        let niri_table = lua.create_table().unwrap();
        register(&lua, &niri_table).unwrap();
        let os: Table = niri_table.get("os").unwrap();

        // Test niri_config_dir
        let func: Function = os.get("niri_config_dir").unwrap();
        let path: String = func.call(()).unwrap();
        let expected_config = config_base.join("niri");
        assert_eq!(path, expected_config.to_string_lossy());
        assert!(
            expected_config.exists(),
            "niri_config_dir should create directory"
        );

        // Test niri_data_dir
        let func: Function = os.get("niri_data_dir").unwrap();
        let path: String = func.call(()).unwrap();
        let expected_data = data_base.join("niri");
        assert_eq!(path, expected_data.to_string_lossy());
        assert!(
            expected_data.exists(),
            "niri_data_dir should create directory"
        );

        // Test niri_cache_dir
        let func: Function = os.get("niri_cache_dir").unwrap();
        let path: String = func.call(()).unwrap();
        let expected_cache = cache_base.join("niri");
        assert_eq!(path, expected_cache.to_string_lossy());
        assert!(
            expected_cache.exists(),
            "niri_cache_dir should create directory"
        );

        // Test niri_state_dir
        let func: Function = os.get("niri_state_dir").unwrap();
        let path: String = func.call(()).unwrap();
        let expected_state = state_base.join("niri");
        assert_eq!(path, expected_state.to_string_lossy());
        assert!(
            expected_state.exists(),
            "niri_state_dir should create directory"
        );

        // Restore old values
        match old_config {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_data {
            Some(v) => std::env::set_var("XDG_DATA_HOME", v),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }
        match old_cache {
            Some(v) => std::env::set_var("XDG_CACHE_HOME", v),
            None => std::env::remove_var("XDG_CACHE_HOME"),
        }
        match old_state {
            Some(v) => std::env::set_var("XDG_STATE_HOME", v),
            None => std::env::remove_var("XDG_STATE_HOME"),
        }
    }

    #[test]
    fn niri_dirs_idempotent_when_dir_exists() {
        // Test that calling the functions multiple times is safe when dirs already exist
        let tmpdir = tempfile::tempdir().unwrap();
        let tmp_path = tmpdir.path();

        let old_config = std::env::var("XDG_CONFIG_HOME").ok();
        std::env::set_var("XDG_CONFIG_HOME", tmp_path);

        // Pre-create the niri directory
        let niri_dir = tmp_path.join("niri");
        std::fs::create_dir_all(&niri_dir).unwrap();

        let lua = Lua::new();
        let niri_table = lua.create_table().unwrap();
        register(&lua, &niri_table).unwrap();
        let os: Table = niri_table.get("os").unwrap();

        // Call multiple times - should not error
        let func: Function = os.get("niri_config_dir").unwrap();
        let path1: String = func.call(()).unwrap();
        let path2: String = func.call(()).unwrap();

        assert_eq!(path1, path2);
        assert_eq!(path1, niri_dir.to_string_lossy());

        // Restore
        match old_config {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }

    // =========================================================================
    // setenv Tests
    // =========================================================================

    #[test]
    fn setenv_sets_variable() {
        // Save and ensure clean state
        let old_val = std::env::var("NIRI_SETENV_TEST").ok();
        std::env::remove_var("NIRI_SETENV_TEST");

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let os: Table = niri.get("os").unwrap();
        let setenv: Function = os.get("setenv").unwrap();
        let getenv: Function = os.get("getenv").unwrap();

        // Set the variable
        setenv
            .call::<()>(("NIRI_SETENV_TEST", "test_value"))
            .unwrap();

        // Verify it was set
        let result: Option<String> = getenv.call("NIRI_SETENV_TEST").unwrap();
        assert_eq!(result, Some("test_value".to_string()));

        // Restore
        match old_val {
            Some(v) => std::env::set_var("NIRI_SETENV_TEST", v),
            None => std::env::remove_var("NIRI_SETENV_TEST"),
        }
    }

    #[test]
    fn setenv_removes_variable_with_nil() {
        // Set up a variable to remove
        std::env::set_var("NIRI_SETENV_REMOVE", "to_remove");

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let os: Table = niri.get("os").unwrap();
        let setenv: Function = os.get("setenv").unwrap();
        let getenv: Function = os.get("getenv").unwrap();

        // Verify it exists first
        let before: Option<String> = getenv.call("NIRI_SETENV_REMOVE").unwrap();
        assert_eq!(before, Some("to_remove".to_string()));

        // Remove by passing nil (None in Lua)
        setenv
            .call::<()>(("NIRI_SETENV_REMOVE", mlua::Value::Nil))
            .unwrap();

        // Verify it was removed
        let after: Option<String> = getenv.call("NIRI_SETENV_REMOVE").unwrap();
        assert!(after.is_none());
    }

    #[test]
    fn setenv_overwrites_existing() {
        let old_val = std::env::var("NIRI_SETENV_OVERWRITE").ok();
        std::env::set_var("NIRI_SETENV_OVERWRITE", "old_value");

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let os: Table = niri.get("os").unwrap();
        let setenv: Function = os.get("setenv").unwrap();
        let getenv: Function = os.get("getenv").unwrap();

        // Overwrite the variable
        setenv
            .call::<()>(("NIRI_SETENV_OVERWRITE", "new_value"))
            .unwrap();

        // Verify it was overwritten
        let result: Option<String> = getenv.call("NIRI_SETENV_OVERWRITE").unwrap();
        assert_eq!(result, Some("new_value".to_string()));

        // Restore
        match old_val {
            Some(v) => std::env::set_var("NIRI_SETENV_OVERWRITE", v),
            None => std::env::remove_var("NIRI_SETENV_OVERWRITE"),
        }
    }

    #[test]
    fn setenv_empty_string() {
        let old_val = std::env::var("NIRI_SETENV_EMPTY").ok();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let os: Table = niri.get("os").unwrap();
        let setenv: Function = os.get("setenv").unwrap();
        let getenv: Function = os.get("getenv").unwrap();

        // Set to empty string
        setenv.call::<()>(("NIRI_SETENV_EMPTY", "")).unwrap();

        // Verify it was set to empty (not nil)
        let result: Option<String> = getenv.call("NIRI_SETENV_EMPTY").unwrap();
        assert_eq!(result, Some("".to_string()));

        // Restore
        match old_val {
            Some(v) => std::env::set_var("NIRI_SETENV_EMPTY", v),
            None => std::env::remove_var("NIRI_SETENV_EMPTY"),
        }
    }
}
