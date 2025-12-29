//! Tests for the derive macros (LuaEnum, LuaConfigProxy, and DirtyFlags).
//!
//! These tests verify that the macro-generated code works correctly.

// Allow the macro to reference niri_lua::traits when testing within the crate
extern crate self as niri_lua;

use niri_lua_derive::LuaEnum;

use crate::traits::{LuaEnumConvert, LuaFieldConvert};

/// Test enum with default kebab-case renaming
#[derive(Debug, Clone, Copy, PartialEq, Eq, LuaEnum)]
#[lua_enum(rename_all = "kebab-case")]
pub enum TestDirection {
    Up,
    Down,
    LeftSide,
    RightSide,
}

/// Test enum with snake_case renaming
#[derive(Debug, Clone, Copy, PartialEq, Eq, LuaEnum)]
#[lua_enum(rename_all = "snake_case")]
pub enum TestMode {
    FastMode,
    SlowMode,
    AutoDetect,
}

/// Test enum with custom variant renames
#[derive(Debug, Clone, Copy, PartialEq, Eq, LuaEnum)]
#[lua_enum(rename_all = "kebab-case")]
pub enum TestWithRenames {
    Normal,
    #[lua_enum(rename = "special-value")]
    Special,
    #[lua_enum(rename = "custom")]
    VeryLongVariantName,
}

#[test]
fn test_enum_to_lua_string_kebab_case() {
    assert_eq!(TestDirection::Up.to_lua_string(), "up");
    assert_eq!(TestDirection::Down.to_lua_string(), "down");
    assert_eq!(TestDirection::LeftSide.to_lua_string(), "left-side");
    assert_eq!(TestDirection::RightSide.to_lua_string(), "right-side");
}

#[test]
fn test_enum_from_lua_string_kebab_case() {
    assert_eq!(
        TestDirection::from_lua_string("up").unwrap(),
        TestDirection::Up
    );
    assert_eq!(
        TestDirection::from_lua_string("down").unwrap(),
        TestDirection::Down
    );
    assert_eq!(
        TestDirection::from_lua_string("left-side").unwrap(),
        TestDirection::LeftSide
    );
    assert_eq!(
        TestDirection::from_lua_string("right-side").unwrap(),
        TestDirection::RightSide
    );
}

#[test]
fn test_enum_to_lua_string_snake_case() {
    assert_eq!(TestMode::FastMode.to_lua_string(), "fast_mode");
    assert_eq!(TestMode::SlowMode.to_lua_string(), "slow_mode");
    assert_eq!(TestMode::AutoDetect.to_lua_string(), "auto_detect");
}

#[test]
fn test_enum_from_lua_string_snake_case() {
    assert_eq!(
        TestMode::from_lua_string("fast_mode").unwrap(),
        TestMode::FastMode
    );
    assert_eq!(
        TestMode::from_lua_string("slow_mode").unwrap(),
        TestMode::SlowMode
    );
    assert_eq!(
        TestMode::from_lua_string("auto_detect").unwrap(),
        TestMode::AutoDetect
    );
}

#[test]
fn test_enum_custom_rename() {
    assert_eq!(TestWithRenames::Normal.to_lua_string(), "normal");
    assert_eq!(TestWithRenames::Special.to_lua_string(), "special-value");
    assert_eq!(
        TestWithRenames::VeryLongVariantName.to_lua_string(),
        "custom"
    );

    assert_eq!(
        TestWithRenames::from_lua_string("normal").unwrap(),
        TestWithRenames::Normal
    );
    assert_eq!(
        TestWithRenames::from_lua_string("special-value").unwrap(),
        TestWithRenames::Special
    );
    assert_eq!(
        TestWithRenames::from_lua_string("custom").unwrap(),
        TestWithRenames::VeryLongVariantName
    );
}

#[test]
fn test_enum_variant_names() {
    let names = TestDirection::variant_names();
    assert_eq!(names, &["up", "down", "left-side", "right-side"]);
}

#[test]
fn test_enum_variant_names_snake_case() {
    let names = TestMode::variant_names();
    assert_eq!(names, &["fast_mode", "slow_mode", "auto_detect"]);
}

#[test]
fn test_enum_variant_names_with_custom_renames() {
    let names = TestWithRenames::variant_names();
    assert_eq!(names, &["normal", "special-value", "custom"]);
}

#[test]
fn test_enum_invalid_value_error() {
    let result = TestDirection::from_lua_string("invalid");
    assert!(result.is_err());

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("invalid"),
        "Error message should contain the invalid input"
    );
    assert!(
        err_msg.contains("up"),
        "Error message should list valid variant 'up'"
    );
    assert!(
        err_msg.contains("down"),
        "Error message should list valid variant 'down'"
    );
    assert!(
        err_msg.contains("left-side"),
        "Error message should list valid variant 'left-side'"
    );
    assert!(
        err_msg.contains("right-side"),
        "Error message should list valid variant 'right-side'"
    );
}

#[test]
fn test_enum_invalid_value_error_snake_case() {
    let result = TestMode::from_lua_string("wrong_value");
    assert!(result.is_err());

    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("wrong_value"));
    assert!(err_msg.contains("fast_mode"));
    assert!(err_msg.contains("slow_mode"));
    assert!(err_msg.contains("auto_detect"));
}

#[test]
fn test_enum_lua_field_convert_to_lua() {
    let value: String = <TestDirection as LuaFieldConvert>::to_lua(&TestDirection::LeftSide);
    assert_eq!(value, "left-side");
}

#[test]
fn test_enum_lua_field_convert_from_lua() {
    let result = <TestDirection as LuaFieldConvert>::from_lua("right-side".to_string());
    assert_eq!(result.unwrap(), TestDirection::RightSide);
}

#[test]
fn test_enum_lua_field_convert_roundtrip() {
    let original = TestMode::SlowMode;
    let lua_repr = <TestMode as LuaFieldConvert>::to_lua(&original);
    let converted = <TestMode as LuaFieldConvert>::from_lua(lua_repr).unwrap();
    assert_eq!(original, converted);
}

#[test]
fn test_enum_lua_field_convert_all_variants() {
    // Test that all variants can roundtrip through LuaFieldConvert
    for variant in [
        TestDirection::Up,
        TestDirection::Down,
        TestDirection::LeftSide,
        TestDirection::RightSide,
    ] {
        let lua_repr = <TestDirection as LuaFieldConvert>::to_lua(&variant);
        let converted = <TestDirection as LuaFieldConvert>::from_lua(lua_repr).unwrap();
        assert_eq!(variant, converted);
    }
}

#[test]
fn test_enum_case_sensitivity() {
    // Verify that conversion is case-sensitive
    assert!(TestDirection::from_lua_string("UP").is_err());
    assert!(TestDirection::from_lua_string("Left-Side").is_err());
    assert!(TestMode::from_lua_string("Fast_Mode").is_err());
}

// ============================================================================
// LuaConfigProxy Tests
// ============================================================================

#[cfg(test)]
mod lua_config_proxy_tests {
    use std::sync::{Arc, Mutex};

    use niri_config::Config;

    use crate::config_dirty::ConfigDirtyFlags;
    use crate::config_state::{ConfigState, DirtyFlag};

    // Note: Testing the actual macro expansion requires integration with
    // the real Config struct. These tests verify the supporting infrastructure.

    #[test]
    fn test_config_state_creation() {
        let config = Arc::new(Mutex::new(Config::default()));
        let dirty_flags = Arc::new(Mutex::new(ConfigDirtyFlags::new()));

        let state = ConfigState::new(config, dirty_flags);

        // Verify we can borrow config
        let _config = state.borrow_config();
    }

    #[test]
    fn test_config_state_clone() {
        let config = Arc::new(Mutex::new(Config::default()));
        let dirty_flags = Arc::new(Mutex::new(ConfigDirtyFlags::new()));

        let state1 = ConfigState::new(config.clone(), dirty_flags.clone());
        let state2 = state1.clone();

        // Both should point to same underlying data
        state1.with_dirty_flags(|flags| {
            flags.layout = true;
        });

        // state2 should see the change (same Arc)
        assert!(state2.is_any_dirty());
    }

    #[test]
    fn test_config_state_dirty_flags() {
        let config = Arc::new(Mutex::new(Config::default()));
        let dirty_flags = Arc::new(Mutex::new(ConfigDirtyFlags::new()));

        let state = ConfigState::new(config, dirty_flags);

        // Initially not dirty
        assert!(!state.is_any_dirty());

        // Mark layout dirty
        state.mark_dirty(DirtyFlag::Layout);

        // Now should be dirty
        assert!(state.is_any_dirty());
    }

    #[test]
    fn test_dirty_flag_enum_variants() {
        // Verify all expected dirty flags exist
        let flags = [
            DirtyFlag::Input,
            DirtyFlag::Outputs,
            DirtyFlag::Layout,
            DirtyFlag::Animations,
            DirtyFlag::WindowRules,
            DirtyFlag::Binds,
            DirtyFlag::Cursor,
            DirtyFlag::Debug,
            DirtyFlag::Misc,
        ];

        // This is a compile-time test - if it compiles, the variants exist
        for flag in flags {
            let _ = format!("{:?}", flag);
        }
    }

    #[test]
    fn test_with_config_closure() {
        let config = Arc::new(Mutex::new(Config::default()));
        let dirty_flags = Arc::new(Mutex::new(ConfigDirtyFlags::new()));

        let state = ConfigState::new(config, dirty_flags);

        // Test with_config closure pattern
        let result = state.with_config(|config| {
            // Access config here
            config.input.keyboard.repeat_delay
        });

        // Should return the value from the closure
        assert!(result > 0);
    }

    #[test]
    fn test_all_dirty_flag_variants() {
        let config = Arc::new(Mutex::new(Config::default()));
        let dirty_flags = Arc::new(Mutex::new(ConfigDirtyFlags::new()));

        let state = ConfigState::new(config, dirty_flags);

        // Test that all DirtyFlag variants can be marked
        let all_flags = [
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

        for flag in all_flags {
            state.mark_dirty(flag);
        }

        assert!(state.is_any_dirty());
    }

    #[test]
    fn test_clone_arcs() {
        let config = Arc::new(Mutex::new(Config::default()));
        let dirty_flags = Arc::new(Mutex::new(ConfigDirtyFlags::new()));

        let state = ConfigState::new(config.clone(), dirty_flags.clone());

        let (config2, dirty2) = state.clone_arcs();

        // Should be same Arc (same pointer)
        assert!(Arc::ptr_eq(&config, &config2));
        assert!(Arc::ptr_eq(&dirty_flags, &dirty2));
    }

    #[test]
    fn test_mark_specific_dirty_flags() {
        let config = Arc::new(Mutex::new(Config::default()));
        let dirty_flags = Arc::new(Mutex::new(ConfigDirtyFlags::new()));

        let state = ConfigState::new(config, dirty_flags);

        // Mark layout dirty
        state.mark_dirty(DirtyFlag::Layout);
        assert!(state.borrow_dirty_flags().layout);
        assert!(!state.borrow_dirty_flags().animations);

        // Mark animations dirty
        state.mark_dirty(DirtyFlag::Animations);
        assert!(state.borrow_dirty_flags().layout);
        assert!(state.borrow_dirty_flags().animations);
    }

    #[test]
    fn test_config_state_borrow_dirty_flags() {
        let config = Arc::new(Mutex::new(Config::default()));
        let dirty_flags = Arc::new(Mutex::new(ConfigDirtyFlags::new()));

        let state = ConfigState::new(config, dirty_flags);

        // Borrow and check
        {
            let flags = state.borrow_dirty_flags();
            assert!(!flags.layout);
        }

        // Mark dirty
        state.mark_dirty(DirtyFlag::Layout);

        // Borrow again and verify
        {
            let flags = state.borrow_dirty_flags();
            assert!(flags.layout);
        }
    }

    #[test]
    fn test_config_modification_through_state() {
        let config = Arc::new(Mutex::new(Config::default()));
        let dirty_flags = Arc::new(Mutex::new(ConfigDirtyFlags::new()));

        let state = ConfigState::new(config, dirty_flags);

        let original = state.with_config(|cfg| cfg.prefer_no_csd);

        // Modify config through state
        state.with_config(|cfg| {
            cfg.prefer_no_csd = !original;
        });

        // Verify change persisted
        let new_value = state.with_config(|cfg| cfg.prefer_no_csd);
        assert_eq!(new_value, !original);
    }
}

// ============================================================================
// DirtyFlags Tests
// ============================================================================

#[cfg(test)]
mod dirty_flags_tests {
    use niri_lua_derive::DirtyFlags;

    /// Test struct for DirtyFlags derive macro
    #[derive(DirtyFlags)]
    pub struct TestDirtyFlags {
        pub layout: bool,
        pub input: bool,
        pub animations: bool,
        pub cursor: bool,
    }

    #[test]
    fn test_dirty_flags_default() {
        let flags = TestDirtyFlags::default();
        assert!(!flags.layout);
        assert!(!flags.input);
        assert!(!flags.animations);
        assert!(!flags.cursor);
    }

    #[test]
    fn test_dirty_flags_new() {
        let flags = TestDirtyFlags::new();
        assert!(!flags.layout);
        assert!(!flags.input);
        assert!(!flags.animations);
        assert!(!flags.cursor);
    }

    #[test]
    fn test_dirty_flags_mark() {
        let mut flags = TestDirtyFlags::new();

        // Mark layout dirty
        flags.mark(TestDirtyFlag::Layout);
        assert!(flags.layout);
        assert!(!flags.input);
        assert!(!flags.animations);

        // Mark input dirty
        flags.mark(TestDirtyFlag::Input);
        assert!(flags.layout);
        assert!(flags.input);
        assert!(!flags.animations);
    }

    #[test]
    fn test_dirty_flags_is_dirty() {
        let mut flags = TestDirtyFlags::new();

        assert!(!flags.is_dirty(TestDirtyFlag::Layout));

        flags.mark(TestDirtyFlag::Layout);
        assert!(flags.is_dirty(TestDirtyFlag::Layout));
        assert!(!flags.is_dirty(TestDirtyFlag::Input));
    }

    #[test]
    fn test_dirty_flags_any() {
        let mut flags = TestDirtyFlags::new();
        assert!(!flags.any());

        flags.mark(TestDirtyFlag::Layout);
        assert!(flags.any());

        flags.clear();
        assert!(!flags.any());

        flags.mark(TestDirtyFlag::Animations);
        assert!(flags.any());
    }

    #[test]
    fn test_dirty_flags_clear() {
        let mut flags = TestDirtyFlags::new();

        flags.mark(TestDirtyFlag::Layout);
        flags.mark(TestDirtyFlag::Input);
        flags.mark(TestDirtyFlag::Animations);

        assert!(flags.any());

        flags.clear();

        assert!(!flags.layout);
        assert!(!flags.input);
        assert!(!flags.animations);
        assert!(!flags.cursor);
        assert!(!flags.any());
    }

    #[test]
    fn test_dirty_flags_clear_flag() {
        let mut flags = TestDirtyFlags::new();

        flags.mark(TestDirtyFlag::Layout);
        flags.mark(TestDirtyFlag::Input);

        assert!(flags.layout);
        assert!(flags.input);

        flags.clear_flag(TestDirtyFlag::Layout);

        assert!(!flags.layout);
        assert!(flags.input);
    }

    #[test]
    fn test_dirty_flags_dirty_flags_method() {
        let mut flags = TestDirtyFlags::new();

        let dirty = flags.dirty_flags();
        assert_eq!(dirty.len(), 0);

        flags.mark(TestDirtyFlag::Layout);
        flags.mark(TestDirtyFlag::Animations);

        let dirty = flags.dirty_flags();
        assert_eq!(dirty.len(), 2);
        assert!(dirty.contains(&TestDirtyFlag::Layout));
        assert!(dirty.contains(&TestDirtyFlag::Animations));
        assert!(!dirty.contains(&TestDirtyFlag::Input));
    }

    #[test]
    fn test_dirty_flag_enum_exists() {
        // Test that the enum was generated correctly
        let _layout = TestDirtyFlag::Layout;
        let _input = TestDirtyFlag::Input;
        let _animations = TestDirtyFlag::Animations;
        let _cursor = TestDirtyFlag::Cursor;
    }

    #[test]
    fn test_dirty_flag_enum_debug() {
        let flag = TestDirtyFlag::Layout;
        let debug_str = format!("{:?}", flag);
        assert!(debug_str.contains("Layout"));
    }

    #[test]
    fn test_dirty_flag_enum_traits() {
        let flag1 = TestDirtyFlag::Layout;
        let flag2 = TestDirtyFlag::Layout;
        let flag3 = TestDirtyFlag::Input;

        // Test PartialEq
        assert_eq!(flag1, flag2);
        assert_ne!(flag1, flag3);

        // Test Clone
        let cloned = flag1;
        assert_eq!(flag1, cloned);

        // Test Copy (implicit)
        let copied = flag1;
        assert_eq!(flag1, copied);
    }

    #[test]
    fn test_all_flags_roundtrip() {
        // Test that all variants roundtrip through mark/is_dirty
        let all_flags = [
            TestDirtyFlag::Layout,
            TestDirtyFlag::Input,
            TestDirtyFlag::Animations,
            TestDirtyFlag::Cursor,
        ];

        for flag in all_flags {
            let mut flags = TestDirtyFlags::new();
            flags.mark(flag);
            assert!(flags.is_dirty(flag));
        }
    }
}

#[cfg(test)]
mod from_lua_table_tests {
    use mlua::Lua;

    use crate::extractors::FromLuaTable;

    #[derive(Default, Debug, PartialEq, niri_lua_derive::FromLuaTable)]
    #[allow(dead_code)]
    struct TestConfig {
        name: String,
        width: f64,
        enabled: bool,
        count: i32,
    }

    #[test]
    fn test_from_lua_table_basic() {
        let lua = Lua::new();
        let table = lua
            .load(
                r#"
            return {
                name = "test",
                width = 100.5,
                enabled = true,
                count = 42
            }
        "#,
            )
            .eval::<mlua::Table>()
            .unwrap();

        let config = TestConfig::from_lua_table(&table).unwrap().unwrap();
        assert_eq!(config.name, "test");
        assert!((config.width - 100.5).abs() < f64::EPSILON);
        assert!(config.enabled);
        assert_eq!(config.count, 42);
    }

    #[test]
    fn test_from_lua_table_partial() {
        let lua = Lua::new();
        let table = lua
            .load(
                r#"
            return {
                name = "partial"
            }
        "#,
            )
            .eval::<mlua::Table>()
            .unwrap();

        let config = TestConfig::from_lua_table(&table).unwrap().unwrap();
        assert_eq!(config.name, "partial");
        assert!((config.width - 0.0).abs() < f64::EPSILON);
        assert!(!config.enabled);
        assert_eq!(config.count, 0);
    }

    #[test]
    fn test_from_lua_table_empty_returns_none() {
        let lua = Lua::new();
        let table = lua.load("return {}").eval::<mlua::Table>().unwrap();

        let result = TestConfig::from_lua_table(&table).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_from_lua_table_kebab_case() {
        let lua = Lua::new();
        let table = lua
            .load(
                r#"
            return {
                ["some-value"] = "test"
            }
        "#,
            )
            .eval::<mlua::Table>()
            .unwrap();

        #[derive(Default, Debug, PartialEq, niri_lua_derive::FromLuaTable)]
        #[allow(dead_code)]
        struct KebabTest {
            some_value: String,
        }

        let config = KebabTest::from_lua_table(&table).unwrap().unwrap();
        assert_eq!(config.some_value, "test");
    }
}
