// Niri Lua API Registry - Complete schema of the Lua API.
//
// This file imports the API schema types and includes the shared data definitions.
// The shared data file (api_data.rs) is also used by build.rs for EmmyLua generation.

use crate::lua_api_schema::*;

// Include the shared API data definitions.
// This file contains all the const definitions (NIRI_LUA_API, module schemas, etc.)
include!("api_data.rs");

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    /// Verify that all modules have non-empty paths
    #[test]
    fn all_modules_have_valid_paths() {
        for module in NIRI_LUA_API.modules {
            assert!(!module.path.is_empty(), "Module has empty path");
            assert!(
                module.path.starts_with("niri"),
                "Module path '{}' should start with 'niri'",
                module.path
            );
        }
    }

    /// Verify that all modules have descriptions
    #[test]
    fn all_modules_have_descriptions() {
        for module in NIRI_LUA_API.modules {
            assert!(
                !module.description.is_empty(),
                "Module '{}' has empty description",
                module.path
            );
        }
    }

    /// Verify that all functions have non-empty names and descriptions
    #[test]
    fn all_functions_have_valid_names_and_descriptions() {
        for module in NIRI_LUA_API.modules {
            for func in module.functions {
                assert!(
                    !func.name.is_empty(),
                    "Function in module '{}' has empty name",
                    module.path
                );
                assert!(
                    !func.description.is_empty(),
                    "Function '{}' in module '{}' has empty description",
                    func.name,
                    module.path
                );
            }
        }
    }

    /// Verify that all function parameters have valid names and types
    #[test]
    fn all_function_params_are_valid() {
        for module in NIRI_LUA_API.modules {
            for func in module.functions {
                for param in func.params {
                    assert!(
                        !param.name.is_empty(),
                        "Parameter in function '{}::{}' has empty name",
                        module.path,
                        func.name
                    );
                    assert!(
                        !param.ty.is_empty(),
                        "Parameter '{}' in function '{}::{}' has empty type",
                        param.name,
                        module.path,
                        func.name
                    );
                }
            }
        }
    }

    /// Verify that all function returns have valid types
    #[test]
    fn all_function_returns_have_types() {
        for module in NIRI_LUA_API.modules {
            for func in module.functions {
                for ret in func.returns {
                    assert!(
                        !ret.ty.is_empty(),
                        "Return in function '{}::{}' has empty type",
                        module.path,
                        func.name
                    );
                }
            }
        }
    }

    /// Verify that all types have valid names and descriptions
    #[test]
    fn all_types_have_valid_names_and_descriptions() {
        for ty in NIRI_LUA_API.types {
            assert!(!ty.name.is_empty(), "Type has empty name");
            assert!(
                !ty.description.is_empty(),
                "Type '{}' has empty description",
                ty.name
            );
        }
    }

    /// Verify that all type methods have valid names and descriptions
    #[test]
    fn all_type_methods_are_valid() {
        for ty in NIRI_LUA_API.types {
            for method in ty.methods {
                assert!(
                    !method.name.is_empty(),
                    "Method in type '{}' has empty name",
                    ty.name
                );
                assert!(
                    !method.description.is_empty(),
                    "Method '{}' in type '{}' has empty description",
                    method.name,
                    ty.name
                );
                assert!(
                    method.is_method,
                    "Method '{}' in type '{}' should have is_method=true",
                    method.name, ty.name
                );
            }
        }
    }

    /// Verify that all type aliases have valid names and types
    #[test]
    fn all_aliases_are_valid() {
        for alias in NIRI_LUA_API.aliases {
            assert!(!alias.name.is_empty(), "Alias has empty name");
            assert!(
                !alias.ty.is_empty(),
                "Alias '{}' has empty type",
                alias.name
            );
        }
    }

    /// Verify module paths are unique
    #[test]
    fn module_paths_are_unique() {
        let mut paths = HashSet::new();
        for module in NIRI_LUA_API.modules {
            assert!(
                paths.insert(module.path),
                "Duplicate module path: '{}'",
                module.path
            );
        }
    }

    /// Verify type names are unique
    #[test]
    fn type_names_are_unique() {
        let mut names = HashSet::new();
        for ty in NIRI_LUA_API.types {
            assert!(names.insert(ty.name), "Duplicate type name: '{}'", ty.name);
        }
    }

    /// Verify alias names are unique and don't conflict with type names
    #[test]
    fn alias_names_are_unique_and_no_type_conflicts() {
        let type_names: HashSet<_> = NIRI_LUA_API.types.iter().map(|t| t.name).collect();
        let mut alias_names = HashSet::new();

        for alias in NIRI_LUA_API.aliases {
            assert!(
                alias_names.insert(alias.name),
                "Duplicate alias name: '{}'",
                alias.name
            );
            assert!(
                !type_names.contains(alias.name),
                "Alias '{}' conflicts with a type name",
                alias.name
            );
        }
    }

    /// Verify expected modules are present
    #[test]
    fn expected_modules_present() {
        let module_paths: HashSet<_> = NIRI_LUA_API.modules.iter().map(|m| m.path).collect();

        let expected = [
            "niri",
            "niri.utils",
            "niri.config",
            "niri.action",
            "niri.state",
            "niri.loop",
            "niri.keymap",
        ];

        for path in expected {
            assert!(
                module_paths.contains(path),
                "Expected module '{}' not found",
                path
            );
        }
    }

    /// Verify expected types are present
    #[test]
    fn expected_types_present() {
        let type_names: HashSet<_> = NIRI_LUA_API.types.iter().map(|t| t.name).collect();

        let expected = ["Timer", "Animation", "Filter", "WindowRule", "Gesture"];

        for name in expected {
            assert!(
                type_names.contains(name),
                "Expected type '{}' not found",
                name
            );
        }
    }

    /// Verify expected aliases are present
    #[test]
    fn expected_aliases_present() {
        let alias_names: HashSet<_> = NIRI_LUA_API.aliases.iter().map(|a| a.name).collect();

        let expected = [
            "Window",
            "Workspace",
            "Output",
            "SizeChange",
            "WorkspaceReference",
        ];

        for name in expected {
            assert!(
                alias_names.contains(name),
                "Expected alias '{}' not found",
                name
            );
        }
    }

    /// Count statistics for the API schema
    #[test]
    fn schema_statistics() {
        let module_count = NIRI_LUA_API.modules.len();
        let type_count = NIRI_LUA_API.types.len();
        let alias_count = NIRI_LUA_API.aliases.len();

        let function_count: usize = NIRI_LUA_API.modules.iter().map(|m| m.functions.len()).sum();
        let field_count: usize = NIRI_LUA_API.modules.iter().map(|m| m.fields.len()).sum();
        let method_count: usize = NIRI_LUA_API.types.iter().map(|t| t.methods.len()).sum();

        // These assertions document expected minimums and will fail if schema shrinks unexpectedly
        assert!(
            module_count >= 7,
            "Expected at least 7 modules, got {module_count}"
        );
        assert!(
            type_count >= 5,
            "Expected at least 5 types, got {type_count}"
        );
        assert!(
            alias_count >= 5,
            "Expected at least 5 aliases, got {alias_count}"
        );
        assert!(
            function_count >= 50,
            "Expected at least 50 functions, got {function_count}"
        );
        assert!(
            field_count >= 3,
            "Expected at least 3 fields, got {field_count}"
        );
        assert!(
            method_count >= 10,
            "Expected at least 10 methods, got {method_count}"
        );
    }

    /// Verify niri.action module has key compositor actions
    #[test]
    fn action_module_has_key_functions() {
        let action_module = NIRI_LUA_API
            .modules
            .iter()
            .find(|m| m.path == "niri.action")
            .expect("niri.action module not found");

        let func_names: HashSet<_> = action_module.functions.iter().map(|f| f.name).collect();

        let expected = [
            "quit",
            "spawn",
            "close_window",
            "focus_window_up",
            "focus_window_down",
            "move_window_up",
            "move_window_down",
        ];

        for name in expected {
            assert!(
                func_names.contains(name),
                "Expected action '{}' not found in niri.action",
                name
            );
        }
    }

    /// Verify niri.state module has state query functions
    #[test]
    fn state_module_has_query_functions() {
        let state_module = NIRI_LUA_API
            .modules
            .iter()
            .find(|m| m.path == "niri.state")
            .expect("niri.state module not found");

        let func_names: HashSet<_> = state_module.functions.iter().map(|f| f.name).collect();

        let expected = ["windows", "focused_window", "workspaces", "outputs"];

        for name in expected {
            assert!(
                func_names.contains(name),
                "Expected state query '{}' not found in niri.state",
                name
            );
        }
    }
}
