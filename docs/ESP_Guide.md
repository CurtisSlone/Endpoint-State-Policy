# Endpoint State Policy (ESP) Language Guide

# Part I – Foundations

## 1. Introduction

The **Endpoint State Policy (ESP)** is a **compliance-as-data fabric language** designed to express security and compliance rules in a structured, machine-readable form. Rather than treating compliance rules as code, ESP represents them as **data definitions** that can be evaluated, versioned, and integrated into a broader trust infrastructure.

ESP serves as the **policy engine** within the ScanSet platform, enabling organizations to map high-level policy requirements (such as STIGs, CIS Benchmarks, or NIST controls) into declarative checks that systems can automatically validate.

By separating *policy definition* (ESP files) from *policy execution* (ScanSet’s evaluation engine and trust infrastructure), ESP provides the foundation for building **Zero Trust Architecture (ZTA) enhancements** where compliance posture becomes part of the trust fabric.

### Why ESP?

* **Compliance-as-Data**: Policies are captured as structured data, not scripts.
* **Platform-Agnostic**: One language to describe requirements across Linux, Windows, cloud, and containers.
* **Policy Engine Role**: ESP definitions serve as the source of truth that ScanSet’s trust fabric consumes.
* **Ecosystem Fit**: While traditional tools stop at compliance reporting, ESP definitions feed into **cryptographic trust attestations** that support continuous Zero Trust enforcement (handled outside this guide).

---

## 2. Language Overview

An ESP file is structured into **blocks**, each with a clear role. The language is designed for **declarative expression** rather than imperative scripting.

At a high level, an ESP file looks like this:

```esp
# Optional metadata about the definition
META
  version `1.0.0`
  author `compliance-team`
  platform `linux`
  compliance_framework `CIS`
META_END

# The compliance definition
DEF
  # Global declarations (variables, states, objects, sets, runtime ops)

  # One or more criteria blocks (CRI) containing compliance tests
  CRI AND
    CTN file_permission_check
      TEST all all
      OBJECT_REF etc_passwd
      STATE_REF correct_permissions
    CTN_END
  CRI_END
DEF_END
```

### Execution Flow

1. **Metadata** is read (optional, but recommended).
2. **Definitions (`DEF`)** declare reusable building blocks:

   * `VAR` (variables)
   * `OBJECT` (resources to examine)
   * `STATE` (expected conditions)
   * `SET` (collections of objects)
   * `RUN` (runtime transformations)
3. **Criteria (`CRI`)** describe logical groupings of compliance checks.
4. **Criteria tests (`CTN`)** pair **objects** (what to check) with **states** (what conditions must hold).
5. The scanner executes tests and reports compliance status.

---

## 3. Core Concepts

ESP builds on a small set of **core concepts** that form the foundation for every compliance definition:

### Objects vs States

* **Objects** identify *what resource* is being examined (file, registry key, service, package, etc.).
* **States** define *what condition* must be true about that resource (permissions, value, regex pattern, etc.).

### Criteria

* **CTN (Criterion)**: Combines an object and one or more states into a single test.
* **CRI (Criteria Block)**: Combines CTNs (or other CRIs) with logical operators (`AND`, `OR`, `NOT`).
* Criteria define the overall compliance logic.

### Variables

* **VAR** allows reuse of values across multiple states and objects.
* Variables keep definitions consistent, easier to maintain, and adaptable across environments.

### Sets

* **SET** combines multiple objects into collections using operations (`union`, `intersection`, `complement`).
* Useful when validating across groups of resources.

### Types (Brief Introduction)

ESP supports a simple but flexible type system:

* **Primitives**: `string`, `int`, `float`, `boolean`, `binary`.
* **Specialized**: `version`, `evr_string`, `record`.
* Types determine what operations are allowed (e.g., `>=` for numbers and versions, `contains` for strings, `pattern_match` for regexes).

This guide will revisit the type system in detail later, but for now remember: **every field in a STATE or OBJECT has a type, and types control which operators can be used.**

### Scoping

In ESP, **scope** determines where a definition can be referenced and how it is resolved. Although most blocks are global by design, some scoping rules are important to understand.

#### Global Scope

* **VAR, OBJECT, STATE, and SET** declarations are global by default.
* Once declared, they can be referenced anywhere within the same `DEF` block.
* This ensures consistency: a variable or object ID always refers to the same definition within a file.

#### Local Scope in CTN and CRI

* A `CTN` may contain inline `OBJECT` or `STATE` definitions. These are **local** to the CTN and cannot be referenced outside of it.
* Similarly, inline elements within a `CRI` are scoped only to that logical grouping.

Example:

```esp
CRI AND
  CTN file_check
    TEST all all
    OBJECT
      path `/tmp`
      filename `test.txt`
    OBJECT_END
    STATE
      permissions string = `0644`
    STATE_END
  CTN_END
CRI_END
```

Here, the object and state exist only within this `CTN`. They are not available globally.

#### Variable Scope

* Variables (`VAR`) are **always global** unless explicitly assigned inside a `RUN` block.
* `RUN` can override or extend variables during execution, but the name must already exist.
* Forward references are not allowed: you cannot initialize a variable with another that hasn’t been declared yet.

#### Best Practices for Scoping

* Use **global scope** for reusable definitions shared across multiple checks.
* Use **local scope** when an object or state is only needed once, and defining it globally would add clutter.
* Keep variable scoping simple to avoid confusion: declare at the top, reference everywhere else.

---

This scoping model will be reinforced in the **Deeper Core Concepts** section and illustrated in the block-by-block walkthroughs.


# Part II – The Building Blocks

## 4. Deeper Look at Core Concepts

Before diving into the individual blocks of ESP, it’s useful to revisit the **core concepts** with more detail and context. This section expands on the foundations introduced earlier and prepares you for the syntax-heavy chapters that follow.

#### Objects and States in Practice

* **Objects**: Think of these as *what you want to examine*. They represent system resources such as files, registry keys, services, packages, or API endpoints.
* **States**: Define the *expected properties* of those resources. A file might need specific permissions, a registry key might need a certain value, or a service might need to be running.

Together, objects and states let you say: *“This resource must look like this.”*

#### Criteria and Logical Composition

* **CTN (Criterion)** is a single check that binds an object to a state.
* **CRI (Criteria Block)** groups multiple CTNs and CRIs together using logical operators like `AND`, `OR`, and `NOT`.
* This structure mirrors how STIGs, CIS Benchmarks, and NIST controls are written: each control may have multiple checks that together define compliance.

Example:

```esp
CRI AND
  CTN file_permission
    TEST all all
    OBJECT_REF etc_passwd
    STATE_REF correct_permissions
  CTN_END
  CTN file_owner
    TEST all all
    OBJECT_REF etc_passwd
    STATE_REF correct_owner
  CTN_END
CRI_END
```

Here, compliance requires *both* the correct permissions *and* the correct owner.

#### Variables and Reuse

Variables (`VAR`) allow definitions to stay DRY (Don’t Repeat Yourself). Instead of hardcoding repeated paths, thresholds, or values, you declare them once and reference them everywhere.

```esp
VAR base_dir string `/etc`

OBJECT config_file
    path VAR base_dir
    filename `security.conf`
OBJECT_END
```

This makes definitions easier to maintain across environments.

#### Sets and Collections

Sets (`SET`) allow you to group objects and apply operations on them. Common operations include:

* **union**: combine objects into a larger group.
* **intersection**: find common members between groups.
* **complement**: subtract one set from another.

This makes it possible to define compliance at scale, e.g., “all critical system files must have restricted permissions.”

#### Scoping Refresher

* **Global Scope**: `VAR`, `OBJECT`, `STATE`, and `SET` are global unless defined inline.
* **Local Scope**: Inline `OBJECT` or `STATE` definitions inside a `CTN` or `CRI` exist only within that block.
* **Variable Scope**: Variables are global unless modified via `RUN`, and must be declared before they’re referenced.

This scoping model helps organize rules clearly and avoid conflicts across definitions.

#### Types and Operations

Every field in ESP has a type. This determines what operations you can perform:

* **string**: can use `=`, `!=`, `contains`, `pattern_match`, etc.
* **int/float**: can use comparison operators like `<`, `>=`.
* **boolean**: can test equality/inequality.
* **version/evr_string**: special comparisons for semantic versions.
* **record/binary**: structured or raw data for more advanced checks.

Operations are always type-safe: you can’t use `contains` on an integer, for example.

#### Execution Lifecycle Refresher

1. **Parse** the ESP file, validate syntax.
2. **Resolve references** (variables, states, objects).
3. **Evaluate** criteria (CTNs and CRIs).
4. **Report/attest** results back to the trust infrastructure.

This consistent flow ensures ESP definitions remain predictable and portable across environments.

---

## 5. Types and Literals

The ESP type system defines how values are written and what operations are valid. Every field in a `STATE`, `OBJECT`, or `VAR` block has a type, which determines both syntax and behavior.

#### Primitive Types

* **string**: Text values enclosed in backticks. Supports comparisons and string operations.
* **int**: 64-bit signed integer. Example: `42`, `-1`.
* **float**: 64-bit floating point. Example: `3.14`.
* **boolean**: Either `true` or `false`.
* **binary**: Raw byte values for low-level content checks.

#### Specialized Types

* **version**: Semantic version strings (e.g., `1.2.3`). Compared using version-aware rules.
* **evr_string**: Epoch-Version-Release style strings (common in RPM/Debian packaging).
* **record**: Structured type grouping fields together. Used in advanced scenarios with `STATE` and `OBJECT`. An overview is provided here, but detailed usage will be covered in later chapters.

#### Strings in ESP

ESP uses **backtick-delimited strings**:

```esp
`simple string`
``  ``          # empty string
`This has a ``backtick`` inside`
```

Multiline strings are supported using triple backticks:

````esp
```
first line
second line
```
````

#### Numbers and Booleans

* Integers must fit within 64-bit signed range.
* Floats use IEEE 754 double precision.
* Booleans are lowercase: `true`, `false`.

#### Operators by Type

* **Comparison**: `=`, `!=`, `<`, `>`, `<=`, `>=` (for strings, numbers, versions).
* **String-specific**: `contains`, `starts`, `ends`, `pattern_match`, `not_contains`, etc.
* **Set operators**: `subset_of`, `superset_of` (for collections).

This chapter provides the foundation. A complete compatibility matrix is included later in the Reference section.

>
> #### Operators Sidebar
> ESP uses operators to evaluate conditions. They fall into two categories:  
>   
> **1. Comparison Operators (used in `STATE`)**  
> - `=` , `!=` – equals / not equals  
> - `<`, `>`, `<=`, `>=` – numeric, string (lexicographic), or version comparisons  
>   
> **2. String Operators (used in `STATE`)**  
> - `ieq`, `ine` – equals / not equals (case-insensitive)  
> - `contains`, `not_contains` – substring membership  
> - `starts`, `not_starts` – starts with / does not start with substring  
> - `ends`, `not_ends` – ends with / does not end with substring  
>   
> **3. Pattern Operators (used in `STATE`)**  
> - `pattern_match` – regex-style match  
> - `matches` – regex-style match (alias)  
>   
> **4. Set Operators (used in `STATE` with collections)**  
> - `subset_of` – collection is a subset of another  
> - `superset_of` – collection is a superset of another  
>   
> **5. Logical Operators (used in `CRI`)**  
> - `AND` – all conditions must be true  
> - `OR` – at least one condition must be true  
> - `NOT` – negate a condition or group  
>   
> You’ll learn **comparison, string, pattern, and set operators** in the `STATE` chapter,  
> and **logical operators** in the `CRI` chapter.  
> A full compatibility matrix appears in the Reference section.

## 6. Metadata (`META`)

The **`META` block** defines descriptive and contextual information about an ESP definition. While optional in the grammar, in practice it is highly recommended — and for the ESP Scanner SDK, certain fields are **required**.

#### Purpose

* Provide context for each compliance definition (author, version, description).
* Tie ESP definitions back to external frameworks (STIG, NIST, CIS, etc.).
* Supply identifiers for trust and traceability (scan ID, control references, tags).

#### Structure

```esp
META
  <field_name> <value>
  ...
META_END
```

* Each line is a key-value pair.
* Values are typically enclosed in backticks (`` ` ``) to allow spaces and special characters.

#### Required Fields

For the ESP Scanner SDK, the following metadata fields must be present:

* `esp_scan_id` – Unique identifier for the scan or test case.
* `control_framework` – Framework being applied (e.g., NIST, STIG, CIS).
* `control` – Specific control identifier (e.g., `AC-2.4`).
* `platform` – Target platform (`windows`, `linux`, `macos`, etc.).
* `criticality` – Importance of the control (`high`, `medium`, `low`).
* `tags` – Comma-separated keywords for grouping or filtering checks.

If these fields are missing, the scanner will fail validation.

#### Example

```esp
META
version `1.0.0`
esp_version `1.0`
author `security-team`
date `2024-01-15`
severity `high`
platform `windows`
description `Monitors Windows processes for security compliance`
control_framework `NIST`
control `AC-2.4`
esp_scan_id `Powershell-Test`
criticality `high`
tags `powershell,process,monitoring`
META_END
```

#### Best Practices

* Always include at least the **required fields** to ensure compatibility with the ESP Scanner SDK.
* Use `esp_scan_id` to track rules across versions and environments.
* Align `control_framework` and `control` to the compliance baseline you are mapping against.
* Apply meaningful `tags` to make large rule sets easier to query and filter.

---

### Next Step

The next building block is **Variables (`VAR`)**, which let you define reusable values across your ESP definitions.

## 7. Variables (`VAR`)

The **`VAR` block** declares reusable values that can be referenced throughout an ESP definition. Variables improve consistency, reduce duplication, and simplify maintenance.

#### Purpose

* Define constants such as file paths, thresholds, usernames, or configuration values.
* Prevent repetition of hardcoded values.
* Allow portability across environments by changing a value once and reusing it everywhere.

#### Structure

```esp
VAR <name> <type> <initial_value?>
```

* **name**: Identifier following `[a-zA-Z_][a-zA-Z0-9_]*`.
* **type**: One of the supported types (`string`, `int`, `float`, `boolean`, `version`, etc.).
* **initial_value**: Optional; if omitted, the variable can be assigned later with a `RUN` block.

#### Examples

```esp
VAR base_path string `/etc`
VAR threshold int 1024
VAR enabled boolean true
```

These variables can be referenced in objects and states:

```esp
OBJECT config_file
    path VAR base_path
    filename `security.conf`
OBJECT_END

STATE file_size_check
    size int > VAR threshold
STATE_END
```

#### Rules and Constraints

* All variables are **global scope**.
* Duplicate variable names are not allowed.
* No forward references during initialization (a variable cannot use one declared later).
* Circular references are not permitted.

Invalid example (circular dependency):

```esp
VAR var_one string VAR var_two
VAR var_two string VAR var_one  # ERROR
```

#### Best Practices

* Use descriptive names like `base_path`, `min_size` rather than short or ambiguous identifiers.
* Place variables near the top of the definition for clarity.
* Store frequently reused values in variables to ease updates.

---

## 8. Objects (`OBJECT`)

The **`OBJECT` block** defines the system resources that an ESP rule will inspect. Objects describe *what to look at* on a target system — for example, files, registry keys, services, packages, or API endpoints. Objects do not define conditions; they only identify resources. Conditions are expressed separately in `STATE` blocks.

#### Purpose

* Identify target resources in a platform-agnostic way.
* Provide context for `STATE` evaluations.
* Serve as reusable building blocks across multiple criteria tests (`CTN`).

#### Structure

```esp
OBJECT <name>
    <field_name> <type> <value>
    [BEHAVIOR <collector_behavior>]
    [FILTER include]
        STATE_REF <state_name>
        ...
    FILTER_END
OBJECT_END
```

* **name**: A unique identifier for the object.
* **fields**: Key-value pairs that describe the resource. Each field has a type.
* **BEHAVIOR**: Alters how the data collector executes for this object.
* **FILTER**: Narrows the object’s scope by requiring it to satisfy specific states.
* **values**: Typically literals or references to variables.

#### Examples

File object:

```esp
OBJECT etc_passwd
    path string `/etc`
    filename `passwd`
OBJECT_END
```

Windows registry object:

```esp
OBJECT reg_key_example
    hive string `HKEY_LOCAL_MACHINE`
    key string `SYSTEM\CurrentControlSet\Services`
    name string `Start`
OBJECT_END
```

Package object with behavior:

```esp
OBJECT ssh_package
    name string `openssh`
    BEHAVIOR find
OBJECT_END
```

Here, the behavior switches from the default collector action (e.g., using `cat` to query installed package info) to a different one (e.g., `find` to locate the package in the filesystem).

Object with filter:

```esp
OBJECT log_file
    path string `/var/log`
    filename `*.log`
    FILTER include
      STATE_REF non_empty
    FILTER_END
OBJECT_END
```

This ensures only log files that satisfy the `non_empty` state are considered.

#### Local vs Global Objects

* **Global Objects**: Declared at the top level inside a `DEF`. Available for reference across the entire definition.
* **Local Objects**: Declared inline within a `CTN`. Only usable within that CTN.

#### Specialized Fields

ESP supports additional specialized fields that allow more control over how collectors gather and interpret data:

* **module**: Specifies a pluggable subsystem or kernel module. Useful for systems with modular components.

  ```esp
  OBJECT kernel_module
      module string `firewalld`
  OBJECT_END
  ```

  This object inspects the `firewalld` kernel module.

* **select**: Chooses a sub-component or attribute from a structured resource.

  ```esp
  OBJECT ssh_config
      path string `/etc/ssh/sshd_config`
      select string `Protocol`
  OBJECT_END
  ```

  This object inspects only the `Protocol` directive inside the SSH config file.

* **parameters**: Supplies additional arguments to the collector, allowing customization of its behavior.

  ```esp
  OBJECT process_list
      module string `ps`
      parameters string `-ef`
  OBJECT_END
  ```

  Here, the collector for processes is given flags to control the output format.

#### Behavior Field

The **`BEHAVIOR` line** changes the execution context of data collectors for the object. It acts as a **mode selector** for the underlying collector logic.

* **Default behavior**: If no `BEHAVIOR` is specified, the collector uses its default method (e.g., file → `cat`, registry → `query`).
* **Alternate behavior**: When specified, the collector uses an alternate strategy (e.g., `find` to search directories instead of reading a file directly).

Example:

```esp
OBJECT config_search
    path string `/etc`
    filename `*.conf`
    BEHAVIOR find
OBJECT_END
```

This instructs the collector to search for all `.conf` files under `/etc` using `find` rather than opening a specific file.

#### Rules and Constraints

* Object names must be unique within a definition.
* Duplicate field names are **not allowed** in an object.
* Fields must match what the scanner’s collector expects (e.g., `path` and `filename` for files, `hive` and `key` for registry).
* Only **global states** may be referenced in `FILTER` blocks.
* If no `BEHAVIOR` is specified, the collector defaults to its standard method (e.g., file → `cat`, package → `query`).

#### Best Practices

* Use descriptive names (`etc_passwd`, `ssh_package`) for clarity.
* Keep objects global if they are reused by multiple checks.
* Use local objects when they are one-off, to reduce clutter.
* Explicitly declare `BEHAVIOR` when the context of collection should differ from the default.
* Apply `FILTER` sparingly to objects to refine large datasets (logs, registries).
* Use specialized fields (`module`, `select`, `parameters`) to adapt objects to real-world system complexity.

---

## 9. States (`STATE`)

The **`STATE` block** defines the expected properties or conditions of an object. While objects identify *what resource* to examine, states describe *how that resource should look* in order to be considered compliant.

#### Purpose

* Express compliance conditions such as permissions, values, or patterns.
* Separate **resource identification** (OBJECT) from **expected condition** (STATE).
* Enable reuse of conditions across multiple objects or checks.

#### Structure

A `STATE` can contain **simple fields** and/or **record blocks**:

```esp
STATE <name>
  # Simple field conditions
  <field_name> <type> <operator> <value>

  # Structured validation using a record block
  record <root_field> <record_type?>
    <nested_field_or_record_conditions>
  record_end
STATE_END
```

* **name**: Unique identifier for the state.
* **Simple fields**: One or more leaf conditions (`field type operator value`).
* **record block**: Special construct allowed only inside `STATE`; validates structured data under `<root_field>`.
* **record_type (optional)**: Specifies the parser/collector for the record (`json`, `yaml`, `xml`, `sql`, `powershell`, `csv`, etc.). Supported types depend on the scanner’s trait implementation.

#### Examples (Simple Fields)

```esp
STATE correct_permissions
  permissions string = `0644`
STATE_END

STATE correct_owner
  owner string ieq `root`
STATE_END

STATE reg_startup_mode
  value int = 2
STATE_END

STATE ssh_version_ok
  version >= `8.0`
STATE_END
```

#### Record Blocks

Record blocks allow validation of structured data (e.g., JSON configs, YAML policies, registry hives, SQL query results, or PowerShell objects). They can be **nested** and combined with simple fields in the same state.

```esp
STATE app_config_ok
  version >= `2.0`

  record config json
    logging.level string ieq `INFO`
    features.audit boolean = true
  record_end
STATE_END
```

**Multiple record blocks**:

```esp
STATE system_policies
  record network json
    port int = 22
  record_end

  record logging json
    level string = `DEBUG`
  record_end
STATE_END
```

**Nested record blocks**:

```esp
STATE deep_check
  record security json
    record policies
      record ssh
        allow_root boolean = false
      record_end
    record_end
  record_end
STATE_END
```

> **Rule of thumb**: Operators apply **only to leaf values**. Parent `record` nodes act as structural containers, not comparison points.

#### Dot Notation

For readability, you can use **dot notation** instead of deeply nested `record` blocks. Dot notation is equivalent to expanding nested blocks.

```esp
STATE dot_notation_example
  record config json
    logging.level string ieq `INFO`
    security.ssh.allow_root boolean = false
  record_end
STATE_END
```

> **Arrays / indices**: Structured data may include array paths (e.g., `users[0].name`) or wildcards (`users[*].role`). Exact syntax depends on the scanner’s record trait implementation.

#### Operators in STATE

States use operators to compare collected values with expected values.

**Comparison**: `=`, `!=`, `<`, `>`, `<=`, `>=`
**String**: `ieq`, `ine`, `contains`, `not_contains`, `starts`, `not_starts`, `ends`, `not_ends`
**Pattern**: `pattern_match`, `matches`
**Set**: `subset_of`, `superset_of`

#### Multiple Constraints

A state can contain **multiple fields with the same name**, which are interpreted as multiple constraints on that property.

```esp
STATE secure_permissions
  permissions string != `0777`
  permissions string != `0666`
STATE_END
```

#### Record Type vs. Record Block

* **`record` type**: A data type usable anywhere a type is valid (`STATE`, `OBJECT`, `VAR`, `parameters`).
* **`record` block**: The `record … record_end` construct, valid only inside `STATE`, for nested structured validation.

#### Local vs Global States

* **Global States**: Declared at the top level inside a `DEF`. Reusable across multiple CTNs.
* **Local States**: Declared inline within a CTN. Only usable within that CTN.

#### Rules and Constraints

* State names must be unique within a definition.
* Duplicate field names are allowed (to express multiple constraints).
* Operators must be valid for the field’s type.
* Record blocks may be nested; operators must appear only on leaf fields.

#### Best Practices

* Use record blocks to validate structured content (configs, API payloads, registry hives).
* Prefer dot notation when possible to reduce nesting depth.
* Always specify the record type (`json`, `sql`, `powershell`, etc.) when the collector requires it.
* Mix simple fields and record checks in the same state to cover both high-level and detailed conditions.

---


## 10. Criterion Tests (`CTN`)

The **`CTN` block** (Criterion) binds an **object** to a **state** and defines how the evaluation should be performed. It is the basic executable unit of compliance in ESP.

#### Purpose

* Link **what to check** (OBJECT) with **how it should look** (STATE).
* Define the scope of evaluation using entity selectors.
* Provide reusable compliance tests that can be combined into larger criteria blocks (`CRI`).

#### Structure

```esp
CTN <name?>
  TEST <existence_check> <item_check> [<state_operator>]
  OBJECT_REF <object_name>
  STATE_REF <state_name>
CTN_END
```

* **name**: Optional identifier for the CTN.
* **TEST**: Defines how objects and states must satisfy the check.
* **OBJECT_REF**: Reference to a defined object (or inline object).
* **STATE_REF**: Reference to a defined state (or inline state).

#### The TEST Line

The `TEST` line has three dimensions, defined by the EBNF:

* **Existence Check (objects)**: `any`, `all`, `none`, `at_least_one`, `only_one`
* **Item Check (states)**: `all`, `at_least_one`, `only_one`, `none_satisfy`
* **State Operator (optional)**: `AND`, `OR`, `ONE`

The **state operator** controls how multiple states are evaluated together for each object.

* `AND` → all states must be true (default, mirrors SCAP/OVAL `check=\"all\"`).
* `OR` → at least one state must be true (mirrors SCAP/OVAL `check=\"at least one\"`).
* `ONE` → exactly one state must be true (mirrors SCAP/OVAL `check=\"only one\"`).

If the **state operator is omitted**, the parser automatically assumes `AND`. This aligns with OVAL’s default semantics and ensures backward compatibility. While optional, it is often clearer to write `AND` explicitly.

**Note**: If you leave out the state operator in ESP, it defaults to `AND`. Best practice is to write it explicitly to avoid ambiguity.

Examples:

* `TEST all all` → All objects must satisfy all states.
* `TEST any all` → At least one object must satisfy all states.
* `TEST all at_least_one OR` → All objects must satisfy at least one of the states, combined with OR logic.
* `TEST only_one none_satisfy ONE` → Exactly one object must exist, and none of its states may be satisfied, evaluated under the ONE operator.

#### Examples

File compliance check:

```esp
CTN passwd_file_check
  TEST all all
  OBJECT_REF etc_passwd
  STATE_REF correct_permissions
  STATE_REF correct_owner
CTN_END
```

This test requires that the `/etc/passwd` file must have both the correct permissions and owner.

Service compliance check:

```esp
CTN ssh_service_running
  TEST any all
  OBJECT_REF ssh_service
  STATE_REF running_state
CTN_END
```

This test passes if at least one referenced service object is in the running state.

State operator with OR:

```esp
CTN ssh_config_check
  TEST all at_least_one OR
  OBJECT_REF ssh_config
  STATE_REF protocol_2
  STATE_REF strong_ciphers
CTN_END
```

This requires that all ssh_config objects satisfy at least one of the two states, evaluated with OR logic.

#### Local Inline Definitions

Objects and states can be defined inline within a CTN instead of referencing global definitions:

```esp
CTN temp_file_check
  TEST all all
  OBJECT
    path `/tmp`
    filename `debug.log`
  OBJECT_END
  STATE
    permissions string != `0777`
  STATE_END
CTN_END
```

This check ensures that `/tmp/debug.log` does not have world-writable permissions.

#### Rules and Constraints

* Each CTN must contain exactly one `TEST` line.
* At least one object and one state must be referenced or defined.
* Inline and referenced definitions cannot conflict in naming.

#### Best Practices

* Use descriptive CTN names for clarity (e.g., `passwd_file_check`).
* Favor global objects and states when reused, but use inline definitions for one-off checks.
* Be explicit with `TEST` semantics to avoid ambiguity.

---

### Next Step

The next building block is **Criteria Blocks (`CRI`)**, which group CTNs and apply logical operators like `AND`, `OR`, and `NOT`.

## 11. Criteria Blocks (`CRI`)

The **`CRI` block** defines how multiple criteria tests (`CTN`) or other criteria blocks (`CRI`) are combined using logical operators. If a `CTN` is the atomic unit of compliance, a `CRI` is the higher-level structure that builds full compliance logic.

#### Purpose

* Combine multiple `CTN` results into a larger logical expression.
* Allow **nesting** of criteria for complex conditions.
* Provide explicit logical control using `AND`, `OR`, and optional negation.

#### Structure

```esp
CRI <operator>
  <criterion_or_nested_criteria>
  ...
CRI_END
```

* **operator**: `AND` or `OR`, determines how child criteria are combined.
* **criterion_or_nested_criteria**: One or more `CTN` or nested `CRI` blocks.
* **Negation**: A `NOT` keyword can be applied at the CTN or CRI level to invert the result.

#### Logical Operators

* **AND** → All child criteria must evaluate true for the CRI to pass.
* **OR** → At least one child criterion must evaluate true for the CRI to pass.
* **NOT** → Negates the result of a child CTN or CRI.

#### Nesting Criteria

`CRI` blocks may contain other `CRI` blocks, creating a tree of logical evaluations.

Example (nested criteria):

```esp
CRI AND
  CTN passwd_file_check
    TEST all all
    OBJECT_REF etc_passwd
    STATE_REF correct_permissions
    STATE_REF correct_owner
  CTN_END

  CRI OR
    CTN ssh_running
      TEST any all
      OBJECT_REF ssh_service
      STATE_REF running_state
    CTN_END

    CTN ssh_disabled
      TEST any all
      OBJECT_REF ssh_service
      STATE_REF disabled_state
    CTN_END
  CRI_END
CRI_END
```

This evaluates: *passwd_file_check must pass* **AND** (*either ssh_running OR ssh_disabled must pass*).

#### Execution Order

* Evaluation is performed **top-down**.
* **Sibling CTNs** inside a CRI are evaluated left-to-right.
* **Nested CRIs** are evaluated fully before their result is combined at the parent level.
* Short-circuiting may occur:

  * In an `AND`, if any child fails, the CRI fails immediately.
  * In an `OR`, if any child passes, the CRI passes immediately.

#### Examples

Simple AND block:

```esp
CRI AND
  CTN passwd_owner
    TEST all all
    OBJECT_REF etc_passwd
    STATE_REF correct_owner
  CTN_END
  CTN passwd_perms
    TEST all all
    OBJECT_REF etc_passwd
    STATE_REF correct_permissions
  CTN_END
CRI_END
```

Using NOT:

```esp
CRI AND
  CTN guest_user_absent
    TEST none all
    OBJECT_REF guest_user
    STATE_REF exists_state
  CTN_END
  NOT CTN weak_password
    TEST any all
    OBJECT_REF passwd_file
    STATE_REF weak_password_state
  CTN_END
CRI_END
```

#### Rules and Constraints

* Every CRI must declare its operator (`AND` or `OR`).
* A CRI must contain at least one CTN or nested CRI.
* Negation (`NOT`) may be applied to any CTN or CRI child.
* Nesting is unlimited but should be used judiciously for readability.

#### Best Practices

* Use `AND` to enforce multiple conditions that all must be true.
* Use `OR` when alternative conditions are acceptable.
* Use `NOT` sparingly and clearly, as it can complicate readability.
* Favor shallow nesting where possible; deeply nested CRIs can be harder to maintain.

---

### Next Step

With CRIs covered, we can now move into **Practical Examples** showing how META, VAR, OBJECT, STATE, CTN, and CRI combine into real-world compliance checks.

## 12. Sets (`SET`)

The **`SET` block** builds *collections of objects* and lets you combine or refine them. Sets are useful when a rule should apply to many resources (e.g., “all critical system files”).

#### Purpose

* Group objects into named collections.
* Compose collections with set algebra (`union`, `intersection`, `complement`).
* Refine collections with **filters** that keep only items satisfying one or more states.
* Reuse the resulting collection elsewhere (via `SET_REF`).

#### Structure

```esp
SET <name> <operator>
  <operands and optional filters>
SET_END
```

* **name**: Unique identifier for the set.
* **operator**: `union` | `intersection` | `complement`.
* **operands**: `OBJECT_REF`, `SET_REF`, or inline `OBJECT` definitions.
* **filters**: Optional `FILTER` block(s) that keep only members satisfying referenced states.

#### Operators & Arity

* **union**: Combines 1 or more operands into a single set.
* **intersection**: Keeps only members common to **2 or more** operands.
* **complement**: Computes **A \ B**. Requires **exactly 2** operands.

#### Referencing Sets

* Use `SET_REF <set_name>` anywhere a set operand or set input is allowed (e.g., inside a `SET`, or an inline `OBJECT` that sources from a set).
* Sets are **global**; reference them from `CTN` by wrapping the set in an inline `OBJECT` or by referencing objects that were members of the set.

#### FILTER Blocks

Filters **narrow** a set to members that satisfy one or more **global states**.

```esp
FILTER include
  STATE_REF <state_name>
  ...
FILTER_END
```

* **Action**: `include` keeps only members that satisfy the listed states.
* **State references must be global** (definition-level) states. Inline CTN states are not allowed.
* A `FILTER` must reference **at least one** state.
* Multiple `FILTER` blocks may be used; they are applied in the order written.

> **Note**: Filters evaluate the referenced **states** against each member of the set using that member as the target **object**.

#### Examples

**1) Union with filter**

```esp
# Combine objects and keep only those passing a permission check
SET critical_objects union
  OBJECT_REF system_registry
  OBJECT_REF config_file
  FILTER include
    STATE_REF permission_check   # must be a global state
  FILTER_END
SET_END
```

**2) Intersection**

```esp
# Items that appear in both sets
SET common_objects intersection
  SET_REF critical_objects
  OBJECT inline_object
    type `security`
  OBJECT_END
SET_END
```

**3) Complement**

```esp
# Items in A that are not in B
SET difference_objects complement
  SET_REF critical_objects
  SET_REF common_objects
SET_END
```

**4) Using a set inside a CTN**

```esp
CTN set_validation
  TEST at_least_one all
  STATE_REF permission_check
  OBJECT set_source
    SET_REF critical_objects  # Supply the set as the object source
  OBJECT_END
CTN_END
```

#### Rules and Constraints

* Set names must be unique within the definition.
* Operands must be valid and resolvable at definition time.
* `FILTER` blocks may **only** reference **global** states (`STATE_REF`).
* `complement` requires exactly **two** operands; `intersection` requires **two or more**; `union` accepts **one or more**.
* Inline `OBJECT` definitions inside a `SET` are **local** to that `SET` and cannot be referenced elsewhere.

#### Best Practices

* Prefer `SET` when the same group of resources is used by multiple CTNs.
* Keep filters small and explicit; push complex logic into named global states.
* Use descriptive names (e.g., `critical_objects`, `monitored_services`).
* Validate that object field names match what collectors expect (e.g., files vs registry entries).

---

### Next Step

Proceed to **Runtime Operations (`RUN`)** to learn how to derive or transform variables and object-derived values used by your sets and states.

### Runtime Operations (`RUN`)

The **`RUN` block** performs **in-file data transformations** and declares or updates a **variable**. Unlike `VAR`, which only declares static values, `RUN` can both **introduce a new variable** and compute its value dynamically. This means you do **not** need a prior `VAR` declaration for a variable assigned in a `RUN` block.

#### Structure

```esp
RUN <target_variable> <OPERATION>
  <parameter line>
  <parameter line>
  ...
RUN_END
```

* **target_variable**: Name of the variable being created or updated. If it does not already exist, the `RUN` block implicitly declares it.
* **OPERATION**: One of `CONCAT | SPLIT | SUBSTRING | REGEX_CAPTURE | ARITHMETIC | COUNT | UNIQUE | MERGE | EXTRACT | END`.
* **Parameter lines** (flat, no nesting):

  * `literal <string|int>`
  * `VAR <variable_name>`
  * `OBJ <object_id> <field>` (extract a field from a **global** object)
  * `pattern <`regex`>`
  * `delimiter <`,`>` / `character <`:`>`
  * `start <int>` / `length <int>`
  * Arithmetic tokens: `+ | - | * | / | %` followed by a number

#### Operation Semantics

* **CONCAT** → Concatenate all components (strings only). Result is a string.
* **SPLIT** → Split a string by `delimiter` (or single `character`). Result is a **collection of strings** (consumed by `COUNT`/`UNIQUE`/`MERGE`).
* **SUBSTRING** → Extract substring from a string with `start` and optional `length`.
* **REGEX_CAPTURE** → Apply `pattern` to a string input; captures the first match. Result is a string.
* **ARITHMETIC** → Start from a numeric input (variable or literal) and apply arithmetic lines (e.g., `+ 100`, `* 2`). Result is numeric.
* **COUNT** → Count items in a collection. Result is an `int`.
* **UNIQUE** → Deduplicate a collection, preserving element type.
* **MERGE** → Merge collections of the **same element type**.
* **EXTRACT** → `OBJ <object_id> <field>` pulls a single field value from a **global** object.
* **END** → Identity/finalize; reserved for chaining flows.

#### Examples

**Implicit variable declaration with CONCAT:**

```esp
RUN config_path CONCAT
  VAR base_path
  literal `/config`
RUN_END
```

Here `config_path` is declared by the `RUN` block; no `VAR` line is required.

**Arithmetic computation:**

```esp
RUN threshold ARITHMETIC
  literal 1024
  + 100
  * 2
RUN_END
```

**Extract a field from an object (EXTRACT):**

```esp
# OBJECT system_registry ...
RUN hive_name EXTRACT
  OBJ system_registry hive
RUN_END
```

**Split and count with chaining:**

```esp
RUN users_list SPLIT
  literal `alice,bob,bob,charlie`
  delimiter `,`
RUN_END

RUN unique_count COUNT
  VAR users_list
RUN_END
```

#### Rules and Constraints

* A `RUN` block **declares the variable** it assigns; `VAR` is optional and only needed if you want to set an initial literal value.
* A `RUN` **must have at least one parameter line**.
* `OBJ <id> <field>` may only reference **definition-level** (global) objects.
* Parameter tokens are **single lines**; there is **no nesting** inside `RUN`.
* Operation input/output types must follow **type compatibility** rules.
* **Literal type inference**: The type of a `literal` is inferred from the operation context. For example:

  * In an **ARITHMETIC** operation, literals are always treated as numeric (`int` or `float`).
  * In **MERGE**, literals are inferred as the element type of the collection (often `string`).
  * In **CONCAT**, literals are treated as `string`.

#### Best Practices

* Use `RUN` for derived or computed values, and `VAR` for static values.
* Keep `RUN` blocks focused: one operation per block; chain via variables when needed.
* Prefer `literal` for constant fragments to avoid ambiguity.
* Use `EXTRACT` when you need a field from a global object for another transformation.
* Standardize delimiters and patterns explicitly for predictable results.

---

## 13. Runtime Operations (`RUN`)

The **`RUN` block** performs **in-file data transformations** and declares or updates a **variable**. Unlike `VAR`, which only declares static values, `RUN` can both **introduce a new variable** and compute its value dynamically. This means you do **not** need a prior `VAR` declaration for a variable assigned in a `RUN` block.

#### Structure

```esp
RUN <target_variable> <OPERATION>
  <parameter line>
  <parameter line>
  ...
RUN_END
```

* **target_variable**: Name of the variable being created or updated. If it does not already exist, the `RUN` block implicitly declares it.
* **OPERATION**: One of `CONCAT | SPLIT | SUBSTRING | REGEX_CAPTURE | ARITHMETIC | COUNT | UNIQUE | MERGE | EXTRACT | END`.
* **Parameter lines** (flat, no nesting):

  * `literal <string|int>`
  * `VAR <variable_name>`
  * `OBJ <object_id> <field>` (extract a field from a **global** object)
  * `pattern <`regex`>`
  * `delimiter <`,`>` / `character <`:`>`
  * `start <int>` / `length <int>`
  * Arithmetic tokens: `+ | - | * | / | %` followed by a number

#### Operation Semantics

* **CONCAT** → Concatenate all components (strings only). Result is a string.
* **SPLIT** → Split a string by `delimiter` (or single `character`). Result is a **collection of strings** (consumed by `COUNT`/`UNIQUE`/`MERGE`).
* **SUBSTRING** → Extract substring from a string with `start` and optional `length`.
* **REGEX_CAPTURE** → Apply `pattern` to a string input; captures the first match. Result is a string.
* **ARITHMETIC** → Start from a numeric input (variable or literal) and apply arithmetic lines (e.g., `+ 100`, `* 2`). Result is numeric.
* **COUNT** → Count items in a collection. Result is an `int`.
* **UNIQUE** → Deduplicate a collection, preserving element type.
* **MERGE** → Merge collections of the **same element type**.
* **EXTRACT** → `OBJ <object_id> <field>` pulls a single field value from a **global** object.
* **END** → Identity/finalize; reserved for chaining flows.

#### Examples

**Implicit variable declaration with CONCAT:**

```esp
RUN config_path CONCAT
  VAR base_path
  literal `/config`
RUN_END
```

Here `config_path` is declared by the `RUN` block; no `VAR` line is required.

**Arithmetic computation:**

```esp
RUN threshold ARITHMETIC
  literal 1024
  + 100
  * 2
RUN_END
```

**Extract a field from an object (EXTRACT):**

```esp
# OBJECT system_registry ...
RUN hive_name EXTRACT
  OBJ system_registry hive
RUN_END
```

**Split and count with chaining:**

```esp
RUN users_list SPLIT
  literal `alice,bob,bob,charlie`
  delimiter `,`
RUN_END

RUN unique_count COUNT
  VAR users_list
RUN_END
```

#### Rules and Constraints

* A `RUN` block **declares the variable** it assigns; `VAR` is optional and only needed if you want to set an initial literal value.
* A `RUN` **must have at least one parameter line**.
* `OBJ <id> <field>` may only reference **definition-level** (global) objects.
* Parameter tokens are **single lines**; there is **no nesting** inside `RUN`.
* Operation input/output types must follow **type compatibility** rules.
* **Literal type inference**: The type of a `literal` is inferred from the operation context. For example:

  * In an **ARITHMETIC** operation, literals are always treated as numeric (`int` or `float`).
  * In **MERGE**, literals are inferred as the element type of the collection (often `string`).
  * In **CONCAT**, literals are treated as `string`.

#### Best Practices

* Use `RUN` for derived or computed values, and `VAR` for static values.
* Keep `RUN` blocks focused: one operation per block; chain via variables when needed.
* Prefer `literal` for constant fragments to avoid ambiguity.
* Use `EXTRACT` when you need a field from a global object for another transformation.
* Standardize delimiters and patterns explicitly for predictable results.

---

## 14. Filter Blocks (`FILTER`)

The **`FILTER` block** is used to reduce or refine the scope of data considered by an `OBJECT` or a `SET`. Filters act like a **where-clause** in a query: they only allow elements through if they satisfy specified `STATE` conditions.

#### Purpose

* Narrow down which items from a collection should be evaluated.
* Attach additional logical constraints to objects or sets.
* Improve performance and clarity by discarding irrelevant data early.

#### Structure

```esp
FILTER include
  STATE_REF <state_name>
  STATE_REF <another_state>
  ...
FILTER_END
```

* **include**: The filter mode (currently only `include` is supported).
* **STATE_REF**: Reference to a previously defined global state. Only items that satisfy these states are included.

#### Where Filters Can Be Used

* **Inside an OBJECT**: Limits collected resources to those that satisfy the referenced states.
* **Inside a SET**: Restricts the members of a set to those that pass the filter conditions.

#### Examples

**Filter inside an object:**

```esp
OBJECT log_file
  path string `/var/log`
  filename `*.log`
  FILTER include
    STATE_REF non_empty
  FILTER_END
OBJECT_END
```

Here, only log files that satisfy the `non_empty` state will be considered.

**Filter inside a set:**

```esp
SET monitored_services
  OBJECT_REF all_services
  FILTER include
    STATE_REF active_state
  FILTER_END
SET_END
```

This builds a set of services but keeps only those that pass the `active_state` check.

#### Execution Semantics

* Filters are **evaluated immediately** when the object or set is collected.
* Each `STATE_REF` inside the filter must evaluate to true for the element to be retained (logical `AND`).
* If multiple filters are stacked (rare), they apply sequentially.

#### Rules and Constraints

* Only **global states** can be referenced inside a `FILTER`. Local states are not valid here.
* Filters cannot directly contain inline conditions; they must use `STATE_REF`.
* An object or set can contain at most one `FILTER` block.

#### Best Practices

* Use filters to cut down noisy datasets (logs, service lists, registry entries).
* Keep filter states simple and reusable across objects/sets.
* Prefer filtering at the **object level** rather than later in criteria, to avoid unnecessary downstream checks.
* Always name filter states descriptively (`non_empty`, `active_state`).

---
## 15. Behavior (`BEHAVIOR`)

The **`BEHAVIOR` line** modifies how an object’s data collector operates. It allows compliance authors to change the **execution context** or **mode of operation** for a given object without redefining its fields.

#### Purpose

* Control the strategy used by the scanner to collect data for an object.
* Enable flexibility where the same object fields may be collected in different ways.
* Provide explicit tuning knobs to match platform- or environment-specific collection methods.

#### Syntax

A behavior element can be expressed **on one line** or across **multiple lines**:

**Single-line form:**

```esp
BEHAVIOR find max_depth 3 recursive true
```

**Multi-line form:**

```esp
BEHAVIOR find
BEHAVIOR max_depth 3
BEHAVIOR recursive true
```

Both forms are equivalent. All behavior values are collected in order and passed to the object’s collector.

#### Behavior Values

According to the grammar, behavior values can be:

* **Identifiers** (e.g., `find`, `scan`, `query`)
* **Integers** (e.g., `3`, `100`)
* **Booleans** (`true` / `false`)

This makes `BEHAVIOR` act like a flexible parameter list for the collector.

#### Examples

**Default behavior (implicit):**

```esp
OBJECT file_config
  path string `/etc`
  filename `hosts`
OBJECT_END
```

Here, the collector defaults to its standard file-reading behavior (e.g., `cat`).

**Explicit behavior with parameters (single-line):**

```esp
OBJECT config_search
  path string `/etc`
  filename `*.conf`
  BEHAVIOR find max_depth 3 recursive true
OBJECT_END
```

This instructs the collector to search recursively for `.conf` files under `/etc` with a maximum depth of 3.

**Equivalent multi-line form:**

```esp
OBJECT config_search
  path string `/etc`
  filename `*.conf`
  BEHAVIOR find
  BEHAVIOR max_depth 3
  BEHAVIOR recursive true
OBJECT_END
```

**Alternate package query:**

```esp
OBJECT ssh_package
  name string `openssh`
  BEHAVIOR query fast_mode
OBJECT_END
```

This changes the behavior so the scanner queries the package manager in `fast_mode` instead of looking for a binary.

#### Execution Semantics

This section explains **how ESP definitions are parsed and executed**, expanding on the grammar to provide authors with a precise mental model of runtime behavior.

#### Parse-Time vs. Runtime

* **Parser (compile-time)**: Validates structure, discovers symbols, checks types, and builds reference tables. Multi-pass parsing ensures forward references are valid (except in `VAR` initialization).
* **Engine (runtime)**: Collects system data, applies behaviors, runs filters, resolves references, executes RUN blocks, and evaluates criteria.

#### Block Ordering

* **File root**: An ESP file starts with an optional `META`, followed by a single `DEF` block.
* **Inside DEF**: Elements (`VAR`, `STATE`, `OBJECT`, `RUN`, `SET`, `CRI`) may appear in any order. Grouping improves readability but is not required.
* **Inside CTN**: Elements must follow strict order: `TEST` → `STATE_REF`(s) → `OBJECT_REF`(s) → local `STATE`(s) → local `OBJECT`(s).

#### Variable & RUN Semantics

* **Global scope**: All `VAR` and `RUN` variables are global.
* **Initialization rule**: `VAR` initialization cannot reference another variable declared later.
* **RUN ordering**: `RUN` blocks execute in file order. Each `RUN` can **declare** its target variable if not already present. Later `RUN`s can depend on variables from earlier `RUN`s.
* **Literal type inference in RUN**: A `literal` token’s type is inferred from the operation:

  * `ARITHMETIC` → numeric (`int`/`float`).
  * `CONCAT`/`MERGE` → string.
  * `COUNT` → collection element type.
* **OBJ extraction in RUN**: `OBJ <object_id> <field>` extracts a field value from a **global object**. Resolution happens at runtime against the object registry. If the object or field is missing, the engine decides whether to fail or return empty (implementation-defined).

#### Filters & Behaviors

* **FILTER blocks**: Apply during data collection. Each `STATE_REF` inside must evaluate to true (logical AND) for the element to be retained. Filters only accept global states.
* **BEHAVIOR lines**: Alter collector execution strategy. They can be single-line or multi-line, and values can be identifiers, integers, or booleans. Example:

  ```esp
  BEHAVIOR find max_depth 3 recursive true
  ```

  or equivalently:

  ```esp
  BEHAVIOR find
  BEHAVIOR max_depth 3
  BEHAVIOR recursive true
  ```

  * Multiple behavior values are aggregated in order.
  * First token acts as a key; following tokens are arguments.
  * Order may matter (e.g., `max_depth` must precede `recursive`).

#### Criteria & Tests

* **TEST line**: Specifies both the *entity check* (`any`, `all`, `none`, `at_least_one`, `only_one`) and the *item check* (`all`, `at_least_one`, `only_one`, `none_satisfy`).
* **State operator**: Optional token (`AND`, `OR`, `ONE`) defines how multiple states are combined for an object. If omitted, defaults to `AND`.
* **CRI blocks**: Combine child CTN/CRI results with `AND` or `OR`. Nesting is supported. Execution order follows the tree structure; short-circuiting is not guaranteed by the grammar (engine-dependent).

#### Record Blocks

* **record ... record_end**: Encapsulates structured data inside a state. Nested `record` blocks allow hierarchical validation.
* **Record types**: Supported record kinds (JSON, SQL, PowerShell objects, etc.) depend on the scanner’s registered traits.
* **Access**: Fields inside records can be validated directly or via dot-notation (`parent.child.key`).
* **Trait system**: The scanner uses trait implementations to interpret record contents and resolve nested fields.

#### Scope & Reference Resolution

* **Global vs. Local**: Only global states/objects/sets can be referenced with `STATE_REF`, `OBJECT_REF`, or `SET_REF`. Local states/objects exist only within their CTN.
* **VAR & RUN**: Declared globally and available across the definition.
* **OBJ references**: Limited to `RUN` blocks and can only target global objects.
* **Resolution order**: References are resolved during the parser’s symbol discovery pass; evaluation then uses the resolved symbol table.

#### Example Flow

1. Parser ingests file → builds symbol tables for META, VAR, OBJECT, STATE, SET.
2. `RUN` blocks execute in order, declaring or updating variables.
3. Objects are collected → behaviors applied → filters evaluated.
4. States are applied to object data, including nested records.
5. CTNs evaluate according to TEST rules and state operator.
6. CRIs combine CTN/CRI results with logical operators.
7. Final DEF result determines compliance outcome.

---

## 16. Reference Resolution Rules

ESP uses references to connect definitions without duplicating values or logic. These references ensure reusability and maintain clear separation of concerns between variables, objects, states, and sets.

### Valid Reference Types

* **`VAR <variable_name>`**
  Refers to a variable. Variables are global by default, whether declared with `VAR` or created via `RUN`.

* **`STATE_REF <state_id>`**
  Refers to a global state defined at the definition level. Local states cannot be referenced outside their CTN.

* **`OBJECT_REF <object_id>`**
  Refers to a global object defined at the definition level. Local objects cannot be referenced outside their CTN.

* **`SET_REF <set_id>`**
  Refers to a set identifier, pulling in its defined members.

* **`OBJ <object_id> <field>`**
  Extracts a single field from a global object. Only valid inside `RUN` blocks. The result type is the same as the field’s type in the object.

### Scope and Resolution Rules

* All variables (`VAR` and `RUN`) exist in the **global scope** and may be referenced anywhere after declaration.
* **Local objects and states** exist only within the CTN that declares them; they cannot be referenced globally.
* **`STATE_REF` and `OBJECT_REF`** always require a global target.
* **`SET_REF`** may only target previously declared sets.
* **`OBJ` references** may only extract fields from global objects.
* Reference resolution is **order-dependent**: a reference must point to something declared earlier in the same definition.

### Examples

**Filter with `STATE_REF`:**

```esp
OBJECT log_file
  path string `/var/log`
  filename `*.log`
  FILTER include
    STATE_REF non_empty
  FILTER_END
OBJECT_END
```

**Criteria with `OBJECT_REF` and `STATE_REF`:**

```esp
CTN passwd_permissions
  OBJECT_REF etc_passwd
  STATE_REF correct_permissions
CTN_END
```

**Using `SET_REF` in a CTN:**

```esp
SET monitored_services
  OBJECT_REF all_services
  FILTER include
    STATE_REF active_state
  FILTER_END
SET_END

CTN services_running
  SET_REF monitored_services
  STATE_REF expected_ports
CTN_END
```

**Extracting a field with `OBJ` inside RUN:**

```esp
OBJECT system_info
  os_name string `Linux`
  version string `9`
OBJECT_END

RUN os_var EXTRACT
  OBJ system_info os_name
RUN_END
```

Here, `os_var` is assigned the value of the `os_name` field from the `system_info` object.

---

## 17. Execution Semantics

This section explains **how ESP definitions are parsed and executed**, expanding on the grammar to provide authors with a precise mental model of runtime behavior.

#### Parse-Time vs. Runtime

* **Parser (compile-time)**: Validates structure, discovers symbols, checks types, and builds reference tables. Multi-pass parsing ensures forward references are valid (except in `VAR` initialization).
* **Engine (runtime)**: Collects system data, applies behaviors, runs filters, resolves references, executes RUN blocks, and evaluates criteria.

#### Block Ordering

* **File root**: An ESP file starts with an optional `META`, followed by a single `DEF` block.
* **Inside DEF**: Elements (`VAR`, `STATE`, `OBJECT`, `RUN`, `SET`, `CRI`) may appear in any order. Grouping improves readability but is not required.
* **Inside CTN**: Elements must follow strict order: `TEST` → `STATE_REF`(s) → `OBJECT_REF`(s) → local `STATE`(s) → local `OBJECT`(s).

#### Variable & RUN Semantics

* **Global scope**: All `VAR` and `RUN` variables are global.
* **Initialization rule**: `VAR` initialization cannot reference another variable declared later.
* **RUN ordering**: `RUN` blocks execute in file order. Each `RUN` can **declare** its target variable if not already present. Later `RUN`s can depend on variables from earlier `RUN`s.
* **Literal type inference in RUN**: A `literal` token’s type is inferred from the operation:

  * `ARITHMETIC` → numeric (`int`/`float`).
  * `CONCAT`/`MERGE` → string.
  * `COUNT` → collection element type.
* **OBJ extraction in RUN**: `OBJ <object_id> <field>` extracts a field value from a **global object**. Resolution happens at runtime against the object registry. If the object or field is missing, the engine decides whether to fail or return empty (implementation-defined).

#### Filters & Behaviors

* **FILTER blocks**: Apply during data collection. Each `STATE_REF` inside must evaluate to true (logical AND) for the element to be retained. Filters only accept global states.
* **BEHAVIOR lines**: Alter collector execution strategy. They can be single-line or multi-line, and values can be identifiers, integers, or booleans. Example:

  ```esp
  BEHAVIOR find max_depth 3 recursive true
  ```

  or equivalently:

  ```esp
  BEHAVIOR find
  BEHAVIOR max_depth 3
  BEHAVIOR recursive true
  ```

  * Multiple behavior values are aggregated in order.
  * First token acts as a key; following tokens are arguments.
  * Order may matter (e.g., `max_depth` must precede `recursive`).

#### Criteria & Tests

* **TEST line**: Specifies both the *entity check* (`any`, `all`, `none`, `at_least_one`, `only_one`) and the *item check* (`all`, `at_least_one`, `only_one`, `none_satisfy`).
* **State operator**: Optional token (`AND`, `OR`, `ONE`) defines how multiple states are combined for an object. If omitted, defaults to `AND`.
* **CRI blocks**: Combine child CTN/CRI results with `AND` or `OR`. Nesting is supported. Execution order follows the tree structure; short-circuiting is not guaranteed by the grammar (engine-dependent).

#### Record Blocks

* **record ... record_end**: Encapsulates structured data inside a state. Nested `record` blocks allow hierarchical validation.
* **Record types**: Supported record kinds (JSON, SQL, PowerShell objects, etc.) depend on the scanner’s registered traits.
* **Access**: Fields inside records can be validated directly or via dot-notation (`parent.child.key`).
* **Trait system**: The scanner uses trait implementations to interpret record contents and resolve nested fields.

#### Scope & Reference Resolution

* **Global vs. Local**: Only global states/objects/sets can be referenced with `STATE_REF`, `OBJECT_REF`, or `SET_REF`. Local states/objects exist only within their CTN.
* **VAR & RUN**: Declared globally and available across the definition.
* **OBJ references**: Limited to `RUN` blocks and can only target global objects.
* **Resolution order**: References are resolved during the parser’s symbol discovery pass; evaluation then uses the resolved symbol table.

#### Example Flow

1. Parser ingests file → builds symbol tables for META, VAR, OBJECT, STATE, SET.
2. `RUN` blocks execute in order, declaring or updating variables.
3. Objects are collected → behaviors applied → filters evaluated.
4. States are applied to object data, including nested records.
5. CTNs evaluate according to TEST rules and state operator.
6. CRIs combine CTN/CRI results with logical operators.
7. Final DEF result determines compliance outcome.

---

# Part III – Putting It Together

## 19. Complete Examples

This part demonstrates how to apply the building blocks of ESP in full definitions. It transitions from foundational rules to **practical examples**, showing compliance authors how to combine META, DEF, VAR, OBJECT, STATE, CTN, CRI, SET, RUN, FILTER, and BEHAVIOR into complete rules.

> **Important structure reminder (from the grammar):** All **CTN** blocks appear **inside a CRI** block. Each **CRI** contains one or more **CTN** (and/or nested CRI). There is **no `CTN_REF`** token. CTNs have a **type** (plugin) name, not an identifier.

---

### 1) File Permission Check

Validate file permissions on a critical system file.

```esp
META
  version `1.0.0`
  esp_version `1.0`
  author `security-team`
  date `2024-01-15`
  severity `high`
  platform `linux`
  description `Ensure /etc/passwd has correct permissions`
  control_framework `CIS`
  control `1.1.1`
  esp_scan_id `file-perms`
  criticality `high`
  tags `file,permissions`
META_END

DEF file_permission_check
  STATE correct_permissions
    mode string = `0644`
  STATE_END

  OBJECT etc_passwd
    path `/etc`
    filename `passwd`
  OBJECT_END

  CRI AND
    CTN file_check
      TEST any all
      STATE_REF correct_permissions
      OBJECT_REF etc_passwd
    CTN_END
  CRI_END
DEF_END
```

---

### 2) File Content Validation

Check that a configuration file contains an expected directive using a **record** block.

```esp
DEF ssh_protocol_check
  STATE ssh_protocol
    record config record_end
      Protocol string = `2`
  STATE_END

  OBJECT ssh_config
    path `/etc/ssh`
    filename `sshd_config`
  OBJECT_END

  CRI AND
    CTN file_content
      TEST any all
      STATE_REF ssh_protocol
      OBJECT_REF ssh_config
    CTN_END
  CRI_END
DEF_END
```

Note: A record models structured config. You can also validate nested values using dot notation in states when applicable.

---

### 3) Package Installation Verification

Use **BEHAVIOR** to change how a package is checked (e.g., query the package manager).

```esp
DEF ssh_package_check
  STATE package_present
    name string = `openssh`
  STATE_END

  OBJECT ssh_package
    name `openssh`
    BEHAVIOR query fast_mode
  OBJECT_END

  CRI AND
    CTN package_check
      TEST any all
      STATE_REF package_present
      OBJECT_REF ssh_package
    CTN_END
  CRI_END
DEF_END
```

---

### 4) Service State Validation with FILTER

Restrict services to those that are **active** using an **OBJECT FILTER** (filters accept only **global** states).

```esp
DEF service_check
  STATE active_state
    status string = `active`
  STATE_END

  OBJECT all_services
    module `systemd`
    BEHAVIOR list
    FILTER include
      STATE_REF active_state
    FILTER_END
  OBJECT_END

  CRI AND
    CTN service_status
      TEST all all
      STATE_REF active_state
      OBJECT_REF all_services
    CTN_END
  CRI_END
DEF_END
```

---

### 5) Multi‑Criteria Check (CRI AND)

Combine multiple CTNs with **CRI AND** so **both** must pass.

```esp
DEF ssh_multi_check
  STATE protocol
    record config record_end
      Protocol string = `2`
  STATE_END

  STATE package_present
    name string = `openssh`
  STATE_END

  OBJECT ssh_config
    path `/etc/ssh`
    filename `sshd_config`
  OBJECT_END

  OBJECT ssh_package
    name `openssh`
    BEHAVIOR query
  OBJECT_END

  CRI AND
    CTN config_check
      TEST any all
      STATE_REF protocol
      OBJECT_REF ssh_config
    CTN_END

    CTN package_check
      TEST any all
      STATE_REF package_present
      OBJECT_REF ssh_package
    CTN_END
  CRI_END
DEF_END
```

---

### 6) Derived Values with RUN

Compute a threshold dynamically and use it in a state condition.

```esp
DEF disk_space_check
  RUN threshold ARITHMETIC
    literal 1024
    + 100
    * 2
  RUN_END

  STATE min_space
    size int >= VAR threshold
  STATE_END

  OBJECT root_fs
    path `/`
    BEHAVIOR usage
  OBJECT_END

  CRI AND
    CTN filesystem_usage
      TEST any all
      STATE_REF min_space
      OBJECT_REF root_fs
    CTN_END
  CRI_END
DEF_END
```

---

### 7) Sets & Aggregation with FILTER

Use **SET** to aggregate objects and apply a global-state **FILTER**, then validate the resulting set inside a CTN with a **local object** that references the set.

```esp
DEF set_aggregation_example
  STATE non_empty
    size int > 0
  STATE_END

  OBJECT ssh_config
    path `/etc/ssh`
    filename `sshd_config`
  OBJECT_END

  OBJECT etc_passwd
    path `/etc`
    filename `passwd`
  OBJECT_END

  SET critical_files union
    OBJECT_REF ssh_config
    OBJECT_REF etc_passwd
    FILTER include
      STATE_REF non_empty
    FILTER_END
  SET_END

  CRI AND
    CTN set_validation
      TEST all all
      STATE_REF non_empty
      OBJECT set_check
        SET_REF critical_files
      OBJECT_END
    CTN_END
  CRI_END
DEF_END
```

---

### 8) Full Control Example

An end‑to‑end, STIG‑style control combining META, VAR, STATE, OBJECT, BEHAVIOR, RUN, SET, CTN, and CRI.

```esp
META
  version `1.0.0`
  esp_version `1.0`
  author `security-team`
  date `2024-01-15`
  severity `medium`
  platform `linux`
  description `Ensure SSH configuration and package compliance`
  control_framework `STIG`
  control `V-12345`
  esp_scan_id `ssh-compliance`
  criticality `medium`
  tags `ssh,stig`
META_END

DEF ssh_full_control
  VAR expected_protocol string = `2`

  STATE protocol
    record config record_end
      Protocol string = VAR expected_protocol
  STATE_END

  STATE package_present
    name string = `openssh`
  STATE_END

  OBJECT ssh_config
    path `/etc/ssh`
    filename `sshd_config`
  OBJECT_END

  OBJECT ssh_package
    name `openssh`
    BEHAVIOR query fast_mode
  OBJECT_END

  CRI AND
    CTN config_check
      TEST any all
      STATE_REF protocol
      OBJECT_REF ssh_config
    CTN_END

    CTN package_check
      TEST any all
      STATE_REF package_present
      OBJECT_REF ssh_package
    CTN_END
  CRI_END
DEF_END
```

---


13. Best Practices
14. Common Pitfalls
15. Troubleshooting

# Part IV – Reference

16. Block-by-Block Reference
17. Glossary & Index