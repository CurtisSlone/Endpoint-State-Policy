# ESP (Endpoint State Policy) - Consolidated EBNF Grammar

(*
ESP EBNF Grammar Notation
========================

- Based on ISO 14977 EBNF
- [abc] denotes character class (any of a, b, or c)
- [^abc] denotes negated character class (anything except a, b, or c)
- [a-z] denotes character range
- - denotes zero or more repetitions
- - denotes one or more repetitions
- ? denotes zero or one (optional)
- | denotes alternation (choice)
- "text" denotes literal text (case-sensitive)
- `` denotes empty string
- Whitespace: single space required between tokens, newline ends statements
- Comments: # to end of line
*)

(*
File Encoding Specification
===========================

- Files MUST be UTF-8 encoded
- No BOM (Byte Order Mark) allowed
- Line endings: \n (LF) preferred, \r\n (CRLF) accepted
- Identifiers: ASCII only [a-zA-Z0-9_]
- String literals: ASCII only [0x20-0x7E] plus whitespace
- Non-ASCII characters will cause parser errors
*)

## Design Principles and Separation of Duties

**Context**: ESP is a platform-agnostic intermediate language for compliance checking, designed to serve as a universal format for expressing compliance rules and validation logic. 
**Design Decision**: Clear separation between structural validation and semantic validation.

## Fundamental Rules

### **Case Sensitivity**

- **All elements are case-sensitive**
- Keywords must be uppercase: `DEF`, `VAR`, `STATE`, `OBJECT`, `CTN`, `CRI`, `SET`, `RUN`, `TEST`, `FILTER`, `META`
- Identifiers are case-sensitive: `my_var` ≠ `My_Var`, `state_check` ≠ `State_Check`
- Operations are lowercase symbols/tokens: `=`, `!=`, `ieq`, `contains`, etc.
- String content in backticks preserves case exactly as written

### **Identifier Uniqueness**

- **No duplicate identifiers within the same scope**
- Global scope: Variable names must be unique across all variables
- Global scope: Definition-level state identifiers must be unique
- Global scope: Definition-level object identifiers must be unique
- Global scope: Set identifiers must be unique
- CTN scope: Local state identifiers must be unique within that CTN
- CTN scope: Local object identifiers must be unique within that CTN
- Duplicate identifier in same scope = **Parser ERROR**

### **Reserved Keywords**

The following keywords are reserved and **cannot be used as identifiers**:

- Structure keywords: `DEF`, `VAR`, `STATE`, `OBJECT`, `CTN`, `CRI`, `SET`, `RUN`, `TEST`, `FILTER`, `META`, `parameters`, `select`, `record`
- End markers: `DEF_END`, `STATE_END`, `OBJECT_END`, `CTN_END`, `CRI_END`, `SET_END`, `RUN_END`, `FILTER_END`, `META_END`, `parameters_end`, `select_end`, `record_end`
- Reference keywords: `STATE_REF`, `OBJECT_REF`, `SET_REF`, `VAR`
- Logical operators: `AND`, `OR`
- Comparison operators: `=`, `!=`, `>`, `<`, `>=`, `<=`
- String operators: `ieq`, `ine`, `contains`, `starts`, `ends`, `not_contains`, `not_starts`, `not_ends`
- Set operators: `subset_of`, `superset_of`
- Pattern operators: `pattern_match`, `matches`
- Arithmetic operators: `+`, `*`, `-`, `/`, `%`

### **Numeric Type Limits**

- **int**: 64-bit signed integer
  - Range: -9,223,372,036,854,775,808 to 9,223,372,036,854,775,807
  - Overflow or underflow = **Parser ERROR**
- **float**: IEEE 754 double precision (64-bit)
  - Approximately 15-17 decimal digits of precision
  - Special values: Infinity, -Infinity, NaN handled per IEEE 754
- **Integer literals**: Must be within int range at parse time
- **Float literals**: Must be representable as IEEE 754 double

### **Minimum Valid Definition**

A valid ESP file must contain:

- One `DEF`...`DEF_END` block
- At least one `CRI`...`CRI_END` block within the DEF
- At least one `CTN` or nested `CRI` within each CRI block
- Empty definitions with no criteria = **Parser ERROR**

### **Block Ordering Rules**

#### **Within DEF Block**

Elements can appear in any order:

- Variable declarations (`VAR`)
- Definition-level states (`STATE`)
- Definition-level objects (`OBJECT`)
- Runtime operations (`RUN`)
- Set operations (`SET`)
- Criteria blocks (`CRI`)

#### **Within CTN Block**

Elements must appear in this specific order:

1. `TEST` specification (required)
2. `STATE_REF` references (optional, multiple allowed)
3. `OBJECT_REF` references (optional, multiple allowed)
4. Local `STATE` blocks (optional, multiple allowed)
5. Local `OBJECT` block (optional, only one allowed)

Violating CTN element order = **Parser ERROR**

### **Variable Initialization Rules**

- **No forward references**: Variables cannot reference other variables during initialization
- **Multi-pass resolution**: Parser collects all declarations before resolving references
- **Circular dependencies forbidden**: Circular initialization chains = **Parser ERROR**
- Example of circular dependency (ERROR):

  ```esp
  VAR var_one string VAR var_two
  VAR var_two string VAR var_one  # ERROR: Circular dependency
  ```

### **SET Operation Constraints**

- **union**: Requires 1 or more operands
- **intersection**: Requires 2 or more operands
- **complement**: Requires exactly 2 operands (A - B operation)
- Invalid operand count = **Parser ERROR**

### ** ESP Parser/Validator Responsibilities (This EBNF)**

- **Structural validation**: Block structure, grammar compliance, token format
- **Variable semantics**: Scope validation, reference resolution, type consistency
- **Universal patterns**: Data flow validation, logical operator correctness
- **Platform-agnostic validation**: Field syntax without platform-specific meaning
- **Reference integrity**: Ensuring variables/objects/states exist syntactically
- **Type compatibility**: Enforce type compatibility matrix for operations

### **Parser Implementation Requirements**

- **Lookahead**: Minimum 2-token lookahead required
  - Distinguish STATE from STATE_REF
  - Distinguish object fields from keywords
- **Multi-Pass Architecture**: Required for forward references
  - Pass 1: Lexical analysis
  - Pass 2: Syntax parsing
  - Pass 3: Symbol discovery
  - Pass 4: Reference resolution
  - Pass 5: Semantic analysis
  - Pass 6: Structural validation
- **Circular Dependency Detection**: Required in dependency analysis pass
  - Detect cycles in variable references
  - Detect cycles in state references
  - Report all cycles found

### **Compliance Scanner Responsibilities**

- **Platform-specific semantics**: Field appropriateness for criterion types (e.g., `filepath` invalid for `registry` tests)
- **Business logic**: Value ranges, platform compatibility, runtime accessibility
- **Content validation**: Platform-specific syntax (registry hives, file paths, SQL)
- **Runtime verification**: Actual system state checking
- **Value format validation**: Whether registry hive names, file paths are valid
- **Regex pattern execution**: Platform-specific regex engine interpretation

### **Regex Pattern Handling**

- Parser stores patterns as literal text without validation
- Scanner responsible for platform-specific regex interpretation
- Users must ensure patterns are valid for target platform
- Common regex features should be documented for portability

## File Structure

```ebnf
(* Start symbol - ESP file structure *)
esp_file ::= metadata? definition

(* Comments - single line only, anywhere in file *)
comment ::= "#" [^\n]* newline
```

## Metadata Block

```ebnf
(* Metadata block - must be at top of file before DEF *)
metadata ::= "META" statement_end metadata_content "META_END" statement_end
metadata_content ::= (metadata_field | comment_line)*
metadata_field ::= field_name space field_value statement_end
field_name ::= identifier
field_value ::= backtick_string | integer_value | float_value | boolean_value
```

### Standard Metadata Fields

| Field | Description | Format | Example |
|-------|-------------|--------|---------|
| version | Definition version | semver | `1.2.0` |
| esp_version | Required ESP version | semver | `1.0` |
| author | Author/team name | string | `security-team` |
| date | Creation/update date | ISO 8601 | `2024-01-15` |
| severity | Severity level | enum | `high\|medium\|low` |
| platform | Target platform | string | `windows\|linux\|macos` |
| description | Human-readable description | string | any text |
| category | Classification | string | `security\|compliance` |
| tags | Comma-separated tags | string | `tag1,tag2,tag3` |
| compliance_framework | Framework reference | string | `NIST-800-53` |

**Note:** Parser accepts any metadata fields. Scanner may validate specific fields.

## Core Structure

```ebnf
definition ::= "DEF" statement_end definition_content "DEF_END" statement_end
(* Definition must contain at least one CRI block *)
definition_content ::= definition_elements criteria+
definition_elements ::= (variable_declarations | definition_states | definition_objects |
                        runtime_operations | set_operations | comment_line)*

criteria ::= "CRI" space logical_operator space? negate_flag? statement_end
             criteria_content "CRI_END" statement_end
logical_operator ::= "AND" | "OR"
negate_flag ::= "true"
(* CRI must contain at least one CTN or nested CRI *)
criteria_content ::= (criteria | criterion)+

criterion ::= "CTN" space criterion_type statement_end ctn_content "CTN_END" statement_end
criterion_type ::= identifier
(* CTN elements must appear in strict order *)
ctn_content ::= test_specification state_references? object_references?
                ctn_states? ctn_object?
```

## Variables and Runtime Operations

```ebnf
(* All variables are global scope *)
variable_declarations ::= variable_declaration+
variable_declaration ::= "VAR" space variable_name space data_type
                        (space initial_value)? statement_end
initial_value ::= direct_value
variable_name ::= identifier  (* Simple identifier following [a-zA-Z_][a-zA-Z0-9_]* *)

runtime_operations ::= run_block+
run_block ::= "RUN" space variable_name space operation_type statement_end
              run_parameters "RUN_END" statement_end
(* Empty RUN blocks not allowed - must have parameters *)

operation_type ::= "CONCAT" | "SPLIT" | "SUBSTRING" | "REGEX_CAPTURE" | "ARITHMETIC" |
                  "COUNT" | "UNIQUE" | "END" | "MERGE" | "EXTRACT"

run_parameters ::= run_parameter+  (* At least one required *)
run_parameter ::= parameter_line statement_end
parameter_line ::= literal_component | variable_component | object_component |
                  pattern_spec | delimiter_spec | character_spec |
                  start_position | length_value | arithmetic_op

(* Core component types - all flat, no nesting *)
literal_component ::= "literal" space (backtick_string | integer_value)
variable_component ::= "VAR" space variable_name  (* Exactly one space after VAR *)
object_component ::= object_extraction

(* Object extraction from global objects *)
object_extraction ::= "OBJ" space object_identifier space item_field
object_identifier ::= identifier
item_field ::= identifier

(* Operation-specific parameters *)
pattern_spec ::= "pattern" space backtick_string
delimiter_spec ::= "delimiter" space backtick_string
character_spec ::= "character" space backtick_string
start_position ::= "start" space integer_value
length_value ::= "length" space integer_value
arithmetic_op ::= "+" | "*" | "-" | "/" | "%"
```

## Test Specifications

```ebnf
test_specification ::= "TEST" space existence_check space item_check
                      (space state_operator)? statement_end

existence_check ::= "any" | "all" | "none" |
                   "at_least_one" | "only_one"

item_check ::= "all" | "at_least_one" | "only_one" | "none_satisfy"

state_operator ::= "AND" | "OR" | "ONE"
```

## State References and Definitions

```ebnf
(* State references - ONLY for definition-level (global) states *)
state_references ::= state_reference+
state_reference ::= "STATE_REF" space state_identifier statement_end

(* Definition-level states: Global scope, can be referenced *)
definition_states ::= definition_state+
definition_state ::= "STATE" space state_identifier statement_end
                    state_content "STATE_END" statement_end

(* CTN-level states: Local scope, CANNOT be referenced *)
ctn_states ::= ctn_state+
ctn_state ::= "STATE" space state_identifier statement_end
              state_content "STATE_END" statement_end

(* Identifier is REQUIRED for all STATE blocks *)
state_identifier ::= identifier  (* Simple identifier following [a-zA-Z_][a-zA-Z0-9_]* *)

state_content ::= state_fields  (* Must have at least one field - no empty states *)
state_fields ::= (state_field | record_check | comment_line)+
comment_line ::= comment newline

state_field ::= field_name space data_type space operation space value_spec statement_end
field_name ::= identifier

(* Record datatype support *)
record_check ::= "record" space data_type? statement_end
                record_content "record_end" statement_end
record_content ::= direct_operation | nested_fields

direct_operation ::= operation space value_spec statement_end
nested_fields ::= record_field+

record_field ::= "field" space field_path space data_type space operation
                space value_spec (space entity_check)? statement_end
field_path ::= path_component ("." path_component)*
path_component ::= identifier | wildcard
wildcard ::= "*"

entity_check ::= "all" | "at_least_one" | "none" | "only_one"
```

## Object Specifications

```ebnf
((* Object references - ONLY for definition-level (global) objects *)
object_references ::= object_reference+
object_reference ::= "OBJECT_REF" space object_identifier statement_end

(* Definition-level objects: Global scope, can be referenced *)
definition_objects ::= definition_object+
definition_object ::= "OBJECT" space object_identifier statement_end
                      object_content "OBJECT_END" statement_end

(* CTN-level objects: Local scope, CANNOT be referenced *)
ctn_object ::= "OBJECT" space object_identifier statement_end
               object_content "OBJECT_END" statement_end

(* Identifier is REQUIRED for all OBJECT blocks *)
object_identifier ::= identifier  (* Simple identifier following [a-zA-Z_][a-zA-Z0-9_]* *)

(* Object content structure - must have at least one element *)
object_content ::= object_elements  (* No empty objects allowed *)
object_elements ::= (object_element | comment_line)+
object_element ::= module_element | parameter_element | select_element |
                   behavior_element | filter_spec | set_reference | object_field

(* Complex object elements with explicit terminators *)
module_element ::= module_field space backtick_string statement_end
module_field ::= "module_name" | "verb" | "noun" | "module_id" | "module_version"

parameter_element ::= "parameters" space data_type statement_end
                     parameter_fields? "parameters_end" statement_end
parameter_fields ::= parameter_field+
parameter_field ::= identifier space field_value statement_end

select_element ::= "select" space data_type statement_end
                  select_fields? "select_end" statement_end
select_fields ::= select_field+
select_field ::= identifier space field_value statement_end

behavior_element ::= "behavior" space behavior_value+ statement_end
behavior_value ::= identifier | integer_value | boolean_value

(* Filters can ONLY reference definition-level states *)
filter_spec ::= "FILTER" space filter_action? statement_end
                filter_references "FILTER_END" statement_end
filter_action ::= "include" | "exclude"
filter_references ::= state_reference+  (* Must have at least one reference *)

(* Set references - can be used within objects *)
set_reference ::= "SET_REF" space set_identifier statement_end

(* Object fields - must be unique within object *)
object_field ::= field_name space field_value statement_end
field_value ::= backtick_string | variable_reference | identifier
(* Duplicate field_name within same object = ERROR *)
```

## Set Operations

```ebnf
(* Sets are global scope only *)
set_operations ::= set_block+
set_block ::= "SET" space set_identifier space set_operation statement_end
              set_content "SET_END" statement_end

set_identifier ::= identifier  (* Simple identifier following [a-zA-Z_][a-zA-Z0-9_]* *)

set_operation ::= "union" | "intersection" | "complement"

(* Set content must have appropriate number of operands based on operation *)
set_content ::= set_operands set_filter?
set_operands ::= set_operand+  (* Number validated based on operation type *)
set_operand ::= operand_type statement_end
operand_type ::= object_spec | object_reference | set_reference

object_spec ::= "OBJECT" statement_end object_content "OBJECT_END"
set_reference ::= "SET_REF" space set_identifier

(* SET filters can ONLY reference definition-level states *)
set_filter ::= "FILTER" space filter_action? statement_end
               filter_references "FILTER_END"
(* filter_references defined in object section - must have at least one *)
```

## Values and Types

```ebnf
value_spec ::= direct_value | variable_reference

direct_value ::= backtick_string | integer_value | boolean_value | multiline_string
variable_reference ::= "VAR" space variable_name

(* Data types *)
data_type ::= "string" | "int" | "float" | "boolean" | "binary" | "record_type" |
              "version" | "evr_string"

(* Operations *)
operation ::= comparison_op | string_op | set_op | pattern_op

comparison_op ::= "=" | "!=" | ">" | "<" | ">=" | "<="

string_op ::= "ieq" | "ine" | "contains" | "starts" | "ends" | "not_contains" | "not_starts" | "not_ends"

set_op ::= "subset_of" | "superset_of"

pattern_op ::= "pattern_match" | "matches"
```

## String Literals and Tokens

```ebnf
(* String literals - everything inside backticks is literal text *)
(* Use `` for literal backtick within string *)
backtick_string ::= "`" backtick_content* "`"
backtick_content ::= non_backtick | escaped_backtick
non_backtick ::= [^`]
escaped_backtick ::= "``"  (* `` represents one literal backtick *)

(* Empty string representation *)
empty_string ::= "``"  (* Empty backticks = empty string *)

(* Raw strings - same as backtick strings *)
raw_string ::= "r`" raw_content* "`"
raw_content ::= non_backtick | escaped_backtick

(* Multiline strings - everything between triple backticks is literal *)
multiline_string ::= triple_backtick | raw_multiline
triple_backtick ::= "```" multiline_content "```"
raw_multiline ::= "r```" multiline_content "```"
multiline_content ::= ([^`] | "`" [^`] | "``" [^`])*  (* Any content except ``` *)

(* Basic tokens - outside backticks *)
identifier ::= [a-zA-Z_][a-zA-Z0-9_]*
integer_value ::= "-"? [0-9]+
float_value ::= "-"? [0-9]+ "." [0-9]+
boolean_value ::= "true" | "false"

(* Whitespace and line handling *)
space ::= " "                  (* Exactly one space *)
spaces ::= space+               (* One or more spaces treated as single space *)
tab ::= "\t"                    (* Tab treated as single space *)
whitespace ::= (space | tab)+  (* Any whitespace normalized to single space *)
newline ::= "\n" | "\r\n"       (* Line terminator *)
empty_line ::= whitespace? newline  (* Blank lines are ignored *)

(* Statement termination *)
statement_end ::= whitespace? comment? newline  (* How statements must end *)
comment ::= "#" [^\n]*          (* Comments to end of line *)

(* NO ESCAPE SEQUENCES outside backticks - only valid tokens allowed *)
```

## Type Compatibility Matrix

### Operations by Data Type

| Operation | string | int | float | boolean | binary | record | version | evr_string |
|-----------|--------|-----|-------|---------|--------|--------|---------|------------|
| **Comparison Operators** |
| = | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| != | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| > | ✓¹ | ✓ | ✓ | ✗ | ✗ | ✗ | ✓² | ✓² |
| < | ✓¹ | ✓ | ✓ | ✗ | ✗ | ✗ | ✓² | ✓² |
| >= | ✓¹ | ✓ | ✓ | ✗ | ✗ | ✗ | ✓² | ✓² |
| <= | ✓¹ | ✓ | ✓ | ✗ | ✗ | ✗ | ✓² | ✓² |
| **String Operators** |
| ieq | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| ine | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| contains | ✓ | ✗ | ✗ | ✗ | ✓³ | ✗ | ✗ | ✗ |
| starts | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| ends | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| not_contains | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| **Pattern Operators** |
| pattern_match | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| matches | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| **Set Operators** |
| subset_of | ✓⁴ | ✓⁴ | ✓⁴ | ✓⁴ | ✗ | ✗ | ✗ | ✗ |
| superset_of | ✓⁴ | ✓⁴ | ✓⁴ | ✓⁴ | ✗ | ✗ | ✗ | ✗ |

**Notes:**

1. String comparison is lexicographic
2. Version comparison follows semantic versioning rules
3. Binary contains performs byte sequence search
4. Set operators require collection types from SET operations

### RUN Operation Type Compatibility

| Operation | Valid Input Types | Output Type |
|-----------|------------------|-------------|
| CONCAT | string only | string |
| SPLIT | string | string (array internally) |
| SUBSTRING | string | string |
| REGEX_CAPTURE | string | string |
| ARITHMETIC | int, float | same as input |
| COUNT | any collection | int |
| UNIQUE | any collection | same as input |
| MERGE | collections of same type | same as input |
| EXTRACT | object | varies by field |

## Implementation Limits (Recommended)

| Constraint | Recommended Limit | Rationale |
|------------|------------------|-----------|
| Max symbols per definition | 10,000 | Memory/performance |
| Max string literal size | 1 MB | Memory management |
| Max nesting depth | 10 levels | Stack overflow prevention |
| Max identifier length | 255 chars | Readability |
| Max line length | 4,096 chars | Buffer management |
| Max file size | 10 MB | Performance |
| Max SET operands | 100 | Complexity management |
| Max CTN per CRI | 1,000 | Performance |

**Note:** Implementations may adjust these limits based on target environment.

## Unified Scoping Model

### Core Principle

**Elements that can be referenced must be defined at the definition level (global scope).**
**Local elements cannot be referenced.**

### Scope Rules by Element Type

| Element | Definition-Level (Global) | CTN-Level (Local) | Reference Keyword |
|---------|--------------------------|-------------------|-------------------|
| **Variables** | ✅ All variables | N/A | `VAR` |
| **States** | ✅ Referenceable | ✅ Non-referenceable | `STATE_REF` |
| **Objects** | ✅ Referenceable | ✅ Non-referenceable | `OBJECT_REF` |
| **Sets** | ✅ All sets | N/A | `SET_REF` |

### Definition Structure and Execution Order

```
# Optional metadata block (must be first)
META
  version `1.0.0`
  author `security-team`
META_END

DEF
  # Global declarations (referenceable)
  [Variable Declarations]     # VAR blocks - all global
  [Definition States]         # STATE blocks - global/referenceable
  [Definition Objects]        # OBJECT blocks - global/referenceable

  # Operations on global elements
  [Runtime Operations]        # RUN blocks - operate on global elements
  [Set Operations]           # SET blocks - create global sets

  # Criteria with local elements
  [Criteria]                 # CRI blocks
    [CTN Blocks]
      [TEST]                 # Test specification
      [State References]     # STATE_REF - reference global states
      [Object References]    # OBJECT_REF - reference global objects
      [Local States]         # STATE blocks - local, non-referenceable
      [Local Object]         # OBJECT block - local, non-referenceable
DEF_END
```

## Reference Resolution Rules

### Valid References

- `VAR variable_name` → Any variable (all are global)
- `STATE_REF state_id` → Definition-level states only
- `OBJECT_REF object_id` → Definition-level objects only
- `SET_REF set_id` → Set identifier only
- `OBJ object_id field` → Extract field from global object (in RUN operations)

### Invalid References (Parser Errors)

- `STATE_REF` → CTN-level state (ERROR: cannot reference local state)
- `OBJECT_REF` → CTN-level object (ERROR: cannot reference local object)
- Any reference to non-existent identifier (ERROR: undefined symbol)
- Any unrecognized token outside backticks (ERROR: invalid token)

## Complete Example

```esp
# ESP Definition for Windows Security Compliance
# Version 2.1.0 - January 2024

META
version `2.1.0`
esp_version `1.0`
author `security-team`
date `2024-01-15`
severity `high`
platform `windows`
description `Validates Windows registry settings for enterprise security compliance`
compliance_framework `NIST-800-53`
tags `registry,security,enterprise`
META_END

DEF
# Global variables - all referenceable (simple identifiers)
VAR base_path string `C:\Windows\System32`  # Backslash is literal
VAR threshold int 1024  # Within 64-bit signed range
VAR temp_config string

# Global states - referenceable via STATE_REF (simple identifiers)
STATE size_check
    size int > VAR threshold
STATE_END

STATE permission_check
    permissions string = `0644`
    # The backticks contain literal text including \n
    content string contains `SECURE\nENABLED`
    # Example with literal backtick using ``
    description string = `This is a ``backtick`` character`
STATE_END

# Global objects - referenceable via OBJECT_REF (simple identifiers)
OBJECT system_registry
    hive `HKEY_LOCAL_MACHINE`
    key `Software\Policies\Security`  # Literal backslashes
    name `PolicyValue`
OBJECT_END

OBJECT config_file
    path VAR base_path
    filename `security.conf`

    parameters record
        CommandName Get-Process
        FilterScript {$_.CPU -gt 100}
        MaxCount 50
    parameters_end

    select string
        Name ProcessName
        CPU CPUUsage
        Memory WorkingSet
    select_end
OBJECT_END

# Runtime operations on global elements
RUN temp_config CONCAT
    VAR base_path
    literal `\config`
RUN_END

RUN calculation ARITHMETIC
    VAR threshold
    + 100
    * 2
RUN_END

RUN extracted_hive EXTRACT
    OBJ system_registry hive
RUN_END

# Global sets - referenceable via SET_REF (simple identifiers)
SET critical_objects union  # union can have 1+ operands
    OBJECT_REF system_registry
    OBJECT_REF config_file
    FILTER include
        STATE_REF permission_check
    FILTER_END
SET_END

# Example of intersection (requires 2+ operands)
SET common_objects intersection
    SET_REF critical_objects
    OBJECT inline_object
        type `security`
    OBJECT_END
SET_END

# Example of complement (requires exactly 2 operands)
SET difference_objects complement
    SET_REF critical_objects
    SET_REF common_objects
SET_END

# Criteria section with tests (required for valid definition)
CRI AND
CTN registry_check
    TEST any all OR
    STATE_REF size_check         # Reference global state
    OBJECT_REF system_registry   # Reference global object
CTN_END

CTN file_check
    # CTN elements must follow strict order: TEST, STATE_REF, OBJECT_REF, STATE, OBJECT
    TEST all all
    STATE_REF permission_check   # Reference global state
    # Local state - not referenceable outside this CTN (unique within CTN)
    STATE local_content
        content string contains `SECURE=true`
        # Check for literal text with special characters
        path string = `C:\Program Files\App\config.ini`
        # Pattern match - scanner interprets for platform
        name string pattern_match `^SEC-\d{4}$`
    STATE_END

    # Local object - not referenceable outside this CTN (unique within CTN)
    OBJECT local_file
        path VAR temp_config              # Variables always referenceable
        filename `*.conf`
        behavior max_depth 3

        parameters string
            timeout 30
            retries 3
        parameters_end

        select record
            path filepath
            size filesize
        select_end
    OBJECT_END
CTN_END

CTN set_validation
    TEST at_least_one all
    OBJECT set_check
        SET_REF critical_objects # Reference global set
    OBJECT_END
CTN_END

CTN empty_check
    TEST any all
    STATE empty_test
        value string = `` # Empty string check
    STATE_END
    OBJECT empty_test
        path ``                      # Empty path
    OBJECT_END
CTN_END

# Example showing case sensitivity and new operators
CTN operator_examples
    TEST any all
    STATE text_operations
        name string ieq `SYSTEM`           # case insensitive equals
        path string starts `/usr/bin`      # starts with
        content string not_contains `ERROR` # not contains
        file_list string subset_of VAR allowed_files
        version string >= `2.0.0`          # version comparison
        status boolean != true              # boolean comparison
    STATE_END
    OBJECT case_test
        # VAR is uppercase keyword, base_path is case-sensitive identifier
        path VAR base_path
    OBJECT_END
CTN_END
CRI_END
DEF_END
```

## Key Design Features

### **1. Simplified Identifiers**

- All identifiers follow simple pattern: `[a-zA-Z_][a-zA-Z0-9_]*`
- No required prefixes (`var_`, `ste_`, `obj_`, `temp_`)
- Consistent naming across variables, states, objects, and sets
- Case-sensitive for uniqueness validation

### **2. Symbolic Operators**

- Comparison operators use standard symbols: `=`, `!=`, `>`, `<`, `>=`, `<=`
- String operators use concise tokens: `ieq`, `ine`, `contains`, `starts`, `ends`, `not_contains`, `not_starts`, `not_ends`
- Set operators use underscore notation: `subset_of`, `superset_of`
- Pattern operators: `pattern_match`, `matches`
- No spaces in operator names for easy parsing

### **3. Explicit Block Terminators**

- `parameters`...`parameters_end` blocks for clear parsing boundaries
- `select`...`select_end` blocks for structured field definitions
- No indentation-based parsing required
- Easy to implement state machine parsing

### **4. String Literal Simplicity**

- Everything inside backticks is literal text
- Use `` for literal backtick character
- Use `` for empty string
- No escape sequences needed inside backticks
- Clean representation of paths, regex, and special characters

### **5. Strict Token Recognition**

- Outside backticks, only valid tokens accepted
- Unrecognized characters cause parser errors
- Clear separation between literal text and structure

### **6. Metadata Block**

- Optional but recommended
- Must be at top of file before DEF
- Extensible key-value structure
- Values in backticks for consistency
- Standard fields defined but custom fields allowed

### **7. Unified Scoping Model**

- Consistent rule: "To be referenced, must be global"
- Clear distinction between global (referenceable) and local (non-referenceable)
- Explicit reference keywords for clarity

### **8. Comment Support**

- Single-line comments with # symbol
- Can appear anywhere in the file
- Continue until end of line

### **9. Flattened RUN Operations**

- All RUN blocks use flat parameter structure (no nesting)
- Simple variable support for operation chaining
- EXTRACT operation for object field extraction

### **10. Enhanced Object Support**

- Module specifications (PowerShell cmdlets, etc.)
- Explicit parameters and select blocks with terminators
- Behavior specifications with attribute-value pairs
- Simple field values (identifiers or backtick strings)

## Architectural Principles

### **Separation of Concerns**

- **Parser Domain**: Structural validation, reference resolution, type consistency
- **Scanner Domain**: Platform-specific semantics, business logic, runtime verification

### **Multi-Pass Architecture Required**

1. **Lexical Analysis**: Token generation and string literal extraction
2. **Syntax Parsing**: AST construction following EBNF rules
3. **Symbol Discovery**: Collect all global declarations
4. **Reference Resolution**: Validate all references point to global symbols
5. **Semantic Analysis**: Type checking and dependency validation
6. **Structural Validation**: Final patterns and completeness checks

### **Forward References Supported**

Symbols can be used before declaration since all symbols are discovered before resolution begins.

### **Parser Validation Responsibilities**

- Ensure STATE_REF only points to definition-level states
- Ensure OBJECT_REF only points to definition-level objects
- Ensure SET_REF only points to valid set identifiers
- Ensure VAR references resolve to declared variables
- Report errors for any unrecognized tokens outside backticks
- Validate block structure and nesting
- Enforce type compatibility matrix
- Detect circular dependencies
- Validate parameters/select block terminators
- Validate operator usage per type compatibility matrix

## Implementation Notes

### Symbol Table Structure

```
Global Symbols:
- variables: Map<string, Variable>      // All VAR declarations
- states: Map<string, State>           // Definition-level states
- objects: Map<string, Object>         // Definition-level objects
- sets: Map<string, Set>              // SET operation results

Local Symbols (per CTN):
- states: Map<string, State>          // CTN-level states
- objects: Map<string, Object>        // CTN-level objects
```

### Error Categories

- **Lexical Errors**: Unrecognized tokens outside backticks
- **Syntax Errors**: Malformed block structure, missing END markers
- **Reference Errors**: Undefined symbols, wrong scope references
- **Type Errors**: Type mismatches in operations per compatibility matrix
- **Semantic Errors**: Circular dependencies, invalid operations
- **Limit Errors**: Exceeding implementation limits
- **Terminator Errors**: Missing parameters_end or select_end markers
- **Operator Errors**: Invalid operator usage for given data types

### Character Encoding Errors

- **Non-ASCII Character**: Unicode or extended ASCII characters in string literals
- **Invalid Encoding**: Malformed UTF-8 byte sequences
- **Unsupported Character**: Characters outside printable ASCII range [0x20-0x7E]