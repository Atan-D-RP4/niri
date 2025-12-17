//! Filesystem utilities for Lua API (niri.fs namespace)
//!
//! Provides filesystem operations and path utilities for Lua configuration scripts.
//! These functions enable conditional configuration based on file existence and
//! executable availability.

use mlua::{Lua, Result, Table};

/// Register the `niri.fs` namespace with filesystem utility functions.
///
/// # Functions
///
/// - `readable(path)` - Returns true if the file exists and is readable
/// - `expand(path)` - Expands `~` and `$VAR` in paths
/// - `which(cmd)` - Finds an executable in PATH, returns full path or nil
pub fn register(lua: &Lua, niri: &Table) -> Result<()> {
    let fs_table = lua.create_table()?;

    // niri.fs.readable(path) -> boolean
    // Returns true if the file exists and is readable, false otherwise.
    // Follows symlinks; broken symlinks return false.
    // Never throws errors.
    fs_table.set(
        "readable",
        lua.create_function(|_, path: String| Ok(std::fs::File::open(&path).is_ok()))?,
    )?;

    // niri.fs.expand(path) -> string
    // Expands `~`, `$VAR`, and `${VAR}` in paths.
    // Unset environment variables expand to empty string (matches Neovim semantics).
    // On Windows, `~` expands to $HOME or %USERPROFILE%.
    // Never throws errors - returns original path on expansion failure.
    fs_table.set(
        "expand",
        lua.create_function(|_, path: String| {
            // Use full_with_context_no_errors for Neovim-aligned semantics:
            // - Unset vars expand to empty string (return Some("") not None)
            // - Never errors
            let home_dir: Option<String> =
                dirs::home_dir().map(|p| p.to_string_lossy().into_owned());
            let result = shellexpand::full_with_context_no_errors(
                &path,
                || home_dir.as_deref(),
                |var| Some(std::env::var(var).unwrap_or_default()),
            );
            Ok(result.into_owned())
        })?,
    )?;

    // niri.fs.which(cmd) -> string | nil
    // Finds an executable in PATH, returns the full path or nil.
    // If cmd contains a path separator or is an absolute path, treats it as a path
    // and returns it if it exists and is executable.
    // On Windows, respects PATHEXT for extension resolution.
    // Never throws errors.
    fs_table.set(
        "which",
        lua.create_function(|_, command: String| {
            if command.is_empty() {
                return Ok(None);
            }
            Ok(which::which(&command)
                .ok()
                .map(|p| p.to_string_lossy().into_owned()))
        })?,
    )?;

    niri.set("fs", fs_table)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use mlua::{Function, Lua, Table};
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn readable_true_for_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, b"test").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let readable: Function = fs_table.get("readable").unwrap();
        assert!(readable.call::<bool>(file_path.to_str().unwrap()).unwrap());
    }

    #[test]
    fn readable_false_for_nonexistent() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let readable: Function = fs_table.get("readable").unwrap();
        assert!(!readable.call::<bool>("/nonexistent/path/xyz").unwrap());
    }

    #[test]
    fn readable_false_for_empty_path() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let readable: Function = fs_table.get("readable").unwrap();
        assert!(!readable.call::<bool>("").unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_following_and_broken_symlink() {
        use std::os::unix::fs::symlink;
        let dir = tempdir().unwrap();
        let target = dir.path().join("target.txt");
        let link = dir.path().join("link.txt");

        fs::write(&target, b"data").unwrap();
        symlink(&target, &link).unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let readable: Function = fs_table.get("readable").unwrap();

        // symlink -> readable
        assert!(readable.call::<bool>(link.to_str().unwrap()).unwrap());

        // remove target -> broken symlink
        fs::remove_file(&target).unwrap();
        assert!(!readable.call::<bool>(link.to_str().unwrap()).unwrap());
    }

    #[test]
    fn expand_env_var() {
        std::env::set_var("NIRI_TEST_HOME", "/home/testuser");
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let expand: Function = fs_table.get("expand").unwrap();
        let expanded = expand.call::<String>("$NIRI_TEST_HOME/.config").unwrap();
        assert_eq!(expanded, "/home/testuser/.config");
        std::env::remove_var("NIRI_TEST_HOME");
    }

    #[test]
    fn expand_braced_env_var() {
        std::env::set_var("NIRI_TEST_VAR", "value");
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let expand: Function = fs_table.get("expand").unwrap();
        let expanded = expand.call::<String>("${NIRI_TEST_VAR}/path").unwrap();
        assert_eq!(expanded, "value/path");
        std::env::remove_var("NIRI_TEST_VAR");
    }

    #[test]
    fn expand_unset_var_to_empty() {
        std::env::remove_var("NIRI_TEST_UNSET_VAR");
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let expand: Function = fs_table.get("expand").unwrap();
        // Unset var should expand to empty string (Neovim semantics)
        let expanded = expand.call::<String>("$NIRI_TEST_UNSET_VAR/path").unwrap();
        assert_eq!(expanded, "/path");
    }

    #[test]
    fn expand_tilde() {
        // Test tilde expansion - should expand to home directory
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let expand: Function = fs_table.get("expand").unwrap();
        let expanded = expand.call::<String>("~/.config").unwrap();
        // Should not start with ~ after expansion
        assert!(!expanded.starts_with('~'));
        // Should end with /.config
        assert!(expanded.ends_with("/.config"));
    }

    #[test]
    fn which_finds_executable_in_path() {
        let dir = tempdir().unwrap();
        let bin = dir.path().join("test-exec");
        fs::write(&bin, b"dummy").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&bin).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&bin, perms).unwrap();
        }

        // Prepend temp dir to PATH
        let old_path = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", dir.path().to_str().unwrap(), old_path);
        std::env::set_var("PATH", &new_path);

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let which_fn: Function = fs_table.get("which").unwrap();
        let res: Option<String> = which_fn.call("test-exec").unwrap();
        assert_eq!(res.map(|p| PathBuf::from(p)), Some(bin));

        std::env::set_var("PATH", &old_path);
    }

    #[test]
    fn which_returns_none_for_missing() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let which_fn: Function = fs_table.get("which").unwrap();
        let res: Option<String> = which_fn.call("__nonexistent_cmd_98765__").unwrap();
        assert!(res.is_none());
    }

    #[test]
    fn which_returns_none_for_empty() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let which_fn: Function = fs_table.get("which").unwrap();
        let res: Option<String> = which_fn.call("").unwrap();
        assert!(res.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn which_with_absolute_path() {
        let dir = tempdir().unwrap();
        let bin = dir.path().join("myapp");
        fs::write(&bin, b"dummy").unwrap();

        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&bin).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin, perms).unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let which_fn: Function = fs_table.get("which").unwrap();

        // which with absolute path should return the same path if executable
        let res: Option<String> = which_fn.call(bin.to_str().unwrap()).unwrap();
        assert_eq!(res.map(|p| PathBuf::from(p)), Some(bin));
    }

    #[cfg(unix)]
    #[test]
    fn which_non_executable_returns_none() {
        let dir = tempdir().unwrap();
        let bin = dir.path().join("not-executable");
        fs::write(&bin, b"dummy").unwrap();

        // Don't set executable bit
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&bin).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&bin, perms).unwrap();

        // Prepend temp dir to PATH
        let old_path = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", dir.path().to_str().unwrap(), old_path);
        std::env::set_var("PATH", &new_path);

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let which_fn: Function = fs_table.get("which").unwrap();

        // Non-executable should not be found
        let res: Option<String> = which_fn.call("not-executable").unwrap();
        assert!(res.is_none());

        std::env::set_var("PATH", &old_path);
    }
}
