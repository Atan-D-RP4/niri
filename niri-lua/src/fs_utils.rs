//! Filesystem utilities for Lua API (niri.fs namespace)
//!
//! Provides filesystem operations and path utilities for Lua configuration scripts.
//! These functions enable conditional configuration based on file existence and
//! executable availability.

use std::path::Path;

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

    // =========================================================================
    // Path Query Functions (Never Throw - return false on error)
    // =========================================================================

    // niri.fs.exists(path) -> boolean
    // Returns true if the path exists (file, directory, or symlink target).
    // Follows symlinks; returns false if symlink is broken.
    // Never throws errors.
    fs_table.set(
        "exists",
        lua.create_function(|_, path: String| Ok(Path::new(&path).exists()))?,
    )?;

    // niri.fs.is_file(path) -> boolean
    // Returns true if path is a regular file. Follows symlinks.
    // Never throws errors.
    fs_table.set(
        "is_file",
        lua.create_function(|_, path: String| Ok(Path::new(&path).is_file()))?,
    )?;

    // niri.fs.is_dir(path) -> boolean
    // Returns true if path is a directory. Follows symlinks.
    // Never throws errors.
    fs_table.set(
        "is_dir",
        lua.create_function(|_, path: String| Ok(Path::new(&path).is_dir()))?,
    )?;

    // niri.fs.is_symlink(path) -> boolean
    // Returns true if path is a symbolic link (does not follow).
    // Never throws errors.
    fs_table.set(
        "is_symlink",
        lua.create_function(|_, path: String| Ok(Path::new(&path).is_symlink()))?,
    )?;

    // niri.fs.is_executable(path) -> boolean
    // Returns true if path exists and has executable permission for current user.
    // Never throws errors.
    fs_table.set(
        "is_executable",
        lua.create_function(|_, path: String| Ok(is_executable(&path)))?,
    )?;

    // =========================================================================
    // Path Manipulation Functions (Pure String Operations)
    // =========================================================================

    // niri.fs.basename(path) -> string
    // Returns the final component of the path.
    // Examples:
    //   "/home/user/file.txt" -> "file.txt"
    //   "/home/user/" -> "user"
    //   "file.txt" -> "file.txt"
    fs_table.set(
        "basename",
        lua.create_function(|_, path: String| {
            Ok(Path::new(&path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default())
        })?,
    )?;

    // niri.fs.dirname(path) -> string
    // Returns the directory component of the path.
    // Examples:
    //   "/home/user/file.txt" -> "/home/user"
    //   "/home/user/" -> "/home"
    //   "file.txt" -> "."
    //   "/" -> "/"
    fs_table.set(
        "dirname",
        lua.create_function(|_, path: String| {
            let p = Path::new(&path);
            // Special case: root path "/" should return "/"
            if p.as_os_str() == "/" {
                return Ok("/".to_string());
            }
            Ok(p.parent()
                .map(|parent| {
                    let s = parent.to_string_lossy().to_string();
                    if s.is_empty() {
                        ".".to_string()
                    } else {
                        s
                    }
                })
                .unwrap_or_else(|| ".".to_string()))
        })?,
    )?;

    // niri.fs.extname(path) -> string
    // Returns the file extension including the dot, or empty string.
    // Examples:
    //   "file.txt" -> ".txt"
    //   "file.tar.gz" -> ".gz"
    //   "file" -> ""
    //   ".bashrc" -> ""
    fs_table.set(
        "extname",
        lua.create_function(|_, path: String| {
            Ok(Path::new(&path)
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_default())
        })?,
    )?;

    // niri.fs.joinpath(...) -> string
    // Joins path components with the OS path separator.
    // Handles absolute paths correctly (absolute path resets the base).
    // Examples:
    //   joinpath("/home", "user", "file.txt") -> "/home/user/file.txt"
    //   joinpath("dir", "/absolute") -> "/absolute"
    //   joinpath("a", "", "b") -> "a/b"
    fs_table.set(
        "joinpath",
        lua.create_function(|_, parts: mlua::Variadic<String>| {
            let mut result = std::path::PathBuf::new();
            for part in parts.iter() {
                if part.is_empty() {
                    continue;
                }
                result.push(part);
            }
            Ok(result.to_string_lossy().to_string())
        })?,
    )?;

    // =========================================================================
    // Directory Listing Functions (Return nil, error on failure)
    // =========================================================================

    // niri.fs.list(dir) -> string[]?, string?
    // Lists directory contents (files and directories). Returns basenames only.
    // Returns sorted array for deterministic results.
    // Returns nil, error_message on failure.
    fs_table.set(
        "list",
        lua.create_function(|_, dir: String| Ok(list_directory(&dir, ListFilter::All, None)))?,
    )?;

    // niri.fs.list_files(dir, pattern?) -> string[]?, string?
    // Lists only files in directory. Optional Lua pattern to filter filenames.
    // Returns sorted array for deterministic results.
    // Returns nil, error_message on failure.
    fs_table.set(
        "list_files",
        lua.create_function(|lua, (dir, pattern): (String, Option<String>)| {
            Ok(list_directory_with_lua_pattern(
                lua,
                &dir,
                ListFilter::FilesOnly,
                pattern,
            ))
        })?,
    )?;

    // niri.fs.list_dirs(dir) -> string[]?, string?
    // Lists only directories in directory.
    // Returns sorted array for deterministic results.
    // Returns nil, error_message on failure.
    fs_table.set(
        "list_dirs",
        lua.create_function(|_, dir: String| Ok(list_directory(&dir, ListFilter::DirsOnly, None)))?,
    )?;

    // =========================================================================
    // Directory Mutation Functions (Return bool, error on failure)
    // =========================================================================

    // niri.fs.mkdir(path, opts?) -> boolean, string?
    // Creates a directory.
    // opts can be:
    //   - boolean: if true, creates parent directories recursively
    //   - table: { recursive = bool, mode = int }
    // Returns true on success, false + error message on failure.
    fs_table.set(
        "mkdir",
        lua.create_function(|_, (path, opts): (String, Option<mlua::Value>)| {
            Ok(mkdir_impl(&path, opts))
        })?,
    )?;

    // niri.fs.rmdir(path) -> boolean, string?
    // Removes an empty directory.
    // Returns true on success, false + error message on failure.
    fs_table.set(
        "rmdir",
        lua.create_function(|_, path: String| match std::fs::remove_dir(&path) {
            Ok(()) => Ok((true, None)),
            Err(e) => Ok((
                false,
                Some(format!("cannot remove directory '{}': {}", path, e)),
            )),
        })?,
    )?;

    // niri.fs.remove(path, opts?) -> boolean, string?
    // Removes a file or directory.
    // opts: { recursive = bool } - if true, removes directories recursively
    // Returns true on success, false + error message on failure.
    fs_table.set(
        "remove",
        lua.create_function(|_, (path, opts): (String, Option<mlua::Table>)| {
            Ok(remove_impl(&path, opts))
        })?,
    )?;

    // =========================================================================
    // File Reading Functions (Return content, error on failure)
    // =========================================================================

    // niri.fs.read(path) -> string?, string?
    // Reads the entire file as a UTF-8 string.
    // Expands ~ in path (but not environment variables).
    // Returns nil, error_message on failure.
    fs_table.set(
        "read",
        lua.create_function(|_, path: String| Ok(read_file(&path)))?,
    )?;

    // niri.fs.readlines(path) -> string[]?, string?
    // Reads file and returns lines as an array (without newline characters).
    // Expands ~ in path (but not environment variables).
    // Returns nil, error_message on failure.
    fs_table.set(
        "readlines",
        lua.create_function(|_, path: String| Ok(read_lines(&path)))?,
    )?;

    // =========================================================================
    // File Writing Functions (Return bool, error on failure)
    // =========================================================================

    // niri.fs.write(path, content) -> boolean, string?
    // Writes content to file, creating or overwriting.
    // Expands ~ in path (but not environment variables).
    // Does NOT auto-create parent directories.
    // Returns true on success, false + error message on failure.
    fs_table.set(
        "write",
        lua.create_function(|_, (path, content): (String, String)| {
            Ok(write_file(&path, &content))
        })?,
    )?;

    // niri.fs.append(path, content) -> boolean, string?
    // Appends content to file, creating if it doesn't exist.
    // Expands ~ in path (but not environment variables).
    // Does NOT auto-create parent directories.
    // Returns true on success, false + error message on failure.
    fs_table.set(
        "append",
        lua.create_function(|_, (path, content): (String, String)| {
            Ok(append_file(&path, &content))
        })?,
    )?;

    // =========================================================================
    // File Operations (copy, rename)
    // =========================================================================

    // niri.fs.copy(src, dst) -> boolean, string?
    // Copies a file from src to dst. Only copies files (not directories).
    // Expands ~ in both paths.
    // Overwrites dst if it exists.
    // Returns true on success, false + error message on failure.
    fs_table.set(
        "copy",
        lua.create_function(|_, (src, dst): (String, String)| Ok(copy_file(&src, &dst)))?,
    )?;

    // niri.fs.rename(src, dst) -> boolean, string?
    // Renames/moves a file or directory from src to dst.
    // Expands ~ in both paths.
    // Overwrites dst if it exists (behavior depends on OS).
    // Returns true on success, false + error message on failure.
    fs_table.set(
        "rename",
        lua.create_function(|_, (src, dst): (String, String)| Ok(rename_path(&src, &dst)))?,
    )?;

    // =========================================================================
    // File Metadata Functions
    // =========================================================================

    // niri.fs.stat(path) -> table?, string?
    // Returns file/directory metadata as a table.
    // Follows symlinks (uses std::fs::metadata).
    // Fields: size (bytes), mtime (unix timestamp), atime, ctime, mode (unix), type, readonly
    // Returns nil, error_message on failure.
    fs_table.set(
        "stat",
        lua.create_function(|lua, path: String| stat_impl(lua, &path))?,
    )?;

    // niri.fs.mtime(path) -> integer?, string?
    // Returns file modification time as Unix timestamp.
    // Returns nil, error_message on failure.
    fs_table.set(
        "mtime",
        lua.create_function(|_, path: String| Ok(mtime_impl(&path)))?,
    )?;

    // niri.fs.size(path) -> integer?, string?
    // Returns file size in bytes.
    // Returns nil, error_message on failure.
    fs_table.set(
        "size",
        lua.create_function(|_, path: String| Ok(size_impl(&path)))?,
    )?;

    // =========================================================================
    // Glob Function (F4.1 - Pattern-based file matching)
    // =========================================================================

    // niri.fs.glob(pattern) -> string[]?, string?
    // Finds files matching a glob pattern (shell-style wildcards).
    // Expands ~ in pattern (but not environment variables).
    // Supports *, ?, [abc], [!abc], and ** for recursive matching.
    // Returns sorted array of normalized full paths for deterministic results.
    // Returns nil, error_message on failure.
    fs_table.set(
        "glob",
        lua.create_function(|_, pattern: String| Ok(glob_impl(&pattern)))?,
    )?;

    // =========================================================================
    // Path Utility Functions
    // =========================================================================

    // niri.fs.abspath(path) -> string?, string?
    // Converts path to absolute, resolves '.' and '..' but does NOT follow symlinks.
    // Returns nil, error if path doesn't exist.
    fs_table.set(
        "abspath",
        lua.create_function(|_, path: String| Ok(abspath_impl(&path)))?,
    )?;

    // niri.fs.normalize(path) -> string
    // Expands ~ and $VAR, resolves '.' and '..', removes redundant separators.
    // Does NOT require path to exist. Never throws errors.
    fs_table.set(
        "normalize",
        lua.create_function(|_, path: String| Ok(normalize_impl(&path)))?,
    )?;

    // niri.fs.find(names, opts?) -> string[]?, string?
    // Find files/directories by name with upward/downward search.
    // Supports simple glob patterns (* and ?) in names for downward search.
    // Returns sorted array of matching paths.
    fs_table.set(
        "find",
        lua.create_function(|_, (names, opts): (mlua::Value, Option<mlua::Table>)| {
            // Parse names: can be string or array of strings
            let name_list: Vec<String> = match names {
                mlua::Value::String(s) => vec![s.to_str()?.to_string()],
                mlua::Value::Table(t) => {
                    let mut list = Vec::new();
                    for pair in t.pairs::<i64, String>() {
                        let (_, name) = pair?;
                        list.push(name);
                    }
                    list
                }
                _ => {
                    return Ok((
                        None::<Vec<String>>,
                        Some("names must be a string or array of strings".to_string()),
                    ))
                }
            };

            // Parse options table
            let find_opts = if let Some(opts_table) = opts {
                FindOptions {
                    path: opts_table.get::<Option<String>>("path")?,
                    upward: opts_table.get::<Option<bool>>("upward")?.unwrap_or(false),
                    stop: opts_table.get::<Option<String>>("stop")?,
                    type_filter: opts_table.get::<Option<String>>("type")?,
                    limit: opts_table
                        .get::<Option<i64>>("limit")?
                        .map(|n| n.max(0) as usize),
                }
            } else {
                FindOptions::default()
            };

            Ok(find_impl(name_list, find_opts))
        })?,
    )?;

    niri.set("fs", fs_table)?;
    Ok(())
}

/// Check if a path is executable by the current user.
fn is_executable(path: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Filter for directory listing operations.
#[derive(Clone, Copy, PartialEq)]
enum ListFilter {
    All,
    FilesOnly,
    DirsOnly,
}

/// List directory contents with optional filtering.
/// Returns (entries, nil) on success, (nil, error_message) on failure.
fn list_directory(
    dir: &str,
    filter: ListFilter,
    pattern: Option<&str>,
) -> (Option<Vec<String>>, Option<String>) {
    let path = Path::new(dir);

    let read_dir = match std::fs::read_dir(path) {
        Ok(rd) => rd,
        Err(e) => {
            return (
                None,
                Some(format!("cannot read directory '{}': {}", dir, e)),
            )
        }
    };

    let mut entries: Vec<String> = Vec::new();

    for entry_result in read_dir {
        let entry = match entry_result {
            Ok(e) => e,
            Err(e) => return (None, Some(format!("error reading directory entry: {}", e))),
        };

        let file_name = entry.file_name();
        let name = file_name.to_string_lossy().to_string();

        // Apply filter based on entry type
        let dominated_by_filter = match filter {
            ListFilter::All => true,
            ListFilter::FilesOnly => {
                // Use entry.file_type() which doesn't follow symlinks for the check,
                // but we want is_file() semantics (follows symlinks)
                entry.path().is_file()
            }
            ListFilter::DirsOnly => entry.path().is_dir(),
        };

        if !dominated_by_filter {
            continue;
        }

        // Apply pattern filter if provided (simple substring match for non-Lua version)
        if let Some(pat) = pattern {
            if !name.contains(pat) {
                continue;
            }
        }

        entries.push(name);
    }

    // Sort for deterministic results
    entries.sort();

    (Some(entries), None)
}

/// List directory contents with Lua pattern filtering.
/// Uses Lua's string.match for pattern matching.
fn list_directory_with_lua_pattern(
    lua: &mlua::Lua,
    dir: &str,
    filter: ListFilter,
    pattern: Option<String>,
) -> (Option<Vec<String>>, Option<String>) {
    let path = Path::new(dir);

    let read_dir = match std::fs::read_dir(path) {
        Ok(rd) => rd,
        Err(e) => {
            return (
                None,
                Some(format!("cannot read directory '{}': {}", dir, e)),
            )
        }
    };

    // Get string.match function for Lua pattern matching
    let string_match: Option<mlua::Function> = pattern.as_ref().and_then(|_| {
        lua.globals()
            .get::<mlua::Table>("string")
            .ok()
            .and_then(|s| s.get::<mlua::Function>("match").ok())
    });

    let mut entries: Vec<String> = Vec::new();

    for entry_result in read_dir {
        let entry = match entry_result {
            Ok(e) => e,
            Err(e) => return (None, Some(format!("error reading directory entry: {}", e))),
        };

        let file_name = entry.file_name();
        let name = file_name.to_string_lossy().to_string();

        // Apply filter based on entry type
        let passes_filter = match filter {
            ListFilter::All => true,
            ListFilter::FilesOnly => entry.path().is_file(),
            ListFilter::DirsOnly => entry.path().is_dir(),
        };

        if !passes_filter {
            continue;
        }

        // Apply Lua pattern filter if provided
        if let (Some(pat), Some(ref match_fn)) = (&pattern, &string_match) {
            let matches: Option<String> = match_fn.call((name.clone(), pat.clone())).ok();
            if matches.is_none() {
                continue;
            }
        }

        entries.push(name);
    }

    // Sort for deterministic results
    entries.sort();

    (Some(entries), None)
}

/// Implementation for mkdir with options handling.
/// opts can be:
///   - nil: create single directory
///   - boolean: if true, create recursively
///   - table: { recursive = bool, mode = int }
fn mkdir_impl(path: &str, opts: Option<mlua::Value>) -> (bool, Option<String>) {
    let recursive = match opts {
        None => false,
        Some(mlua::Value::Boolean(b)) => b,
        Some(mlua::Value::Table(t)) => t.get::<bool>("recursive").unwrap_or(false),
        _ => false,
    };

    let result = if recursive {
        std::fs::create_dir_all(path)
    } else {
        std::fs::create_dir(path)
    };

    match result {
        Ok(()) => (true, None),
        Err(e) => (
            false,
            Some(format!("cannot create directory '{}': {}", path, e)),
        ),
    }
}

/// Implementation for remove with options handling.
/// opts: { recursive = bool }
fn remove_impl(path: &str, opts: Option<mlua::Table>) -> (bool, Option<String>) {
    let recursive = opts
        .and_then(|t| t.get::<bool>("recursive").ok())
        .unwrap_or(false);

    let p = Path::new(path);

    // Check if path exists first for better error messages
    if !p.exists() {
        return (
            false,
            Some(format!(
                "cannot remove '{}': No such file or directory",
                path
            )),
        );
    }

    let result = if p.is_dir() {
        if recursive {
            std::fs::remove_dir_all(path)
        } else {
            std::fs::remove_dir(path)
        }
    } else {
        std::fs::remove_file(path)
    };

    match result {
        Ok(()) => (true, None),
        Err(e) => (false, Some(format!("cannot remove '{}': {}", path, e))),
    }
}

/// Implementation for abspath: converts to absolute, resolves '.' and '..'
/// but does NOT follow symlinks. Returns nil, error if path doesn't exist.
fn abspath_impl(path: &str) -> (Option<String>, Option<String>) {
    // First expand ~ and environment variables
    let home_dir: Option<String> = dirs::home_dir().map(|p| p.to_string_lossy().into_owned());
    let expanded = shellexpand::full_with_context_no_errors(
        path,
        || home_dir.as_deref(),
        |var| Some(std::env::var(var).unwrap_or_default()),
    );

    let p = Path::new(expanded.as_ref());

    // Make absolute if relative
    let absolute = if p.is_absolute() {
        p.to_path_buf()
    } else {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(p),
            Err(e) => return (None, Some(format!("cannot get current directory: {}", e))),
        }
    };

    // Normalize by resolving . and .. without following symlinks
    let normalized = normalize_path_components(&absolute);

    // Check if path exists
    if !normalized.exists() {
        return (
            None,
            Some(format!(
                "path does not exist: '{}'",
                normalized.to_string_lossy()
            )),
        );
    }

    (Some(normalized.to_string_lossy().into_owned()), None)
}

/// Implementation for normalize: expands ~ and $VAR, resolves '.' and '..',
/// removes redundant separators. Does NOT require path to exist.
fn normalize_impl(path: &str) -> String {
    // Expand ~ and environment variables
    let home_dir: Option<String> = dirs::home_dir().map(|p| p.to_string_lossy().into_owned());
    let expanded = shellexpand::full_with_context_no_errors(
        path,
        || home_dir.as_deref(),
        |var| Some(std::env::var(var).unwrap_or_default()),
    );

    let p = Path::new(expanded.as_ref());

    // Normalize path components
    let normalized = normalize_path_components(p);

    normalized.to_string_lossy().into_owned()
}

/// Expand tilde in path (but not environment variables).
/// Used for file I/O functions.
fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") || path == "~" {
        if let Some(home) = dirs::home_dir() {
            if path == "~" {
                return home.to_string_lossy().into_owned();
            }
            return format!("{}{}", home.to_string_lossy(), &path[1..]);
        }
    }
    path.to_string()
}

/// Read entire file as UTF-8 string.
/// Returns (content, nil) on success, (nil, error_message) on failure.
fn read_file(path: &str) -> (Option<String>, Option<String>) {
    let expanded = expand_tilde(path);
    match std::fs::read_to_string(&expanded) {
        Ok(content) => (Some(content), None),
        Err(e) => (None, Some(format!("cannot read file '{}': {}", path, e))),
    }
}

/// Read file and return lines as a vector (without newline characters).
/// Returns (lines, nil) on success, (nil, error_message) on failure.
fn read_lines(path: &str) -> (Option<Vec<String>>, Option<String>) {
    let expanded = expand_tilde(path);
    match std::fs::read_to_string(&expanded) {
        Ok(content) => {
            // Split by lines, handling both \n and \r\n
            let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
            (Some(lines), None)
        }
        Err(e) => (None, Some(format!("cannot read file '{}': {}", path, e))),
    }
}

/// Write content to file, creating or overwriting.
/// Returns (true, nil) on success, (false, error_message) on failure.
fn write_file(path: &str, content: &str) -> (bool, Option<String>) {
    let expanded = expand_tilde(path);
    match std::fs::write(&expanded, content) {
        Ok(()) => (true, None),
        Err(e) => (false, Some(format!("cannot write file '{}': {}", path, e))),
    }
}

/// Append content to file, creating if it doesn't exist.
/// Returns (true, nil) on success, (false, error_message) on failure.
fn append_file(path: &str, content: &str) -> (bool, Option<String>) {
    use std::fs::OpenOptions;
    use std::io::Write;

    let expanded = expand_tilde(path);
    let result = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&expanded)
        .and_then(|mut file| file.write_all(content.as_bytes()));

    match result {
        Ok(()) => (true, None),
        Err(e) => (
            false,
            Some(format!("cannot append to file '{}': {}", path, e)),
        ),
    }
}

/// Copy a file from src to dst.
/// Returns (true, nil) on success, (false, error_message) on failure.
fn copy_file(src: &str, dst: &str) -> (bool, Option<String>) {
    let src_expanded = expand_tilde(src);
    let dst_expanded = expand_tilde(dst);

    // Check if source is a file (not a directory)
    let src_path = Path::new(&src_expanded);
    if !src_path.is_file() {
        return (
            false,
            Some(format!(
                "cannot copy '{}': not a file or does not exist",
                src
            )),
        );
    }

    match std::fs::copy(&src_expanded, &dst_expanded) {
        Ok(_) => (true, None),
        Err(e) => (
            false,
            Some(format!("cannot copy '{}' to '{}': {}", src, dst, e)),
        ),
    }
}

/// Rename/move a file or directory from src to dst.
/// Returns (true, nil) on success, (false, error_message) on failure.
fn rename_path(src: &str, dst: &str) -> (bool, Option<String>) {
    let src_expanded = expand_tilde(src);
    let dst_expanded = expand_tilde(dst);

    match std::fs::rename(&src_expanded, &dst_expanded) {
        Ok(()) => (true, None),
        Err(e) => (
            false,
            Some(format!("cannot rename '{}' to '{}': {}", src, dst, e)),
        ),
    }
}

/// Get file metadata and return as a Lua table.
/// Returns (table, nil) on success, (nil, error_message) on failure.
fn stat_impl(lua: &mlua::Lua, path: &str) -> mlua::Result<(Option<mlua::Table>, Option<String>)> {
    use std::time::UNIX_EPOCH;

    let expanded = expand_tilde(path);
    let metadata = match std::fs::metadata(&expanded) {
        Ok(m) => m,
        Err(e) => return Ok((None, Some(format!("cannot stat '{}': {}", path, e)))),
    };

    let table = lua.create_table()?;

    // Size in bytes
    table.set("size", metadata.len())?;

    // File type
    let file_type = if metadata.is_file() {
        "file"
    } else if metadata.is_dir() {
        "directory"
    } else {
        "other"
    };
    table.set("type", file_type)?;

    // Readonly
    table.set("readonly", metadata.permissions().readonly())?;

    // Modification time as Unix timestamp
    if let Ok(mtime) = metadata.modified() {
        if let Ok(duration) = mtime.duration_since(UNIX_EPOCH) {
            table.set("mtime", duration.as_secs())?;
        }
    }

    // Access time as Unix timestamp
    if let Ok(atime) = metadata.accessed() {
        if let Ok(duration) = atime.duration_since(UNIX_EPOCH) {
            table.set("atime", duration.as_secs())?;
        }
    }

    // Creation time (ctime) - platform dependent
    use std::os::unix::fs::MetadataExt;
    // On Unix, ctime is status change time (not creation time)
    table.set("ctime", metadata.ctime() as u64)?;
    // Unix mode (permissions)
    table.set("mode", metadata.mode())?;

    Ok((Some(table), None))
}

/// Get file modification time as Unix timestamp.
/// Returns (timestamp, nil) on success, (nil, error_message) on failure.
fn mtime_impl(path: &str) -> (Option<u64>, Option<String>) {
    use std::time::UNIX_EPOCH;

    let expanded = expand_tilde(path);
    match std::fs::metadata(&expanded) {
        Ok(metadata) => match metadata.modified() {
            Ok(mtime) => match mtime.duration_since(UNIX_EPOCH) {
                Ok(duration) => (Some(duration.as_secs()), None),
                Err(_) => (
                    None,
                    Some(format!(
                        "cannot get mtime for '{}': time before epoch",
                        path
                    )),
                ),
            },
            Err(e) => (
                None,
                Some(format!("cannot get mtime for '{}': {}", path, e)),
            ),
        },
        Err(e) => (None, Some(format!("cannot stat '{}': {}", path, e))),
    }
}

/// Get file size in bytes.
/// Returns (size, nil) on success, (nil, error_message) on failure.
fn size_impl(path: &str) -> (Option<u64>, Option<String>) {
    let expanded = expand_tilde(path);
    match std::fs::metadata(&expanded) {
        Ok(metadata) => (Some(metadata.len()), None),
        Err(e) => (None, Some(format!("cannot stat '{}': {}", path, e))),
    }
}

/// Glob for files matching a shell-style pattern.
/// Returns (paths, nil) on success, (nil, error_message) on failure.
/// Paths are returned as normalized full paths, sorted for determinism.
fn glob_impl(pattern: &str) -> (Option<Vec<String>>, Option<String>) {
    // Expand tilde in pattern (but not environment variables, per convention)
    let expanded = expand_tilde(pattern);

    // Parse the glob pattern
    let glob_pattern = match glob::glob(&expanded) {
        Ok(paths) => paths,
        Err(e) => {
            return (
                None,
                Some(format!("invalid glob pattern '{}': {}", pattern, e)),
            )
        }
    };

    // Collect all matching paths
    let mut results: Vec<String> = Vec::new();
    for entry in glob_pattern {
        match entry {
            Ok(path) => {
                // Normalize and convert to string
                let normalized = normalize_path_components(&path);
                results.push(normalized.to_string_lossy().into_owned());
            }
            Err(e) => {
                // GlobError contains path info - report but continue
                // This happens when we can't read a directory during traversal
                return (None, Some(format!("error reading path during glob: {}", e)));
            }
        }
    }

    // Sort for deterministic results
    results.sort();

    (Some(results), None)
}

/// Find options for find_impl.
#[derive(Default)]
struct FindOptions {
    /// Starting directory (defaults to current working directory)
    path: Option<String>,
    /// Search upward toward root instead of downward
    upward: bool,
    /// Stop upward search at this directory
    stop: Option<String>,
    /// Filter by type: "file", "directory", or None for any
    type_filter: Option<String>,
    /// Maximum number of results
    limit: Option<usize>,
}

/// Find files/directories by name with upward/downward search.
/// Returns (paths, nil) on success, (nil, error_message) on failure.
fn find_impl(names: Vec<String>, opts: FindOptions) -> (Option<Vec<String>>, Option<String>) {
    use std::path::PathBuf;

    if names.is_empty() {
        return (Some(Vec::new()), None);
    }

    // Determine starting directory
    let start_path = match &opts.path {
        Some(p) => {
            let expanded = expand_tilde(p);
            PathBuf::from(expanded)
        }
        None => match std::env::current_dir() {
            Ok(cwd) => cwd,
            Err(e) => return (None, Some(format!("cannot get current directory: {}", e))),
        },
    };

    // Ensure start path exists and is a directory
    if !start_path.is_dir() {
        return (
            None,
            Some(format!(
                "starting path is not a directory: '{}'",
                start_path.display()
            )),
        );
    }

    let mut results: Vec<String> = Vec::new();
    let limit = opts.limit.unwrap_or(usize::MAX);

    if opts.upward {
        // Upward search: walk from start_path toward root
        let stop_path = opts.stop.as_ref().map(|s| {
            let expanded = expand_tilde(s);
            PathBuf::from(expanded)
        });

        let mut current = start_path;
        loop {
            // Check if we've reached the stop directory
            if let Some(ref stop) = stop_path {
                if current == *stop {
                    break;
                }
            }

            // Check for each name in this directory
            for name in &names {
                let candidate = current.join(name);
                if candidate.exists() {
                    // Apply type filter if specified
                    let passes_filter = match opts.type_filter.as_deref() {
                        Some("file") => candidate.is_file(),
                        Some("directory") => candidate.is_dir(),
                        _ => true,
                    };

                    if passes_filter {
                        results.push(candidate.to_string_lossy().into_owned());
                        if results.len() >= limit {
                            return (Some(results), None);
                        }
                    }
                }
            }

            // Move to parent directory
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => break, // Reached root
            }
        }
    } else {
        // Downward search: recursive traversal from start_path
        fn search_downward(
            dir: &Path,
            names: &[String],
            type_filter: Option<&str>,
            results: &mut Vec<String>,
            limit: usize,
        ) -> std::result::Result<(), std::io::Error> {
            if results.len() >= limit {
                return Ok(());
            }

            let entries = std::fs::read_dir(dir)?;

            for entry_result in entries {
                let entry = entry_result?;
                let path = entry.path();
                let file_name = entry.file_name();
                let name_str = file_name.to_string_lossy();

                // Check if this entry matches any of the names
                for name in names {
                    // Support simple glob patterns (* and ?)
                    let matches = if name.contains('*') || name.contains('?') {
                        glob_match(name, &name_str)
                    } else {
                        name_str == *name
                    };

                    if matches {
                        // Apply type filter if specified
                        let passes_filter = match type_filter {
                            Some("file") => path.is_file(),
                            Some("directory") => path.is_dir(),
                            _ => true,
                        };

                        if passes_filter {
                            results.push(path.to_string_lossy().into_owned());
                            if results.len() >= limit {
                                return Ok(());
                            }
                        }
                    }
                }

                // Recurse into directories
                if path.is_dir() {
                    // Ignore errors in subdirectories (permission denied, etc.)
                    let _ = search_downward(&path, names, type_filter, results, limit);
                }
            }

            Ok(())
        }

        // Ignore top-level errors but return empty result
        let _ = search_downward(
            &start_path,
            &names,
            opts.type_filter.as_deref(),
            &mut results,
            limit,
        );
    }

    // Sort for deterministic results
    results.sort();

    (Some(results), None)
}

/// Simple glob pattern matching for find() function.
/// Supports * (any characters) and ? (single character).
fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();

    fn match_helper(pattern: &[char], text: &[char]) -> bool {
        match (pattern.first(), text.first()) {
            (None, None) => true,
            (Some('*'), _) => {
                // * matches zero or more characters
                // Try matching rest of pattern with current text
                match_helper(&pattern[1..], text)
                    // Or skip one character in text and try again with *
                    || (!text.is_empty() && match_helper(pattern, &text[1..]))
            }
            (Some('?'), Some(_)) => {
                // ? matches exactly one character
                match_helper(&pattern[1..], &text[1..])
            }
            (Some(p), Some(t)) if *p == *t => match_helper(&pattern[1..], &text[1..]),
            _ => false,
        }
    }

    match_helper(&pattern_chars, &text_chars)
}

/// Normalize path by resolving '.' and '..' components without following symlinks.
/// This is a pure path operation that doesn't touch the filesystem.
fn normalize_path_components(path: &Path) -> std::path::PathBuf {
    use std::path::Component;

    let mut result = std::path::PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(p) => result.push(p.as_os_str()),
            Component::RootDir => result.push(component.as_os_str()),
            Component::CurDir => {
                // Skip '.' - current directory reference
            }
            Component::ParentDir => {
                // Go up one level if possible, otherwise keep the '..'
                if result.file_name().is_some() {
                    result.pop();
                } else if !result.has_root() {
                    result.push("..");
                }
                // If we have a root but no file_name, we're at root - ignore the '..'
            }
            Component::Normal(c) => result.push(c),
        }
    }

    // Handle empty result (e.g., from normalizing ".")
    if result.as_os_str().is_empty() {
        result.push(".");
    }

    result
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use mlua::{Function, Lua, Table};
    use serial_test::serial;
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
    #[serial]
    fn which_finds_executable_in_path() {
        let dir = tempdir().unwrap();
        let bin = dir.path().join("test-exec");
        fs::write(&bin, b"dummy").unwrap();

        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&bin).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&bin, perms).unwrap();
        }

        let old_path = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", dir.path().to_str().unwrap(), old_path);
        std::env::set_var("PATH", &new_path);

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let which_fn: Function = fs_table.get("which").unwrap();
        let res: Option<String> = which_fn.call("test-exec").unwrap();
        assert_eq!(res.map(PathBuf::from), Some(bin));

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
        assert_eq!(res.map(PathBuf::from), Some(bin));
    }

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

    // =========================================================================
    // Path Query Function Tests
    // =========================================================================

    #[test]
    fn exists_true_for_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, b"test").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let exists_fn: Function = fs_table.get("exists").unwrap();
        assert!(exists_fn.call::<bool>(file_path.to_str().unwrap()).unwrap());
    }

    #[test]
    fn exists_true_for_dir() {
        let dir = tempdir().unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let exists_fn: Function = fs_table.get("exists").unwrap();
        assert!(exists_fn
            .call::<bool>(dir.path().to_str().unwrap())
            .unwrap());
    }

    #[test]
    fn exists_false_for_nonexistent() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let exists_fn: Function = fs_table.get("exists").unwrap();
        assert!(!exists_fn.call::<bool>("/nonexistent/path/xyz").unwrap());
    }

    #[test]
    fn is_file_true_for_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, b"test").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let is_file_fn: Function = fs_table.get("is_file").unwrap();
        assert!(is_file_fn
            .call::<bool>(file_path.to_str().unwrap())
            .unwrap());
    }

    #[test]
    fn is_file_false_for_dir() {
        let dir = tempdir().unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let is_file_fn: Function = fs_table.get("is_file").unwrap();
        assert!(!is_file_fn
            .call::<bool>(dir.path().to_str().unwrap())
            .unwrap());
    }

    #[test]
    fn is_file_false_for_nonexistent() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let is_file_fn: Function = fs_table.get("is_file").unwrap();
        assert!(!is_file_fn.call::<bool>("/nonexistent/path").unwrap());
    }

    #[test]
    fn is_dir_true_for_dir() {
        let dir = tempdir().unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let is_dir_fn: Function = fs_table.get("is_dir").unwrap();
        assert!(is_dir_fn
            .call::<bool>(dir.path().to_str().unwrap())
            .unwrap());
    }

    #[test]
    fn is_dir_false_for_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, b"test").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let is_dir_fn: Function = fs_table.get("is_dir").unwrap();
        assert!(!is_dir_fn.call::<bool>(file_path.to_str().unwrap()).unwrap());
    }

    #[test]
    fn is_dir_false_for_nonexistent() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let is_dir_fn: Function = fs_table.get("is_dir").unwrap();
        assert!(!is_dir_fn.call::<bool>("/nonexistent/path").unwrap());
    }

    #[test]
    fn is_symlink_true_for_symlink() {
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
        let is_symlink_fn: Function = fs_table.get("is_symlink").unwrap();
        assert!(is_symlink_fn.call::<bool>(link.to_str().unwrap()).unwrap());
    }

    #[test]
    fn is_symlink_false_for_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, b"test").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let is_symlink_fn: Function = fs_table.get("is_symlink").unwrap();
        assert!(!is_symlink_fn
            .call::<bool>(file_path.to_str().unwrap())
            .unwrap());
    }

    #[test]
    fn is_symlink_false_for_nonexistent() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let is_symlink_fn: Function = fs_table.get("is_symlink").unwrap();
        assert!(!is_symlink_fn.call::<bool>("/nonexistent/path").unwrap());
    }

    #[test]
    fn is_executable_true_for_executable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let bin = dir.path().join("test-exec");
        fs::write(&bin, b"dummy").unwrap();

        let mut perms = fs::metadata(&bin).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin, perms).unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let is_exec_fn: Function = fs_table.get("is_executable").unwrap();
        assert!(is_exec_fn.call::<bool>(bin.to_str().unwrap()).unwrap());
    }

    #[test]
    fn is_executable_false_for_non_executable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let file = dir.path().join("not-exec");
        fs::write(&file, b"dummy").unwrap();

        let mut perms = fs::metadata(&file).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&file, perms).unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let is_exec_fn: Function = fs_table.get("is_executable").unwrap();
        assert!(!is_exec_fn.call::<bool>(file.to_str().unwrap()).unwrap());
    }

    #[test]
    fn is_executable_false_for_nonexistent() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let is_exec_fn: Function = fs_table.get("is_executable").unwrap();
        assert!(!is_exec_fn.call::<bool>("/nonexistent/path").unwrap());
    }

    #[test]
    fn is_file_follows_symlink() {
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
        let is_file_fn: Function = fs_table.get("is_file").unwrap();

        // is_file should follow symlink and return true for the file
        assert!(is_file_fn.call::<bool>(link.to_str().unwrap()).unwrap());
    }

    #[test]
    fn exists_false_for_broken_symlink() {
        use std::os::unix::fs::symlink;
        let dir = tempdir().unwrap();
        let target = dir.path().join("target.txt");
        let link = dir.path().join("link.txt");

        fs::write(&target, b"data").unwrap();
        symlink(&target, &link).unwrap();
        fs::remove_file(&target).unwrap(); // Break the symlink

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let exists_fn: Function = fs_table.get("exists").unwrap();

        // exists follows symlinks, so broken symlink returns false
        assert!(!exists_fn.call::<bool>(link.to_str().unwrap()).unwrap());
    }

    #[test]
    fn is_symlink_true_for_broken_symlink() {
        use std::os::unix::fs::symlink;
        let dir = tempdir().unwrap();
        let target = dir.path().join("target.txt");
        let link = dir.path().join("link.txt");

        fs::write(&target, b"data").unwrap();
        symlink(&target, &link).unwrap();
        fs::remove_file(&target).unwrap(); // Break the symlink

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let is_symlink_fn: Function = fs_table.get("is_symlink").unwrap();

        // is_symlink does NOT follow, so broken symlink still returns true
        assert!(is_symlink_fn.call::<bool>(link.to_str().unwrap()).unwrap());
    }

    // =========================================================================
    // Path Manipulation Function Tests
    // =========================================================================

    #[test]
    fn basename_extracts_filename() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let basename_fn: Function = fs_table.get("basename").unwrap();

        assert_eq!(
            basename_fn.call::<String>("/home/user/file.txt").unwrap(),
            "file.txt"
        );
    }

    #[test]
    fn basename_handles_trailing_slash() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let basename_fn: Function = fs_table.get("basename").unwrap();

        assert_eq!(basename_fn.call::<String>("/home/user/").unwrap(), "user");
    }

    #[test]
    fn basename_simple_filename() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let basename_fn: Function = fs_table.get("basename").unwrap();

        assert_eq!(basename_fn.call::<String>("file.txt").unwrap(), "file.txt");
    }

    #[test]
    fn basename_empty_string() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let basename_fn: Function = fs_table.get("basename").unwrap();

        assert_eq!(basename_fn.call::<String>("").unwrap(), "");
    }

    #[test]
    fn dirname_extracts_directory() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let dirname_fn: Function = fs_table.get("dirname").unwrap();

        assert_eq!(
            dirname_fn.call::<String>("/home/user/file.txt").unwrap(),
            "/home/user"
        );
    }

    #[test]
    fn dirname_handles_trailing_slash() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let dirname_fn: Function = fs_table.get("dirname").unwrap();

        assert_eq!(dirname_fn.call::<String>("/home/user/").unwrap(), "/home");
    }

    #[test]
    fn dirname_simple_filename_returns_dot() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let dirname_fn: Function = fs_table.get("dirname").unwrap();

        assert_eq!(dirname_fn.call::<String>("file.txt").unwrap(), ".");
    }

    #[test]
    fn dirname_root_path() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let dirname_fn: Function = fs_table.get("dirname").unwrap();

        assert_eq!(dirname_fn.call::<String>("/").unwrap(), "/");
    }

    #[test]
    fn extname_extracts_extension() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let extname_fn: Function = fs_table.get("extname").unwrap();

        assert_eq!(extname_fn.call::<String>("file.txt").unwrap(), ".txt");
    }

    #[test]
    fn extname_last_extension_only() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let extname_fn: Function = fs_table.get("extname").unwrap();

        assert_eq!(extname_fn.call::<String>("file.tar.gz").unwrap(), ".gz");
    }

    #[test]
    fn extname_no_extension() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let extname_fn: Function = fs_table.get("extname").unwrap();

        assert_eq!(extname_fn.call::<String>("file").unwrap(), "");
    }

    #[test]
    fn extname_dotfile_no_extension() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let extname_fn: Function = fs_table.get("extname").unwrap();

        // .bashrc is a dotfile, not a file with extension "bashrc"
        assert_eq!(extname_fn.call::<String>(".bashrc").unwrap(), "");
    }

    #[test]
    fn joinpath_combines_paths() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let joinpath_fn: Function = fs_table.get("joinpath").unwrap();

        let result: String = joinpath_fn.call(("/home", "user", "file.txt")).unwrap();
        assert_eq!(result, "/home/user/file.txt");
    }

    #[test]
    fn joinpath_absolute_resets() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let joinpath_fn: Function = fs_table.get("joinpath").unwrap();

        let result: String = joinpath_fn.call(("dir", "/absolute")).unwrap();
        assert_eq!(result, "/absolute");
    }

    #[test]
    fn joinpath_skips_empty() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let joinpath_fn: Function = fs_table.get("joinpath").unwrap();

        let result: String = joinpath_fn.call(("a", "", "b")).unwrap();
        assert_eq!(result, "a/b");
    }

    #[test]
    fn joinpath_single_component() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let joinpath_fn: Function = fs_table.get("joinpath").unwrap();

        let result: String = joinpath_fn.call(("single",)).unwrap();
        assert_eq!(result, "single");
    }

    #[test]
    fn joinpath_no_components() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let joinpath_fn: Function = fs_table.get("joinpath").unwrap();

        let result: String = joinpath_fn.call(()).unwrap();
        assert_eq!(result, "");
    }

    // =========================================================================
    // Directory Listing Function Tests
    // =========================================================================

    #[test]
    fn list_returns_all_entries() {
        let dir = tempdir().unwrap();
        // Create some files and directories
        fs::write(dir.path().join("file1.txt"), b"").unwrap();
        fs::write(dir.path().join("file2.txt"), b"").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let list_fn: Function = fs_table.get("list").unwrap();

        let (entries, err): (Option<Vec<String>>, Option<String>) =
            list_fn.call(dir.path().to_str().unwrap()).unwrap();

        assert!(err.is_none());
        let entries = entries.unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries.contains(&"file1.txt".to_string()));
        assert!(entries.contains(&"file2.txt".to_string()));
        assert!(entries.contains(&"subdir".to_string()));
    }

    #[test]
    fn list_returns_sorted_entries() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("zebra.txt"), b"").unwrap();
        fs::write(dir.path().join("apple.txt"), b"").unwrap();
        fs::write(dir.path().join("mango.txt"), b"").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let list_fn: Function = fs_table.get("list").unwrap();

        let (entries, _): (Option<Vec<String>>, Option<String>) =
            list_fn.call(dir.path().to_str().unwrap()).unwrap();

        let entries = entries.unwrap();
        assert_eq!(entries, vec!["apple.txt", "mango.txt", "zebra.txt"]);
    }

    #[test]
    fn list_empty_dir() {
        let dir = tempdir().unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let list_fn: Function = fs_table.get("list").unwrap();

        let (entries, err): (Option<Vec<String>>, Option<String>) =
            list_fn.call(dir.path().to_str().unwrap()).unwrap();

        assert!(err.is_none());
        assert_eq!(entries.unwrap().len(), 0);
    }

    #[test]
    fn list_nonexistent_dir_returns_error() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let list_fn: Function = fs_table.get("list").unwrap();

        let (entries, err): (Option<Vec<String>>, Option<String>) =
            list_fn.call("/nonexistent/path/xyz").unwrap();

        assert!(entries.is_none());
        assert!(err.is_some());
        assert!(err.unwrap().contains("cannot read directory"));
    }

    #[test]
    fn list_files_returns_only_files() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("file1.txt"), b"").unwrap();
        fs::write(dir.path().join("file2.txt"), b"").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let list_files_fn: Function = fs_table.get("list_files").unwrap();

        let (entries, err): (Option<Vec<String>>, Option<String>) =
            list_files_fn.call(dir.path().to_str().unwrap()).unwrap();

        assert!(err.is_none());
        let entries = entries.unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.contains(&"file1.txt".to_string()));
        assert!(entries.contains(&"file2.txt".to_string()));
        assert!(!entries.contains(&"subdir".to_string()));
    }

    #[test]
    fn list_files_with_lua_pattern() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("file1.lua"), b"").unwrap();
        fs::write(dir.path().join("file2.lua"), b"").unwrap();
        fs::write(dir.path().join("file3.txt"), b"").unwrap();
        fs::write(dir.path().join("config.lua"), b"").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let list_files_fn: Function = fs_table.get("list_files").unwrap();

        // Filter with Lua pattern "%.lua$" (files ending in .lua)
        let (entries, err): (Option<Vec<String>>, Option<String>) = list_files_fn
            .call((dir.path().to_str().unwrap(), "%.lua$"))
            .unwrap();

        assert!(err.is_none());
        let entries = entries.unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries.contains(&"file1.lua".to_string()));
        assert!(entries.contains(&"file2.lua".to_string()));
        assert!(entries.contains(&"config.lua".to_string()));
        assert!(!entries.contains(&"file3.txt".to_string()));
    }

    #[test]
    fn list_files_with_pattern_no_matches() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("file1.txt"), b"").unwrap();
        fs::write(dir.path().join("file2.txt"), b"").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let list_files_fn: Function = fs_table.get("list_files").unwrap();

        // No .lua files exist
        let (entries, err): (Option<Vec<String>>, Option<String>) = list_files_fn
            .call((dir.path().to_str().unwrap(), "%.lua$"))
            .unwrap();

        assert!(err.is_none());
        assert_eq!(entries.unwrap().len(), 0);
    }

    #[test]
    fn list_files_nonexistent_dir_returns_error() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let list_files_fn: Function = fs_table.get("list_files").unwrap();

        let (entries, err): (Option<Vec<String>>, Option<String>) =
            list_files_fn.call("/nonexistent/path/xyz").unwrap();

        assert!(entries.is_none());
        assert!(err.is_some());
    }

    #[test]
    fn list_dirs_returns_only_directories() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("file1.txt"), b"").unwrap();
        fs::create_dir(dir.path().join("subdir1")).unwrap();
        fs::create_dir(dir.path().join("subdir2")).unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let list_dirs_fn: Function = fs_table.get("list_dirs").unwrap();

        let (entries, err): (Option<Vec<String>>, Option<String>) =
            list_dirs_fn.call(dir.path().to_str().unwrap()).unwrap();

        assert!(err.is_none());
        let entries = entries.unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.contains(&"subdir1".to_string()));
        assert!(entries.contains(&"subdir2".to_string()));
        assert!(!entries.contains(&"file1.txt".to_string()));
    }

    #[test]
    fn list_dirs_empty_when_no_subdirs() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("file1.txt"), b"").unwrap();
        fs::write(dir.path().join("file2.txt"), b"").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let list_dirs_fn: Function = fs_table.get("list_dirs").unwrap();

        let (entries, err): (Option<Vec<String>>, Option<String>) =
            list_dirs_fn.call(dir.path().to_str().unwrap()).unwrap();

        assert!(err.is_none());
        assert_eq!(entries.unwrap().len(), 0);
    }

    #[test]
    fn list_dirs_nonexistent_dir_returns_error() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let list_dirs_fn: Function = fs_table.get("list_dirs").unwrap();

        let (entries, err): (Option<Vec<String>>, Option<String>) =
            list_dirs_fn.call("/nonexistent/path/xyz").unwrap();

        assert!(entries.is_none());
        assert!(err.is_some());
    }

    #[test]
    fn list_files_follows_symlinks_to_files() {
        use std::os::unix::fs::symlink;
        let dir = tempdir().unwrap();

        // Create a file and a symlink to it
        let target = dir.path().join("target.txt");
        fs::write(&target, b"data").unwrap();
        symlink(&target, dir.path().join("link.txt")).unwrap();

        // Create a directory and a symlink to it
        fs::create_dir(dir.path().join("subdir")).unwrap();
        symlink(dir.path().join("subdir"), dir.path().join("dirlink")).unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let list_files_fn: Function = fs_table.get("list_files").unwrap();

        let (entries, _): (Option<Vec<String>>, Option<String>) =
            list_files_fn.call(dir.path().to_str().unwrap()).unwrap();

        let entries = entries.unwrap();
        // Should include both the file and the symlink to file
        assert!(entries.contains(&"target.txt".to_string()));
        assert!(entries.contains(&"link.txt".to_string()));
        // Should not include directory symlink
        assert!(!entries.contains(&"dirlink".to_string()));
    }

    #[test]
    fn list_dirs_follows_symlinks_to_dirs() {
        use std::os::unix::fs::symlink;
        let dir = tempdir().unwrap();

        // Create a directory and a symlink to it
        fs::create_dir(dir.path().join("subdir")).unwrap();
        symlink(dir.path().join("subdir"), dir.path().join("dirlink")).unwrap();

        // Create a file symlink
        let target = dir.path().join("target.txt");
        fs::write(&target, b"data").unwrap();
        symlink(&target, dir.path().join("filelink")).unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let list_dirs_fn: Function = fs_table.get("list_dirs").unwrap();

        let (entries, _): (Option<Vec<String>>, Option<String>) =
            list_dirs_fn.call(dir.path().to_str().unwrap()).unwrap();

        let entries = entries.unwrap();
        // Should include both the directory and the symlink to directory
        assert!(entries.contains(&"subdir".to_string()));
        assert!(entries.contains(&"dirlink".to_string()));
        // Should not include file symlink
        assert!(!entries.contains(&"filelink".to_string()));
    }

    // =========================================================================
    // Directory Mutation Function Tests (mkdir, rmdir, remove)
    // =========================================================================

    #[test]
    fn mkdir_creates_directory() {
        let dir = tempdir().unwrap();
        let new_dir = dir.path().join("newdir");

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let mkdir_fn: Function = fs_table.get("mkdir").unwrap();

        let (ok, err): (bool, Option<String>) = mkdir_fn.call(new_dir.to_str().unwrap()).unwrap();

        assert!(ok);
        assert!(err.is_none());
        assert!(new_dir.is_dir());
    }

    #[test]
    fn mkdir_fails_for_existing_dir() {
        let dir = tempdir().unwrap();
        // Directory already exists

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let mkdir_fn: Function = fs_table.get("mkdir").unwrap();

        let (ok, err): (bool, Option<String>) =
            mkdir_fn.call(dir.path().to_str().unwrap()).unwrap();

        assert!(!ok);
        assert!(err.is_some());
        assert!(err.unwrap().contains("cannot create directory"));
    }

    #[test]
    fn mkdir_fails_without_recursive_for_nested() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("a/b/c");

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let mkdir_fn: Function = fs_table.get("mkdir").unwrap();

        // Without recursive option, nested path should fail
        let (ok, err): (bool, Option<String>) = mkdir_fn.call(nested.to_str().unwrap()).unwrap();

        assert!(!ok);
        assert!(err.is_some());
    }

    #[test]
    fn mkdir_recursive_with_boolean() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("a/b/c");

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let mkdir_fn: Function = fs_table.get("mkdir").unwrap();

        // Pass true as second argument for recursive
        let (ok, err): (bool, Option<String>) =
            mkdir_fn.call((nested.to_str().unwrap(), true)).unwrap();

        assert!(ok);
        assert!(err.is_none());
        assert!(nested.is_dir());
    }

    #[test]
    fn mkdir_recursive_with_table() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("x/y/z");

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        // Pass table with recursive = true
        let result: (bool, Option<String>) = lua
            .load(format!(
                r#"
                local niri = ...
                return niri.fs.mkdir("{}", {{ recursive = true }})
            "#,
                nested.to_str().unwrap().replace('\\', "\\\\")
            ))
            .call(niri)
            .unwrap();

        assert!(result.0);
        assert!(result.1.is_none());
        assert!(nested.is_dir());
    }

    #[test]
    fn rmdir_removes_empty_directory() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let rmdir_fn: Function = fs_table.get("rmdir").unwrap();

        let (ok, err): (bool, Option<String>) = rmdir_fn.call(subdir.to_str().unwrap()).unwrap();

        assert!(ok);
        assert!(err.is_none());
        assert!(!subdir.exists());
    }

    #[test]
    fn rmdir_fails_for_nonempty_directory() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("file.txt"), b"data").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let rmdir_fn: Function = fs_table.get("rmdir").unwrap();

        let (ok, err): (bool, Option<String>) = rmdir_fn.call(subdir.to_str().unwrap()).unwrap();

        assert!(!ok);
        assert!(err.is_some());
        assert!(subdir.exists()); // Should still exist
    }

    #[test]
    fn rmdir_fails_for_nonexistent() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let rmdir_fn: Function = fs_table.get("rmdir").unwrap();

        let (ok, err): (bool, Option<String>) = rmdir_fn.call("/nonexistent/path/xyz").unwrap();

        assert!(!ok);
        assert!(err.is_some());
    }

    #[test]
    fn remove_removes_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("file.txt");
        fs::write(&file, b"data").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let remove_fn: Function = fs_table.get("remove").unwrap();

        let (ok, err): (bool, Option<String>) = remove_fn.call(file.to_str().unwrap()).unwrap();

        assert!(ok);
        assert!(err.is_none());
        assert!(!file.exists());
    }

    #[test]
    fn remove_removes_empty_directory() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let remove_fn: Function = fs_table.get("remove").unwrap();

        let (ok, err): (bool, Option<String>) = remove_fn.call(subdir.to_str().unwrap()).unwrap();

        assert!(ok);
        assert!(err.is_none());
        assert!(!subdir.exists());
    }

    #[test]
    fn remove_fails_for_nonempty_without_recursive() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("file.txt"), b"data").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let remove_fn: Function = fs_table.get("remove").unwrap();

        let (ok, err): (bool, Option<String>) = remove_fn.call(subdir.to_str().unwrap()).unwrap();

        assert!(!ok);
        assert!(err.is_some());
        assert!(subdir.exists());
    }

    #[test]
    fn remove_recursive_removes_nonempty_directory() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("file.txt"), b"data").unwrap();
        fs::create_dir(subdir.join("nested")).unwrap();
        fs::write(subdir.join("nested/inner.txt"), b"more").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();

        // Use Lua to call with opts table
        let result: (bool, Option<String>) = lua
            .load(format!(
                r#"
                local niri = ...
                return niri.fs.remove("{}", {{ recursive = true }})
            "#,
                subdir.to_str().unwrap().replace('\\', "\\\\")
            ))
            .call(niri)
            .unwrap();

        assert!(result.0);
        assert!(result.1.is_none());
        assert!(!subdir.exists());
    }

    #[test]
    fn remove_fails_for_nonexistent() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let remove_fn: Function = fs_table.get("remove").unwrap();

        let (ok, err): (bool, Option<String>) = remove_fn.call("/nonexistent/path/xyz").unwrap();

        assert!(!ok);
        assert!(err.is_some());
        assert!(err.unwrap().contains("No such file or directory"));
    }

    // =========================================================================
    // Path Utility Function Tests (abspath, normalize)
    // =========================================================================

    #[test]
    fn abspath_returns_absolute_for_existing_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("file.txt");
        fs::write(&file, b"data").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let abspath_fn: Function = fs_table.get("abspath").unwrap();

        let (result, err): (Option<String>, Option<String>) =
            abspath_fn.call(file.to_str().unwrap()).unwrap();

        assert!(err.is_none());
        let result = result.unwrap();
        assert!(Path::new(&result).is_absolute());
        assert!(result.ends_with("file.txt"));
    }

    #[test]
    fn abspath_returns_error_for_nonexistent() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let abspath_fn: Function = fs_table.get("abspath").unwrap();

        let (result, err): (Option<String>, Option<String>) =
            abspath_fn.call("/nonexistent/path/xyz").unwrap();

        assert!(result.is_none());
        assert!(err.is_some());
        assert!(err.unwrap().contains("does not exist"));
    }

    #[test]
    fn abspath_resolves_dot_and_dotdot() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let abspath_fn: Function = fs_table.get("abspath").unwrap();

        // Path with . and ..
        let path_with_dots = format!("{}/./subdir/../subdir", dir.path().to_str().unwrap());
        let (result, err): (Option<String>, Option<String>) =
            abspath_fn.call(path_with_dots).unwrap();

        assert!(err.is_none());
        let result = result.unwrap();
        // Should resolve to the subdir without . and ..
        assert!(result.ends_with("subdir"));
        assert!(!result.contains("./"));
        assert!(!result.contains(".."));
    }

    #[test]
    fn abspath_expands_tilde() {
        // This test only works if home dir exists and has some content
        if let Some(home) = dirs::home_dir() {
            if home.exists() {
                let lua = Lua::new();
                let niri = lua.create_table().unwrap();
                register(&lua, &niri).unwrap();
                let fs_table: Table = niri.get("fs").unwrap();
                let abspath_fn: Function = fs_table.get("abspath").unwrap();

                let (result, err): (Option<String>, Option<String>) = abspath_fn.call("~").unwrap();

                assert!(err.is_none());
                let result = result.unwrap();
                assert_eq!(result, home.to_string_lossy());
            }
        }
    }

    #[test]
    fn normalize_resolves_dot_and_dotdot() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let normalize_fn: Function = fs_table.get("normalize").unwrap();

        let result: String = normalize_fn.call("/a/b/./c/../d").unwrap();

        assert_eq!(result, "/a/b/d");
    }

    #[test]
    fn normalize_handles_relative_paths() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let normalize_fn: Function = fs_table.get("normalize").unwrap();

        let result: String = normalize_fn.call("a/b/../c").unwrap();

        assert_eq!(result, "a/c");
    }

    #[test]
    fn normalize_expands_tilde() {
        if let Some(home) = dirs::home_dir() {
            let lua = Lua::new();
            let niri = lua.create_table().unwrap();
            register(&lua, &niri).unwrap();
            let fs_table: Table = niri.get("fs").unwrap();
            let normalize_fn: Function = fs_table.get("normalize").unwrap();

            let result: String = normalize_fn.call("~/some/path").unwrap();

            let expected = format!("{}/some/path", home.to_string_lossy());
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn normalize_handles_current_dir() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let normalize_fn: Function = fs_table.get("normalize").unwrap();

        let result: String = normalize_fn.call(".").unwrap();

        assert_eq!(result, ".");
    }

    #[test]
    fn normalize_handles_parent_at_root() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let normalize_fn: Function = fs_table.get("normalize").unwrap();

        // Going up past root should stay at root
        let result: String = normalize_fn.call("/a/../../../b").unwrap();

        assert_eq!(result, "/b");
    }

    #[test]
    fn normalize_removes_redundant_separators() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let normalize_fn: Function = fs_table.get("normalize").unwrap();

        // Multiple slashes should be normalized
        let result: String = normalize_fn.call("/a//b///c").unwrap();

        assert_eq!(result, "/a/b/c");
    }

    #[test]
    fn normalize_does_not_require_path_to_exist() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let normalize_fn: Function = fs_table.get("normalize").unwrap();

        // This path doesn't exist, but normalize should still work
        let result: String = normalize_fn.call("/nonexistent/path/./to/../file").unwrap();

        assert_eq!(result, "/nonexistent/path/file");
    }

    // =========================================================================
    // File Reading Function Tests (read, readlines)
    // =========================================================================

    #[test]
    fn read_returns_file_content() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello world").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let read_fn: Function = fs_table.get("read").unwrap();

        let (content, err): (Option<String>, Option<String>) =
            read_fn.call(file.to_str().unwrap()).unwrap();

        assert!(err.is_none());
        assert_eq!(content.unwrap(), "hello world");
    }

    #[test]
    fn read_returns_empty_string_for_empty_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("empty.txt");
        fs::write(&file, "").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let read_fn: Function = fs_table.get("read").unwrap();

        let (content, err): (Option<String>, Option<String>) =
            read_fn.call(file.to_str().unwrap()).unwrap();

        assert!(err.is_none());
        assert_eq!(content.unwrap(), "");
    }

    #[test]
    fn read_returns_error_for_nonexistent_file() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let read_fn: Function = fs_table.get("read").unwrap();

        let (content, err): (Option<String>, Option<String>) =
            read_fn.call("/nonexistent/path/file.txt").unwrap();

        assert!(content.is_none());
        assert!(err.is_some());
        assert!(err.unwrap().contains("cannot read file"));
    }

    #[test]
    fn read_expands_tilde() {
        // Create a file in a temp location and test tilde expansion conceptually
        // We test that tilde expansion works by checking it doesn't fail on valid home paths
        if dirs::home_dir().is_some() {
            // We can't write to home, but we can verify tilde expansion by reading a nonexistent
            // file and checking the error message doesn't contain the literal ~
            let lua = Lua::new();
            let niri = lua.create_table().unwrap();
            register(&lua, &niri).unwrap();
            let fs_table: Table = niri.get("fs").unwrap();
            let read_fn: Function = fs_table.get("read").unwrap();

            let (content, err): (Option<String>, Option<String>) =
                read_fn.call("~/.niri_test_nonexistent_12345").unwrap();

            assert!(content.is_none());
            assert!(err.is_some());
            // The error should reference the expanded path, not the literal ~
            let err_msg = err.unwrap();
            // It should still mention the original path for user clarity
            assert!(err_msg.contains("cannot read file"));
        }
    }

    #[test]
    fn read_preserves_newlines() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("multiline.txt");
        fs::write(&file, "line1\nline2\nline3").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let read_fn: Function = fs_table.get("read").unwrap();

        let (content, err): (Option<String>, Option<String>) =
            read_fn.call(file.to_str().unwrap()).unwrap();

        assert!(err.is_none());
        assert_eq!(content.unwrap(), "line1\nline2\nline3");
    }

    #[test]
    fn readlines_returns_lines_array() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("lines.txt");
        fs::write(&file, "line1\nline2\nline3").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let readlines_fn: Function = fs_table.get("readlines").unwrap();

        let (lines, err): (Option<Vec<String>>, Option<String>) =
            readlines_fn.call(file.to_str().unwrap()).unwrap();

        assert!(err.is_none());
        let lines = lines.unwrap();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "line1");
        assert_eq!(lines[1], "line2");
        assert_eq!(lines[2], "line3");
    }

    #[test]
    fn readlines_handles_crlf() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("crlf.txt");
        fs::write(&file, "line1\r\nline2\r\nline3").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let readlines_fn: Function = fs_table.get("readlines").unwrap();

        let (lines, err): (Option<Vec<String>>, Option<String>) =
            readlines_fn.call(file.to_str().unwrap()).unwrap();

        assert!(err.is_none());
        let lines = lines.unwrap();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "line1");
        assert_eq!(lines[1], "line2");
        assert_eq!(lines[2], "line3");
    }

    #[test]
    fn readlines_empty_file_returns_empty_array() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("empty.txt");
        fs::write(&file, "").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let readlines_fn: Function = fs_table.get("readlines").unwrap();

        let (lines, err): (Option<Vec<String>>, Option<String>) =
            readlines_fn.call(file.to_str().unwrap()).unwrap();

        assert!(err.is_none());
        assert_eq!(lines.unwrap().len(), 0);
    }

    #[test]
    fn readlines_single_line_no_newline() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("single.txt");
        fs::write(&file, "single line").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let readlines_fn: Function = fs_table.get("readlines").unwrap();

        let (lines, err): (Option<Vec<String>>, Option<String>) =
            readlines_fn.call(file.to_str().unwrap()).unwrap();

        assert!(err.is_none());
        let lines = lines.unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "single line");
    }

    #[test]
    fn readlines_returns_error_for_nonexistent() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let readlines_fn: Function = fs_table.get("readlines").unwrap();

        let (lines, err): (Option<Vec<String>>, Option<String>) =
            readlines_fn.call("/nonexistent/path/file.txt").unwrap();

        assert!(lines.is_none());
        assert!(err.is_some());
        assert!(err.unwrap().contains("cannot read file"));
    }

    #[test]
    fn readlines_trailing_newline() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("trailing.txt");
        fs::write(&file, "line1\nline2\n").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let readlines_fn: Function = fs_table.get("readlines").unwrap();

        let (lines, err): (Option<Vec<String>>, Option<String>) =
            readlines_fn.call(file.to_str().unwrap()).unwrap();

        assert!(err.is_none());
        let lines = lines.unwrap();
        // str.lines() does not include an empty line for trailing newline
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "line1");
        assert_eq!(lines[1], "line2");
    }

    // =========================================================================
    // File Writing Function Tests (write, append)
    // =========================================================================

    #[test]
    fn write_creates_new_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("new.txt");

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let write_fn: Function = fs_table.get("write").unwrap();

        let (success, err): (bool, Option<String>) = write_fn
            .call((file.to_str().unwrap(), "hello world"))
            .unwrap();

        assert!(success);
        assert!(err.is_none());
        assert_eq!(fs::read_to_string(&file).unwrap(), "hello world");
    }

    #[test]
    fn write_overwrites_existing_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("existing.txt");
        fs::write(&file, "old content").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let write_fn: Function = fs_table.get("write").unwrap();

        let (success, err): (bool, Option<String>) = write_fn
            .call((file.to_str().unwrap(), "new content"))
            .unwrap();

        assert!(success);
        assert!(err.is_none());
        assert_eq!(fs::read_to_string(&file).unwrap(), "new content");
    }

    #[test]
    fn write_returns_error_for_nonexistent_parent() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let write_fn: Function = fs_table.get("write").unwrap();

        let (success, err): (bool, Option<String>) = write_fn
            .call(("/nonexistent/parent/dir/file.txt", "content"))
            .unwrap();

        assert!(!success);
        assert!(err.is_some());
        assert!(err.unwrap().contains("cannot write file"));
    }

    #[test]
    fn write_empty_content() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("empty.txt");

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let write_fn: Function = fs_table.get("write").unwrap();

        let (success, err): (bool, Option<String>) =
            write_fn.call((file.to_str().unwrap(), "")).unwrap();

        assert!(success);
        assert!(err.is_none());
        assert_eq!(fs::read_to_string(&file).unwrap(), "");
    }

    #[test]
    fn write_and_read_roundtrip() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("roundtrip.txt");
        let content = "line1\nline2\nline3";

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let write_fn: Function = fs_table.get("write").unwrap();
        let read_fn: Function = fs_table.get("read").unwrap();

        // Write
        let (success, _): (bool, Option<String>) =
            write_fn.call((file.to_str().unwrap(), content)).unwrap();
        assert!(success);

        // Read back
        let (read_content, err): (Option<String>, Option<String>) =
            read_fn.call(file.to_str().unwrap()).unwrap();
        assert!(err.is_none());
        assert_eq!(read_content.unwrap(), content);
    }

    #[test]
    fn append_creates_new_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("new_append.txt");

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let append_fn: Function = fs_table.get("append").unwrap();

        let (success, err): (bool, Option<String>) =
            append_fn.call((file.to_str().unwrap(), "first")).unwrap();

        assert!(success);
        assert!(err.is_none());
        assert_eq!(fs::read_to_string(&file).unwrap(), "first");
    }

    #[test]
    fn append_adds_to_existing_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("append.txt");
        fs::write(&file, "start").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let append_fn: Function = fs_table.get("append").unwrap();

        let (success, err): (bool, Option<String>) =
            append_fn.call((file.to_str().unwrap(), "-end")).unwrap();

        assert!(success);
        assert!(err.is_none());
        assert_eq!(fs::read_to_string(&file).unwrap(), "start-end");
    }

    #[test]
    fn append_multiple_times() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("multi_append.txt");

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let append_fn: Function = fs_table.get("append").unwrap();

        append_fn
            .call::<(bool, Option<String>)>((file.to_str().unwrap(), "one"))
            .unwrap();
        append_fn
            .call::<(bool, Option<String>)>((file.to_str().unwrap(), "\ntwo"))
            .unwrap();
        append_fn
            .call::<(bool, Option<String>)>((file.to_str().unwrap(), "\nthree"))
            .unwrap();

        assert_eq!(fs::read_to_string(&file).unwrap(), "one\ntwo\nthree");
    }

    #[test]
    fn append_returns_error_for_nonexistent_parent() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let append_fn: Function = fs_table.get("append").unwrap();

        let (success, err): (bool, Option<String>) = append_fn
            .call(("/nonexistent/parent/dir/file.txt", "content"))
            .unwrap();

        assert!(!success);
        assert!(err.is_some());
        assert!(err.unwrap().contains("cannot append to file"));
    }

    #[test]
    fn append_empty_content() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("append_empty.txt");
        fs::write(&file, "original").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let append_fn: Function = fs_table.get("append").unwrap();

        let (success, err): (bool, Option<String>) =
            append_fn.call((file.to_str().unwrap(), "")).unwrap();

        assert!(success);
        assert!(err.is_none());
        assert_eq!(fs::read_to_string(&file).unwrap(), "original");
    }

    // =========================================================================
    // File Operations Tests (copy, rename)
    // =========================================================================

    #[test]
    fn copy_file_success() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("source.txt");
        let dst = dir.path().join("dest.txt");
        fs::write(&src, "content to copy").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let copy_fn: Function = fs_table.get("copy").unwrap();

        let (success, err): (bool, Option<String>) = copy_fn
            .call((src.to_str().unwrap(), dst.to_str().unwrap()))
            .unwrap();

        assert!(success);
        assert!(err.is_none());
        // Both files should exist with same content
        assert!(src.exists());
        assert!(dst.exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "content to copy");
    }

    #[test]
    fn copy_overwrites_existing_dest() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("source.txt");
        let dst = dir.path().join("dest.txt");
        fs::write(&src, "new content").unwrap();
        fs::write(&dst, "old content").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let copy_fn: Function = fs_table.get("copy").unwrap();

        let (success, err): (bool, Option<String>) = copy_fn
            .call((src.to_str().unwrap(), dst.to_str().unwrap()))
            .unwrap();

        assert!(success);
        assert!(err.is_none());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "new content");
    }

    #[test]
    fn copy_fails_for_nonexistent_source() {
        let dir = tempdir().unwrap();
        let dst = dir.path().join("dest.txt");

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let copy_fn: Function = fs_table.get("copy").unwrap();

        let (success, err): (bool, Option<String>) = copy_fn
            .call(("/nonexistent/file.txt", dst.to_str().unwrap()))
            .unwrap();

        assert!(!success);
        assert!(err.is_some());
        assert!(err.unwrap().contains("cannot copy"));
    }

    #[test]
    fn copy_fails_for_directory_source() {
        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("source_dir");
        let dst = dir.path().join("dest.txt");
        fs::create_dir(&src_dir).unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let copy_fn: Function = fs_table.get("copy").unwrap();

        let (success, err): (bool, Option<String>) = copy_fn
            .call((src_dir.to_str().unwrap(), dst.to_str().unwrap()))
            .unwrap();

        assert!(!success);
        assert!(err.is_some());
        assert!(err.unwrap().contains("not a file"));
    }

    #[test]
    fn copy_fails_for_nonexistent_parent_dest() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("source.txt");
        fs::write(&src, "content").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let copy_fn: Function = fs_table.get("copy").unwrap();

        let (success, err): (bool, Option<String>) = copy_fn
            .call((src.to_str().unwrap(), "/nonexistent/parent/dest.txt"))
            .unwrap();

        assert!(!success);
        assert!(err.is_some());
    }

    #[test]
    fn rename_file_success() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("old_name.txt");
        let dst = dir.path().join("new_name.txt");
        fs::write(&src, "content").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let rename_fn: Function = fs_table.get("rename").unwrap();

        let (success, err): (bool, Option<String>) = rename_fn
            .call((src.to_str().unwrap(), dst.to_str().unwrap()))
            .unwrap();

        assert!(success);
        assert!(err.is_none());
        // Source should not exist, dest should exist
        assert!(!src.exists());
        assert!(dst.exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "content");
    }

    #[test]
    fn rename_directory_success() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("old_dir");
        let dst = dir.path().join("new_dir");
        fs::create_dir(&src).unwrap();
        fs::write(src.join("file.txt"), "content").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let rename_fn: Function = fs_table.get("rename").unwrap();

        let (success, err): (bool, Option<String>) = rename_fn
            .call((src.to_str().unwrap(), dst.to_str().unwrap()))
            .unwrap();

        assert!(success);
        assert!(err.is_none());
        assert!(!src.exists());
        assert!(dst.exists());
        assert!(dst.join("file.txt").exists());
    }

    #[test]
    fn rename_fails_for_nonexistent_source() {
        let dir = tempdir().unwrap();
        let dst = dir.path().join("dest.txt");

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let rename_fn: Function = fs_table.get("rename").unwrap();

        let (success, err): (bool, Option<String>) = rename_fn
            .call(("/nonexistent/file.txt", dst.to_str().unwrap()))
            .unwrap();

        assert!(!success);
        assert!(err.is_some());
        assert!(err.unwrap().contains("cannot rename"));
    }

    #[test]
    fn rename_fails_for_nonexistent_parent_dest() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("source.txt");
        fs::write(&src, "content").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let rename_fn: Function = fs_table.get("rename").unwrap();

        let (success, err): (bool, Option<String>) = rename_fn
            .call((src.to_str().unwrap(), "/nonexistent/parent/dest.txt"))
            .unwrap();

        assert!(!success);
        assert!(err.is_some());
    }

    // =========================================================================
    // File Metadata Tests (stat, mtime, size)
    // =========================================================================

    #[test]
    fn stat_returns_file_metadata() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let stat_fn: Function = fs_table.get("stat").unwrap();

        let (stat, err): (Option<Table>, Option<String>) =
            stat_fn.call(file.to_str().unwrap()).unwrap();

        assert!(err.is_none());
        let stat = stat.unwrap();

        // Check size
        let size: u64 = stat.get("size").unwrap();
        assert_eq!(size, 5);

        // Check type
        let file_type: String = stat.get("type").unwrap();
        assert_eq!(file_type, "file");

        // Check readonly (should be false by default)
        let readonly: bool = stat.get("readonly").unwrap();
        assert!(!readonly);

        // Check mtime exists and is reasonable (> 0)
        let mtime: u64 = stat.get("mtime").unwrap();
        assert!(mtime > 0);
    }

    #[test]
    fn stat_returns_directory_metadata() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let stat_fn: Function = fs_table.get("stat").unwrap();

        let (stat, err): (Option<Table>, Option<String>) =
            stat_fn.call(subdir.to_str().unwrap()).unwrap();

        assert!(err.is_none());
        let stat = stat.unwrap();

        let file_type: String = stat.get("type").unwrap();
        assert_eq!(file_type, "directory");
    }

    #[test]
    fn stat_returns_error_for_nonexistent() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let stat_fn: Function = fs_table.get("stat").unwrap();

        let (stat, err): (Option<Table>, Option<String>) =
            stat_fn.call("/nonexistent/path").unwrap();

        assert!(stat.is_none());
        assert!(err.is_some());
        assert!(err.unwrap().contains("cannot stat"));
    }

    #[test]
    fn stat_includes_unix_mode() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let stat_fn: Function = fs_table.get("stat").unwrap();

        let (stat, _): (Option<Table>, Option<String>) =
            stat_fn.call(file.to_str().unwrap()).unwrap();
        let stat = stat.unwrap();

        // Mode should exist on Unix
        let mode: u32 = stat.get("mode").unwrap();
        // Check it's a regular file (S_IFREG = 0o100000)
        assert!(mode & 0o100000 != 0);
    }

    #[test]
    fn mtime_returns_timestamp() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let mtime_fn: Function = fs_table.get("mtime").unwrap();

        let (mtime, err): (Option<u64>, Option<String>) =
            mtime_fn.call(file.to_str().unwrap()).unwrap();

        assert!(err.is_none());
        let mtime = mtime.unwrap();
        // Should be a reasonable timestamp (after year 2020)
        assert!(mtime > 1577836800); // 2020-01-01
    }

    #[test]
    fn mtime_returns_error_for_nonexistent() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let mtime_fn: Function = fs_table.get("mtime").unwrap();

        let (mtime, err): (Option<u64>, Option<String>) =
            mtime_fn.call("/nonexistent/path").unwrap();

        assert!(mtime.is_none());
        assert!(err.is_some());
    }

    #[test]
    fn size_returns_file_size() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello world").unwrap(); // 11 bytes

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let size_fn: Function = fs_table.get("size").unwrap();

        let (size, err): (Option<u64>, Option<String>) =
            size_fn.call(file.to_str().unwrap()).unwrap();

        assert!(err.is_none());
        assert_eq!(size.unwrap(), 11);
    }

    #[test]
    fn size_returns_zero_for_empty_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("empty.txt");
        fs::write(&file, "").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let size_fn: Function = fs_table.get("size").unwrap();

        let (size, err): (Option<u64>, Option<String>) =
            size_fn.call(file.to_str().unwrap()).unwrap();

        assert!(err.is_none());
        assert_eq!(size.unwrap(), 0);
    }

    #[test]
    fn size_returns_error_for_nonexistent() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let size_fn: Function = fs_table.get("size").unwrap();

        let (size, err): (Option<u64>, Option<String>) = size_fn.call("/nonexistent/path").unwrap();

        assert!(size.is_none());
        assert!(err.is_some());
    }

    // =========================================================================
    // Glob Function Tests (F4.1)
    // =========================================================================

    #[test]
    fn glob_matches_wildcard() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("file1.txt"), b"").unwrap();
        fs::write(dir.path().join("file2.txt"), b"").unwrap();
        fs::write(dir.path().join("file3.lua"), b"").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let glob_fn: Function = fs_table.get("glob").unwrap();

        // Match all .txt files
        let pattern = format!("{}/*.txt", dir.path().to_str().unwrap());
        let (results, err): (Option<Vec<String>>, Option<String>) = glob_fn.call(pattern).unwrap();

        assert!(err.is_none());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
        // Results should be sorted
        assert!(results[0].ends_with("file1.txt"));
        assert!(results[1].ends_with("file2.txt"));
    }

    #[test]
    fn glob_matches_question_mark() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a1.txt"), b"").unwrap();
        fs::write(dir.path().join("a2.txt"), b"").unwrap();
        fs::write(dir.path().join("ab.txt"), b"").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let glob_fn: Function = fs_table.get("glob").unwrap();

        // Match a?.txt (single character wildcard)
        let pattern = format!("{}/a?.txt", dir.path().to_str().unwrap());
        let (results, err): (Option<Vec<String>>, Option<String>) = glob_fn.call(pattern).unwrap();

        assert!(err.is_none());
        let results = results.unwrap();
        assert_eq!(results.len(), 3); // a1.txt, a2.txt, ab.txt all match a?.txt
    }

    #[test]
    fn glob_recursive_double_star() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("sub");
        let nested = subdir.join("nested");
        fs::create_dir(&subdir).unwrap();
        fs::create_dir(&nested).unwrap();
        fs::write(dir.path().join("root.txt"), b"").unwrap();
        fs::write(subdir.join("sub.txt"), b"").unwrap();
        fs::write(nested.join("deep.txt"), b"").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let glob_fn: Function = fs_table.get("glob").unwrap();

        // Match all .txt files recursively
        let pattern = format!("{}/**/*.txt", dir.path().to_str().unwrap());
        let (results, err): (Option<Vec<String>>, Option<String>) = glob_fn.call(pattern).unwrap();

        assert!(err.is_none());
        let results = results.unwrap();
        // Should find sub.txt and deep.txt (** doesn't match root level)
        assert!(results.len() >= 2);
        assert!(results.iter().any(|p| p.ends_with("sub.txt")));
        assert!(results.iter().any(|p| p.ends_with("deep.txt")));
    }

    #[test]
    fn glob_no_matches_returns_empty_array() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("file.txt"), b"").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let glob_fn: Function = fs_table.get("glob").unwrap();

        // Pattern that won't match anything
        let pattern = format!("{}/*.xyz", dir.path().to_str().unwrap());
        let (results, err): (Option<Vec<String>>, Option<String>) = glob_fn.call(pattern).unwrap();

        assert!(err.is_none());
        assert_eq!(results.unwrap().len(), 0);
    }

    #[test]
    fn glob_returns_sorted_results() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("z.txt"), b"").unwrap();
        fs::write(dir.path().join("a.txt"), b"").unwrap();
        fs::write(dir.path().join("m.txt"), b"").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let glob_fn: Function = fs_table.get("glob").unwrap();

        let pattern = format!("{}/*.txt", dir.path().to_str().unwrap());
        let (results, err): (Option<Vec<String>>, Option<String>) = glob_fn.call(pattern).unwrap();

        assert!(err.is_none());
        let results = results.unwrap();
        assert_eq!(results.len(), 3);
        // Should be sorted: a.txt, m.txt, z.txt
        assert!(results[0].ends_with("a.txt"));
        assert!(results[1].ends_with("m.txt"));
        assert!(results[2].ends_with("z.txt"));
    }

    #[test]
    fn glob_returns_full_paths() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("file.txt"), b"").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let glob_fn: Function = fs_table.get("glob").unwrap();

        let pattern = format!("{}/*.txt", dir.path().to_str().unwrap());
        let (results, err): (Option<Vec<String>>, Option<String>) = glob_fn.call(pattern).unwrap();

        assert!(err.is_none());
        let results = results.unwrap();
        assert_eq!(results.len(), 1);
        // Result should be a full path, not just basename
        let result_path = Path::new(&results[0]);
        assert!(result_path.is_absolute() || results[0].contains(dir.path().to_str().unwrap()));
        assert!(results[0].ends_with("file.txt"));
    }

    #[test]
    fn glob_character_class() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("file1.txt"), b"").unwrap();
        fs::write(dir.path().join("file2.txt"), b"").unwrap();
        fs::write(dir.path().join("file3.txt"), b"").unwrap();
        fs::write(dir.path().join("filea.txt"), b"").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let glob_fn: Function = fs_table.get("glob").unwrap();

        // Match file[12].txt
        let pattern = format!("{}/file[12].txt", dir.path().to_str().unwrap());
        let (results, err): (Option<Vec<String>>, Option<String>) = glob_fn.call(pattern).unwrap();

        assert!(err.is_none());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|p| p.ends_with("file1.txt")));
        assert!(results.iter().any(|p| p.ends_with("file2.txt")));
    }

    #[test]
    fn glob_invalid_pattern_returns_error() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let glob_fn: Function = fs_table.get("glob").unwrap();

        // Invalid pattern with unclosed bracket
        let (results, err): (Option<Vec<String>>, Option<String>) =
            glob_fn.call("/tmp/[unclosed").unwrap();

        assert!(results.is_none());
        assert!(err.is_some());
        assert!(err.unwrap().contains("invalid glob pattern"));
    }

    #[test]
    fn glob_matches_directories() {
        let dir = tempdir().unwrap();
        let subdir1 = dir.path().join("dir1");
        let subdir2 = dir.path().join("dir2");
        fs::create_dir(&subdir1).unwrap();
        fs::create_dir(&subdir2).unwrap();
        fs::write(dir.path().join("file.txt"), b"").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let glob_fn: Function = fs_table.get("glob").unwrap();

        // Match all entries (including directories)
        let pattern = format!("{}/dir*", dir.path().to_str().unwrap());
        let (results, err): (Option<Vec<String>>, Option<String>) = glob_fn.call(pattern).unwrap();

        assert!(err.is_none());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|p| p.ends_with("dir1")));
        assert!(results.iter().any(|p| p.ends_with("dir2")));
    }

    // ==================== find() function tests ====================

    #[test]
    fn find_downward_single_file() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("target.txt"), b"").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();
        fs::write(dir.path().join("subdir").join("other.txt"), b"").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let find_fn: Function = fs_table.get("find").unwrap();

        let opts = lua.create_table().unwrap();
        opts.set("path", dir.path().to_str().unwrap()).unwrap();

        let (results, err): (Option<Vec<String>>, Option<String>) =
            find_fn.call(("target.txt", opts)).unwrap();

        assert!(err.is_none());
        let results = results.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].ends_with("target.txt"));
    }

    #[test]
    fn find_downward_multiple_matches() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("config.lua"), b"").unwrap();
        fs::create_dir(dir.path().join("subdir1")).unwrap();
        fs::write(dir.path().join("subdir1").join("config.lua"), b"").unwrap();
        fs::create_dir(dir.path().join("subdir2")).unwrap();
        fs::write(dir.path().join("subdir2").join("config.lua"), b"").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let find_fn: Function = fs_table.get("find").unwrap();

        let opts = lua.create_table().unwrap();
        opts.set("path", dir.path().to_str().unwrap()).unwrap();

        let (results, err): (Option<Vec<String>>, Option<String>) =
            find_fn.call(("config.lua", opts)).unwrap();

        assert!(err.is_none());
        let results = results.unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn find_downward_with_wildcard() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("file1.txt"), b"").unwrap();
        fs::write(dir.path().join("file2.txt"), b"").unwrap();
        fs::write(dir.path().join("other.log"), b"").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let find_fn: Function = fs_table.get("find").unwrap();

        let opts = lua.create_table().unwrap();
        opts.set("path", dir.path().to_str().unwrap()).unwrap();

        let (results, err): (Option<Vec<String>>, Option<String>) =
            find_fn.call(("*.txt", opts)).unwrap();

        assert!(err.is_none());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|p| p.ends_with(".txt")));
    }

    #[test]
    fn find_downward_with_question_mark() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a1.txt"), b"").unwrap();
        fs::write(dir.path().join("b2.txt"), b"").unwrap();
        fs::write(dir.path().join("abc.txt"), b"").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let find_fn: Function = fs_table.get("find").unwrap();

        let opts = lua.create_table().unwrap();
        opts.set("path", dir.path().to_str().unwrap()).unwrap();

        // Match two-character names before .txt
        let (results, err): (Option<Vec<String>>, Option<String>) =
            find_fn.call(("??.txt", opts)).unwrap();

        assert!(err.is_none());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn find_upward_finds_marker() {
        let dir = tempdir().unwrap();
        // Create project structure: root/.git, root/src/main.rs
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src").join("main.rs"), b"").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let find_fn: Function = fs_table.get("find").unwrap();

        let opts = lua.create_table().unwrap();
        opts.set("path", dir.path().join("src").to_str().unwrap())
            .unwrap();
        opts.set("upward", true).unwrap();

        let (results, err): (Option<Vec<String>>, Option<String>) =
            find_fn.call((".git", opts)).unwrap();

        assert!(err.is_none());
        let results = results.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].ends_with(".git"));
    }

    #[test]
    fn find_upward_with_stop_directory() {
        let dir = tempdir().unwrap();
        // Create: root/marker, root/a/b/c (start from c, stop at a)
        fs::write(dir.path().join("marker"), b"").unwrap();
        fs::create_dir_all(dir.path().join("a").join("b").join("c")).unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let find_fn: Function = fs_table.get("find").unwrap();

        let opts = lua.create_table().unwrap();
        opts.set(
            "path",
            dir.path().join("a").join("b").join("c").to_str().unwrap(),
        )
        .unwrap();
        opts.set("upward", true).unwrap();
        opts.set("stop", dir.path().join("a").to_str().unwrap())
            .unwrap();

        let (results, err): (Option<Vec<String>>, Option<String>) =
            find_fn.call(("marker", opts)).unwrap();

        assert!(err.is_none());
        let results = results.unwrap();
        // Should NOT find marker because we stopped at 'a' before reaching root
        assert!(results.is_empty());
    }

    #[test]
    fn find_with_type_filter_file() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("config"), b"").unwrap();
        fs::create_dir(dir.path().join("config_dir")).unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let find_fn: Function = fs_table.get("find").unwrap();

        let opts = lua.create_table().unwrap();
        opts.set("path", dir.path().to_str().unwrap()).unwrap();
        opts.set("type", "file").unwrap();

        let (results, err): (Option<Vec<String>>, Option<String>) =
            find_fn.call(("config*", opts)).unwrap();

        assert!(err.is_none());
        let results = results.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].ends_with("config"));
    }

    #[test]
    fn find_with_type_filter_directory() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("config"), b"").unwrap();
        fs::create_dir(dir.path().join("config_dir")).unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let find_fn: Function = fs_table.get("find").unwrap();

        let opts = lua.create_table().unwrap();
        opts.set("path", dir.path().to_str().unwrap()).unwrap();
        opts.set("type", "directory").unwrap();

        let (results, err): (Option<Vec<String>>, Option<String>) =
            find_fn.call(("config*", opts)).unwrap();

        assert!(err.is_none());
        let results = results.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].ends_with("config_dir"));
    }

    #[test]
    fn find_with_limit() {
        let dir = tempdir().unwrap();
        for i in 1..=5 {
            fs::write(dir.path().join(format!("file{}.txt", i)), b"").unwrap();
        }

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let find_fn: Function = fs_table.get("find").unwrap();

        let opts = lua.create_table().unwrap();
        opts.set("path", dir.path().to_str().unwrap()).unwrap();
        opts.set("limit", 2).unwrap();

        let (results, err): (Option<Vec<String>>, Option<String>) =
            find_fn.call(("*.txt", opts)).unwrap();

        assert!(err.is_none());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn find_multiple_names() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), b"").unwrap();
        fs::write(dir.path().join("package.json"), b"").unwrap();
        fs::write(dir.path().join("other.txt"), b"").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let find_fn: Function = fs_table.get("find").unwrap();

        let opts = lua.create_table().unwrap();
        opts.set("path", dir.path().to_str().unwrap()).unwrap();

        // Create array of names to search
        let names = lua.create_table().unwrap();
        names.set(1, "Cargo.toml").unwrap();
        names.set(2, "package.json").unwrap();

        let (results, err): (Option<Vec<String>>, Option<String>) =
            find_fn.call((names, opts)).unwrap();

        assert!(err.is_none());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn find_no_matches_returns_empty() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("other.txt"), b"").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let find_fn: Function = fs_table.get("find").unwrap();

        let opts = lua.create_table().unwrap();
        opts.set("path", dir.path().to_str().unwrap()).unwrap();

        let (results, err): (Option<Vec<String>>, Option<String>) =
            find_fn.call(("nonexistent.xyz", opts)).unwrap();

        assert!(err.is_none());
        let results = results.unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn find_invalid_path_returns_error() {
        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let find_fn: Function = fs_table.get("find").unwrap();

        let opts = lua.create_table().unwrap();
        opts.set("path", "/nonexistent/path/xyz").unwrap();

        let (results, err): (Option<Vec<String>>, Option<String>) =
            find_fn.call(("file.txt", opts)).unwrap();

        assert!(results.is_none());
        assert!(err.is_some());
        assert!(err.unwrap().contains("not a directory"));
    }

    #[test]
    fn find_results_are_sorted() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("z.txt"), b"").unwrap();
        fs::write(dir.path().join("a.txt"), b"").unwrap();
        fs::write(dir.path().join("m.txt"), b"").unwrap();

        let lua = Lua::new();
        let niri = lua.create_table().unwrap();
        register(&lua, &niri).unwrap();
        let fs_table: Table = niri.get("fs").unwrap();
        let find_fn: Function = fs_table.get("find").unwrap();

        let opts = lua.create_table().unwrap();
        opts.set("path", dir.path().to_str().unwrap()).unwrap();

        let (results, err): (Option<Vec<String>>, Option<String>) =
            find_fn.call(("*.txt", opts)).unwrap();

        assert!(err.is_none());
        let results = results.unwrap();
        assert_eq!(results.len(), 3);
        // Should be sorted
        assert!(results[0].ends_with("a.txt"));
        assert!(results[1].ends_with("m.txt"));
        assert!(results[2].ends_with("z.txt"));
    }

    // Test the internal glob_match helper function
    #[test]
    fn glob_match_star_pattern() {
        assert!(super::glob_match("*.txt", "file.txt"));
        assert!(super::glob_match("*.txt", ".txt"));
        assert!(!super::glob_match("*.txt", "file.log"));
        assert!(super::glob_match("file*", "file.txt"));
        assert!(super::glob_match("file*", "file"));
        assert!(super::glob_match("*", "anything"));
    }

    #[test]
    fn glob_match_question_pattern() {
        assert!(super::glob_match("?.txt", "a.txt"));
        assert!(!super::glob_match("?.txt", "ab.txt"));
        assert!(super::glob_match("file?.txt", "file1.txt"));
        assert!(!super::glob_match("file?.txt", "file12.txt"));
    }

    #[test]
    fn glob_match_combined_patterns() {
        assert!(super::glob_match("*.?", "file.c"));
        assert!(!super::glob_match("*.?", "file.rs"));
        assert!(super::glob_match("?*", "ab"));
        assert!(super::glob_match("?*", "a"));
        assert!(!super::glob_match("?*", ""));
    }
}
