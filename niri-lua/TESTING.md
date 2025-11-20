# Niri Lua Testing Guide

This document provides comprehensive guidelines for understanding and writing tests for the Niri Lua API. The test suite contains 332 tests covering all major modules with extensive edge case validation.

## Overview

The niri-lua crate contains comprehensive unit tests organized by module. The test organization follows Rust conventions with inline test modules in each source file.

### Test Statistics
- **Total Tests**: 332
- **Success Rate**: 100%
- **Coverage**: All major public APIs
- **Test Execution Time**: ~2-3 seconds

### Module Breakdown
| Module | Tests | Focus |
|--------|-------|-------|
| config_api | 152 | Configuration API validation and application |
| extractors | 66 | Lua value extraction and type conversion |
| validators | 67 | Configuration value validation with boundaries |
| runtime | 34 | Lua runtime management and interaction |
| ipc_bridge | 14 | IPC data type conversion (Window, Workspace, Output) |
| plugin_system | 20 | Plugin discovery, loading, and management |
| test_utils | 10 | Shared test utilities and helpers |
| Other modules | ~30 | Hot reload, event emitter, module loader, etc. |

## Test Organization

Tests are organized using Rust's built-in module system with the following structure:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;  // For shared test utilities
    
    // ========================================================================
    // <Function Name> Tests
    // ========================================================================
    
    #[test]
    fn test_function_nominal_case() {
        // Arrange
        // Act
        // Assert
    }
    
    #[test]
    fn test_function_edge_case() {
        // Arrange
        // Act
        // Assert
    }
    
    #[test]
    fn test_function_error_case() {
        // Arrange
        // Act
        // Assert
    }
}
```

### Naming Conventions

Tests follow a consistent naming pattern:

**Pattern**: `test_<function_name>_<scenario>`

**Examples**:
- `test_extract_string_opt_with_value` - function succeeds with valid data
- `test_validate_percentage_boundary_zero` - boundary condition test
- `test_apply_animations_spring_config` - complex configuration test
- `test_window_to_lua_floating` - specific variant test

**Scenario Suffixes**:
- `_valid` / `_nominal` - Normal operation
- `_invalid` / `_error` - Error condition
- `_empty` / `_nil` - Empty/null input
- `_boundary_<type>` - Boundary conditions
- `_<variant>` - Specific variant or type
- `_multiple` - Multiple items/operations
- `_minimal` - Minimal valid input

## Test Utilities Module

The `test_utils` module provides shared test fixtures and helper functions to reduce duplication and improve test consistency.

### Available Helpers

#### Lua Environment
```rust
// Create a test Lua environment with a table
let (lua, table) = create_test_lua_table();
```

#### Window Fixtures
```rust
// Create a minimal test window
let window = create_test_window(123);

// Create a window with custom properties
let window = create_test_window_with()
    .id(456)
    .title("Custom Title")
    .is_floating(true)
    .is_urgent(true)
    .build();
```

#### Workspace Fixtures
```rust
// Create a minimal test workspace
let workspace = create_test_workspace(1);

// Create a workspace with custom properties
let workspace = create_test_workspace_with()
    .id(2)
    .name("Work")
    .output("HDMI-1")
    .is_urgent(true)
    .build();
```

#### Output Fixtures
```rust
// Create a test output
let output = create_test_output("DP-1");

// Create a disabled output
let output = create_disabled_test_output("DP-2");
```

#### Lua Value Helpers
```rust
// Create Lua values for testing
let str_val = lua_string(&lua, "hello");
let num_val = lua_number(&lua, 42.5);
let int_val = lua_integer(&lua, -100);
let bool_val = lua_bool(&lua, true);
```

#### Runtime
```rust
// Create a test Lua runtime with standard library
let lua = create_test_runtime()?;
```

## Common Test Patterns

### Pattern 1: Validation Testing

Tests that verify input validation with boundaries and edge cases:

```rust
#[test]
fn test_validate_percentage_valid() {
    let result = validate_percentage(50.0);
    assert!(result.is_ok());
}

#[test]
fn test_validate_percentage_boundary_zero() {
    let result = validate_percentage(0.0);
    assert!(result.is_ok());
}

#[test]
fn test_validate_percentage_boundary_max() {
    let result = validate_percentage(100.0);
    assert!(result.is_ok());
}

#[test]
fn test_validate_percentage_too_high() {
    let result = validate_percentage(101.0);
    assert!(result.is_err());
}

#[test]
fn test_validate_percentage_wrong_type() {
    // Use Lua table to simulate incorrect type
    let result = validate_percentage_from_lua(&table, "field");
    assert!(result.is_err());
}
```

### Pattern 2: Configuration API Testing

Tests that verify configuration application with multiple scenarios:

```rust
#[test]
fn test_apply_animations_spring_config() {
    // Create minimal Lua environment
    let lua = create_test_runtime().unwrap();
    
    // Set up configuration
    lua.load(r#"
        config.animations.spring.damping = 0.5
        config.animations.spring.stiffness = 0.8
    "#).exec().unwrap();
    
    // Apply configuration
    let result = apply_animations(&lua);
    assert!(result.is_ok());
    
    // Verify values were applied
    let animations = result.unwrap();
    // Check specific animation properties
}
```

### Pattern 3: Data Conversion Testing

Tests that verify conversion between Rust types and Lua types:

```rust
#[test]
fn test_window_to_lua_floating() {
    let lua = Lua::new();
    let window = create_test_window_with()
        .id(456)
        .is_floating(true)
        .is_urgent(true)
        .build();
    
    let table = window_to_lua(&lua, &window).unwrap();
    
    assert_eq!(table.get::<u64>("id").unwrap(), 456);
    assert_eq!(table.get::<bool>("is_floating").unwrap(), true);
    assert_eq!(table.get::<bool>("is_urgent").unwrap(), true);
}
```

### Pattern 4: Error Handling Testing

Tests that verify proper error conditions and messages:

```rust
#[test]
fn test_extract_string_opt_nil() {
    let (lua, table) = create_test_lua_table();
    let result = extract_string_opt(&table, "nonexistent");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None);
}

#[test]
fn test_extract_string_opt_wrong_type_number() {
    let (lua, table) = create_test_lua_table();
    table.set("field", 42).unwrap();
    let result = extract_string_opt(&table, "field");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None);  // Returns None for wrong type
}
```

## Test Categories

### Boundary Tests
Tests that verify behavior at numerical boundaries:
- Minimum values: 0, -∞, empty strings
- Maximum values: system limits, large numbers
- Just valid/invalid: values at transition points

Example: `test_validate_refresh_rate_low_boundary`, `test_validate_scale_boundary_high`

### Type Validation Tests
Tests that verify type checking and conversion:
- Correct types accepted
- Wrong types rejected or handled gracefully
- Nil/None values handled appropriately

Example: `test_validate_bool_true`, `test_validate_curve_wrong_type`

### Integration Tests
Tests that verify multiple components working together:
- Multiple configurations applied
- State consistency
- Complex object creation

Example: `test_apply_all_misc_configs_together`, `test_multiple_plugin_operations`

### Fixture Tests
Tests that verify data structure conversions:
- Window/Workspace/Output conversions
- Optional fields handling
- Nested data structures

Example: `test_window_to_lua`, `test_workspace_to_lua_urgent`

## Running Tests

### Run All Tests
```bash
cargo test --package niri-lua
```

### Run Specific Module Tests
```bash
cargo test --package niri-lua validators::
cargo test --package niri-lua extractors::
cargo test --package niri-lua config_api::
```

### Run Specific Test
```bash
cargo test --package niri-lua test_apply_animations_spring_config -- --exact
```

### Run Tests with Output
```bash
cargo test --package niri-lua -- --nocapture
```

### Run Tests with Thread Count
```bash
cargo test --package niri-lua -- --test-threads=1
```

## Best Practices

### 1. Use Test Utilities
Leverage `test_utils` module for common fixtures:

```rust
// ✓ Good - Uses shared utilities
let window = create_test_window(123);
let (lua, table) = create_test_lua_table();

// ✗ Avoid - Duplicates test setup code
let window = Window { /* 20+ lines */ };
let lua = Lua::new();
let table = lua.create_table().unwrap();
```

### 2. Follow AAA Pattern
Organize tests into Arrange, Act, Assert phases:

```rust
// Arrange - Set up test data
let window = create_test_window(123);
let lua = Lua::new();

// Act - Execute the function
let table = window_to_lua(&lua, &window)?;

// Assert - Verify results
assert_eq!(table.get::<u64>("id")?, 123);
```

### 3. Test Edge Cases
Include tests for boundaries and error conditions:

```rust
#[test]
fn test_validate_percentage_boundary_zero() { /* */ }

#[test]
fn test_validate_percentage_boundary_max() { /* */ }

#[test]
fn test_validate_percentage_invalid_negative() { /* */ }

#[test]
fn test_validate_percentage_wrong_type() { /* */ }
```

### 4. Use Descriptive Names
Names should clearly indicate what is being tested:

```rust
// ✓ Good - Clear and specific
fn test_validate_accel_speed_negative()

// ✗ Avoid - Vague
fn test_accel_speed()
```

### 5. Organize with Comments
Use section comments to group related tests:

```rust
// ========================================================================
// extract_string_opt tests
// ========================================================================

#[test]
fn test_extract_string_opt_with_value() { /* */ }

#[test]
fn test_extract_string_opt_nil() { /* */ }
```

### 6. Keep Tests Focused
Each test should verify one specific behavior:

```rust
// ✓ Good - Single focus
#[test]
fn test_window_to_lua_floating() {
    // Only tests that floating flag is set correctly
}

// ✗ Avoid - Multiple concerns
#[test]
fn test_window_to_lua_all_properties() {
    // Tests 10+ different properties at once
}
```

## Test Coverage

### Coverage by Module
The test suite achieves the following coverage:

- **extractors.rs**: All 15 functions covered, 100+ test scenarios
- **validators.rs**: All 20+ validators covered, boundary + type tests
- **config_api.rs**: All 50+ configuration options covered
- **runtime_api.rs**: All 5 trait methods covered
- **ipc_bridge.rs**: All 4 conversion functions covered
- **plugin_system.rs**: All 10+ methods covered

### Coverage Gaps
- **Hot reload**: File system integration (limited by test environment)
- **Event emitter**: Limited integration testing
- **Module loader**: Filesystem-dependent tests limited

## Debugging Tests

### Run Single Test with Backtrace
```bash
RUST_BACKTRACE=1 cargo test --package niri-lua test_apply_animations_spring_config -- --exact --nocapture
```

### Print Debug Information
```rust
#[test]
fn test_example() {
    let value = some_function();
    dbg!(value);  // Will print in test output with --nocapture
}
```

### Check Lua Values
```rust
#[test]
fn test_lua_conversion() {
    let lua = Lua::new();
    let table = lua.create_table().unwrap();
    table.set("key", "value").unwrap();
    
    // Print Lua value
    let value: String = table.get("key").unwrap();
    println!("Lua value: {}", value);
}
```

## Adding New Tests

### Checklist for New Tests
- [ ] Function/module being tested is clearly identified
- [ ] Test name follows `test_<function>_<scenario>` pattern
- [ ] Test uses appropriate fixtures from `test_utils`
- [ ] Both success and failure cases tested
- [ ] Boundary conditions included where applicable
- [ ] Test is isolated and doesn't depend on other tests
- [ ] Comments explain complex test logic
- [ ] Test runs successfully: `cargo test --package niri-lua`

### Template for New Test

```rust
#[test]
fn test_new_function_scenario() {
    // Arrange
    let input = create_test_fixture();
    let expected = known_result;
    
    // Act
    let result = new_function(input);
    
    // Assert
    assert_eq!(result, expected);
}
```

## Continuous Integration

Tests are automatically run on:
- Pull requests
- Commits to main branch
- Release builds

All 332 tests must pass before merging.

## Resources

- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [MLua Documentation](https://docs.rs/mlua/latest/mlua/)
- [Niri IPC Types](../niri-ipc/src/lib.rs)
- [Config Types](../niri-config/src/lib.rs)
