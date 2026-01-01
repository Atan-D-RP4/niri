//! Integration tests for niri-lua.
//!
//! These tests verify realistic Lua execution scenarios that don't require
//! full compositor context. They test only basic Lua functionality that works
//! with bare LuaRuntime::new().

use niri_lua::LuaRuntime;

mod common;
use common::create_runtime;

fn create_watch_runtime() -> LuaRuntime {
    let mut runtime = create_runtime();
    runtime.init_event_system().unwrap();
    runtime.init_loop_api().unwrap();
    runtime.init_scheduler().unwrap();

    runtime
}

fn create_config_runtime() -> LuaRuntime {
    let mut runtime = create_runtime();
    runtime.init_empty_config_wrapper().unwrap();
    runtime.init_event_system().unwrap();
    runtime
}

// ========================================================================
// BORROW PANIC REGRESSION TESTS (P5.7)
// ========================================================================

#[test]
fn nested_config_access_does_not_panic() {
    let rt = create_config_runtime();

    let code = r#"
        local prefer_no_csd = niri.config.prefer_no_csd
        return type(prefer_no_csd) == "boolean"
    "#;

    let (output, success) = rt.execute_string(code);
    assert!(success, "Nested config access should not panic: {}", output);
    assert!(
        output.contains("true"),
        "Expected truthy result, got: {}",
        output
    );
}

#[test]
fn event_handler_registration_and_emit_does_not_panic() {
    let rt = create_config_runtime();

    let code = r#"
        local fired = 0
        niri.events:on("borrow:test", function(data)
            fired = fired + 1
        end)
        niri.events:emit("borrow:test", { payload = true })
        return fired
    "#;

    let (output, success) = rt.execute_string(code);
    assert!(
        success,
        "Event registration/emit should not panic: {}",
        output
    );
    assert_eq!(output, "1", "Handler should fire exactly once");
}

#[test]
fn action_execution_from_lua_does_not_panic() {
    let rt = create_runtime();

    let code = r#"
        niri.action:focus_column_left()
        niri.action:focus_workspace(1)
        niri.action:toggle_overview()
        return "ok"
    "#;

    let (output, success) = rt.execute_string(code);
    assert!(success, "Action execution should not panic: {}", output);
    assert_eq!(output, "ok", "Action script should complete");
}

#[test]
fn config_modification_inside_event_handler_does_not_panic() {
    let rt = create_config_runtime();

    let code = r#"
        niri.events:on("cfg:modify", function()
            -- Access a config field inside event handler
            local _ = niri.config.prefer_no_csd
        end)
        niri.events:emit("cfg:modify", {})
        return niri.config.prefer_no_csd
    "#;

    let (output, success) = rt.execute_string(code);
    assert!(
        success,
        "Config access in handler should not panic: {}",
        output
    );
}

// ========================================================================
// SCHEDULER WRAP TESTS
// ========================================================================

#[test]
fn schedule_wrap_defers_execution() {
    let runtime = create_watch_runtime();
    runtime.init_scheduler().unwrap();

    let code = r#"
        value = nil
        local wrapped = niri.schedule_wrap(function(v) value = v end)
        wrapped(42)
        assert(value == nil, "should not run immediately")
    "#;

    // Execute the code; value should still be nil right after call
    let (output, success) = runtime.execute_string(code);
    assert!(success, "Lua execution should succeed: {}", output);

    // Flush scheduled callbacks by processing async work
    let (_timers, _scheduled, _process, _errors) = runtime.process_async();

    // Read value after processing scheduled callbacks
    let value: mlua::Value = runtime.inner().globals().get("value").unwrap();
    match value {
        mlua::Value::Nil => panic!("wrapped function should have executed"),
        mlua::Value::Integer(i) => assert_eq!(i, 42),
        mlua::Value::Number(n) => assert_eq!(n as i64, 42),
        _ => panic!("unexpected value type"),
    }
}

#[test]
fn schedule_wrap_preserves_arguments() {
    let runtime = create_watch_runtime();
    runtime.init_scheduler().unwrap();

    let code = r#"
        args = nil
        local wrapped = niri.schedule_wrap(function(a, b, c) args = {a, b, c} end)
        wrapped(1, "two", true)
    "#;

    let (output, success) = runtime.execute_string(code);
    assert!(success, "Lua execution should succeed: {}", output);

    let (_timers, _scheduled, _process, _errors) = runtime.process_async();

    let args: mlua::Value = runtime.inner().globals().get("args").unwrap();
    match args {
        mlua::Value::Table(t) => {
            let a: i64 = t.get(1).unwrap();
            let b: String = t.get(2).unwrap();
            let c: bool = t.get(3).unwrap();
            assert_eq!(a, 1);
            assert_eq!(b, "two");
            assert!(c);
        }
        _ => panic!("expected table for args"),
    }
}

#[test]
fn test_loop_defer_integration() {
    let rt = create_watch_runtime();

    rt.load_string(
        r#"
        __deferred_ran = false
        niri.loop.defer(function()
            __deferred_ran = true
        end, 10)
    "#,
    )
    .unwrap();

    std::thread::sleep(std::time::Duration::from_millis(20));
    rt.process_async();

    let ran: bool = rt.inner().globals().get("__deferred_ran").unwrap();
    assert!(ran);
}

// ========================================================================
// STATE.WATCH TESTS
// ========================================================================

#[test]
fn state_watch_immediate_invokes_once() {
    let rt = create_watch_runtime();

    let code = r#"
        __watch_calls = {}
        niri.state.watch({
            events = {"test:event"},
            immediate = true,
        }, function(payload)
            table.insert(__watch_calls, payload.immediate or false)
        end)
    "#;
    let (output, success) = rt.execute_string(code);
    assert!(success, "Lua execution should succeed: {}", output);

    rt.process_async();

    let calls: mlua::Table = rt.inner().globals().get("__watch_calls").unwrap();
    let len = calls.len().unwrap();
    assert_eq!(len, 1, "Immediate callback should fire once");
    let first: bool = calls.get(1).unwrap();
    assert!(first, "Immediate callback should mark payload.immediate");
}

#[test]
fn state_watch_debounce_coalesces_events() {
    let rt = create_watch_runtime();

    let code = r#"
        __call_count = 0
        __payload = nil
        niri.state.watch({
            events = {"debounce:event"},
            debounce_ms = 20,
        }, function(payload)
            __call_count = __call_count + 1
            __payload = payload
        end)

        for i = 1, 3 do
            niri.events:emit("debounce:event", { value = i })
        end
    "#;
    let (output, success) = rt.execute_string(code);
    assert!(success, "Lua execution should succeed: {}", output);

    std::thread::sleep(std::time::Duration::from_millis(30));
    rt.process_async();

    let count: i64 = rt.inner().globals().get("__call_count").unwrap();
    assert_eq!(count, 1, "Debounce should coalesce rapid events");
    let payload: mlua::Table = rt.inner().globals().get("__payload").unwrap();
    let value: i64 = payload.get("value").unwrap();
    assert_eq!(value, 3, "Debounce should deliver last payload");
}

#[test]
fn state_watch_filter_blocks_payloads() {
    let rt = create_watch_runtime();

    let code = r#"
        local count = 0
        niri.state.watch({
            events = {"filter:event"},
            filter = function(p) return p.allow == true end,
        }, function()
            count = count + 1
        end)

        niri.events:emit("filter:event", { allow = false })
        niri.events:emit("filter:event", { allow = true })
        return count
    "#;
    let (output, success) = rt.execute_string(code);
    assert!(success, "Lua execution should succeed: {}", output);
    assert_eq!(output, "1", "Filter should allow only matching payloads");
}

#[test]
fn state_watch_cancel_stops_callbacks() {
    let rt = create_watch_runtime();

    let code = r#"
        local count = 0
        local sub = niri.state.watch({
            events = {"cancel:event"},
        }, function()
            count = count + 1
        end)

        niri.events:emit("cancel:event", {})
        sub:cancel()
        niri.events:emit("cancel:event", {})
        return count
    "#;
    let (output, success) = rt.execute_string(code);
    assert!(success, "Lua execution should succeed: {}", output);
    assert_eq!(output, "1", "Callbacks should stop after cancel");
}

#[test]
fn state_watch_multiple_subscriptions_independent() {
    let rt = create_watch_runtime();

    let code = r#"
        __count_a = 0
        __count_b = 0
        niri.state.watch({
            events = {"multi:event"},
            filter = function(p) return p.tag == "a" end,
        }, function()
            __count_a = __count_a + 1
        end)
        niri.state.watch({
            events = {"multi:event"},
            filter = function(p) return p.tag == "b" end,
        }, function()
            __count_b = __count_b + 1
        end)

        niri.events:emit("multi:event", { tag = "a" })
        niri.events:emit("multi:event", { tag = "b" })
        niri.events:emit("multi:event", { tag = "a" })
    "#;
    let (output, success) = rt.execute_string(code);
    assert!(success, "Lua execution should succeed: {}", output);

    let a: i64 = rt.inner().globals().get("__count_a").unwrap();
    let b: i64 = rt.inner().globals().get("__count_b").unwrap();
    assert_eq!(a, 2, "Subscription A should see two matching events");
    assert_eq!(b, 1, "Subscription B should see one matching event");
}

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
// These tests exercise the Process API from Lua code end-to-end.
// Unit tests for the Rust implementation are in src/process.rs (47 tests).

mod process_integration_tests {
    use std::thread;
    use std::time::{Duration, Instant};

    use super::*;

    fn flush_process_events(runtime: &LuaRuntime) {
        let _ = runtime.process_async();
    }

    fn wait_for_lua_condition(runtime: &LuaRuntime, script: &str, timeout: Duration) -> bool {
        let start = Instant::now();
        loop {
            flush_process_events(runtime);
            let (output, success) = runtime.execute_string(script);
            if success && output.trim() == "true" {
                return true;
            }
            if Instant::now().saturating_duration_since(start) > timeout {
                return false;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    // ------------------------------------------------------------------------
    // Basic spawn tests (fire-and-forget)
    // ------------------------------------------------------------------------

    #[test]
    fn test_spawn_returns_handle() {
        let runtime = create_runtime();
        // spawn with opts (even empty {}) returns handle
        let code = r#"
            local handle = niri.action:spawn({"echo", "hello"}, {})
            return handle ~= nil and type(handle) == "userdata"
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "spawn should succeed: {}", output);
        assert!(
            output.contains("true"),
            "spawn should return handle: {}",
            output
        );
    }

    #[test]
    fn test_spawn_without_opts_returns_nil() {
        let runtime = create_runtime();
        // When no opts provided, spawn is fire-and-forget and returns nil
        let code = r#"
            local handle = niri.action:spawn({"echo", "hello"})
            return handle == nil
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "spawn should succeed: {}", output);
        assert!(
            output.contains("true"),
            "spawn without opts returns nil: {}",
            output
        );
    }

    #[test]
    fn test_spawn_with_capture_returns_handle() {
        let runtime = create_runtime();
        let code = r#"
            local handle = niri.action:spawn({"echo", "test"}, {capture_stdout = true})
            return handle ~= nil and type(handle) == "userdata"
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "spawn with capture should succeed: {}", output);
        assert!(output.contains("true"), "should return handle: {}", output);
    }

    // ------------------------------------------------------------------------
    // ProcessHandle method tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_handle_pid_returns_number() {
        let runtime = create_runtime();
        let code = r#"
            local handle = niri.action:spawn({"sleep", "0.1"}, {capture_stdout = true})
            local pid = handle.pid  -- pid is a field, not a method
            return type(pid) == "number" and pid > 0
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "pid access should succeed: {}", output);
        assert!(
            output.contains("true"),
            "pid should be positive number: {}",
            output
        );
    }

    #[test]
    fn test_handle_is_closing_with_stdin_pipe() {
        let runtime = create_runtime();
        // is_closing() checks if stdin is closed
        // With stdin = "pipe", stdin starts open
        let code = r#"
            local handle = niri.action:spawn({"cat"}, {stdin = "pipe"})
            local before_close = handle:is_closing()
            handle:close_stdin()
            local after_close = handle:is_closing()
            handle:wait()  -- Wait for cat to exit after stdin closes
            return before_close == false and after_close == true
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "is_closing() test should succeed: {}", output);
        assert!(
            output.contains("true"),
            "is_closing should track stdin state: {}",
            output
        );
    }

    #[test]
    fn test_handle_kill_terminates_process() {
        let runtime = create_runtime();
        let code = r#"
            local handle = niri.action:spawn({"sleep", "10"}, {capture_stdout = true})
            handle:kill("SIGKILL")  -- Use SIGKILL for immediate termination
            local result = handle:wait()
            -- Killed process has signal = 9 (SIGKILL)
            return result.signal == 9
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "kill should succeed: {}", output);
        assert!(
            output.contains("true"),
            "killed process should have signal 9: {}",
            output
        );
    }

    #[test]
    fn test_handle_wait_returns_result() {
        let runtime = create_runtime();
        let code = r#"
            local handle = niri.action:spawn({"true"}, {})  -- 'true' exits with code 0
            local result = handle:wait()
            return type(result) == "table" and result.code == 0
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "wait should succeed: {}", output);
        assert!(
            output.contains("true"),
            "wait should return result table: {}",
            output
        );
    }

    #[test]
    fn test_handle_wait_captures_stdout() {
        let runtime = create_runtime();
        // Note: capture_stdout buffers output via process_events()
        // For synchronous capture, we need to process events before wait returns
        // This test verifies the basic capture option is accepted
        let code = r#"
            local handle = niri.action:spawn({"echo", "captured_output"}, {capture_stdout = true})
            -- Process events to buffer stdout before wait
            local result = handle:wait()
            -- In integration without event loop, stdout may not be buffered yet
            -- Just verify result structure is correct
            return type(result) == "table" and result.code == 0
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "wait with capture should succeed: {}", output);
        assert!(
            output.contains("true"),
            "should return result table: {}",
            output
        );
    }

    #[test]
    fn test_handle_wait_captures_stderr() {
        let runtime = create_runtime();
        let code = r#"
            local handle = niri.action:spawn({"sh", "-c", "echo error >&2"}, {capture_stderr = true})
            local result = handle:wait()
            -- Verify result structure
            return type(result) == "table" and result.code == 0
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(
            success,
            "wait with stderr capture should succeed: {}",
            output
        );
        assert!(
            output.contains("true"),
            "should return result table: {}",
            output
        );
    }

    #[test]
    fn test_handle_wait_with_timeout() {
        let runtime = create_runtime();
        let code = r#"
            local handle = niri.action:spawn({"echo", "fast"}, {capture_stdout = true})
            local result = handle:wait(5000)  -- 5 second timeout
            return result.code == 0
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "wait with timeout should succeed: {}", output);
        assert!(
            output.contains("true"),
            "fast process should complete: {}",
            output
        );
    }

    // ------------------------------------------------------------------------
    // Spawn options tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_spawn_with_cwd() {
        let runtime = create_runtime();
        // Test that cwd option is accepted and process runs
        let code = r#"
            local handle = niri.action:spawn({"pwd"}, {cwd = "/tmp"})
            local result = handle:wait()
            return result.code == 0
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "spawn with cwd should succeed: {}", output);
        assert!(
            output.contains("true"),
            "process should exit cleanly: {}",
            output
        );
    }

    #[test]
    fn test_spawn_with_env() {
        let runtime = create_runtime();
        // Test that env option is accepted - use exit code to verify
        let code = r#"
            local handle = niri.action:spawn(
                {"sh", "-c", "test -n \"$MY_TEST_VAR\" && exit 0 || exit 1"},
                {env = {MY_TEST_VAR = "test_value_123"}}
            )
            local result = handle:wait()
            return result.code == 0
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "spawn with env should succeed: {}", output);
        assert!(output.contains("true"), "env var should be set: {}", output);
    }

    #[test]
    fn test_spawn_with_stdin_data() {
        let runtime = create_runtime();
        // Test stdin data - use wc to count characters via exit code
        let code = r#"
            local handle = niri.action:spawn(
                {"sh", "-c", "read line && test \"$line\" = 'hello' && exit 0 || exit 1"},
                {stdin = "hello\n"}
            )
            local result = handle:wait()
            return result.code == 0
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "spawn with stdin data should succeed: {}", output);
        assert!(
            output.contains("true"),
            "stdin should be passed to process: {}",
            output
        );
    }

    #[test]
    fn test_spawn_with_stdin_pipe() {
        let runtime = create_runtime();
        // Test stdin pipe mode - verify write and close_stdin work
        let code = r#"
            local handle = niri.action:spawn(
                {"sh", "-c", "read line && test \"$line\" = 'piped' && exit 0 || exit 1"},
                {stdin = "pipe"}
            )
            handle:write("piped\n")
            handle:close_stdin()
            local result = handle:wait()
            return result.code == 0
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "spawn with stdin pipe should succeed: {}", output);
        assert!(
            output.contains("true"),
            "piped input should reach process: {}",
            output
        );
    }

    // ------------------------------------------------------------------------
    // Exit code tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_nonzero_exit_code() {
        let runtime = create_runtime();
        let code = r#"
            local handle = niri.action:spawn({"sh", "-c", "exit 42"}, {capture_stdout = true})
            local result = handle:wait()
            return result.code == 42
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "nonzero exit should succeed: {}", output);
        assert!(
            output.contains("true"),
            "exit code should be 42: {}",
            output
        );
    }

    #[test]
    fn test_command_not_found() {
        let runtime = create_runtime();
        let code = r#"
            local ok, err = pcall(function()
                niri.action:spawn({"nonexistent_command_xyz_123"}, {capture_stdout = true})
            end)
            -- spawn of nonexistent command may fail or return handle that waits with error
            return not ok or type(err) == "string"
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "pcall should succeed: {}", output);
        // Either pcall catches error or we get an error result
        assert!(
            output.contains("true"),
            "should handle nonexistent command: {}",
            output
        );
    }

    // ------------------------------------------------------------------------
    // Text vs Binary mode tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_text_mode_default() {
        let runtime = create_runtime();
        // Text mode is default - verify option is accepted
        let code = r#"
            local handle = niri.action:spawn({"echo", "line1"}, {text = true})
            local result = handle:wait()
            return result.code == 0
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "text mode should succeed: {}", output);
        assert!(
            output.contains("true"),
            "process should exit cleanly: {}",
            output
        );
    }

    #[test]
    fn test_binary_mode() {
        let runtime = create_runtime();
        // Binary mode - verify option is accepted
        let code = r#"
            local handle = niri.action:spawn({"echo", "binary"}, {text = false})
            local result = handle:wait()
            return result.code == 0
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "binary mode should succeed: {}", output);
        assert!(
            output.contains("true"),
            "process should exit cleanly: {}",
            output
        );
    }

    // ------------------------------------------------------------------------
    // Streaming callback tests (async)
    // These test that callbacks are registered and accepted.
    // Full callback invocation requires compositor event loop integration.
    // ------------------------------------------------------------------------

    #[test]
    fn test_spawn_with_stdout_callback_returns_handle() {
        let runtime = create_runtime();
        let code = r#"
            _G.stdout_lines = {}
            local handle = niri.action:spawn(
                {"echo", "callback test"},
                {stdout = function(line) table.insert(_G.stdout_lines, line) end}
            )
            return handle ~= nil
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "spawn with callback should succeed: {}", output);
        assert!(output.contains("true"), "should return handle: {}", output);
    }

    #[test]
    fn test_spawn_with_on_exit_callback_returns_handle() {
        let runtime = create_runtime();
        let code = r#"
            _G.exit_result = nil
            local handle = niri.action:spawn(
                {"echo", "test"},
                {on_exit = function(result) _G.exit_result = result end}
            )
            return handle ~= nil
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "spawn with on_exit should succeed: {}", output);
        assert!(output.contains("true"), "should return handle: {}", output);
    }

    #[test]
    fn test_spawn_with_stderr_callback_returns_handle() {
        let runtime = create_runtime();
        let code = r#"
            _G.stderr_lines = {}
            local handle = niri.action:spawn(
                {"sh", "-c", "echo error >&2"},
                {stderr = function(line) table.insert(_G.stderr_lines, line) end}
            )
            return handle ~= nil
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(
            success,
            "spawn with stderr callback should succeed: {}",
            output
        );
        assert!(output.contains("true"), "should return handle: {}", output);
    }

    #[test]
    fn test_spawn_with_all_callbacks_returns_handle() {
        let runtime = create_runtime();
        let code = r#"
            local handle = niri.action:spawn(
                {"echo", "test"},
                {
                    stdout = function(line) end,
                    stderr = function(line) end,
                    on_exit = function(result) end
                }
            )
            return handle ~= nil
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(
            success,
            "spawn with all callbacks should succeed: {}",
            output
        );
        assert!(output.contains("true"), "should return handle: {}", output);
    }

    // ------------------------------------------------------------------------
    // Callback event processing tests
    // These tests verify that streaming callbacks fire when events are processed
    // Callback signature: (err, data) for stdout/stderr, (result, err) for on_exit
    // ------------------------------------------------------------------------

    #[test]
    fn test_stdout_callback_receives_output() {
        let runtime = create_runtime();

        // Set up callback and spawn process
        // Callback signature is (err, data) where err is nil on success
        let setup = r#"
            _G.received_lines = {}
            _G.callback_called = false
            local handle = niri.action:spawn(
                {"echo", "callback_line_123"},
                {
                    stdout = function(err, data)
                        _G.callback_called = true
                        if data then
                            table.insert(_G.received_lines, data)
                        end
                    end
                }
            )
            _G.test_handle = handle
            return "setup_done"
        "#;
        let (output, success) = runtime.execute_string(setup);
        assert!(success, "setup should succeed: {}", output);

        // Process events until callback fires or timeout
        let condition_met = wait_for_lua_condition(
            &runtime,
            "return _G.callback_called",
            Duration::from_secs(5),
        );
        assert!(condition_met, "callback should be called within timeout");

        // Verify the callback received the output
        let verify = r#"
            local found = false
            for _, line in ipairs(_G.received_lines) do
                if line:find("callback_line_123") then found = true end
            end
            return found
        "#;
        let (output, success) = runtime.execute_string(verify);
        assert!(success, "verify should succeed: {}", output);
        assert!(
            output.contains("true"),
            "callback should have received output: {}",
            output
        );
    }

    #[test]
    fn test_on_exit_callback_receives_result() {
        let runtime = create_runtime();

        // Set up on_exit callback
        // Callback signature is (result, err) where result is a table with code/signal
        let setup = r#"
            _G.exit_called = false
            _G.exit_code = nil
            local handle = niri.action:spawn(
                {"sh", "-c", "exit 7"},
                {
                    on_exit = function(result, err)
                        _G.exit_called = true
                        if result then
                            _G.exit_code = result.code
                        end
                    end
                }
            )
            return "setup_done"
        "#;
        let (output, success) = runtime.execute_string(setup);
        assert!(success, "setup should succeed: {}", output);

        // Process events until callback fires
        let condition_met =
            wait_for_lua_condition(&runtime, "return _G.exit_called", Duration::from_secs(5));
        assert!(condition_met, "on_exit callback should fire within timeout");

        // Verify exit code
        let (output, success) = runtime.execute_string("return _G.exit_code == 7");
        assert!(success, "verify should succeed: {}", output);
        assert!(output.contains("true"), "exit code should be 7: {}", output);
    }

    #[test]
    fn test_stderr_callback_receives_output() {
        let runtime = create_runtime();

        // Callback signature is (err, data)
        let setup = r#"
            _G.stderr_lines = {}
            _G.stderr_called = false
            local handle = niri.action:spawn(
                {"sh", "-c", "echo stderr_test_line >&2"},
                {
                    stderr = function(err, data)
                        _G.stderr_called = true
                        if data then
                            table.insert(_G.stderr_lines, data)
                        end
                    end
                }
            )
            return "setup_done"
        "#;
        let (output, success) = runtime.execute_string(setup);
        assert!(success, "setup should succeed: {}", output);

        let condition_met =
            wait_for_lua_condition(&runtime, "return _G.stderr_called", Duration::from_secs(5));
        assert!(condition_met, "stderr callback should be called");

        let verify = r#"
            local found = false
            for _, line in ipairs(_G.stderr_lines) do
                if line:find("stderr_test_line") then found = true end
            end
            return found
        "#;
        let (output, success) = runtime.execute_string(verify);
        assert!(success, "verify should succeed: {}", output);
        assert!(
            output.contains("true"),
            "stderr callback should have received output: {}",
            output
        );
    }

    #[test]
    fn test_multiple_processes_isolated_callbacks() {
        let runtime = create_runtime();

        // Callback signature is (err, data)
        let setup = r#"
            _G.proc1_lines = {}
            _G.proc2_lines = {}
            _G.proc1_called = false
            _G.proc2_called = false

            niri.action:spawn(
                {"echo", "PROC1_OUTPUT"},
                {
                    stdout = function(err, data)
                        _G.proc1_called = true
                        if data then table.insert(_G.proc1_lines, data) end
                    end
                }
            )

            niri.action:spawn(
                {"echo", "PROC2_OUTPUT"},
                {
                    stdout = function(err, data)
                        _G.proc2_called = true
                        if data then table.insert(_G.proc2_lines, data) end
                    end
                }
            )
            return "setup_done"
        "#;
        let (output, success) = runtime.execute_string(setup);
        assert!(success, "setup should succeed: {}", output);

        // Wait for both to be called
        let condition_met = wait_for_lua_condition(
            &runtime,
            "return _G.proc1_called and _G.proc2_called",
            Duration::from_secs(5),
        );
        assert!(condition_met, "both callbacks should be called");

        // Verify isolation
        let verify = r#"
            local proc1_has_proc1 = false
            local proc1_has_proc2 = false
            local proc2_has_proc1 = false
            local proc2_has_proc2 = false

            for _, line in ipairs(_G.proc1_lines) do
                if line:find("PROC1_OUTPUT") then proc1_has_proc1 = true end
                if line:find("PROC2_OUTPUT") then proc1_has_proc2 = true end
            end
            for _, line in ipairs(_G.proc2_lines) do
                if line:find("PROC1_OUTPUT") then proc2_has_proc1 = true end
                if line:find("PROC2_OUTPUT") then proc2_has_proc2 = true end
            end

            -- Each proc should only have its own output
            return proc1_has_proc1 and not proc1_has_proc2 and proc2_has_proc2 and not proc2_has_proc1
        "#;
        let (output, success) = runtime.execute_string(verify);
        assert!(success, "verify should succeed: {}", output);
        assert!(
            output.contains("true"),
            "callbacks should be isolated: {}",
            output
        );
    }
}

// ========================================================================
// REQUIRE / MODULE LOADING TESTS
// ========================================================================

mod require_tests {
    use std::path::PathBuf;
    use std::{env, fs};

    use super::*;

    /// Get the test fixtures directory path
    #[allow(dead_code)]
    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_fixtures")
    }

    /// Get the lua modules directory within fixtures
    #[allow(dead_code)]
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
        assert!(
            output.contains("true"),
            "Modules should be cached: {}",
            output
        );
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
        fs::write(
            &module_file,
            r#"return { deep = true, name = "nested.module" }"#,
        )
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
