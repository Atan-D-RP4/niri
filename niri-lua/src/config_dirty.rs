//! Dirty flag tracking for config changes.
//!
//! When Lua modifies config values, dirty flags are set to indicate which
//! compositor subsystems need to be updated. The compositor polls these
//! flags after Lua execution and refreshes the appropriate subsystems.
//!
//! ## Field-Level Tracking
//!
//! In addition to section-level boolean flags, this module provides field-level
//! dirty path tracking via `dirty_paths`. This allows the compositor to know
//! exactly which fields changed (e.g., "layout.gaps" vs "layout.border").

use indexmap::IndexSet;

/// Tracks which config subsystems have been modified.
///
/// Each flag corresponds to a subsystem that may need to be refreshed
/// when its configuration changes. Additionally, `dirty_paths` tracks
/// the specific field paths that were modified.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ConfigDirtyFlags {
    /// Layout-related changes (gaps, borders, focus ring, etc.)
    pub layout: bool,
    /// Input device configuration (keyboard, touchpad, mouse, etc.)
    pub input: bool,
    /// Cursor configuration (theme, size, hiding behavior)
    pub cursor: bool,
    /// Keyboard-specific changes (layout, repeat rate, etc.)
    pub keyboard: bool,
    /// Output configuration (resolution, scale, position)
    pub outputs: bool,
    /// Animation configuration (durations, curves, slowdown)
    pub animations: bool,
    /// Window rules (matching criteria, actions)
    pub window_rules: bool,
    /// Layer rules
    pub layer_rules: bool,
    /// Key bindings
    pub binds: bool,
    /// Gesture configuration
    pub gestures: bool,
    /// Overview configuration
    pub overview: bool,
    /// Recent windows (MRU) configuration
    pub recent_windows: bool,
    /// Clipboard configuration
    pub clipboard: bool,
    /// Hotkey overlay configuration
    pub hotkey_overlay: bool,
    /// Config notification settings
    pub config_notification: bool,
    /// Debug settings
    pub debug: bool,
    /// Xwayland satellite settings
    pub xwayland_satellite: bool,
    /// Miscellaneous settings (prefer_no_csd, screenshot_path, etc.)
    pub misc: bool,
    /// Spawn at startup configuration
    pub spawn_at_startup: bool,
    /// Environment variables
    pub environment: bool,
    /// Workspace configuration
    pub workspaces: bool,
    /// Field-level dirty paths (e.g., "layout.gaps", "input.touchpad.natural_scroll")
    pub dirty_paths: IndexSet<String>,
}

impl ConfigDirtyFlags {
    /// Create a new set of dirty flags with all flags cleared.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a specific field path as dirty.
    pub fn mark_dirty(&mut self, path: &str) {
        self.dirty_paths.insert(path.to_string());
    }

    /// Take the dirty paths, returning them and clearing the set.
    pub fn take_dirty_paths(&mut self) -> Vec<String> {
        std::mem::take(&mut self.dirty_paths).into_iter().collect()
    }

    /// Check if any flag is set.
    pub fn any(&self) -> bool {
        self.layout
            || self.input
            || self.cursor
            || self.keyboard
            || self.outputs
            || self.animations
            || self.window_rules
            || self.layer_rules
            || self.binds
            || self.gestures
            || self.overview
            || self.recent_windows
            || self.clipboard
            || self.hotkey_overlay
            || self.config_notification
            || self.debug
            || self.xwayland_satellite
            || self.misc
            || self.spawn_at_startup
            || self.environment
            || self.workspaces
            || !self.dirty_paths.is_empty()
    }

    /// Clear all flags.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Take all flags, returning the current state and resetting to default.
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_flags_are_false() {
        let flags = ConfigDirtyFlags::default();
        assert!(!flags.layout);
        assert!(!flags.input);
        assert!(!flags.cursor);
        assert!(!flags.keyboard);
        assert!(!flags.outputs);
        assert!(!flags.animations);
        assert!(!flags.window_rules);
        assert!(!flags.misc);
    }

    #[test]
    fn test_any_returns_false_when_all_clear() {
        let flags = ConfigDirtyFlags::default();
        assert!(!flags.any());
    }

    #[test]
    fn test_any_returns_true_when_one_set() {
        let flags = ConfigDirtyFlags {
            layout: true,
            ..Default::default()
        };
        assert!(flags.any());
    }

    #[test]
    fn test_clear_resets_all_flags() {
        let mut flags = ConfigDirtyFlags {
            layout: true,
            input: true,
            cursor: true,
            ..Default::default()
        };
        flags.clear();
        assert!(!flags.any());
    }

    #[test]
    fn test_take_returns_current_and_resets() {
        let mut flags = ConfigDirtyFlags {
            layout: true,
            animations: true,
            ..Default::default()
        };

        let taken = flags.take();

        // Original should be cleared
        assert!(!flags.any());
        assert!(!flags.layout);

        // Taken should have the values
        assert!(taken.layout);
        assert!(taken.animations);
    }

    #[test]
    fn test_mark_dirty_adds_path() {
        let mut flags = ConfigDirtyFlags::default();
        flags.mark_dirty("layout.gaps");
        flags.mark_dirty("input.touchpad.natural_scroll");

        assert!(flags.dirty_paths.contains("layout.gaps"));
        assert!(flags.dirty_paths.contains("input.touchpad.natural_scroll"));
        assert_eq!(flags.dirty_paths.len(), 2);
    }

    #[test]
    fn test_mark_dirty_deduplicates() {
        let mut flags = ConfigDirtyFlags::default();
        flags.mark_dirty("layout.gaps");
        flags.mark_dirty("layout.gaps");
        flags.mark_dirty("layout.gaps");

        assert_eq!(flags.dirty_paths.len(), 1);
    }

    #[test]
    fn test_take_dirty_paths_returns_and_clears() {
        let mut flags = ConfigDirtyFlags::default();
        flags.mark_dirty("layout.gaps");
        flags.mark_dirty("layout.border");

        let paths = flags.take_dirty_paths();

        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&"layout.gaps".to_string()));
        assert!(paths.contains(&"layout.border".to_string()));
        assert!(flags.dirty_paths.is_empty());
    }

    #[test]
    fn test_any_returns_true_when_dirty_paths_set() {
        let mut flags = ConfigDirtyFlags::default();
        assert!(!flags.any());

        flags.mark_dirty("layout.gaps");
        assert!(flags.any());
    }

    #[test]
    fn test_clear_also_clears_dirty_paths() {
        let mut flags = ConfigDirtyFlags::default();
        flags.layout = true;
        flags.mark_dirty("layout.gaps");

        flags.clear();

        assert!(!flags.layout);
        assert!(flags.dirty_paths.is_empty());
        assert!(!flags.any());
    }
}
