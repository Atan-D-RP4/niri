# Niri-Lua Redundancy Check - Complete Analysis

## Report Overview

**Report File:** `niri-lua_redundancy_report_20250210.md`  
**Generated:** 2025-02-10  
**Crate Size:** 19,632 LOC across 26 source files

---

## Quick Navigation

### Executive Findings
- **3 CRITICAL issues** (HIGH confidence, actionable)
- **2 MODERATE issues** (MEDIUM confidence, review recommended)
- **3 INTENTIONAL deferrals** (Tier 5 features, keep as-is)
- **4 EASY WINS** (Clippy fixes, <1 hour)

### Quick Stats
| Category | Impact |  Effort |
|----------|--------|---------|
| Phase 1 (Quick Wins) | 39 LOC | 1-2 hrs |
| Phase 2 (Medium) | 119 LOC | 2-4 hrs |
| Phase 3 (Full) | 389 LOC | 4-6 hrs |
| **Total** | **~524 LOC** | **~8-12 hrs** |

---

## Critical Issues (Must Fix)

### 🔴 Issue #1: Position Change Parsing Duplication
- **File:** `src/action_proxy.rs:65-121`
- **Size:** 57 LOC
- **Problem:** Standalone implementation, mirrors `parse_utils.rs` logic
- **Solution:** Move to `parse_utils.rs`, reuse infrastructure
- **Confidence:** HIGH

### 🔴 Issue #2: Table-Building Boilerplate  
- **File:** `src/config_api.rs:52-150` (registers animations)
- **Size:** 270 LOC (56+ `.set()` calls, 12x repeated patterns)
- **Problem:** Macro-refactorable animation configuration
- **Solution:** Create `set_animation_table!` macro, reduce 150→20 LOC
- **Confidence:** HIGH

### 🔴 Issue #3: Repetitive Extractor Helpers
- **File:** `src/extractors.rs` (6 functions, 115+ `is_some()` chains)
- **Size:** 150 LOC
- **Problem:** Near-identical `extract_*_opt()` functions
- **Solution:** Generic `extract_opt<T>()` with `FromLua` trait
- **Confidence:** HIGH

---

## Moderate Issues (Review Recommended)

### 🟡 Issue #4: Duplicate Color Converters
- **File:** `src/config_api.rs:880-892`
- **Size:** 12 LOC (2 functions)
- **Problem:** `color_to_hex()` and `color_to_hex_noalpha()` identical except alpha
- **Solution:** Merge into single function with boolean parameter
- **Confidence:** MEDIUM

### 🟡 Issue #5: Test Boilerplate
- **Files:** Multiple test modules
- **Size:** 30 LOC
- **Problem:** Repeated Lua setup across test files
- **Solution:** Extract helpers to `test_utils.rs`
- **Confidence:** MEDIUM

---

## Intentional Deferrals (✓ Keep As-Is)

### ✓ Tier 5 Features
- `plugin_system.rs` (716 LOC) - Ready but not integrated
- `module_loader.rs` - Ready but not integrated
- Status: Keep as-is for future Tier 5 integration

### ✓ Design Decisions
- Dual-mode runtime API - Performance optimization, intentional
- Status: Keep defensive fallback logic, by design

---

## Quick Wins (Clippy Fixes)

### ⚡ 1. Remove `assert!(true)` - config.rs:674
```diff
- assert!(true); // Test passed
+ // (remove line)
```
Impact: 1 LOC, 0 effort

### ⚡ 2. Remove duplicate attribute - test_utils.rs:15  
```diff
- #![cfg(test)]
+ // (remove - already in lib.rs)
```
Impact: 1 LOC, 0 effort

### ⚡ 3. Fix bool assertion - config_api.rs:825
```diff
- assert_eq!(x, false)
+ assert!(!x)
```
Impact: 0 LOC (semantic), 0 effort

### ⚡ 4. Add allow annotations - 4 files (5 instances)
```diff
+ #[allow(clippy::arc_with_non_send_sync)]
+ // LuaRuntime is !Send but only used on main thread
  let handlers = Arc::new(Mutex::new(...));
```
Impact: 5 lines, 0 effort

---

## Dependencies Analysis

**Result:** ✓ All 10 workspace dependencies actively used

- ✓ anyhow - error handling
- ✓ async-channel - runtime API channels  
- ✓ calloop - timer manager
- ✓ log - debug/warn logging
- ✓ mlua - Lua runtime (core)
- ✓ niri-config - config extraction
- ✓ niri-ipc - action types
- ✓ regex - color validation
- ✓ serde - config structures
- ✓ serde_json - IPC serialization

**Action:** No dead dependencies. Keep all.

---

## Recommended Action Plan

### Week 1: Phase 1 (Quick Wins)
```
Task: Fix all clippy warnings and trivial issues
Time: 1-2 hours
Files: config.rs, test_utils.rs, config_api.rs, event_system.rs
PR: Small, low-risk refactor
```

### Week 2-3: Phase 2 (Medium Effort)
```
Task 1: Consolidate test helpers into test_utils.rs (30 LOC)
Task 2: Move position parsing to parse_utils.rs (57 LOC)
Task 3: Verify tempfile dependency usage (0-5 LOC)
Time: 2-4 hours
Files: extractors, action_proxy, parse_utils, test modules
PR: 1-2 medium PRs
```

### Week 4-5: Phase 3 (Full Refactoring)
```
Task 1: Extract animation table macro (150 LOC reduction)
Time: 2-3 hours
Files: config_api.rs
PR: Medium PR with snapshot tests

Task 2: Generic extractor refactoring (100+ LOC reduction)  
Time: 2-3 hours
Files: extractors.rs, all callers
PR: Medium-large PR with unit tests
```

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|-----------|
| Macro complexity reduces readability | Medium | Low | Use clear names, document with examples |
| Generic extraction breaks types | Low | High | Start with subset, add unit tests |
| Removing plugin system code too early | Low | High | DON'T remove—it's intentional, keep as-is |
| Tests fail after refactor | Low | High | Use snapshot tests for Lua values |

---

## Testing Strategy

### Phase 1
- Run full test suite: `cargo test`
- Clippy check: `cargo clippy --all-targets` (expect 0 warnings)

### Phase 2
- Re-run full test suite
- Integration test for moved parsing functions
- Verify test helper equivalence

### Phase 3
- Snapshot tests for generated Lua tables
- Unit tests for each FromLua impl
- Property-based tests for config structures

---

## File-by-File Summary

| File | LOC | Status | Issues |
|------|-----|--------|--------|
| lib.rs | 106 | ✓ Good | None |
| runtime.rs | 1388 | ✓ Good | None |
| config_api.rs | 950 | 🔴 Critical | Boilerplate (270 LOC) |
| action_proxy.rs | 1473 | 🔴 Critical | Position parsing dup (57 LOC) |
| extractors.rs | 1660 | 🔴 Critical | Repetitive helpers (150 LOC) |
| config_wrapper.rs | 2085 | ✓ Good | None |
| api_registry.rs | 2516 | ✓ Good | None |
| loop_api.rs | 793 | ✓ Good | None |
| validators.rs | 868 | ✓ Good | None |
| event_handlers.rs | 329 | ✓ Good | None |
| events_proxy.rs | 496 | ✓ Good | None (Arc warning) |
| plugin_system.rs | 716 | ✓ Deferred | Tier 5 feature (keep) |
| module_loader.rs | - | ✓ Deferred | Tier 5 feature (keep) |
| Other (13 files) | ~1700 | ✓ Good | None |
| **TOTAL** | **19,632** | | **~524 LOC improvable** |

---

## Report Sections in Detail

1. **Executive Summary** (page 1)
   - Overview of findings and impact estimates

2. **Section 1: Critical Findings**
   - Issue #1: Duplicate position parsing (57 LOC)
   - Issue #2: Table-building boilerplate (270 LOC)
   - Issue #3: Repetitive extractors (150 LOC)
   - With code examples and proposed solutions

3. **Section 2: Moderate Findings**
   - Issue #4: Duplicate color converters (12 LOC)
   - Issue #5: Test boilerplate (30 LOC)

4. **Section 3: Unused/Dead Code**
   - Plugin system & module loader (intentional deferral)
   - Config.rs unused branches

5. **Section 4: Efficiency Improvements**
   - Over-generalization in extractors
   - Performance considerations

6. **Section 5: Clippy Warnings**
   - 8 warnings documented and solutions provided

7. **Section 6: Dependency Analysis**
   - All dependencies verified as used

8. **Section 7: Technical Debt Summary**
   - Table with impact/effort estimates

9. **Section 8: Prioritized TODO List**
   - Phase-by-phase action plan with time estimates

10. **Section 9: Before/After Metrics**
    - Impact of each phase quantified

11. **Section 10: Methodology & Confidence**
    - How analysis was conducted, confidence levels

12. **Section 11: Risks & Mitigations**
    - Risk assessment with mitigation strategies

13. **Section 12: Recommendations**
    - Short/medium/long term guidance

14. **Section 13: Testing Strategy**
    - How to test each phase of refactoring

15. **Appendix: Code Snippets**
    - Ready-to-use macro proposals
    - Trait implementation examples
    - Helper function templates

---

## How to Use This Report

### For Code Reviewers
1. Start with "Critical Issues" section
2. Review the evidence sections with code snippets
3. Check confidence levels before recommending changes
4. See "Risks & Mitigations" before approving refactors

### For Project Maintainers  
1. Review "Recommendations for Maintainers" section
2. Follow prioritized TODO list in phases
3. Use "Testing Strategy" for each phase
4. Reference "Methodology" if disputing findings

### For Refactoring Work
1. Start with Phase 1 (quick wins, low risk)
2. Use code snippets in Appendix as templates
3. Cross-reference specific issues by file/line number
4. Follow testing strategy for each phase

---

## Contact & Questions

**Report Generated:** 2025-02-10  
**Crate Version:** 25.8.0+ (niri-lua)  
**Analysis Tool:** Custom Rust static analysis + grep verification  

For questions about specific findings, refer to the detailed section with:
- Line numbers
- Code snippets
- Confidence level
- Proposed solution

---

**END OF README**
