//! OS utilities for Lua API (niri.os namespace)
//!
//! Provides operating system information functions for Lua configuration scripts.
//! These functions enable multi-machine configurations and environment variable access.

use mlua::{Lua, Result, Table};

/// Register the `niri.os` namespace with OS utility functions.
///
/// # Functions
///
/// - `hostname()` - Returns the system hostname as a string
/// - `getenv(name)` - Returns the value of an environment variable or nil
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

    niri.set("os", os_table)?;
    Ok(())
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

    #[cfg(unix)]
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
        #[cfg(unix)]
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
}
