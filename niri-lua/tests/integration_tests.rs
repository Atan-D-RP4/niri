//! Integration tests for niri-lua.
//!
//! These tests verify realistic Lua execution scenarios that don't require
//! full compositor context. They test only basic Lua functionality that works
//! with bare LuaRuntime::new().

use niri_lua::LuaRuntime;

mod common;
use common::create_runtime;

// ========================================================================
// BASIC LUA EXECUTION TESTS
// ========================================================================

#[test]
fn test_closure_capture() {
    let runtime = create_runtime();
    let code = r#"
        local function make_counter(start)
            local count = start
            return function()
                count = count + 1
                return count
            end
        end
        
        local counter = make_counter(10)
        counter()
        counter()
        return counter()
    "#;
    let (output, success) = runtime.execute_string(code);
    assert!(success, "Closures should work");
    assert!(output.contains("13"), "Output: {}", output);
}

// ========================================================================
// ERROR HANDLING TESTS
// ========================================================================

#[test]
fn test_type_error_detection() {
    let runtime = create_runtime();
    let (output, success) = runtime.execute_string("return 'string' + 5");
    assert!(!success, "Type error should fail");
    assert!(
        output.contains("Error") || output.contains("attempt"),
        "Error message should be descriptive: {}",
        output
    );
}

// ========================================================================
// CONTROL FLOW TESTS
// ========================================================================

#[test]
fn test_if_else_branches() {
    let runtime = create_runtime();
    let code = r#"
        local function check(x)
            if x > 10 then
                return "big"
            elseif x > 5 then
                return "medium"
            else
                return "small"
            end
        end
        return check(3) .. " " .. check(7) .. " " .. check(15)
    "#;
    let (output, success) = runtime.execute_string(code);
    assert!(success, "Conditionals should work");
    assert!(output.contains("small"), "Output: {}", output);
    assert!(output.contains("medium"), "Output: {}", output);
    assert!(output.contains("big"), "Output: {}", output);
}

#[test]
fn test_for_loop_numeric() {
    let runtime = create_runtime();
    let code = r#"
        local sum = 0
        for i = 1, 100 do
            sum = sum + i
        end
        return sum
    "#;
    let (output, success) = runtime.execute_string(code);
    assert!(success, "For loop should work");
    assert!(output.contains("5050"), "Output: {}", output); // Sum of 1..100
}

// ========================================================================
// STANDARD LIBRARY TESTS
// ========================================================================

// ========================================================================
// EDGE CASE TESTS: NUMERIC VALUES
// ========================================================================

#[test]
fn test_large_integer() {
    let runtime = create_runtime();
    let (output, success) = runtime.execute_string("return 9007199254740992"); // 2^53
    assert!(success, "Large integer should work");
    assert!(output.contains("9007199254740992"), "Output: {}", output);
}

#[test]
fn test_float_precision() {
    let runtime = create_runtime();
    let (output, success) = runtime.execute_string("return 0.1 + 0.2");
    assert!(success, "Float math should work");
    // Check that output is close to 0.3 (floating point imprecision)
    let value: f64 = output.parse().unwrap();
    assert!(
        (value - 0.3).abs() < 0.0001,
        "Float should be ~0.3, got: {}",
        value
    );
}

#[test]
fn test_infinity_values() {
    let runtime = create_runtime();
    let (output, success) = runtime.execute_string("return math.huge, -math.huge");
    assert!(success, "Infinity should work");
    assert!(output.contains("inf"), "Output: {}", output);
}

#[test]
fn test_nan_value() {
    let runtime = create_runtime();
    let (output, success) = runtime.execute_string("return 0/0");
    assert!(success, "NaN should work");
    assert!(output.contains("nan"), "Output: {}", output);
}

#[test]
fn test_negative_numbers() {
    let runtime = create_runtime();
    let (output, success) = runtime.execute_string("return -42 + -3.14");
    assert!(success, "Negative numbers should work");
    // Result should be approximately -45.14
    assert!(output.contains("-45"), "Output: {}", output);
}

// ========================================================================
// EDGE CASE TESTS: STRING HANDLING
// ========================================================================

#[test]
fn test_unicode_strings() {
    let runtime = create_runtime();
    let (output, success) = runtime.execute_string("return 'Hello 世界 🌍'");
    assert!(success, "Unicode should work");
    assert!(output.contains("Hello 世界 🌍"), "Output: {}", output);
}

#[test]
fn test_empty_string() {
    let runtime = create_runtime();
    let (output, success) = runtime.execute_string("return ''");
    assert!(success, "Empty string should work");
    assert!(output.is_empty(), "Output: '{}'", output);
}

#[test]
fn test_string_concatenation() {
    let runtime = create_runtime();
    let code = r#"
        local result = ""
        for i = 1, 5 do
            result = result .. tostring(i)
        end
        return result
    "#;
    let (output, success) = runtime.execute_string(code);
    assert!(success, "String concatenation should work");
    assert!(output.contains("12345"), "Output: {}", output);
}

#[test]
fn test_multiline_string() {
    let runtime = create_runtime();
    let (output, success) = runtime.execute_string(r#"return "Line1\nLine2\nLine3""#);
    assert!(success, "Multiline string should work");
    assert!(output.contains("Line1"), "Output: {}", output);
}

// ========================================================================
// EDGE CASE TESTS: TABLE STRUCTURES
// ========================================================================

#[test]
fn test_empty_table() {
    let runtime = create_runtime();
    let (output, success) = runtime.execute_string("local t = {}; return #t");
    assert!(success, "Empty table should work");
    assert!(output.contains("0"), "Output: {}", output);
}

#[test]
fn test_nested_table_access() {
    let runtime = create_runtime();
    let code = r#"
        local t = {
            level1 = {
                level2 = {
                    level3 = {
                        value = 42
                    }
                }
            }
        }
        return t.level1.level2.level3.value
    "#;
    let (output, success) = runtime.execute_string(code);
    assert!(success, "Nested table access should work");
    assert!(output.contains("42"), "Output: {}", output);
}

#[test]
fn test_array_like_table() {
    let runtime = create_runtime();
    let code = r#"
        local t = {10, 20, 30, 40, 50}
        local sum = 0
        for i = 1, #t do
            sum = sum + t[i]
        end
        return sum
    "#;
    let (output, success) = runtime.execute_string(code);
    assert!(success, "Array-like table should work");
    assert!(output.contains("150"), "Output: {}", output);
}

#[test]
fn test_mixed_table() {
    let runtime = create_runtime();
    let code = r#"
        local t = {
            [1] = "first",
            [2] = "second",
            name = "test",
            value = 42
        }
        return t[1] .. " " .. t.name .. " " .. tostring(t.value)
    "#;
    let (output, success) = runtime.execute_string(code);
    assert!(success, "Mixed table should work");
    assert!(output.contains("first"), "Output: {}", output);
    assert!(output.contains("test"), "Output: {}", output);
    assert!(output.contains("42"), "Output: {}", output);
}

// ========================================================================
// PRINT OUTPUT TESTS
// ========================================================================

#[test]
fn test_print_single_value() {
    let runtime = create_runtime();
    let (output, success) = runtime.execute_string("print('hello')");
    assert!(success, "Print should succeed");
    assert!(output.contains("hello"), "Output: {}", output);
}

#[test]
fn test_print_multiple_values() {
    let runtime = create_runtime();
    let (output, success) = runtime.execute_string("print(1, 2, 3)");
    assert!(success, "Print multiple should succeed");
    assert!(output.contains("1"), "Output: {}", output);
    assert!(output.contains("2"), "Output: {}", output);
    assert!(output.contains("3"), "Output: {}", output);
}

#[test]
fn test_print_and_return() {
    let runtime = create_runtime();
    let (output, success) = runtime.execute_string("print('debug'); return 'result'");
    assert!(success, "Print and return should work");
    assert!(output.contains("debug"), "Output: {}", output);
    assert!(output.contains("result"), "Output: {}", output);
}

#[test]
fn test_multiple_print_calls() {
    let runtime = create_runtime();
    let code = r#"
        print("Line 1")
        print("Line 2")
        print("Line 3")
    "#;
    let (output, success) = runtime.execute_string(code);
    assert!(success, "Multiple prints should work");
    assert!(output.contains("Line 1"), "Output: {}", output);
    assert!(output.contains("Line 2"), "Output: {}", output);
    assert!(output.contains("Line 3"), "Output: {}", output);
}

// ========================================================================
// MULTIPLE EXECUTION TESTS
// ========================================================================

#[test]
fn test_consecutive_executions_independent() {
    let runtime = create_runtime();

    // Execute multiple independent scripts
    let (output1, success1) = runtime.execute_string("return 1 + 1");
    assert!(success1 && output1.contains("2"), "First execution failed");

    let (output2, success2) = runtime.execute_string("return 2 + 2");
    assert!(success2 && output2.contains("4"), "Second execution failed");

    let (output3, success3) = runtime.execute_string("return 3 + 3");
    assert!(success3 && output3.contains("6"), "Third execution failed");
}

#[test]
fn test_global_state_persists() {
    let runtime = create_runtime();

    // Set a global variable
    let _ = runtime.load_string("global_var = 42");

    // Access it in next execution
    let (output, success) = runtime.execute_string("return global_var");
    assert!(success, "Global state should persist");
    assert!(output.contains("42"), "Output: {}", output);
}

#[test]
fn test_error_then_success() {
    let runtime = create_runtime();

    // First execution fails
    let (_, success1) = runtime.execute_string("error('fail')");
    assert!(!success1);

    // Second execution should still work
    let (output2, success2) = runtime.execute_string("return 'success'");
    assert!(success2, "Should recover after error");
    assert!(output2.contains("success"), "Output: {}", output2);
}

// ========================================================================
// REALISTIC SCRIPTING PATTERNS
// ========================================================================

#[test]
fn test_helper_function_library_pattern() {
    let runtime = create_runtime();
    let code = r#"
        -- Define helper library
        local utils = {}
        
        function utils.clamp(value, min, max)
            if value < min then return min end
            if value > max then return max end
            return value
        end
        
        function utils.map(tbl, fn)
            local result = {}
            for i, v in ipairs(tbl) do
                table.insert(result, fn(v))
            end
            return result
        end
        
        -- Use the helpers
        local values = {-5, 10, 20, 30}
        local clamped = utils.map(values, function(v)
            return utils.clamp(v, 0, 15)
        end)
        
        return table.concat(clamped, ",")
    "#;
    let (output, success) = runtime.execute_string(code);
    assert!(success, "Helper library pattern should work");
    assert!(output.contains("0,10,15,15"), "Output: {}", output);
}

#[test]
fn test_config_builder_pattern() {
    let runtime = create_runtime();
    let code = r#"
        -- Builder pattern for configuration
        local Config = {}
        Config.__index = Config
        
        function Config.new()
            return setmetatable({
                _values = {}
            }, Config)
        end
        
        function Config:set(key, value)
            self._values[key] = value
            return self
        end
        
        function Config:get(key)
            return self._values[key]
        end
        
        -- Use the builder
        local cfg = Config.new()
            :set("name", "test")
            :set("value", 42)
            :set("enabled", true)
        
        return cfg:get("name") .. "=" .. tostring(cfg:get("value"))
    "#;
    let (output, success) = runtime.execute_string(code);
    assert!(success, "Builder pattern should work");
    assert!(output.contains("test=42"), "Output: {}", output);
}

#[test]
fn test_data_transformation_pipeline() {
    let runtime = create_runtime();
    let code = r#"
        local data = {1, 2, 3, 4, 5}
        
        -- Filter even numbers
        local filtered = {}
        for _, v in ipairs(data) do
            if v % 2 == 0 then
                table.insert(filtered, v)
            end
        end
        
        -- Double each value
        local doubled = {}
        for _, v in ipairs(filtered) do
            table.insert(doubled, v * 2)
        end
        
        return table.concat(doubled, ",")
    "#;
    let (output, success) = runtime.execute_string(code);
    assert!(success, "Data pipeline should work");
    assert!(output.contains("4,8"), "Output: {}", output);
}

// ========================================================================
// EDGE CASES: EMPTY AND NIL
// ========================================================================

#[test]
fn test_empty_string_execution() {
    let runtime = create_runtime();
    let (output, success) = runtime.execute_string("");
    assert!(success, "Empty string should succeed");
    assert!(output.is_empty(), "Empty string should produce no output");
}

#[test]
fn test_return_nil() {
    let runtime = create_runtime();
    let (output, success) = runtime.execute_string("return nil");
    assert!(success, "Return nil should succeed");
    // Nil typically produces empty output
    assert!(
        output.is_empty() || output.contains("nil"),
        "Output: '{}'",
        output
    );
}

#[test]
fn test_undefined_variable_is_nil() {
    let runtime = create_runtime();
    let (output, success) = runtime.execute_string("return undefined_variable");
    assert!(success, "Undefined variable returns nil");
    assert!(
        output.is_empty() || output.contains("nil"),
        "Output: '{}'",
        output
    );
}

// ========================================================================
// DEEP NESTING AND COMPLEXITY
// ========================================================================

#[test]
fn test_deeply_nested_calls() {
    let runtime = create_runtime();
    let code = "return (((((5 + 4) * 3) - 2) / 1) + 0)";
    let (output, success) = runtime.execute_string(code);
    assert!(success, "Nested operations should work");
    assert!(output.contains("25"), "Output: {}", output);
}

#[test]
fn test_long_computation() {
    let runtime = create_runtime();
    let code = r#"
        local result = 0
        for i = 1, 1000 do
            result = result + i
        end
        return result
    "#;
    let (output, success) = runtime.execute_string(code);
    assert!(success, "Long computation should work");
    assert!(output.contains("500500"), "Output: {}", output); // Sum 1..1000
}

#[test]
fn test_recursive_fibonacci() {
    let runtime = create_runtime();
    let code = r#"
        local function fib(n)
            if n <= 1 then return n end
            return fib(n-1) + fib(n-2)
        end
        return fib(10)
    "#;
    let (output, success) = runtime.execute_string(code);
    assert!(success, "Recursive fibonacci should work");
    assert!(output.contains("55"), "Output: {}", output);
}

// ========================================================================
// BOOLEAN AND LOGIC TESTS
// ========================================================================

#[test]
fn test_boolean_operations() {
    let runtime = create_runtime();
    let code = r#"
        local a = true
        local b = false
        if (a and not b) or (b and not a) then
            return "xor-like"
        end
        return "other"
    "#;
    let (output, success) = runtime.execute_string(code);
    assert!(success, "Boolean operations should work");
    assert!(output.contains("xor-like"), "Output: {}", output);
}

#[test]
fn test_truthiness() {
    let runtime = create_runtime();
    let code = r#"
        local results = {}
        if 0 then table.insert(results, "0") end
        if "" then table.insert(results, "empty") end
        if false then table.insert(results, "false") end
        if nil then table.insert(results, "nil") end
        if true then table.insert(results, "true") end
        return table.concat(results, ",")
    "#;
    let (output, success) = runtime.execute_string(code);
    assert!(success, "Truthiness should work");
    // In Lua, only false and nil are falsy, everything else is truthy
    assert!(output.contains("0"), "Output: {}", output);
    assert!(output.contains("empty"), "Output: {}", output);
    assert!(output.contains("true"), "Output: {}", output);
}

// ========================================================================
// VARIADIC FUNCTIONS
// ========================================================================

#[test]
fn test_variadic_function() {
    let runtime = create_runtime();
    let code = r#"
        local function sum(...)
            local s = 0
            for _, v in ipairs({...}) do
                s = s + v
            end
            return s
        end
        return sum(1, 2, 3, 4, 5)
    "#;
    let (output, success) = runtime.execute_string(code);
    assert!(success, "Variadic function should work");
    assert!(output.contains("15"), "Output: {}", output);
}

#[test]
fn test_select_builtin() {
    let runtime = create_runtime();
    let code = r#"
        local function test(...)
            return select(1, ...)
        end
        return test(100, 200, 300)
    "#;
    let (output, success) = runtime.execute_string(code);
    assert!(success, "select builtin should work");
    assert!(output.contains("100"), "Output: {}", output);
}

// ========================================================================
// METATABLES (basic test without compositor context)
// ========================================================================

#[test]
fn test_metatable_index() {
    let runtime = create_runtime();
    let code = r#"
        local mt = {
            __index = function(t, k)
                return "default"
            end
        }
        local t = setmetatable({a = 1}, mt)
        return t.a .. " " .. t.b
    "#;
    let (output, success) = runtime.execute_string(code);
    assert!(success, "Metatable __index should work");
    assert!(output.contains("1"), "Output: {}", output);
    assert!(output.contains("default"), "Output: {}", output);
}

#[test]
fn test_metatable_add() {
    let runtime = create_runtime();
    let code = r#"
        local mt = {
            __add = function(a, b)
                return {value = a.value + b.value}
            end
        }
        local a = setmetatable({value = 10}, mt)
        local b = setmetatable({value = 20}, mt)
        local c = a + b
        return c.value
    "#;
    let (output, success) = runtime.execute_string(code);
    assert!(success, "Metatable __add should work");
    assert!(output.contains("30"), "Output: {}", output);
}

// ========================================================================
// PROCESS CONTROL API TESTS
// ========================================================================

mod process_integration_tests {
    use std::thread;
    use std::time::{Duration, Instant};

    use super::*;

    fn flush_process_events(runtime: &LuaRuntime) {
        let _ = runtime.process_async();
    }

    fn wait_for_lua_condition(runtime: &LuaRuntime, script: &str, timeout: Duration) {
        let start = Instant::now();
        loop {
            flush_process_events(runtime);
            let (output, success) = runtime.execute_string(script);
            if success && output == "1" {
                break;
            }
            if Instant::now().saturating_duration_since(start) > timeout {
                panic!("timed out waiting for Lua condition: {}", script);
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

}

// ========================================================================
// REQUIRE / MODULE LOADING TESTS
// ========================================================================

mod require_tests {
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    /// Get the test fixtures directory path
    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_fixtures")
    }

    /// Get the lua modules directory within fixtures
    fn lua_modules_dir() -> PathBuf {
        fixtures_dir().join("lua")
    }

    #[test]
    fn test_require_caches_modules() {
        let runtime = create_runtime();

        // First, set up a module in the cache to simulate a loaded module
        let code = r#"
            -- Manually populate the cache to test caching behavior
            __niri_loaded["cached_module"] = { value = 42 }
            
            -- Now require should return the cached value
            local m1 = require("cached_module")
            local m2 = require("cached_module")
            
            -- Both should be the same table reference
            return m1 == m2 and m1.value == 42
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Cache test should succeed");
        assert!(output.contains("true"), "Modules should be cached: {}", output);
    }

    #[test]
    fn test_require_not_found_lists_paths() {
        let runtime = create_runtime();

        let code = r#"
            local ok, err = pcall(function()
                require("nonexistent_module_xyz")
            end)
            return tostring(err)
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "pcall should succeed");
        assert!(
            output.contains("not found"),
            "Error should mention 'not found': {}",
            output
        );
        assert!(
            output.contains("no file"),
            "Error should list searched paths: {}",
            output
        );
    }

    #[test]
    fn test_relative_require_without_context_fails() {
        let runtime = create_runtime();

        // In execute_string, there's no current file context, so relative require should fail
        let code = r#"
            local ok, err = pcall(function()
                require("./relative_module")
            end)
            return ok and "succeeded" or "failed"
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "pcall should succeed");
        assert_eq!(
            output, "failed",
            "Relative require without file context should fail"
        );
    }

    #[test]
    fn test_load_file_sets_current_file_context() {
        // Create a temporary test file
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let test_file = temp_dir.path().join("test_context.lua");
        fs::write(&test_file, "return __niri_current_file").expect("Failed to write test file");

        let runtime = create_runtime();
        let result = runtime.load_file(&test_file);

        assert!(result.is_ok(), "load_file should succeed");
        // The current file should be set to the absolute path
        let value = result.unwrap();
        let path_str = value.to_string().expect("Should be a string");
        assert!(
            path_str.contains("test_context.lua"),
            "Current file should be set: {}",
            path_str
        );
    }

    #[test]
    fn test_require_from_file_with_relative_path() {
        // Create a temp directory structure:
        // temp/
        //   main.lua       - requires ./helper
        //   helper.lua     - returns { name = "helper" }
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

        let helper_file = temp_dir.path().join("helper.lua");
        fs::write(&helper_file, r#"return { name = "helper", value = 123 }"#)
            .expect("Failed to write helper file");

        let main_file = temp_dir.path().join("main.lua");
        fs::write(
            &main_file,
            r#"
            local helper = require("./helper")
            return helper.name .. ":" .. tostring(helper.value)
        "#,
        )
        .expect("Failed to write main file");

        let runtime = create_runtime();
        let result = runtime.load_file(&main_file);

        assert!(result.is_ok(), "load_file should succeed: {:?}", result);
        let value = result.unwrap();
        let output = value.to_string().expect("Should be a string");
        assert_eq!(output, "helper:123", "Should load relative module");
    }

    #[test]
    fn test_require_with_dot_notation() {
        // Test that dot notation in module names is properly converted to path separators
        // We test this via the resolve_module function's behavior, since setting up
        // XDG_CONFIG_HOME for a full integration test is complex.
        //
        // The key behavior: require("foo.bar") should search for foo/bar.lua

        // Create a temp directory structure simulating XDG_CONFIG_HOME:
        // temp/
        //   niri/
        //     lua/
        //       nested/
        //         module.lua  - returns { deep = true }
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

        let lua_dir = temp_dir.path().join("niri").join("lua");
        let nested_dir = lua_dir.join("nested");
        fs::create_dir_all(&nested_dir).expect("Failed to create nested dir");

        let module_file = nested_dir.join("module.lua");
        fs::write(&module_file, r#"return { deep = true, name = "nested.module" }"#)
            .expect("Failed to write module file");

        // Set XDG_CONFIG_HOME to temp_dir so the custom require finds our module
        // Note: This affects the global environment, but tests run in separate processes
        env::set_var("XDG_CONFIG_HOME", temp_dir.path());

        // Create a fresh runtime after setting env var
        let runtime = create_runtime();

        // Now require("nested.module") should find temp/niri/lua/nested/module.lua
        let code = r#"
            local mod = require("nested.module")
            return mod.deep and mod.name == "nested.module" and "success" or "wrong_content"
        "#;

        let (output, success) = runtime.execute_string(code);
        assert!(
            success,
            "Dot notation require should succeed: output={}",
            output
        );
        assert_eq!(output, "success", "Module should load with dot notation");

        // Clean up env var
        env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    fn test_require_init_lua_convention() {
        // Create a temp directory structure:
        // temp/
        //   mypackage/
        //     init.lua  - returns { type = "package" }
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

        let pkg_dir = temp_dir.path().join("mypackage");
        fs::create_dir_all(&pkg_dir).expect("Failed to create package dir");

        let init_file = pkg_dir.join("init.lua");
        fs::write(&init_file, r#"return { type = "package", version = 1 }"#)
            .expect("Failed to write init.lua");

        // Test that init.lua is found when requiring a directory name
        let main_file = temp_dir.path().join("main.lua");
        fs::write(
            &main_file,
            r#"
            local pkg = require("./mypackage")
            return pkg.type .. ":" .. tostring(pkg.version)
        "#,
        )
        .expect("Failed to write main file");

        let runtime = create_runtime();
        let result = runtime.load_file(&main_file);

        assert!(result.is_ok(), "load_file should succeed: {:?}", result);
        let value = result.unwrap();
        let output = value.to_string().expect("Should be a string");
        assert_eq!(output, "package:1", "Should load init.lua from directory");
    }

    #[test]
    fn test_nested_require_preserves_context() {
        // Create a temp directory structure:
        // temp/
        //   main.lua    - requires ./level1
        //   level1.lua  - requires ./level2, returns combined
        //   level2.lua  - returns { level = 2 }
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

        let level2_file = temp_dir.path().join("level2.lua");
        fs::write(&level2_file, r#"return { level = 2 }"#).expect("Failed to write level2");

        let level1_file = temp_dir.path().join("level1.lua");
        fs::write(
            &level1_file,
            r#"
            local l2 = require("./level2")
            return { level = 1, nested = l2 }
        "#,
        )
        .expect("Failed to write level1");

        let main_file = temp_dir.path().join("main.lua");
        fs::write(
            &main_file,
            r#"
            local l1 = require("./level1")
            return l1.level .. ":" .. l1.nested.level
        "#,
        )
        .expect("Failed to write main file");

        let runtime = create_runtime();
        let result = runtime.load_file(&main_file);

        assert!(
            result.is_ok(),
            "Nested require should succeed: {:?}",
            result
        );
        let value = result.unwrap();
        let output = value.to_string().expect("Should be a string");
        assert_eq!(output, "1:2", "Nested modules should work");
    }

    #[test]
    fn test_module_returns_nil_becomes_true() {
        // Lua convention: if a module returns nil, require returns true
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

        let nil_module = temp_dir.path().join("nil_module.lua");
        fs::write(
            &nil_module,
            r#"
            -- Module that doesn't return anything (returns nil)
            local x = 1 + 1
        "#,
        )
        .expect("Failed to write nil_module");

        let main_file = temp_dir.path().join("main.lua");
        fs::write(
            &main_file,
            r#"
            local result = require("./nil_module")
            return type(result) == "boolean" and result == true
        "#,
        )
        .expect("Failed to write main file");

        let runtime = create_runtime();
        let result = runtime.load_file(&main_file);

        assert!(result.is_ok(), "load_file should succeed: {:?}", result);
        let value = result.unwrap();
        let output = value.to_string().expect("Should be a string");
        assert!(
            output.contains("true"),
            "Module returning nil should become true: {}",
            output
        );
    }

    #[test]
    fn test_require_parent_directory() {
        // Create a temp directory structure:
        // temp/
        //   shared.lua        - returns { shared = true }
        //   subdir/
        //     child.lua       - requires ../shared
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

        let shared_file = temp_dir.path().join("shared.lua");
        fs::write(&shared_file, r#"return { shared = true }"#).expect("Failed to write shared");

        let subdir = temp_dir.path().join("subdir");
        fs::create_dir_all(&subdir).expect("Failed to create subdir");

        let child_file = subdir.join("child.lua");
        fs::write(
            &child_file,
            r#"
            local shared = require("../shared")
            return shared.shared and "yes" or "no"
        "#,
        )
        .expect("Failed to write child");

        let runtime = create_runtime();
        let result = runtime.load_file(&child_file);

        assert!(
            result.is_ok(),
            "Parent directory require should succeed: {:?}",
            result
        );
        let value = result.unwrap();
        let output = value.to_string().expect("Should be a string");
        assert_eq!(output, "yes", "Should load module from parent directory");
    }
}
