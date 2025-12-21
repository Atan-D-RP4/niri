//! Integration tests for the Lua REPL
//!
//! These tests verify that the REPL can execute Lua code and properly capture output.

mod common;
use common::create_runtime;

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::create_runtime;

    #[test]
    fn test_execute_simple_arithmetic() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return 2 + 2");

        assert!(success, "Arithmetic should execute successfully");
        assert!(output.contains("4"), "Output should contain the result 4");
    }

    #[test]
    fn test_execute_with_print() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("print('Hello'); print('World')");

        assert!(success, "Code with print should execute successfully");
        assert!(output.contains("Hello"), "Output should contain 'Hello'");
        assert!(output.contains("World"), "Output should contain 'World'");
    }

    #[test]
    fn test_execute_with_error() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("error('test error')");

        assert!(!success, "Code with error should fail");
        assert!(output.contains("Error"), "Output should contain 'Error'");
    }

    #[test]
    fn test_execute_syntax_error() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("print(1 +");

        assert!(!success, "Syntax error should fail");
        assert!(
            output.contains("Error"),
            "Output should contain error message"
        );
    }

    #[test]
    fn test_execute_with_variables() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("local x = 10; local y = 20; return x + y");

        assert!(success, "Code with variables should execute successfully");
        assert!(output.contains("30"), "Output should contain 30");
    }

    #[test]
    fn test_execute_with_table() {
        let runtime = create_runtime();
        let (output, success) =
            runtime.execute_string("local t = {a = 1, b = 2}; return t.a + t.b");

        assert!(success, "Code with tables should execute successfully");
        assert!(output.contains("3"), "Output should contain 3");
    }

    #[test]
    fn test_execute_with_function() {
        let runtime = create_runtime();
        let code = r#"
local function add(a, b)
    return a + b
end
return add(5, 7)
        "#;
        let (output, success) = runtime.execute_string(code);

        assert!(success, "Code with function should execute successfully");
        assert!(output.contains("12"), "Output should contain 12");
    }

    #[test]
    fn test_execute_with_loop() {
        let runtime = create_runtime();
        let code = r#"
local sum = 0
for i = 1, 5 do
    sum = sum + i
end
return sum
        "#;
        let (output, success) = runtime.execute_string(code);

        assert!(success, "Code with loop should execute successfully");
        assert!(
            output.contains("15"),
            "Output should contain 15 (sum of 1..5)"
        );
    }

    #[test]
    fn test_execute_with_multiple_prints() {
        let runtime = create_runtime();
        let code = r#"
print("Line 1")
print("Line 2")
print("Line 3")
        "#;
        let (output, success) = runtime.execute_string(code);

        assert!(
            success,
            "Code with multiple prints should execute successfully"
        );
        assert!(output.contains("Line 1"), "Output should contain 'Line 1'");
        assert!(output.contains("Line 2"), "Output should contain 'Line 2'");
        assert!(output.contains("Line 3"), "Output should contain 'Line 3'");
    }

    #[test]
    fn test_execute_empty_code() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("");

        assert!(success, "Empty code should succeed");
        assert!(output.is_empty(), "Empty code should produce no output");
    }

    #[test]
    fn test_execute_nil_return() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return nil");

        assert!(success, "Returning nil should succeed");
        // Nil return should have no output since we skip nil strings
        assert!(
            output.is_empty() || output.contains("nil"),
            "Output should be empty or contain nil"
        );
    }

    #[test]
    fn test_execute_string_with_special_chars() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("print('Hello\\nWorld')");

        assert!(success, "Code with escaped characters should execute");
        assert!(output.contains("Hello"), "Output should contain 'Hello'");
    }

    #[test]
    fn test_ipc_executor_integration() {
        use niri_lua::IpcLuaExecutor;

        let runtime = create_runtime();
        #[allow(clippy::arc_with_non_send_sync)]
        let executor = IpcLuaExecutor::new(Arc::new(Mutex::new(Some(runtime))));

        let (output, success) = executor.execute("return 1 + 1");
        assert!(success, "Executor should succeed");
        assert!(output.contains("2"), "Output should contain 2");
    }

    #[test]
    fn test_ipc_executor_with_error() {
        use niri_lua::IpcLuaExecutor;

        let runtime = create_runtime();
        #[allow(clippy::arc_with_non_send_sync)]
        let executor = IpcLuaExecutor::new(Arc::new(Mutex::new(Some(runtime))));
        let (output, success) = executor.execute("error('test')");
        assert!(!success, "Executor should fail on error");
        assert!(output.contains("Error"), "Output should contain error");
    }

    #[test]
    fn test_ipc_executor_no_runtime() {
        use niri_lua::IpcLuaExecutor;

        #[allow(clippy::arc_with_non_send_sync)]
        let executor = IpcLuaExecutor::new(Arc::new(Mutex::new(None)));
        let (output, success) = executor.execute("print('test')");

        assert!(!success, "Executor should fail without runtime");
        assert!(
            output.contains("not initialized"),
            "Output should indicate runtime not initialized"
        );
    }

    #[test]
    fn test_consecutive_executions() {
        let runtime = create_runtime();

        // First execution
        let (output1, success1) = runtime.execute_string("return 1");
        assert!(success1);
        assert!(output1.contains("1"));

        // Second execution - should work fine
        let (output2, success2) = runtime.execute_string("return 2");
        assert!(success2);
        assert!(output2.contains("2"));

        // Third execution - should work fine
        let (output3, success3) = runtime.execute_string("return 3");
        assert!(success3);
        assert!(output3.contains("3"));
    }

    #[test]
    fn test_multiline_code() {
        let runtime = create_runtime();
        let code = "local x = 10\nlocal y = 20\nprint(x + y)";
        let (output, success) = runtime.execute_string(code);

        assert!(success, "Multiline code should execute");
        assert!(output.contains("30"), "Output should contain 30");
    }

    #[test]
    fn test_comment_handling() {
        let runtime = create_runtime();
        let code = "-- This is a comment\nprint('Hello') -- inline comment";
        let (output, success) = runtime.execute_string(code);

        assert!(success, "Code with comments should execute");
        assert!(output.contains("Hello"), "Output should contain 'Hello'");
    }

    #[test]
    fn test_string_output_with_numbers() {
        let runtime = create_runtime();
        let code = "print(42); print(3.14); print(true); print(false)";
        let (output, success) = runtime.execute_string(code);

        assert!(success, "Code with various types should execute");
        assert!(output.contains("42"), "Output should contain 42");
        assert!(output.contains("3.14"), "Output should contain 3.14");
        assert!(output.contains("true"), "Output should contain true");
        assert!(output.contains("false"), "Output should contain false");
    }

    // --- New comprehensive output formatting tests ---

    #[test]
    fn test_return_simple_string() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return 'hello'");
        assert!(success, "Should execute successfully");
        assert_eq!(output, "hello", "Simple string should be returned as-is");
    }

    #[test]
    fn test_return_number() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return 42");
        assert!(success, "Should execute successfully");
        assert_eq!(output, "42", "Number should be converted to string");
    }

    #[test]
    fn test_return_float() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return 3.14");
        assert!(success, "Should execute successfully");
        assert_eq!(output, "3.14", "Float should be converted to string");
    }

    #[test]
    fn test_return_boolean_true() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return true");
        assert!(success, "Should execute successfully");
        assert_eq!(output, "true", "Boolean true should be converted to 'true'");
    }

    #[test]
    fn test_return_boolean_false() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return false");
        assert!(success, "Should execute successfully");
        assert_eq!(
            output, "false",
            "Boolean false should be converted to 'false'"
        );
    }

    #[test]
    fn test_return_nil() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return nil");
        assert!(success, "Should execute successfully");
        assert!(
            output.is_empty(),
            "nil should produce empty output, got: '{}'",
            output
        );
    }

    #[test]
    fn test_return_simple_array_table() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return {1, 2, 3}");
        assert!(success, "Should execute successfully");
        println!("Simple array table output: '{}'", output);
        // Tables are now pretty-printed like vim.print()
        assert!(output.contains("1"), "Should contain 1");
        assert!(output.contains("2"), "Should contain 2");
        assert!(output.contains("3"), "Should contain 3");
    }

    #[test]
    fn test_return_table_with_string_keys() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return {name = 'test', value = 42}");
        assert!(success, "Should execute successfully");
        println!("Table with string keys output: '{}'", output);
        // Tables are now pretty-printed like vim.print()
        assert!(output.contains("name"), "Should contain key 'name'");
        assert!(output.contains("test"), "Should contain value 'test'");
        assert!(output.contains("value"), "Should contain key 'value'");
        assert!(output.contains("42"), "Should contain value 42");
    }

    #[test]
    fn test_return_nested_table() {
        let runtime = create_runtime();
        let (output, success) =
            runtime.execute_string("return {{id = 1, name = 'a'}, {id = 2, name = 'b'}}");
        assert!(success, "Should execute successfully");
        println!("Nested table output: '{}'", output);
        // Tables are now pretty-printed like vim.print()
        assert!(output.contains("id"), "Should contain key 'id'");
        assert!(output.contains("name"), "Should contain key 'name'");
    }

    #[test]
    fn test_return_empty_table() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return {}");
        assert!(success, "Should execute successfully");
        println!("Empty table output: '{}'", output);
        // Empty tables are formatted as {}
        assert!(
            output.contains("{}"),
            "Empty table should be formatted as {{}}"
        );
    }

    #[test]
    fn test_return_large_array() {
        let runtime = create_runtime();
        let code = "local t = {} for i=1,100 do t[i] = i end return t";
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        println!("Large array output length: {}", output.len());
        // Tables are now pretty-printed - large arrays should have content
        assert!(!output.is_empty(), "Large array should produce output");
        assert!(output.contains("1"), "Should contain first element");
    }

    #[test]
    fn test_return_large_complex_table() {
        let runtime = create_runtime();
        let code =
            "local t = {} for i=1,50 do t[i] = {id=i, value='item'..i, active=i%2==0} end return t";
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        println!("Large complex table output length: {}", output.len());
        // Tables are now pretty-printed
        assert!(
            !output.is_empty(),
            "Large complex table should produce output"
        );
        assert!(output.contains("id"), "Should contain 'id' key");
    }

    #[test]
    fn test_table_representation_includes_structure() {
        let runtime = create_runtime();
        // Tables are now pretty-printed like vim.print()
        let (output, success) = runtime.execute_string("return {a=1, b={c=2}}");
        assert!(success, "Should execute successfully");
        println!("Table structure output: '{}'", output);
        assert!(output.contains("a"), "Should contain key 'a'");
        assert!(output.contains("b"), "Should contain key 'b'");
        assert!(output.contains("c"), "Should contain nested key 'c'");
    }

    #[test]
    fn test_no_explicit_return() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("local x = 5 + 3");
        assert!(success, "Should execute successfully");
        println!("No explicit return output: '{}'", output);
        // No explicit return should produce empty output
        assert!(
            output.is_empty(),
            "No explicit return should produce empty output"
        );
    }

    #[test]
    fn test_print_and_return_value() {
        let runtime = create_runtime();
        let code = "print('First'); print('Second'); return 'value'";
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        println!("Print and return output: '{}'", output);
        // Should contain both printed lines and return value
        assert!(output.contains("First"), "Should contain first print");
        assert!(output.contains("Second"), "Should contain second print");
        assert!(output.contains("value"), "Should contain return value");
    }

    #[test]
    fn test_print_with_multiple_args() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("print('hello', 'world', 42, true)");
        assert!(success, "Should execute successfully");
        println!("Print with multiple args: '{}'", output);
        assert!(output.contains("hello"), "Should contain first arg");
        // Multiple args separated by tabs or similar
        assert!(output.len() > 5, "Output should contain multiple arguments");
    }

    #[test]
    fn test_print_numbers_and_types() {
        let runtime = create_runtime();
        let code = r#"
            print(42)
            print(3.14)
            print(true)
            print(false)
            print(nil)
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        println!("Print numbers and types: '{}'", output);
        assert!(output.contains("42"), "Should print integer");
        assert!(output.contains("3.14"), "Should print float");
        assert!(output.contains("true"), "Should print boolean true");
        assert!(output.contains("false"), "Should print boolean false");
    }

    #[test]
    fn test_error_with_message() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("error('custom error message')");
        assert!(!success, "Should fail on error");
        println!("Error output: '{}'", output);
        assert!(output.contains("Error"), "Should contain 'Error'");
        assert!(
            output.contains("custom"),
            "Should contain custom error message"
        );
    }

    #[test]
    fn test_syntax_error_output() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return 1 + + 2");
        assert!(!success, "Should fail on syntax error");
        println!("Syntax error output: '{}'", output);
        assert!(output.contains("Error"), "Should contain error message");
    }

    #[test]
    fn test_runtime_error_output() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return 5 + 'string'");
        assert!(!success, "Should fail on runtime type error");
        println!("Runtime error output: '{}'", output);
        assert!(output.contains("Error"), "Should contain error message");
    }

    #[test]
    fn test_consecutive_print_calls() {
        let runtime = create_runtime();
        let code = r#"
            print("A")
            print("B")
            print("C")
            print("D")
            print("E")
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        println!("Consecutive print output: '{}'", output);
        assert!(output.contains("A"), "Should contain A");
        assert!(output.contains("B"), "Should contain B");
        assert!(output.contains("C"), "Should contain C");
        assert!(output.contains("D"), "Should contain D");
        assert!(output.contains("E"), "Should contain E");
    }

    #[test]
    fn test_print_concatenated_strings() {
        let runtime = create_runtime();
        let code = "print('Hello ' .. 'World ' .. '2025')";
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        println!("Concatenated string output: '{}'", output);
        assert!(output.contains("Hello"), "Should contain Hello");
        assert!(output.contains("World"), "Should contain World");
    }

    #[test]
    fn test_return_long_string() {
        let runtime = create_runtime();
        let code = "return 'Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.'";
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        assert!(
            output.contains("Lorem"),
            "Should contain part of long string"
        );
    }

    #[test]
    fn test_return_multiline_string() {
        let runtime = create_runtime();
        let code = r#"return "Line 1\nLine 2\nLine 3""#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        println!("Multiline string output: '{}'", output);
        assert!(!output.is_empty(), "Should have output");
    }

    // ========================================================================
    // Edge Case Tests: Numeric Values
    // ========================================================================

    #[test]
    fn test_return_zero() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return 0");
        assert!(success, "Should execute successfully");
        assert_eq!(output, "0", "Zero should format as '0'");
    }

    #[test]
    fn test_return_negative_integer() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return -42");
        assert!(success, "Should execute successfully");
        assert_eq!(output, "-42", "Negative integer should format correctly");
    }

    #[test]
    fn test_return_large_integer() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return 1000000000000");
        assert!(success, "Should execute successfully");
        assert_eq!(
            output, "1000000000000",
            "Large integer should format correctly"
        );
    }

    #[test]
    fn test_return_small_float() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return 0.5");
        assert!(success, "Should execute successfully");
        assert_eq!(output, "0.5", "Small float should format correctly");
    }

    #[test]
    fn test_return_very_small_float() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return 0.000001");
        assert!(success, "Should execute successfully");
        // Lua may format as scientific notation or decimal
        assert!(
            output == "0.000001" || output == "1e-06",
            "Very small float should format correctly, got: {}",
            output
        );
    }

    #[test]
    fn test_return_negative_float() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return -3.14159");
        assert!(success, "Should execute successfully");
        assert!(
            output.contains("-3.14"),
            "Negative float should format correctly"
        );
    }

    #[test]
    fn test_return_infinity() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return math.huge");
        assert!(success, "Should execute successfully");
        assert_eq!(output, "inf", "Infinity should format as 'inf'");
    }

    #[test]
    fn test_return_negative_infinity() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return -math.huge");
        assert!(success, "Should execute successfully");
        assert_eq!(output, "-inf", "Negative infinity should format as '-inf'");
    }

    #[test]
    fn test_return_nan() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return 0/0");
        assert!(success, "Should execute successfully");
        assert_eq!(output, "nan", "NaN should format as 'nan'");
    }

    // ========================================================================
    // Edge Case Tests: String Handling
    // ========================================================================

    #[test]
    fn test_return_empty_string() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return ''");
        assert!(success, "Should execute successfully");
        assert_eq!(output, "", "Empty string should produce empty output");
    }

    #[test]
    fn test_print_empty_string() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("print('')");
        assert!(success, "Should execute successfully");
        assert_eq!(output, "", "Print empty string should produce empty output");
    }

    #[test]
    fn test_return_string_with_quotes() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string(r#"return "It's a \"quoted\" string""#);
        assert!(success, "Should execute successfully");
        assert!(
            output.contains("quoted"),
            "String with quotes should be returned"
        );
    }

    #[test]
    fn test_return_unicode_string() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return 'Hello 世界 🌍'");
        assert!(success, "Should execute successfully");
        assert_eq!(
            output, "Hello 世界 🌍",
            "Unicode string should be preserved"
        );
    }

    #[test]
    fn test_print_unicode_output() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("print('emoji: 🚀 ✨ 🔥')");
        assert!(success, "Should execute successfully");
        assert_eq!(
            output, "emoji: 🚀 ✨ 🔥",
            "Unicode in print should be preserved"
        );
    }

    #[test]
    fn test_return_string_with_tabs() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return 'Hello\\tWorld'");
        assert!(success, "Should execute successfully");
        assert!(
            output.contains("Hello"),
            "String with tab should be preserved"
        );
    }

    #[test]
    fn test_return_string_with_newlines() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return 'Line1\\nLine2'");
        assert!(success, "Should execute successfully");
        assert!(
            output.contains("Line1"),
            "String with newlines should preserve content"
        );
    }

    #[test]
    fn test_string_with_null_bytes() {
        let runtime = create_runtime();
        let (_output, success) = runtime.execute_string("return 'Hello\\0World'");
        assert!(success, "Should handle null bytes without crashing");
    }

    // ========================================================================
    // Edge Case Tests: Boolean and Nil
    // ========================================================================

    #[test]
    fn test_print_boolean_true() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("print(true)");
        assert!(success, "Should execute successfully");
        assert_eq!(output, "true", "Boolean true should print as 'true'");
    }

    #[test]
    fn test_print_boolean_false() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("print(false)");
        assert!(success, "Should execute successfully");
        assert_eq!(output, "false", "Boolean false should print as 'false'");
    }

    #[test]
    fn test_print_nil() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("print(nil)");
        assert!(success, "Should execute successfully");
        assert_eq!(output, "nil", "nil should print as 'nil'");
    }

    // ========================================================================
    // Edge Case Tests: Print with Mixed Types
    // ========================================================================

    #[test]
    fn test_print_with_all_types() {
        let runtime = create_runtime();
        let code = "print(42, 'string', true, nil, false, 3.14)";
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        assert!(output.contains("42"), "Should contain number");
        assert!(output.contains("string"), "Should contain string");
        assert!(output.contains("true"), "Should contain boolean");
    }

    #[test]
    fn test_print_multiple_lines_with_return() {
        let runtime = create_runtime();
        let code = "print('Line 1'); print('Line 2'); return 'Result'";
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines.len() >= 2, "Should have multiple lines");
        assert!(lines[0] == "Line 1", "First line should be 'Line 1'");
        assert!(lines[1] == "Line 2", "Second line should be 'Line 2'");
        assert!(lines[2] == "Result", "Last line should be return value");
    }

    // ========================================================================
    // Edge Case Tests: Lua Standard Library
    // ========================================================================

    #[test]
    fn test_math_operations() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return math.sqrt(16)");
        assert!(success, "Should execute successfully");
        assert_eq!(output, "4", "math.sqrt should work");
    }

    #[test]
    fn test_table_library() {
        let runtime = create_runtime();
        let code = "local t = {}; table.insert(t, 1); return #t";
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        assert_eq!(output, "1", "table library should work");
    }

    #[test]
    fn test_string_library() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return string.len('hello')");
        assert!(success, "Should execute successfully");
        assert_eq!(output, "5", "string.len should work");
    }

    #[test]
    fn test_string_upper() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return string.upper('hello')");
        assert!(success, "Should execute successfully");
        assert_eq!(output, "HELLO", "string.upper should work");
    }

    #[test]
    fn test_string_sub() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return string.sub('hello', 1, 3)");
        assert!(success, "Should execute successfully");
        assert_eq!(output, "hel", "string.sub should work");
    }

    // ========================================================================
    // Edge Case Tests: Control Flow
    // ========================================================================

    #[test]
    fn test_if_statement_true() {
        let runtime = create_runtime();
        let code = "if 1 > 0 then return 'yes' else return 'no' end";
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        assert_eq!(output, "yes", "If true branch should execute");
    }

    #[test]
    fn test_if_statement_false() {
        let runtime = create_runtime();
        let code = "if 1 > 2 then return 'yes' else return 'no' end";
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        assert_eq!(output, "no", "If false branch should execute");
    }

    #[test]
    fn test_for_loop_range() {
        let runtime = create_runtime();
        let code = "local sum = 0; for i=1,5 do sum = sum + i end; return sum";
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        assert_eq!(output, "15", "For loop should calculate correct sum");
    }

    #[test]
    fn test_while_loop() {
        let runtime = create_runtime();
        let code = "local i = 0; while i < 5 do i = i + 1 end; return i";
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        assert_eq!(output, "5", "While loop should execute correctly");
    }

    #[test]
    fn test_repeat_until_loop() {
        let runtime = create_runtime();
        let code = "local i = 0; repeat i = i + 1 until i >= 3; return i";
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        assert_eq!(output, "3", "Repeat-until loop should execute correctly");
    }

    // ========================================================================
    // Edge Case Tests: Functions and Closures
    // ========================================================================

    #[test]
    fn test_function_return_value() {
        let runtime = create_runtime();
        let code = "local function add(a, b) return a + b end; return add(3, 4)";
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        assert_eq!(output, "7", "Function should return correct value");
    }

    #[test]
    fn test_recursive_function() {
        let runtime = create_runtime();
        let code = "local function fact(n) if n <= 1 then return 1 else return n * fact(n-1) end end; return fact(5)";
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        assert_eq!(output, "120", "Recursive function should work");
    }

    #[test]
    fn test_variadic_function() {
        let runtime = create_runtime();
        let code = "local function sum(...) local s=0; for _,v in ipairs({...}) do s=s+v end; return s end; return sum(1,2,3,4,5)";
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        assert_eq!(output, "15", "Variadic function should work");
    }

    // ========================================================================
    // Edge Case Tests: Error Handling and Edge Cases
    // ========================================================================

    #[test]
    fn test_division_by_zero() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return 1 / 0");
        assert!(
            success,
            "Should execute successfully (Lua allows division by zero)"
        );
        assert_eq!(output, "inf", "Division by zero should produce infinity");
    }

    #[test]
    fn test_undefined_variable() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return undefined_var");
        // In Lua, accessing undefined variables returns nil, not an error
        assert!(success, "Should execute successfully");
        assert_eq!(
            output, "",
            "Undefined variable should return nil (empty output)"
        );
    }

    #[test]
    fn test_type_error() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return 'string' + 5");
        assert!(!success, "Type error should fail");
        assert!(output.contains("Error"), "Should contain error message");
    }

    #[test]
    fn test_call_non_function() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("return (42)()");
        assert!(!success, "Calling non-function should fail");
        assert!(output.contains("Error"), "Should contain error message");
    }

    #[test]
    fn test_break_outside_loop() {
        let runtime = create_runtime();
        let (output, success) = runtime.execute_string("break");
        assert!(!success, "Break outside loop should fail");
        assert!(output.contains("Error"), "Should contain error message");
    }

    // ========================================================================
    // Edge Case Tests: Deeply Nested Structures
    // ========================================================================

    #[test]
    fn test_deeply_nested_function_calls() {
        let runtime = create_runtime();
        let code = "return (((((5 + 4) * 3) - 2) / 1) + 0)";
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        assert_eq!(output, "25", "Nested operations should calculate correctly");
    }

    #[test]
    fn test_long_string_concatenation() {
        let runtime = create_runtime();
        let code = "local s = ''; for i=1,100 do s = s .. 'x' end; return string.len(s)";
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Should execute successfully");
        assert_eq!(output, "100", "String concatenation should work");
    }

    // ========================================================================
    // Edge Case Tests: Persistence Across Calls
    // ========================================================================

    #[test]
    fn test_separate_runtimes_isolated() {
        let runtime1 = create_runtime();
        let runtime2 = create_runtime();

        // Set a variable in runtime1
        runtime1.load_string("test_var = 42").unwrap();

        // Try to access it in runtime2 - should return nil, not error
        let (output, success) = runtime2.execute_string("return test_var");
        assert!(
            success,
            "Accessing undefined variable returns nil, not error"
        );
        assert_eq!(
            output, "",
            "Variable from runtime1 should not be accessible in runtime2 (returns nil)"
        );
    }

    // ========================================================================
    // Phase R4: Events Proxy API Tests
    // ========================================================================

    #[test]
    fn test_events_proxy_on_method() {
        let mut runtime = create_runtime();
        runtime
            .init_event_system()
            .expect("Failed to init event system");

        let code = r#"
            _test_called = false
            local id = niri.events:on("test:event", function(data)
                _test_called = true
            end)
            return id
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "events:on should execute successfully");
        assert_eq!(output, "1", "First handler should have ID 1");
    }

    #[test]
    fn test_events_proxy_once_method() {
        let mut runtime = create_runtime();
        runtime
            .init_event_system()
            .expect("Failed to init event system");

        let code = r#"
            _test_count = 0
            niri.events:once("test:event", function(data)
                _test_count = _test_count + 1
            end)
            niri.events:emit("test:event", {})
            niri.events:emit("test:event", {})
            niri.events:emit("test:event", {})
            return _test_count
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "events:once should execute successfully");
        assert_eq!(output, "1", "once handler should fire only once");
    }

    #[test]
    fn test_events_proxy_off_method() {
        let mut runtime = create_runtime();
        runtime
            .init_event_system()
            .expect("Failed to init event system");

        let code = r#"
            _test_count = 0
            local id = niri.events:on("test:event", function(data)
                _test_count = _test_count + 1
            end)
            niri.events:emit("test:event", {})
            niri.events:off("test:event", id)
            niri.events:emit("test:event", {})
            return _test_count
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "events:off should execute successfully");
        assert_eq!(output, "1", "handler should not fire after being removed");
    }

    #[test]
    fn test_events_proxy_emit_method() {
        let mut runtime = create_runtime();
        runtime
            .init_event_system()
            .expect("Failed to init event system");

        let code = r#"
            _test_value = nil
            niri.events:on("custom:event", function(data)
                _test_value = data.message
            end)
            niri.events:emit("custom:event", { message = "hello world" })
            return _test_value
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "events:emit should execute successfully");
        assert_eq!(output, "hello world", "emit should pass data to handlers");
    }

    #[test]
    fn test_events_proxy_list_method() {
        let mut runtime = create_runtime();
        runtime
            .init_event_system()
            .expect("Failed to init event system");

        let code = r#"
            niri.events:on("event1", function() end)
            niri.events:on("event1", function() end)
            niri.events:on("event2", function() end)
            local info = niri.events:list()
            return info.total
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "events:list should execute successfully");
        assert_eq!(output, "3", "list should return correct total count");
    }

    #[test]
    fn test_events_proxy_clear_method() {
        let mut runtime = create_runtime();
        runtime
            .init_event_system()
            .expect("Failed to init event system");

        let code = r#"
            niri.events:on("event1", function() end)
            niri.events:on("event1", function() end)
            niri.events:on("event2", function() end)
            niri.events:clear("event1")
            local info = niri.events:list()
            return info.total
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "events:clear should execute successfully");
        assert_eq!(
            output, "1",
            "clear should remove handlers for specific event"
        );
    }

    #[test]
    fn test_events_proxy_multiple_handlers_same_event() {
        let mut runtime = create_runtime();
        runtime
            .init_event_system()
            .expect("Failed to init event system");

        let code = r#"
            _test_sum = 0
            niri.events:on("test:event", function() _test_sum = _test_sum + 1 end)
            niri.events:on("test:event", function() _test_sum = _test_sum + 10 end)
            niri.events:on("test:event", function() _test_sum = _test_sum + 100 end)
            niri.events:emit("test:event", {})
            return _test_sum
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Multiple handlers should work");
        assert_eq!(output, "111", "All handlers should fire");
    }

    #[test]
    fn test_events_proxy_emit_primitive_value() {
        let mut runtime = create_runtime();
        runtime
            .init_event_system()
            .expect("Failed to init event system");

        // When emitting a primitive, it should be wrapped in { value = ... }
        let code = r#"
            _test_value = nil
            niri.events:on("test:event", function(data)
                _test_value = data.value
            end)
            niri.events:emit("test:event", 42)
            return _test_value
        "#;
        let (output, success) = runtime.execute_string(code);
        assert!(success, "Emitting primitive should work");
        assert_eq!(output, "42", "Primitive should be wrapped in table");
    }

    // ========================================================================
    // Phase R5: Action Proxy API Tests
    // ========================================================================

    #[test]
    fn test_action_proxy_exists() {
        use std::sync::Arc;

        let mut runtime = create_runtime();

        // Register action proxy with a no-op callback
        let callback: niri_lua::ActionCallback = Arc::new(|_action| Ok(()));
        runtime
            .register_action_proxy(callback)
            .expect("Failed to register action proxy");

        let (output, success) = runtime.execute_string("return type(niri.action)");
        assert!(success, "Should execute successfully");
        assert_eq!(output, "userdata", "niri.action should be a userdata");
    }

    #[test]
    fn test_action_proxy_spawn_method() {
        use std::sync::{Arc, Mutex as StdMutex};

        use niri_ipc::Action;

        let mut runtime = create_runtime();
        let captured_actions: Arc<StdMutex<Vec<Action>>> = Arc::new(StdMutex::new(vec![]));

        let actions_clone = captured_actions.clone();
        let callback: niri_lua::ActionCallback = Arc::new(move |action| {
            actions_clone.lock().unwrap().push(action);
            Ok(())
        });

        runtime
            .register_action_proxy(callback)
            .expect("Failed to register action proxy");

        let (_, success) = runtime.execute_string(r#"niri.action:spawn({"echo", "hello"})"#);
        assert!(success, "spawn should execute successfully");

        let actions = captured_actions.lock().unwrap();
        assert_eq!(actions.len(), 1, "Should have captured one action");
        if let Action::Spawn { command } = &actions[0] {
            assert_eq!(command, &vec!["echo", "hello"]);
        } else {
            panic!("Expected Spawn action, got {:?}", actions[0]);
        }
    }

    #[test]
    fn test_action_proxy_focus_column_methods() {
        use std::sync::{Arc, Mutex as StdMutex};

        use niri_ipc::Action;

        let mut runtime = create_runtime();
        let captured_actions: Arc<StdMutex<Vec<Action>>> = Arc::new(StdMutex::new(vec![]));

        let actions_clone = captured_actions.clone();
        let callback: niri_lua::ActionCallback = Arc::new(move |action| {
            actions_clone.lock().unwrap().push(action);
            Ok(())
        });

        runtime
            .register_action_proxy(callback)
            .expect("Failed to register action proxy");

        let code = r#"
            niri.action:focus_column_left()
            niri.action:focus_column_right()
            niri.action:focus_column_first()
            niri.action:focus_column_last()
        "#;
        let (_, success) = runtime.execute_string(code);
        assert!(success, "Focus column methods should execute successfully");

        let actions = captured_actions.lock().unwrap();
        assert_eq!(actions.len(), 4, "Should have captured four actions");
        assert!(matches!(actions[0], Action::FocusColumnLeft {}));
        assert!(matches!(actions[1], Action::FocusColumnRight {}));
        assert!(matches!(actions[2], Action::FocusColumnFirst {}));
        assert!(matches!(actions[3], Action::FocusColumnLast {}));
    }

    #[test]
    fn test_action_proxy_workspace_methods() {
        use std::sync::{Arc, Mutex as StdMutex};

        use niri_ipc::{Action, WorkspaceReferenceArg};

        let mut runtime = create_runtime();
        let captured_actions: Arc<StdMutex<Vec<Action>>> = Arc::new(StdMutex::new(vec![]));

        let actions_clone = captured_actions.clone();
        let callback: niri_lua::ActionCallback = Arc::new(move |action| {
            actions_clone.lock().unwrap().push(action);
            Ok(())
        });

        runtime
            .register_action_proxy(callback)
            .expect("Failed to register action proxy");

        let code = r#"
            niri.action:focus_workspace(1)
            niri.action:focus_workspace("dev")
        "#;
        let (_, success) = runtime.execute_string(code);
        assert!(success, "Workspace methods should execute successfully");

        let actions = captured_actions.lock().unwrap();
        assert_eq!(actions.len(), 2, "Should have captured two actions");

        if let Action::FocusWorkspace { reference } = &actions[0] {
            assert!(matches!(reference, WorkspaceReferenceArg::Index(1)));
        } else {
            panic!("Expected FocusWorkspace action");
        }

        if let Action::FocusWorkspace { reference } = &actions[1] {
            if let WorkspaceReferenceArg::Name(name) = reference {
                assert_eq!(name, "dev");
            } else {
                panic!("Expected workspace name reference");
            }
        } else {
            panic!("Expected FocusWorkspace action");
        }
    }

    #[test]
    fn test_action_proxy_size_change_parsing() {
        use std::sync::{Arc, Mutex as StdMutex};

        use niri_ipc::{Action, SizeChange};

        let mut runtime = create_runtime();
        let captured_actions: Arc<StdMutex<Vec<Action>>> = Arc::new(StdMutex::new(vec![]));

        let actions_clone = captured_actions.clone();
        let callback: niri_lua::ActionCallback = Arc::new(move |action| {
            actions_clone.lock().unwrap().push(action);
            Ok(())
        });

        runtime
            .register_action_proxy(callback)
            .expect("Failed to register action proxy");

        let code = r#"
            niri.action:set_window_width(500)
            niri.action:set_window_width("50%")
            niri.action:set_window_width("+10")
            niri.action:set_window_width("-5%")
        "#;
        let (_, success) = runtime.execute_string(code);
        assert!(success, "Size change methods should execute successfully");

        let actions = captured_actions.lock().unwrap();
        assert_eq!(actions.len(), 4, "Should have captured four actions");

        // Fixed value
        if let Action::SetWindowWidth { change, .. } = &actions[0] {
            assert!(matches!(change, SizeChange::SetFixed(500)));
        } else {
            panic!("Expected SetWindowWidth action");
        }

        // Proportion
        if let Action::SetWindowWidth { change, .. } = &actions[1] {
            if let SizeChange::SetProportion(p) = change {
                assert!((p - 0.5).abs() < 0.001);
            } else {
                panic!("Expected SetProportion");
            }
        }

        // Adjust fixed
        if let Action::SetWindowWidth { change, .. } = &actions[2] {
            assert!(matches!(change, SizeChange::AdjustFixed(10)));
        }

        // Adjust proportion negative
        if let Action::SetWindowWidth { change, .. } = &actions[3] {
            if let SizeChange::AdjustProportion(p) = change {
                assert!((p + 0.05).abs() < 0.001);
            } else {
                panic!("Expected AdjustProportion");
            }
        }
    }

    #[test]
    fn test_action_proxy_overview_methods() {
        use std::sync::{Arc, Mutex as StdMutex};

        use niri_ipc::Action;

        let mut runtime = create_runtime();
        let captured_actions: Arc<StdMutex<Vec<Action>>> = Arc::new(StdMutex::new(vec![]));

        let actions_clone = captured_actions.clone();
        let callback: niri_lua::ActionCallback = Arc::new(move |action| {
            actions_clone.lock().unwrap().push(action);
            Ok(())
        });

        runtime
            .register_action_proxy(callback)
            .expect("Failed to register action proxy");

        let code = r#"
            niri.action:toggle_overview()
            niri.action:open_overview()
            niri.action:close_overview()
        "#;
        let (_, success) = runtime.execute_string(code);
        assert!(success, "Overview methods should execute successfully");

        let actions = captured_actions.lock().unwrap();
        assert_eq!(actions.len(), 3, "Should have captured three actions");
        assert!(matches!(actions[0], Action::ToggleOverview {}));
        assert!(matches!(actions[1], Action::OpenOverview {}));
        assert!(matches!(actions[2], Action::CloseOverview {}));
    }

    #[test]
    fn test_action_proxy_quit_with_confirmation() {
        use std::sync::{Arc, Mutex as StdMutex};

        use niri_ipc::Action;

        let mut runtime = create_runtime();
        let captured_actions: Arc<StdMutex<Vec<Action>>> = Arc::new(StdMutex::new(vec![]));

        let actions_clone = captured_actions.clone();
        let callback: niri_lua::ActionCallback = Arc::new(move |action| {
            actions_clone.lock().unwrap().push(action);
            Ok(())
        });

        runtime
            .register_action_proxy(callback)
            .expect("Failed to register action proxy");

        let code = r#"
            niri.action:quit()
            niri.action:quit(true)
        "#;
        let (_, success) = runtime.execute_string(code);
        assert!(success, "quit methods should execute successfully");

        let actions = captured_actions.lock().unwrap();
        assert_eq!(actions.len(), 2, "Should have captured two actions");

        assert!(matches!(
            actions[0],
            Action::Quit {
                skip_confirmation: false
            }
        ));
        assert!(matches!(
            actions[1],
            Action::Quit {
                skip_confirmation: true
            }
        ));
    }

    #[test]
    fn test_action_proxy_screenshot_methods() {
        use std::sync::{Arc, Mutex as StdMutex};

        use niri_ipc::Action;

        let mut runtime = create_runtime();
        let captured_actions: Arc<StdMutex<Vec<Action>>> = Arc::new(StdMutex::new(vec![]));

        let actions_clone = captured_actions.clone();
        let callback: niri_lua::ActionCallback = Arc::new(move |action| {
            actions_clone.lock().unwrap().push(action);
            Ok(())
        });

        runtime
            .register_action_proxy(callback)
            .expect("Failed to register action proxy");

        let code = r#"
            niri.action:screenshot()
            niri.action:screenshot_screen()
            niri.action:screenshot_window()
        "#;
        let (_, success) = runtime.execute_string(code);
        assert!(success, "Screenshot methods should execute successfully");

        let actions = captured_actions.lock().unwrap();
        assert_eq!(actions.len(), 3, "Should have captured three actions");
        assert!(matches!(actions[0], Action::Screenshot { .. }));
        assert!(matches!(actions[1], Action::ScreenshotScreen { .. }));
        assert!(matches!(actions[2], Action::ScreenshotWindow { .. }));
    }

    #[test]
    fn test_action_proxy_move_column_methods() {
        use std::sync::{Arc, Mutex as StdMutex};

        use niri_ipc::Action;

        let mut runtime = create_runtime();
        let captured_actions: Arc<StdMutex<Vec<Action>>> = Arc::new(StdMutex::new(vec![]));

        let actions_clone = captured_actions.clone();
        let callback: niri_lua::ActionCallback = Arc::new(move |action| {
            actions_clone.lock().unwrap().push(action);
            Ok(())
        });

        runtime
            .register_action_proxy(callback)
            .expect("Failed to register action proxy");

        let code = r#"
            niri.action:move_column_left()
            niri.action:move_column_right()
            niri.action:move_column_to_first()
            niri.action:move_column_to_last()
        "#;
        let (_, success) = runtime.execute_string(code);
        assert!(success, "Move column methods should execute successfully");

        let actions = captured_actions.lock().unwrap();
        assert_eq!(actions.len(), 4, "Should have captured four actions");
        assert!(matches!(actions[0], Action::MoveColumnLeft {}));
        assert!(matches!(actions[1], Action::MoveColumnRight {}));
        assert!(matches!(actions[2], Action::MoveColumnToFirst {}));
        assert!(matches!(actions[3], Action::MoveColumnToLast {}));
    }

    #[test]
    fn test_action_proxy_floating_methods() {
        use std::sync::{Arc, Mutex as StdMutex};

        use niri_ipc::Action;

        let mut runtime = create_runtime();
        let captured_actions: Arc<StdMutex<Vec<Action>>> = Arc::new(StdMutex::new(vec![]));

        let actions_clone = captured_actions.clone();
        let callback: niri_lua::ActionCallback = Arc::new(move |action| {
            actions_clone.lock().unwrap().push(action);
            Ok(())
        });

        runtime
            .register_action_proxy(callback)
            .expect("Failed to register action proxy");

        let code = r#"
            niri.action:toggle_window_floating()
            niri.action:move_window_to_floating()
            niri.action:move_window_to_tiling()
            niri.action:focus_floating()
            niri.action:focus_tiling()
        "#;
        let (_, success) = runtime.execute_string(code);
        assert!(success, "Floating methods should execute successfully");

        let actions = captured_actions.lock().unwrap();
        assert_eq!(actions.len(), 5, "Should have captured five actions");
        assert!(matches!(actions[0], Action::ToggleWindowFloating { .. }));
        assert!(matches!(actions[1], Action::MoveWindowToFloating { .. }));
        assert!(matches!(actions[2], Action::MoveWindowToTiling { .. }));
        assert!(matches!(actions[3], Action::FocusFloating {}));
        assert!(matches!(actions[4], Action::FocusTiling {}));
    }
}
