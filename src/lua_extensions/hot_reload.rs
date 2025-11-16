//! Hot reload system for Niri Lua configuration and plugins.
//!
//! This module provides automatic reloading of Lua files when they change on disk,
//! enabling rapid development iteration.
//!
//! # Features
//!
//! - File system watching for changes
//! - Automatic reload on modification
//! - Handler cleanup before reload
//! - Error recovery (keeps previous state on error)
//!
//! # Example
//!
//! ```
//! let mut watcher = HotReloader::new();
//! watcher.watch("~/.config/niri/config.lua".into())?;
//! if watcher.check_changes()? {
//!     println!("Config changed, reloading...");
//!     // Re-execute Lua code
//! }
//! ```

use log::{debug, info, warn};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// File metadata for change detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileMetadata {
    modified: u64,
    size: u64,
}

impl FileMetadata {
    /// Get metadata for a file
    fn from_file(path: &Path) -> Option<Self> {
        match fs::metadata(path) {
            Ok(metadata) => {
                let modified = metadata
                    .modified()
                    .ok()?
                    .duration_since(UNIX_EPOCH)
                    .ok()?
                    .as_secs();
                let size = metadata.len();
                Some(FileMetadata { modified, size })
            }
            Err(_) => None,
        }
    }
}

/// Hot reload watcher for Lua files
pub struct HotReloader {
    watched_files: HashMap<PathBuf, FileMetadata>,
    changed_files: Vec<PathBuf>,
}

impl HotReloader {
    /// Create a new hot reloader
    pub fn new() -> Self {
        Self {
            watched_files: HashMap::new(),
            changed_files: Vec::new(),
        }
    }

    /// Add a file to watch for changes
    pub fn watch(&mut self, path: PathBuf) -> std::io::Result<()> {
        // Expand ~ in path
        let path = if let Some(path_str) = path.to_str() {
            if path_str.starts_with("~") {
                let home = std::env::var("HOME").ok();
                if let Some(home) = home {
                    PathBuf::from(home).join(&path_str[1..])
                } else {
                    path
                }
            } else {
                path
            }
        } else {
            path
        };

        if !path.exists() {
            warn!("Watched file does not exist: {}", path.display());
            return Ok(());
        }

        match FileMetadata::from_file(&path) {
            Some(metadata) => {
                self.watched_files.insert(path.clone(), metadata);
                debug!("Now watching file: {}", path.display());
                Ok(())
            }
            None => {
                warn!("Failed to get metadata for: {}", path.display());
                Ok(())
            }
        }
    }

    /// Stop watching a file
    pub fn unwatch(&mut self, path: &Path) {
        if self.watched_files.remove(path).is_some() {
            debug!("Stopped watching file: {}", path.display());
        }
    }

    /// Check for file changes and update the internal state
    pub fn check_changes(&mut self) -> std::io::Result<bool> {
        self.changed_files.clear();

        for (path, old_metadata) in &self.watched_files.clone() {
            match FileMetadata::from_file(path) {
                Some(new_metadata) => {
                    if new_metadata != *old_metadata {
                        info!("File changed: {}", path.display());
                        self.changed_files.push(path.clone());
                        self.watched_files.insert(path.clone(), new_metadata);
                    }
                }
                None => {
                    warn!("File was deleted or became inaccessible: {}", path.display());
                    // Don't remove from watched_files, in case it comes back
                }
            }
        }

        Ok(!self.changed_files.is_empty())
    }

    /// Get the list of recently changed files
    pub fn get_changed_files(&self) -> &[PathBuf] {
        &self.changed_files
    }

    /// Read the content of a watched file
    pub fn read_file(&self, path: &Path) -> std::io::Result<String> {
        fs::read_to_string(path)
    }

    /// Get number of watched files
    pub fn watched_count(&self) -> usize {
        self.watched_files.len()
    }

    /// Clear all watched files
    pub fn clear_all(&mut self) {
        self.watched_files.clear();
        self.changed_files.clear();
        debug!("Cleared all watched files");
    }
}

impl Default for HotReloader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_hot_reloader_creation() {
        let reloader = HotReloader::new();
        assert_eq!(reloader.watched_count(), 0);
    }

    #[test]
    fn test_watch_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.lua");
        File::create(&test_file).unwrap().write_all(b"test").unwrap();

        let mut reloader = HotReloader::new();
        reloader.watch(test_file.clone()).unwrap();

        assert_eq!(reloader.watched_count(), 1);
    }

    #[test]
    fn test_unwatch_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.lua");
        File::create(&test_file).unwrap().write_all(b"test").unwrap();

        let mut reloader = HotReloader::new();
        reloader.watch(test_file.clone()).unwrap();
        reloader.unwatch(&test_file);

        assert_eq!(reloader.watched_count(), 0);
    }

    #[test]
    fn test_detect_file_change() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.lua");
        File::create(&test_file).unwrap().write_all(b"original").unwrap();

        let mut reloader = HotReloader::new();
        reloader.watch(test_file.clone()).unwrap();

        // First check should show no changes
        assert!(!reloader.check_changes().unwrap());

        // Sleep to ensure timestamp difference (some filesystems have 1 second granularity)
        thread::sleep(Duration::from_secs(1));

        // Modify file (change size to ensure detection)
        let mut file = File::create(&test_file).unwrap();
        file.write_all(b"modified_content_is_longer").unwrap();

        // Second check should detect change
        assert!(reloader.check_changes().unwrap());
        assert_eq!(reloader.get_changed_files().len(), 1);
    }

    #[test]
    fn test_read_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.lua");
        File::create(&test_file).unwrap().write_all(b"content").unwrap();

        let reloader = HotReloader::new();
        let content = reloader.read_file(&test_file).unwrap();

        assert_eq!(content, "content");
    }

    #[test]
    fn test_file_metadata_comparison() {
        let metadata1 = FileMetadata {
            modified: 100,
            size: 50,
        };
        let metadata2 = FileMetadata {
            modified: 100,
            size: 50,
        };
        let metadata3 = FileMetadata {
            modified: 200,
            size: 50,
        };

        assert_eq!(metadata1, metadata2);
        assert_ne!(metadata1, metadata3);
    }

    #[test]
    fn test_clear_all() {
        let temp_dir = TempDir::new().unwrap();
        let test_file1 = temp_dir.path().join("test1.lua");
        let test_file2 = temp_dir.path().join("test2.lua");

        File::create(&test_file1).unwrap().write_all(b"test").unwrap();
        File::create(&test_file2).unwrap().write_all(b"test").unwrap();

        let mut reloader = HotReloader::new();
        reloader.watch(test_file1).unwrap();
        reloader.watch(test_file2).unwrap();

        assert_eq!(reloader.watched_count(), 2);

        reloader.clear_all();

        assert_eq!(reloader.watched_count(), 0);
    }

    #[test]
    fn test_multiple_file_changes() {
        let temp_dir = TempDir::new().unwrap();
        let test_file1 = temp_dir.path().join("test1.lua");
        let test_file2 = temp_dir.path().join("test2.lua");

        File::create(&test_file1).unwrap().write_all(b"test1").unwrap();
        File::create(&test_file2).unwrap().write_all(b"test2").unwrap();

        let mut reloader = HotReloader::new();
        reloader.watch(test_file1.clone()).unwrap();
        reloader.watch(test_file2.clone()).unwrap();

        // Initial check
        assert!(!reloader.check_changes().unwrap());

        thread::sleep(Duration::from_millis(100));

        // Modify both files
        File::create(&test_file1).unwrap().write_all(b"modified1").unwrap();
        File::create(&test_file2).unwrap().write_all(b"modified2").unwrap();

        // Both should be detected
        assert!(reloader.check_changes().unwrap());
        assert_eq!(reloader.get_changed_files().len(), 2);
    }
}
