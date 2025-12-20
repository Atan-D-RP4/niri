//! Integration tests for niri-lua.
//!
//! These tests verify realistic Lua execution scenarios that don't require
//! full compositor context. They test only basic Lua functionality that works
//! with bare LuaRuntime::new().

use niri_lua::LuaRuntime;

mod common;
use common::{create_runtime, create_runtime_with_process_api};

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

    #[test]
    fn test_spawn_wait_returns_stdout_and_exit_code() {
        let (runtime, _manager) = create_runtime_with_process_api();
        let (output, success) = runtime.execute_string(
            r#"
                local proc = niri.process.spawn({"printf", "ready"}, {})
                local result = proc:wait()
                return string.format("%d:%s", result.code, result.stdout)
            "#,
        );
        assert!(success, "spawn should succeed");
        assert_eq!(
            output, "0:ready",
            "wait should capture exit code and stdout"
        );
    }

    #[test]
    fn test_detach_returns_nil_handle() {
        let (runtime, _manager) = create_runtime_with_process_api();
        let (output, success) = runtime.execute_string(
            r#"
                local handle = niri.process.spawn({"true"}, { detach = true })
                return handle == nil and "true" or "false"
            "#,
        );
        assert!(success, "detach spawn should succeed");
        assert_eq!(output, "true", "detach mode must not return a handle");
    }

    #[test]
    fn test_spawn_with_stdout_capture() {
        // Test that spawn with stdout=true captures output via wait()
        // This tests the synchronous path which is more reliable in unit tests
        let (runtime, _manager) = create_runtime_with_process_api();

        let (output, success) = runtime.execute_string(
            r#"
                local handle = niri.process.spawn({"echo", "hello"}, {
                    stdout = true
                })
                local result = handle:wait(5000)
                return result.stdout
            "#,
        );
        assert!(success, "spawn with wait should succeed");
        assert_eq!(output.trim(), "hello", "should capture stdout");
    }

    #[test]
    fn test_spawn_respects_env_and_cwd_options() {
        let (runtime, _manager) = create_runtime_with_process_api();
        let (output, success) = runtime.execute_string(
            r#"
                local proc = niri.process.spawn({"/bin/sh", "-c", "printf '%s:%s' \"$MY_VAR\" \"$(pwd)\""}, {
                    env = { MY_VAR = "value-123" },
                    cwd = "/tmp",
                })
                local result = proc:wait()
                return result.stdout
            "#,
        );
        assert!(success, "env/cwd spawn should succeed");
        assert_eq!(output, "value-123:/tmp", "env var and cwd should propagate");
    }
}
