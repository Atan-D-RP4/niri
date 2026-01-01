//! Configuration state wrapper for Lua proxy objects.
//!
//! This module provides the `ConfigState` type that wraps the shared configuration
//! and dirty flags, allowing proxy objects to safely access and modify config values.

use std::cell::{Ref, RefCell, RefMut};
use std::fmt;
use std::rc::Rc;

use niri_config::Config;

use crate::config_dirty::ConfigDirtyFlags;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigStateError {
    ConfigBorrowError,
    DirtyFlagsBorrowError,
}

impl fmt::Display for ConfigStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigBorrowError => write!(f, "config borrow error"),
            Self::DirtyFlagsBorrowError => write!(f, "dirty flags borrow error"),
        }
    }
}

impl std::error::Error for ConfigStateError {}

impl From<ConfigStateError> for mlua::Error {
    fn from(err: ConfigStateError) -> Self {
        mlua::Error::external(err)
    }
}

/// Represents which part of the config has been modified.
///
/// This is used to efficiently determine what needs to be reconfigured
/// after Lua script execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DirtyFlag {
    /// Input settings changed (keyboard, mouse, touch, etc.)
    Input,
    /// Output/monitor settings changed
    Outputs,
    /// Layout settings changed (gaps, focus ring, border, etc.)
    Layout,
    /// Animation settings changed
    Animations,
    /// Window rules changed
    WindowRules,
    /// Layer rules changed
    LayerRules,
    /// Key bindings changed
    Binds,
    /// Cursor settings changed
    Cursor,
    /// Keyboard-specific settings changed
    Keyboard,
    /// Gesture settings changed
    Gestures,
    /// Overview settings changed
    Overview,
    /// Recent windows settings changed
    RecentWindows,
    /// Clipboard settings changed
    Clipboard,
    /// Hotkey overlay settings changed
    HotkeyOverlay,
    /// Config notification settings changed
    ConfigNotification,
    /// Debug options changed
    Debug,
    /// Xwayland satellite settings changed
    XwaylandSatellite,
    /// Miscellaneous settings changed
    Misc,
    /// Spawn-at-startup commands changed
    SpawnAtStartup,
    /// Environment variables changed
    Environment,
    /// Workspace configuration changed
    Workspaces,
}

/// Shared configuration state for Lua proxy objects.
///
/// This wrapper holds references to the configuration and dirty flags,
/// allowing proxy objects to read and modify config values safely.
///
/// # Thread Safety
///
/// `ConfigState` uses `Rc<RefCell<_>>` internally, which is suitable for the
/// single-threaded Lua runtime.
#[derive(Clone)]
pub struct ConfigState {
    /// The configuration being edited
    config: Rc<RefCell<Config>>,
    /// Flags indicating which parts of the config have been modified
    dirty_flags: Rc<RefCell<ConfigDirtyFlags>>,
}

impl ConfigState {
    /// Create a new config state with the given config and dirty flags.
    pub fn new(config: Rc<RefCell<Config>>, dirty_flags: Rc<RefCell<ConfigDirtyFlags>>) -> Self {
        Self {
            config,
            dirty_flags,
        }
    }

    /// Try to borrow the configuration for reading.
    pub fn try_borrow_config(&self) -> Result<Ref<'_, Config>, ConfigStateError> {
        self.config
            .try_borrow()
            .map_err(|_| ConfigStateError::ConfigBorrowError)
    }

    /// Try to borrow the dirty flags for reading.
    pub fn try_borrow_dirty_flags(&self) -> Result<Ref<'_, ConfigDirtyFlags>, ConfigStateError> {
        self.dirty_flags
            .try_borrow()
            .map_err(|_| ConfigStateError::DirtyFlagsBorrowError)
    }

    /// Borrow the configuration for reading.
    pub fn borrow_config(&self) -> Ref<'_, Config> {
        self.config.borrow()
    }

    /// Borrow the configuration for writing.
    pub fn borrow_config_mut(&self) -> RefMut<'_, Config> {
        self.config.borrow_mut()
    }

    /// Borrow the dirty flags for reading.
    pub fn borrow_dirty_flags(&self) -> Ref<'_, ConfigDirtyFlags> {
        self.dirty_flags.borrow()
    }

    /// Borrow the dirty flags for writing.
    pub fn borrow_dirty_flags_mut(&self) -> RefMut<'_, ConfigDirtyFlags> {
        self.dirty_flags.borrow_mut()
    }

    /// Execute a closure with mutable access to the config.
    ///
    /// This is a convenience method for read-modify-write operations.
    pub fn with_config<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Config) -> R,
    {
        let mut config = self.borrow_config_mut();
        f(&mut config)
    }

    /// Execute a closure with mutable access to the dirty flags.
    pub fn with_dirty_flags<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut ConfigDirtyFlags) -> R,
    {
        let mut flags = self.borrow_dirty_flags_mut();
        f(&mut flags)
    }

    /// Mark a config section as dirty.
    ///
    /// This should be called after modifying any config value.
    pub fn mark_dirty(&self, flag: DirtyFlag) {
        let mut flags = self.borrow_dirty_flags_mut();
        match flag {
            DirtyFlag::Input => flags.input = true,
            DirtyFlag::Outputs => flags.outputs = true,
            DirtyFlag::Layout => flags.layout = true,
            DirtyFlag::Animations => flags.animations = true,
            DirtyFlag::WindowRules => flags.window_rules = true,
            DirtyFlag::LayerRules => flags.layer_rules = true,
            DirtyFlag::Binds => flags.binds = true,
            DirtyFlag::Cursor => flags.cursor = true,
            DirtyFlag::Keyboard => flags.keyboard = true,
            DirtyFlag::Gestures => flags.gestures = true,
            DirtyFlag::Overview => flags.overview = true,
            DirtyFlag::RecentWindows => flags.recent_windows = true,
            DirtyFlag::Clipboard => flags.clipboard = true,
            DirtyFlag::HotkeyOverlay => flags.hotkey_overlay = true,
            DirtyFlag::ConfigNotification => flags.config_notification = true,
            DirtyFlag::Debug => flags.debug = true,
            DirtyFlag::XwaylandSatellite => flags.xwayland_satellite = true,
            DirtyFlag::Misc => flags.misc = true,
            DirtyFlag::SpawnAtStartup => flags.spawn_at_startup = true,
            DirtyFlag::Environment => flags.environment = true,
            DirtyFlag::Workspaces => flags.workspaces = true,
        }
    }

    /// Check if any config section is dirty.
    pub fn is_any_dirty(&self) -> bool {
        let flags = self.borrow_dirty_flags();
        flags.any()
    }

    /// Get the raw Rc references for creating child proxies.
    pub fn clone_rcs(&self) -> (Rc<RefCell<Config>>, Rc<RefCell<ConfigDirtyFlags>>) {
        (Rc::clone(&self.config), Rc::clone(&self.dirty_flags))
    }
}

impl std::fmt::Debug for ConfigState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigState")
            .field("config", &"<Config>")
            .field("dirty_flags", &"<ConfigDirtyFlags>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_state_creation() {
        let config = Rc::new(RefCell::new(Config::default()));
        let dirty = Rc::new(RefCell::new(ConfigDirtyFlags::default()));
        let state = ConfigState::new(config, dirty);

        assert!(!state.is_any_dirty());
    }

    #[test]
    fn test_config_state_clone() {
        let config = Rc::new(RefCell::new(Config::default()));
        let dirty = Rc::new(RefCell::new(ConfigDirtyFlags::default()));
        let state1 = ConfigState::new(config, dirty);
        let state2 = state1.clone();

        // Modify via state1
        state1.with_dirty_flags(|flags| {
            flags.layout = true;
        });

        // Should be visible via state2 (same Rc)
        assert!(state2.is_any_dirty());
    }

    #[test]
    fn test_with_config() {
        let config = Rc::new(RefCell::new(Config::default()));
        let dirty = Rc::new(RefCell::new(ConfigDirtyFlags::default()));
        let state = ConfigState::new(config, dirty);

        let original = state.with_config(|cfg| cfg.prefer_no_csd);
        state.with_config(|cfg| {
            cfg.prefer_no_csd = !original;
        });
        let new = state.with_config(|cfg| cfg.prefer_no_csd);

        assert_eq!(new, !original);
    }

    #[test]
    fn test_clone_rcs() {
        let config = Rc::new(RefCell::new(Config::default()));
        let dirty = Rc::new(RefCell::new(ConfigDirtyFlags::default()));
        let state = ConfigState::new(config.clone(), dirty.clone());

        let (config2, dirty2) = state.clone_rcs();

        // Should be same Rc (same pointer)
        assert!(Rc::ptr_eq(&config, &config2));
        assert!(Rc::ptr_eq(&dirty, &dirty2));
    }

    #[test]
    fn test_mark_dirty() {
        let config = Rc::new(RefCell::new(Config::default()));
        let dirty = Rc::new(RefCell::new(ConfigDirtyFlags::default()));
        let state = ConfigState::new(config, dirty);

        assert!(!state.is_any_dirty());

        state.mark_dirty(DirtyFlag::Layout);
        assert!(state.is_any_dirty());
        assert!(state.borrow_dirty_flags().layout);

        state.mark_dirty(DirtyFlag::Animations);
        assert!(state.borrow_dirty_flags().animations);
    }

    #[test]
    fn test_all_dirty_flags() {
        let config = Rc::new(RefCell::new(Config::default()));
        let dirty = Rc::new(RefCell::new(ConfigDirtyFlags::default()));
        let state = ConfigState::new(config, dirty);

        // Test that all DirtyFlag variants work
        let flags = [
            DirtyFlag::Input,
            DirtyFlag::Outputs,
            DirtyFlag::Layout,
            DirtyFlag::Animations,
            DirtyFlag::WindowRules,
            DirtyFlag::LayerRules,
            DirtyFlag::Binds,
            DirtyFlag::Cursor,
            DirtyFlag::Keyboard,
            DirtyFlag::Gestures,
            DirtyFlag::Overview,
            DirtyFlag::RecentWindows,
            DirtyFlag::Clipboard,
            DirtyFlag::HotkeyOverlay,
            DirtyFlag::ConfigNotification,
            DirtyFlag::Debug,
            DirtyFlag::XwaylandSatellite,
            DirtyFlag::Misc,
            DirtyFlag::SpawnAtStartup,
            DirtyFlag::Environment,
            DirtyFlag::Workspaces,
        ];

        for flag in flags {
            state.mark_dirty(flag);
        }

        assert!(state.is_any_dirty());
    }
}
