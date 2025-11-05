# ESP EBNF Implementation Checklist

## 🔴 CRITICAL - Must Complete for EBNF Compliance

### 1. SET Expansion (3-5 days)

- [ ] Fix borrow checker in `set_expansion.rs` lines 44-47
- [ ] Complete `expand_set_ref_in_declaration()` function
- [ ] Add `expand_all_sets()` entry point
- [ ] Integrate with `ResolutionEngine::resolve_context()`
- [ ] Test: Basic SET union/intersection/complement
- [ ] Test: Nested SET_REF expansion
- [ ] Test: Circular dependency detection
- [ ] Test: SET with FILTER spec

### 2. FILTER Evaluation (2-3 days)

- [ ] Complete `FilterEvaluator::evaluate_filter()`
- [ ] Implement `evaluate_state_against_data()`
- [ ] Add `apply_object_filters()` function
- [ ] Integrate with `ExecutionEngine::collect_data_for_criterion()`
- [ ] Test: FILTER include on match
- [ ] Test: FILTER exclude on match
- [ ] Test: Multiple FILTER specs (AND logic)
- [ ] Test: SET-level FILTER application

---

## 🟠 HIGH - Needed for Full Feature Set

### 3. BEHAVIOR Framework (3-4 days)

- [ ] Add `BehaviorSpec` struct to `CtnContract`
- [ ] Add `validate_behaviors()` method
- [ ] Update `file_contracts.rs` with behavior declarations
- [ ] Update `rpm_contracts.rs` with timeout/cache behaviors
- [ ] Update all other contract files
- [ ] Test: Behavior validation
- [ ] Test: Unsupported behavior rejection
- [ ] Test: Required behavior enforcement

### 4. Pattern Matching (1 day)

- [ ] Create `execution/comparisons/pattern.rs`
- [ ] Implement regex compilation with caching
- [ ] Add `pattern_match()` function
- [ ] Integrate with `string.rs` comparisons
- [ ] Add regex/lazy_static to Cargo.toml
- [ ] Test: Basic pattern matching
- [ ] Test: Regex cache usage
- [ ] Test: Invalid pattern error handling

---

## 🟡 MEDIUM - Nice to Have

### 5. EVR Comparison (2-3 days)

- [ ] Create `types/evr.rs` module
- [ ] Implement `EvrString::parse()`
- [ ] Implement RPM comparison algorithm
- [ ] Implement `Ord` trait for `EvrString`
- [ ] Create `execution/comparisons/evr.rs`
- [ ] Integrate with comparison engine
- [ ] Test: Epoch precedence
- [ ] Test: Numeric segment comparison
- [ ] Test: Release comparison
- [ ] Test: Complex version strings

---

## Integration & Testing

### Integration Points

- [ ] Verify SET expansion called in resolution phase 6
- [ ] Verify FILTER evaluation in collection phase
- [ ] Verify pattern/EVR in comparison routing
- [ ] Verify all contracts have behavior declarations

### Unit Tests (20+ tests)

- [ ] SET: Union, intersection, complement
- [ ] SET: Nested refs, circular detection
- [ ] FILTER: Include/exclude logic
- [ ] FILTER: Multiple state evaluation
- [ ] BEHAVIOR: Validation and parameters
- [ ] PATTERN: Basic matching and cache
- [ ] EVR: All comparison operators

### Integration Tests (5 ESP files)

- [ ] Create `tests/set_expansion_test.esp`
- [ ] Create `tests/filter_test.esp`
- [ ] Create `tests/behavior_test.esp`
- [ ] Create `tests/pattern_test.esp`
- [ ] Create `tests/evr_test.esp`
- [ ] Run full test suite

---

## Documentation

- [ ] Update README with new features
- [ ] Document BEHAVIOR specifications per CTN type
- [ ] Add pattern matching examples
- [ ] Add EVR comparison examples
- [ ] Update ESP Guide with complete syntax

---

## Current Status

**Overall Completion:** 82%

| Feature | Status | Priority | Effort |
|---------|--------|----------|--------|
| SET Expansion | 40% (needs fixes) | 🔴 CRITICAL | 3-5 days |
| FILTER Evaluation | 20% (stub only) | 🔴 CRITICAL | 2-3 days |
| BEHAVIOR Framework | 60% (partial) | 🟠 HIGH | 3-4 days |
| Pattern Matching | 0% (not started) | 🟡 MEDIUM | 1 day |
| EVR Comparison | 0% (not started) | 🟡 MEDIUM | 2-3 days |

**Total Remaining Effort:** 11-16 days

---

## Implementation Order

### Week 1: Critical Path

1. Day 1-2: Fix SET expansion
2. Day 3: Complete SET logic
3. Day 4: Complete FILTER evaluation
4. Day 5: Integration testing

### Week 2: Features

6. Day 6-7: BEHAVIOR framework
7. Day 8: Pattern matching
8. Day 9-10: Testing and fixes

### Week 3: Polish

9. Day 11-13: EVR comparison
10. Day 14-15: End-to-end testing
11. Day 16: Documentation

---

## Files to Modify

### Scanner Base

```
esp_scanner_base/src/
├── resolution/
│   ├── set_expansion.rs        [FIX + COMPLETE]
│   └── engine.rs               [ADD phase 6 call]
├── execution/
│   ├── filter_evaluation.rs    [COMPLETE implementation]
│   ├── engine.rs               [ADD filter in collection]
│   └── comparisons/
│       ├── pattern.rs          [NEW]
│       └── evr.rs              [NEW]
├── types/
│   └── evr.rs                  [NEW]
└── strategies/
    └── ctn_contract.rs         [EXTEND with BehaviorSpec]
```

### Scanner SDK

```
esp_scanner_sdk/src/
├── contracts/
│   ├── file_contracts.rs       [UPDATE behaviors]
│   ├── rpm_contracts.rs        [UPDATE behaviors]
│   ├── systemd_contracts.rs    [UPDATE behaviors]
│   ├── sysctl_contracts.rs     [UPDATE behaviors]
│   └── selinux_contracts.rs    [UPDATE behaviors]
└── Cargo.toml                  [ADD regex, lazy_static]
```

---

## Next Action

**Choose one to start:**

1. **SET Expansion** (most critical blocker)
   - Fixes immediate EBNF compliance gap
   - Unblocks SET_REF usage

2. **FILTER Evaluation** (good starting point)
   - Cleaner implementation (no borrow issues)
   - Builds confidence
   - High value feature

3. **Pattern Matching** (quick win)
   - Simple, standalone feature
   - Immediate value
   - Good for momentum
