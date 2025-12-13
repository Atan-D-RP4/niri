# Niri Lua Testing Guide

This document provides comprehensive guidelines for understanding and writing tests for the Niri Lua API. The test suite provides coverage of all major modules with extensive edge case validation.

## Overview

The niri-lua crate contains comprehensive unit tests organized by module. The test organization follows Rust conventions with inline test modules in each source file.

### Test Statistics
- **Total Tests**: 570
- **Success Rate**: 100%
- **Coverage**: All major public APIs
- **Test Execution Time**: ~10-15 seconds total

### Test Breakdown by Category
| Category | Count | Run Command | Focus |
|----------|-------|-------------|-------|
| Unit Tests (lib) | 427 | `cargo test --lib` | Module functions, data extraction, validation |
| Integration Tests | 39 | `cargo test --test integration_tests` | Full Lua execution, end-to-end workflows |
| REPL Integration Tests | 104 | `cargo test --test repl_integration` | REPL execution, events proxy, action proxy |

## Test Infrastructure History

The niri-lua test suite has evolved significantly through focused improvements in testing infrastructure and coverage:

### Initial State
- **Test Count**: ~104 tests
- **Focus**: Primarily REPL integration tests
- **Coverage**: Event system, action proxy, basic configuration

### Current State
- **Test Count**: 570 tests (5.5x growth)
- **Coverage**: Comprehensive snapshot testing across all modules
- **Infrastructure**: Snapshot testing, fixture generation, reusable test utilities

### Key Improvements

#### 1. Snapshot Testing Infrastructure (Added)
- Added 7 API schema snapshot tests to `api_registry.rs` for schema validation
- Added 12 event data snapshot tests to `event_data.rs` for event structure serialization
- Added 8 error format snapshot tests to `extractors.rs` for Lua value extraction
- Added 8 config transformation snapshot tests to `config_wrapper.rs` for configuration application
- Added 8 action parsing snapshot tests to `action_proxy.rs` for action data transformations

**Impact**: Enables regression detection for complex data structures without manual assertion maintenance

#### 2. Test Code Cleanup (Removed)
- Deleted 7 orphaned snapshot files from removed `validators` and `plugin_system` modules
- Cleaned up test data that no longer had corresponding code

**Impact**: Improved test maintainability and removed dead code

#### 3. Integration Test Rewrite (Modified)
- Rewrote `tests/integration_tests.rs` with 52 working tests
- Consolidated testing patterns and reduced duplication with REPL tests
- Added end-to-end workflow coverage

**Impact**: Clearer separation of concerns between unit, integration, and REPL tests

#### 4. Clippy Warnings (Fixed)
- Fixed clippy warning in `repl_integration.rs`

**Impact**: Clean compilation with no warnings

#### 5. Test Redundancy Elimination (Dec 2024)
- Removed 13 duplicate tests from `tests/integration_tests.rs`
- Consolidated with `tests/repl_integration.rs` to eliminate redundancy
- Reduced test code by ~200 LOC with zero coverage loss

**Impact**: Simplified test maintenance while preserving 100% test success rate

#### 6. Unused Code Removal (Dec 2024)
- Deleted 8 unused test utility functions from `src/test_utils.rs`
- Removed `TestDataBuilder` struct (62 lines) and related helpers
- Reduced `test_utils.rs` from 478 to 273 lines

**Impact**: Cleaner test utility module with only actively-used fixtures

#### 7. Runtime Factory Consolidation (Dec 2024)
- Created `tests/common.rs` with shared `create_runtime()` implementation
- Eliminated duplicate runtime creation functions across test files
- Both integration and REPL test suites now use shared implementation

**Impact**: Reduced code duplication and easier to maintain runtime setup

#### 8. Orphaned File Cleanup (Dec 2024)
- Removed 7 orphaned snapshot files from `src/snapshots/` for removed modules
- Cleaned up test data that no longer had corresponding code

**Impact**: Improved test directory organization and removed stale artifacts

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

### Testing Infrastructure from Codebase

niri-lua tests leverage simplified patterns from the rest of the codebase:

#### 1. Helper Functions with #[track_caller]

Following the pattern from niri-config, use `#[track_caller]` helper functions to reduce boilerplate and provide better error messages:

```rust
#[track_caller]
pub fn load_lua_code(code: &str) -> LuaResult<Lua> {
    let lua = create_test_runtime()?;
    lua.load(code).exec()?;
    Ok(lua)
}

#[track_caller]
pub fn get_lua_global<T: mlua::FromLua>(lua: &Lua, name: &str) -> LuaResult<T> {
    lua.globals().get(name)
}
```

**Benefits**:
- Error messages show the caller's location (better debugging)
- Reduces repetitive test setup code
- Consistent with niri-config testing patterns

#### 2. Snapshot Testing with `insta`

For complex validation results, use the `insta` crate for snapshot testing:

```rust
#[test]
fn test_complex_validation() {
    use insta::assert_snapshot;
    
    let lua = Lua::new();
    let table = lua.create_table().unwrap();
    let result = validate_config(&LuaValue::Table(table));
    
    // Snapshot captures the debug output
    assert_snapshot!(format!("{:?}", result));
}
```

**Benefits**:
- Easier to verify complex output (tables, nested structures)
- Snapshots are versioned and reviewed in git
- Updates with `cargo test` when intentional changes occur
- Better for comparing large validation results

**Workflow**:
1. Run test: `cargo test --package niri-lua`
2. Review: Check the `.snap.new` file was created
3. Accept: Move `.snap.new` to `.snap` or use `cargo insta review`

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

// Load and run Lua code
let lua = load_lua_code("x = 42")?;

// Extract global variable
let value: i32 = get_lua_global(&lua, "x")?;
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

### Pattern 5: Simplified Testing with #[track_caller] Helpers

Use the simplified helper functions to reduce boilerplate:

```rust
#[test]
fn test_load_and_access_config() {
    // Much simpler than manually creating Lua, loading code, and extracting values
    let lua = load_lua_code("config = { value = 42 }").unwrap();
    let config: mlua::Table = get_lua_global(&lua, "config").unwrap();
    
    let value: i32 = config.get("value").unwrap();
    assert_eq!(value, 42);
}
```

**Before** (without helpers):
```rust
let lua = Lua::new();
lua.load_std_libs(mlua::prelude::LuaStdLib::ALL_SAFE).unwrap();
lua.load("config = { value = 42 }").exec().unwrap();
let config: mlua::Table = lua.globals().get("config").unwrap();
let value: i32 = config.get("value").unwrap();
```

**Benefits of helpers**:
- Reduced code repetition
- Better error messages with #[track_caller]
- Consistent with codebase patterns
- Easier to maintain

### Pattern 6: Snapshot Testing for Complex Validation

Use `insta` snapshot testing for complex results:

```rust
#[test]
fn test_complex_config_validation() {
    use insta::assert_snapshot;
    
    let lua = Lua::new();
    let table = lua.create_table().unwrap();
    let result = ConfigValidator::validate_config(&LuaValue::Table(table));
    
    // Automatically creates snapshot of the result
    assert_snapshot!(format!("{:?}", result));
}
```

**Better than manual assertions for**:
- Large nested structures
- Complex validation output
- Results that change infrequently
- Test output that's hard to write assertions for

**Update workflow**:
```bash
# Run tests and create snapshots
cargo test --package niri-lua

# Review snapshots created in niri-lua/src/snapshots/
# Move .snap.new to .snap files to accept them

# Or use cargo-insta if installed
cargo insta review
```

## Snapshot Testing Infrastructure

The niri-lua crate uses the `insta` crate for snapshot-based regression testing. Snapshots capture Debug output of complex structures and are stored in version control for easy review.

### Snapshot Organization

Snapshots are located in `src/snapshots/` directory with the following naming prefixes:

| Prefix | Module | Purpose |
|--------|--------|---------|
| `api_` | `api_registry.rs`, `api_data.rs` | Schema validation, API structure dumps |
| `event_data_` | `event_system.rs` | Event data structure serialization |
| `extractors_` | `extractors.rs` | Lua value extraction and type conversion results |
| `config_wrapper_` | `config_api.rs` | Configuration wrapper and application |
| `action_proxy_` | `action_proxy.rs` | Action proxy data transformations |

### Working with Snapshots

#### Reviewing New Snapshots
When tests create new snapshots, `insta` generates `.snap.new` files:

```bash
# Run tests to generate .snap.new files
cargo test --package niri-lua

# Review and accept snapshots
cargo insta review
```

#### Manual Snapshot Acceptance
```bash
# Move .snap.new to .snap to accept
mv src/snapshots/api_registry__test_name.snap.new \
   src/snapshots/api_registry__test_name.snap
```

#### Updating Snapshots After Intentional Changes
If you've intentionally changed code and need to update snapshots:

```bash
# Accept all pending snapshot changes
cargo insta accept

# Or review them one by one
cargo insta review
```

## Test Categories by Type

Tests in niri-lua fall into four main categories:

### 1. Unit Tests
**Location**: Inline in source files with `#[cfg(test)]` modules  
**Count**: 419 tests  
**Run**: `cargo test --lib`

Tests individual functions and small components:
- Value extraction (`extractors.rs`)
- Type validation and conversion
- Configuration parsing
- Data structure transformations

**Example**:
```rust
#[test]
fn test_extract_string_opt_with_value() {
    let (lua, table) = create_test_lua_table();
    table.set("field", "hello").unwrap();
    let result = extract_string_opt(&table, "field").unwrap();
    assert_eq!(result, Some("hello".to_string()));
}
```

### 2. Snapshot Tests
**Location**: Source files using `insta::assert_snapshot!` macro  
**Count**: Integrated within unit and integration tests  
**Purpose**: Regression detection for complex structures

Uses `insta` to capture Debug output and compare against stored snapshots:
- API structure validation
- Configuration state changes
- Event data serialization
- Complex extraction results

**Example**:
```rust
#[test]
fn test_api_schema_completeness() {
    use insta::assert_snapshot;
    let schema = ApiRegistry::schema();
    assert_snapshot!(format!("{:?}", schema));
}
```

**Workflow**:
1. Run test: `cargo test --lib api_registry`
2. Review: Check `src/snapshots/*.snap.new` files
3. Accept: `cargo insta review` or `cargo insta accept`

### 3. Integration Tests
**Location**: `tests/integration_tests.rs`  
**Count**: 52 tests  
**Run**: `cargo test --test integration_tests`

Tests full Lua execution workflows and component interaction:
- Configuration loading and application
- Window/workspace/output conversions
- Multi-step state changes
- Configuration file parsing

**Example**:
```rust
#[test]
fn test_lua_config_applies_to_niri_config() {
    let lua = create_test_runtime().unwrap();
    lua.load(r#"
        config.layout = "floating"
        config.misc.gaps = 10
    "#).exec().unwrap();
    
    let result = apply_config(&lua);
    assert!(result.is_ok());
}
```

### 4. REPL Integration Tests
**Location**: `tests/repl_integration.rs`  
**Count**: 104 tests  
**Run**: `cargo test --test repl_integration`

Tests the interactive REPL environment and advanced features:
- REPL command execution and output
- Event system (handlers, event data)
- Action proxy (compositing actions)
- Error handling and edge cases
- Lua stdlib availability

**Example**:
```rust
#[test]
fn test_repl_executes_simple_command() {
    let mut runtime = create_repl_runtime();
    let output = runtime.execute("return 1 + 1");
    assert_eq!(output, "2");
}
```

## Adding New Tests

### Where to Add Different Test Types

#### For New Unit Tests
Add directly in the source file being tested:

```rust
// In src/extractors.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_new_extractor_function() {
        // Test code
    }
}
```

#### For New Integration Tests
Add to `tests/integration_tests.rs`:

```rust
#[test]
fn test_new_integration_scenario() {
    let lua = create_test_runtime().unwrap();
    // Test code
}
```

#### For New REPL Tests
Add to `tests/repl_integration.rs`:

```rust
#[test]
fn test_new_repl_feature() {
    let mut runtime = create_repl_runtime();
    // Test code
}
```

#### For New Snapshot Tests
Add snapshot assertions to any test file:

```rust
#[test]
fn test_complex_structure_snapshot() {
    use insta::assert_snapshot;
    let result = complex_operation();
    assert_snapshot!(format!("{:?}", result));
}
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

- **runtime.rs**: Core runtime management, REPL execution
- **extractors.rs**: Lua value extraction and type conversion
- **config_api.rs**: Configuration options covered
- **runtime_api.rs**: State query methods covered
- **ipc_bridge.rs**: IPC data type conversion functions covered
- **repl_integration.rs**: Comprehensive integration tests for REPL, events, and actions

### Coverage Gaps

#### Critical Gaps (High Priority)

| File | Lines | Issue | Action Required |
|------|-------|-------|-----------------|
| `api_registry.rs` | ~1533 | **NO TESTS** | Add schema validation tests, field completeness tests |
| `state_api.rs` | ~6 | Placeholder only | Implement when API is built out |

#### Moderate Gaps (Medium Priority)

| Module | Current State | Improvement Needed |
|--------|---------------|-------------------|
| `runtime_api.rs` | 15 tests | Add tests for snapshot staleness edge cases |
| `event_system.rs` | 3 tests | Add concurrent handler tests |
| `lib.rs` | 1 test | Add initialization edge case tests |

#### Known Limitations

- **Hot reload**: File system integration (limited by test environment)
- **Event emitter**: Limited integration testing
- **Module loader**: Filesystem-dependent tests limited
- **Event handler snapshots**: Staleness not tested (see Architecture Notes below)

## Integration Testing Plan

The niri-lua crate requires integration tests that exercise the full stack from Lua script execution through to compositor state changes. This section outlines the planned integration testing strategy.

### Current Integration Test Coverage

| Test File | Coverage | Status |
|-----------|----------|--------|
| `tests/repl_integration.rs` | REPL execution, Lua stdlib, events proxy, action proxy | ✅ Comprehensive (1530 lines) |

### Planned Integration Tests

#### 1. Runtime State API Integration (`tests/runtime_state_integration.rs`)

Tests the `niri.state.*` API with realistic compositor state:

```rust
// TODO: Implement these tests
#[test]
fn test_state_windows_returns_all_windows() {
    // Setup mock compositor with multiple windows
    // Query via niri.state.windows()
    // Verify all windows returned with correct properties
}

#[test]
fn test_state_focused_window_tracks_focus_changes() {
    // Setup mock compositor
    // Change focus between windows
    // Verify niri.state.focused_window() reflects changes
}

#[test]
fn test_event_handler_snapshot_staleness() {
    // Register event handler that modifies state
    // Emit event that triggers handler
    // Verify handler sees pre-modification snapshot (not live state)
}
```

#### 2. Configuration Application Integration (`tests/config_integration.rs`)

Tests end-to-end configuration flow:

```rust
// TODO: Implement these tests
#[test]
fn test_lua_config_applies_to_niri_config() {
    // Load Lua config with layout changes
    // Apply pending changes
    // Verify niri-config struct updated correctly
}

#[test]
fn test_config_reload_preserves_runtime_state() {
    // Set up runtime with state
    // Reload configuration
    // Verify state preserved, config updated
}
```

#### 3. Event System Integration (`tests/event_system_integration.rs`)

Tests event emission and handler execution:

```rust
// TODO: Implement these tests
#[test]
fn test_window_events_propagate_to_lua() {
    // Register Lua handler for window:open
    // Simulate window open from compositor
    // Verify handler called with correct data
}

#[test]
fn test_concurrent_event_handlers() {
    // Register multiple handlers for same event
    // Emit event
    // Verify all handlers called in order
}
```

### Integration Test Infrastructure Needs

1. **Mock Compositor State**: Need `MockCompositorState` implementing `CompositorState` trait
2. **Event Simulation**: Helpers to simulate compositor events
3. **Async Test Support**: For testing timer-based callbacks
4. **Snapshot Assertions**: For complex state comparisons

### Running Integration Tests

```bash
# Run all integration tests
cargo test --package niri-lua --test '*'

# Run specific integration test file
cargo test --package niri-lua --test repl_integration

# Run with output for debugging
cargo test --package niri-lua --test repl_integration -- --nocapture
```

## Architecture Notes for Testing

### Event Handler Snapshot Staleness

The `niri.state.*` API uses a **dual-mode query pattern**:

1. **Event Handler Mode**: Uses a pre-captured `StateSnapshot` (thread-local)
2. **Normal Mode (REPL/Timers)**: Uses idle callback + channel for live queries

**Staleness Limitation**: When Lua code runs inside an event handler, the state snapshot
is captured *before* the handler executes. If the handler triggers actions that modify
compositor state, those changes will NOT be visible to subsequent `niri.state.*` calls
within the same handler.

**Example of staleness:**
```lua
niri.on("window:open", function(event)
    -- This sees the snapshot from BEFORE the handler started
    local windows = niri.state.windows()
    
    -- This action modifies compositor state
    niri.action:focus_window(event.id)
    
    -- This STILL sees the old snapshot, NOT the updated state
    local focused = niri.state.focused_window()
    -- focused may not reflect the focus change!
end)
```

**Mitigation Strategies:**
1. Use event data passed to the handler (already contains relevant info)
2. Use timers to defer state queries: `niri.loop.new_timer():start(0, 0, function() ... end)`
3. For critical state checks, query outside event handlers

**Testing Implications:**
- Tests should verify handlers receive correct event data
- Tests should verify snapshot isolation (handlers don't see their own changes)
- Tests should verify deferred queries see updated state

## Known Issues and Cleanup Opportunities

The following issues have been identified through testing reviews and are documented for future cleanup:

### Test Duplication
**Location**: Between `tests/integration_tests.rs` and `tests/repl_integration.rs`
**Issue**: Some test scenarios are duplicated across both test files
**Impact**: Maintenance burden when changing test patterns
**Priority**: Low - Tests are independent and pass
**Future Action**: Consolidate overlapping test cases and establish clear boundaries between test categories

### Unused Test Helpers
**Location**: `src/test_utils.rs`
**Issue**: Several test fixture builders and helper functions have low usage
**Examples**: Some workspace/window builder methods, conversion helpers
**Impact**: Code maintenance complexity
**Priority**: Low - Helper functions don't affect runtime
**Future Action**: Profile test helper usage and remove/consolidate unused utilities

### Snapshot Organization
**Current**: Snapshots use module-based naming prefixes (`api_`, `event_data_`, etc.)
**Opportunity**: Consider grouping snapshots by functionality or feature area
**Impact**: Minor - Organizational improvement only
**Future Action**: No immediate action needed; revisit if snapshot count grows significantly

---

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

### Checklist for New Tests
- [ ] Test location matches test type (lib, integration, or REPL)
- [ ] Test name follows `test_<function>_<scenario>` pattern
- [ ] Test uses appropriate fixtures from `test_utils`
- [ ] Both success and failure cases tested where applicable
- [ ] Boundary conditions included for validation tests
- [ ] Test is isolated and doesn't depend on other tests
- [ ] Comments explain complex test logic or unusual patterns
- [ ] For snapshot tests: snapshot has been reviewed and accepted
- [ ] All tests pass: `cargo test --package niri-lua`
- [ ] Lint passes: `cargo clippy --package niri-lua`

## Running Tests

### Run All Tests
```bash
cargo test --package niri-lua
```

### Run Tests by Category
```bash
# Unit tests only
cargo test --lib --package niri-lua

# Integration tests only
cargo test --test integration_tests --package niri-lua

# REPL integration tests only
cargo test --test repl_integration --package niri-lua
```

### Run Specific Module Tests
```bash
cargo test --package niri-lua config_api::
cargo test --package niri-lua extractors::
cargo test --package niri-lua action_proxy::
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

### Review and Accept Snapshots
```bash
# Interactive review of new snapshots
cargo insta review

# Accept all snapshots at once
cargo insta accept
```

## Continuous Integration

Tests are automatically run on:
- Pull requests
- Commits to main branch
- Release builds

All tests must pass before merging.

## Resources

- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [MLua Documentation](https://docs.rs/mlua/latest/mlua/)
- [Niri IPC Types](../niri-ipc/src/lib.rs)
- [Config Types](../niri-config/src/lib.rs)
