# Changelog with Security Notes

# 04 NOV 2025 - CORE EBNF COMPLETION

## Description: Completed all core features of the scanner, documentation for the Scanner-Development-Guide, and other testing Criterion Contracts to ensure all features are operational

# 27 DEC 2025 - RECORD FIELD VALIDATION & WILDCARD EXPANSION

## Description: Fixed critical issues with record field validation including wildcard path expansion, numeric index support in field paths, and corrected comparison operator direction for string and numeric operations

### Compiler Fixes (esp_compiler)
- **helpers.rs**: Added `Token::Integer` handling to `parse_field_path()` to support numeric indices in field paths (e.g., `spec.containers.0.name`)
- **blocks.rs**: Confirmed `is_data_type_name()` check prevents `field` keyword from being incorrectly parsed as a data type

### Scanner Base Fixes (esp_scanner_base)
- **comparisons.rs**: Fixed swapped `actual`/`expected` variable names in `compare_with()` method that caused `contains`, `not_contains`, and comparison operators (`>`, `<`, `>=`, `<=`) to evaluate in the wrong direction
- **record_validation.rs**: Implemented `expand_wildcard_path()` function for recursive JSON traversal supporting:
  - Field navigation (`PathComponent::Field`)
  - Array index access (`PathComponent::Index`)
  - Wildcard expansion (`PathComponent::Wildcard`)

### Features Now Enabled
- Numeric index paths: `field spec.containers.0.name string = \`value\``
- Wildcard paths: `field spec.containers.*.image string contains \`nginx\` at_least_one`
- Nested wildcards: `field spec.containers.*.ports.*.containerPort int > 1024 all`
- All entity checks with wildcards: `all`, `at_least_one`, `none`, `only_one`

### Validation
- 5/5 field keyword test policies passing
- 17 K8s STIG policies validated (9 compliant, 8 non-compliant as expected)
