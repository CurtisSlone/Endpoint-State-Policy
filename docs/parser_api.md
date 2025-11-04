# ESP Parser FFI Reference Documentation

## Overview

The ESP Parser library provides a Foreign Function Interface (FFI) for parsing and processing ICS (Intermediate Compliance Syntax) files through a comprehensive 7-stage pipeline. The library supports both single file and batch directory processing, with execution data extraction capabilities for scanner integration.

### Architecture Overview

The parser operates with clear separation of concerns:
- **Parser Responsibility**: Syntax validation, variable resolution, reference validation, and execution data preparation
- **Scanner Responsibility**: Platform-specific API execution, compliance checking, and result aggregation
- **Execution Data**: Resolved, ready-to-execute ICS definitions with no semantic interpretation by the parser

### 7-Stage Pipeline

1. **File Processing**: File validation and encoding detection
2. **Lexical Analysis**: Token generation and string literal extraction
3. **Syntax Analysis**: AST construction following EBNF grammar
4. **Symbol Discovery**: Symbol table construction and relationship tracking
5. **Reference Validation**: Reference resolution and dependency validation
6. **Semantic Analysis**: Type checking and dependency analysis
7. **Structural Validation**: Final patterns and completeness verification

## Core Data Types

### Error Codes

```c
typedef enum {
    ESP_SUCCESS = 0,
    ESP_FILE_NOT_FOUND = 1,
    ESP_PARSE_FAILED = 2,
    ESP_VALIDATION_FAILED = 3,
    ESP_INTERNAL_ERROR = 4,
    ESP = 5,
    ESP_NULL_POINTER = 6
} EspErrorCode;
```

### Result Handles

```c
typedef struct {
    size_t id;  // Internal result identifier for result storage lookup
} EspParseResult;

typedef struct {
    size_t id;  // Internal batch result identifier
} EspBatchResults;
```

**Note**: Result handles use internal storage with unique IDs. Multiple handles can exist simultaneously, and each must be explicitly freed.

## Library Initialization

### Initialization

```c
IcsErrorCode ics_init(void);
```

**Description**: Initialize the parser library with default settings. Sets up logging and validates pipeline components.

**Returns**: 
- `ICS_SUCCESS`: Initialization completed successfully
- `ICS_INTERNAL_ERROR`: Initialization failed

**Note**: Must be called once before using other library functions. Thread-safe for concurrent initialization calls.

### Version Information

```c
const char* ics_parser_version(void);
uint32_t ics_parser_version_major(void);
uint32_t ics_parser_version_minor(void);
uint32_t ics_parser_version_patch(void);
```

**Description**: Get library version information.

**Returns**: Version string or numeric components

**Note**: Version string points to static memory and does not need to be freed.

## Core Parsing Functions

### Single File Parsing

```c
IcsErrorCode ics_parse_file(
    const char* file_path,
    IcsParseResult** result
);
```

**Description**: Parses a single ICS file through the complete 7-stage pipeline. Performs syntax validation, symbol resolution, reference validation, and semantic analysis.

**Parameters**:
- `file_path`: Null-terminated UTF-8 string path to the ICS file
- `result`: Pointer to receive the allocated parse result handle

**Returns**: 
- `ICS_SUCCESS`: File parsed successfully
- `ICS_FILE_NOT_FOUND`: File does not exist or cannot be read (maps to `PipelineError::FileProcessing`)
- `ICS_PARSE_FAILED`: Syntax or lexical errors (maps to `PipelineError::LexicalAnalysis` or `PipelineError::SyntaxAnalysis`)
- `ICS_VALIDATION_FAILED`: Semantic/structural validation failed (maps to validation pipeline errors)
- `ICS_INVALID_PATH`: Path contains invalid UTF-8 characters
- `ICS_NULL_POINTER`: `file_path` or `result` is NULL
- `ICS_INTERNAL_ERROR`: Internal processing error

### Directory Batch Processing

```c
IcsErrorCode ics_parse_directory(
    const char* dir_path,
    IcsBatchResults** results
);
```

**Description**: Processes all `.ics` files in a directory. Continues processing even if individual files fail.

**Parameters**:
- `dir_path`: Null-terminated UTF-8 string path to directory
- `results`: Pointer to receive the allocated batch results handle

**Returns**: 
- `ICS_SUCCESS`: Directory processed (individual file results available in batch results)
- `ICS_INVALID_PATH`: Directory path invalid or inaccessible
- `ICS_NULL_POINTER`: `dir_path` or `results` is NULL
- `ICS_INTERNAL_ERROR`: Internal processing error

## Result Inspection Functions

### Basic Parse Metrics

```c
uint32_t ics_result_token_count(const IcsParseResult* result);
uint32_t ics_result_symbol_count(const IcsParseResult* result);
bool ics_result_is_valid(const IcsParseResult* result);
uint64_t ics_result_duration_ms(const IcsParseResult* result);
```

**Description**: Extract basic parsing metrics from a result.

**Parameters**:
- `result`: Valid parse result handle

**Returns**: 
- Respective metric value
- `0`/`false` if result is NULL or invalid

**Note**: `ics_result_is_valid()` returns true only if all validation stages (reference, semantic, and structural) passed.

### Batch Result Metrics

```c
uint32_t ics_batch_success_count(const IcsBatchResults* results);
uint32_t ics_batch_failure_count(const IcsBatchResults* results);
```

**Description**: Get success/failure counts from batch processing.

**Parameters**:
- `results`: Valid batch results handle

**Returns**: Count of successful or failed files

**Note**: Total count = success_count + failure_count

## Execution Data Functions

These functions transform the validated AST into execution-ready data for scanner consumption. All variable references are resolved, and the data is structured for processing by scanner modules.

### Complete Execution Data

```c
IcsErrorCode ics_get_execution_data(
    const IcsParseResult* result,
    char** execution_json
);
```

**Description**: Extracts complete execution-ready data as JSON. This includes all criteria with resolved references, fully resolved variables, and complete object/state definitions.

**Parameters**:
- `result`: Valid parse result handle
- `execution_json`: Pointer to receive allocated JSON string

**Returns**: 
- `ICS_SUCCESS`: Execution data extracted successfully
- `ICS_NULL_POINTER`: `result` or `execution_json` is NULL
- `ICS_INTERNAL_ERROR`: Transformation or serialization failed

**JSON Structure** (based on `ExecutionData` type):
```json
{
  "criteria": [
    {
      "ctn_type": "file_test",
      "test_specification": {
        "existence_check": "any",
        "item_check": "all",
        "state_operator": "AND"
      },
      "resolved_global_states": [
        {
          "id": "permission_check",
          "operations": [
            {
              "field": "permissions",
              "operation": "=",
              "value": "0644",
              "data_type": "string",
              "entity_check": null
            }
          ],
          "is_global": true,
          "source_info": "Global state definition"
        }
      ],
      "resolved_global_objects": [
        {
          "id": "config_file",
          "fields": {
            "path": "/etc/security.conf",
            "owner": "root"
          },
          "is_global": true,
          "source_info": "Global object definition"
        }
      ],
      "local_states": [],
      "local_object": null
    }
  ],
  "resolved_variables": {
    "config_path": "/etc/security.conf",
    "service_port": "8080"
  },
  "global_objects": {
    "config_file": {
      "id": "config_file",
      "fields": {
        "path": "/etc/security.conf",
        "owner": "root"
      },
      "is_global": true,
      "source_info": "Global object definition"
    }
  },
  "global_states": {
    "permission_check": {
      "id": "permission_check",
      "operations": [
        {
          "field": "permissions",
          "operation": "=",
          "value": "0644",
          "data_type": "string",
          "entity_check": null
        }
      ],
      "is_global": true,
      "source_info": "Global state definition"
    }
  }
}
```

### Resolved Variables Only

```c
IcsErrorCode ics_get_resolved_variables(
    const IcsParseResult* result,
    char** variables_json
);
```

**Description**: Extracts only the resolved variable mappings as JSON. All variable references and RUN operations have been processed to final values.

**Parameters**:
- `result`: Valid parse result handle
- `variables_json`: Pointer to receive allocated JSON string

**Returns**: 
- `ICS_SUCCESS`: Variables extracted successfully
- `ICS_NULL_POINTER`: `result` or `variables_json` is NULL
- `ICS_INTERNAL_ERROR`: Resolution or serialization failed

**JSON Structure**:
```json
{
  "base_path": "/etc/security",
  "config_file": "/etc/security/app.conf",
  "service_port": "8080"
}
```

### Execution Metadata

```c
IcsErrorCode ics_get_execution_metadata(
    const IcsParseResult* result,
    char** metadata_json
);
```

**Description**: Extracts processing metadata, validation status, and scanner planning information.

**Parameters**:
- `result`: Valid parse result handle
- `metadata_json`: Pointer to receive allocated JSON string

**Returns**: 
- `ICS_SUCCESS`: Metadata extracted successfully
- `ICS_NULL_POINTER`: `result` or `metadata_json` is NULL
- `ICS_INTERNAL_ERROR`: Extraction or serialization failed

**JSON Structure** (based on `ExecutionMetadata` type):
```json
{
  "validation_passed": true,
  "total_criteria": 5,
  "has_dependencies": false,
  "global_object_count": 3,
  "global_state_count": 2,
  "processing_stats": {
    "token_count": 1247,
    "symbol_count": 23,
    "duration_ms": 45,
    "file_size_bytes": 8192
  },
  "ctn_types": ["file_test", "process_test"]
}
```

## Logging Functions

The parser provides structured logging integration for scanner applications.

### Consumer Context Setup

```c
IcsErrorCode ics_log_set_consumer_context(
    const char* consumer_id,
    const char* module
);
```

**Description**: Set consumer identification for logging context.

**Parameters**:
- `consumer_id`: Unique identifier for the consumer application
- `module`: Optional module name (can be NULL)

**Returns**: 
- `ICS_SUCCESS`: Context set successfully
- `ICS_NULL_POINTER`: `consumer_id` is NULL

### Error Logging

```c
IcsErrorCode ics_log_consumer_error(
    const char* code,
    const char* message,
    size_t context_count,
    const char* const* context_keys,
    const char* const* context_values
);
```

**Description**: Log an error from consumer code with structured context.

**Parameters**:
- `code`: Consumer error code (see valid codes below)
- `message`: Error message
- `context_count`: Number of context key-value pairs (maximum 4 handled optimally)
- `context_keys`: Array of context key strings
- `context_values`: Array of context value strings

**Valid Consumer Error Codes**:
- `C001`: Consumer initialization failure
- `C002`: Consumer configuration error
- `C003`: Consumer shutdown error
- `C010`: Consumer pipeline error
- `C011`: Consumer pass failure
- `C012`: Consumer state mismatch
- `C020`: Consumer data validation error
- `C021`: Consumer format error
- `C022`: Consumer encoding error
- `C030`: Consumer memory error
- `C031`: Consumer timeout error
- `C032`: Consumer capacity error
- `C040`: Consumer I/O error
- `C041`: Consumer network error
- `C042`: Consumer permission error

**Returns**:
- `ICS_SUCCESS`: Message logged successfully
- `ICS_NULL_POINTER`: Required parameter is NULL
- `ICS_INVALID_PATH`: Invalid error code (maps to invalid consumer code)

### Other Logging Levels

```c
IcsErrorCode ics_log_consumer_warning(
    const char* message,
    size_t context_count,
    const char* const* context_keys,
    const char* const* context_values
);

IcsErrorCode ics_log_consumer_info(
    const char* message,
    size_t context_count,
    const char* const* context_keys,
    const char* const* context_values
);

IcsErrorCode ics_log_consumer_debug(
    const char* message,
    size_t context_count,
    const char* const* context_keys,
    const char* const* context_values
);
```

**Description**: Log messages at different severity levels with structured context.

**Parameters**: Same as `ics_log_consumer_error` except no error code required

**Returns**: Same as `ics_log_consumer_error`

## Memory Management

### String Deallocation

```c
void ics_free_string(char* s);
```

**Description**: Deallocates strings returned by the library.

**Parameters**:
- `s`: String pointer returned by library functions

**Note**: ALL strings returned by `ics_get_*` functions must be freed using this function. Using standard `free()` will cause undefined behavior.

### Result Deallocation

```c
void ics_result_free(IcsParseResult* result);
void ics_batch_results_free(IcsBatchResults* results);
```

**Description**: Deallocates result handles and associated memory.

**Parameters**:
- `result`/`results`: Handle to deallocate

**Note**: Result handles use internal storage. Freeing a handle removes it from internal storage and deallocates associated memory.

## Complete Usage Examples

### Basic File Processing

```c
#include "ics_parser.h"
#include <stdio.h>
#include <stdlib.h>

int main() {
    // Initialize library
    if (ics_init() != ICS_SUCCESS) {
        fprintf(stderr, "Failed to initialize ICS parser\n");
        return 1;
    }
    
    printf("ICS Parser v%s initialized\n", ics_parser_version());
    
    // Parse file
    IcsParseResult* result = NULL;
    IcsErrorCode code = ics_parse_file("config.ics", &result);
    
    if (code == ICS_SUCCESS) {
        if (ics_result_is_valid(result)) {
            printf("Parse successful:\n");
            printf("  Tokens: %u\n", ics_result_token_count(result));
            printf("  Symbols: %u\n", ics_result_symbol_count(result));
            printf("  Duration: %llu ms\n", ics_result_duration_ms(result));
            
            // Get execution data
            char* execution_json = NULL;
            if (ics_get_execution_data(result, &execution_json) == ICS_SUCCESS) {
                printf("Execution data: %s\n", execution_json);
                ics_free_string(execution_json);
            }
        } else {
            printf("Parse completed but validation failed\n");
        }
        
        ics_result_free(result);
    } else {
        printf("Parse failed with code: %d\n", code);
    }
    
    return 0;
}
```

### Batch Processing

```c
void process_directory(const char* dir_path) {
    IcsBatchResults* batch = NULL;
    IcsErrorCode code = ics_parse_directory(dir_path, &batch);
    
    if (code == ICS_SUCCESS) {
        uint32_t successful = ics_batch_success_count(batch);
        uint32_t failed = ics_batch_failure_count(batch);
        uint32_t total = successful + failed;
        
        printf("Batch processing complete:\n");
        printf("  Total files: %u\n", total);
        printf("  Successful: %u\n", successful);
        printf("  Failed: %u\n", failed);
        
        if (total > 0) {
            printf("  Success rate: %.1f%%\n", (successful * 100.0 / total));
        }
        
        ics_batch_results_free(batch);
    } else {
        printf("Batch processing failed: %d\n", code);
    }
}
```

### Error Handling Pattern

```c
IcsErrorCode code = ics_parse_file(file_path, &result);
switch (code) {
    case ICS_SUCCESS:
        // Continue processing
        break;
    case ICS_FILE_NOT_FOUND:
        printf("File not found: %s\n", file_path);
        break;
    case ICS_PARSE_FAILED:
        printf("Syntax error in file: %s\n", file_path);
        break;
    case ICS_VALIDATION_FAILED:
        printf("Validation failed for file: %s\n", file_path);
        break;
    case ICS_INVALID_PATH:
        printf("Invalid path encoding: %s\n", file_path);
        break;
    case ICS_NULL_POINTER:
        printf("Programming error: NULL pointer passed\n");
        break;
    case ICS_INTERNAL_ERROR:
        printf("Internal parser error\n");
        break;
    default:
        printf("Unknown error code: %d\n", code);
        break;
}
```

### Resource Cleanup Pattern

```c
IcsParseResult* result = NULL;
char* json_string = NULL;

// Parse file
if (ics_parse_file(file_path, &result) == ICS_SUCCESS) {
    // Get execution data
    if (ics_get_execution_data(result, &json_string) == ICS_SUCCESS) {
        // Use json_string...
        
        // Always free the string
        ics_free_string(json_string);
    }
    
    // Always free the result
    ics_result_free(result);
}
```

## Thread Safety

### Safe Concurrent Operations

Multiple threads can safely:
- Call `ics_parse_file` or `ics_parse_directory` concurrently
- Access different result handles concurrently  
- Call logging functions concurrently
- Call version/info functions concurrently

### Unsafe Operations

Multiple threads should NOT:
- Access the same result handle concurrently
- Free a result handle while another thread is using it
- Pass result handles between threads without proper synchronization

## Compilation and Linking

### C/C++ Integration

```c
// Header inclusion
#include "ics_parser.h"

// Link with the Rust library:
// gcc -o scanner scanner.c -lics_parser
```

### Build Requirements

The FFI functionality requires the `ffi` feature to be enabled during compilation:

```bash
# Build with FFI support
cargo build --features ffi --release
```

## Implementation Notes and Limitations

### File Processing Constraints

- **Maximum file size**: 50MB per file
- **Supported extensions**: Only `.ics` files are processed
- **Character encoding**: Files must be valid UTF-8
- **Path constraints**: Limited by system path length limits

### Memory Management

- **Result storage**: Uses internal `HashMap` with `LazyLock<Mutex<HashMap<usize, Box<PipelineResult>>>>` for thread-safe result lifetime management
- **String allocation**: All returned strings use Rust's allocator via `CString`
- **Memory safety**: Proper cleanup prevents memory leaks when following the documented patterns

### Parser Behavior

- **Error recovery**: Parser attempts recovery at keyword boundaries for syntax errors
- **Validation continuation**: All validation stages run even if earlier stages fail
- **Batch processing**: Individual file failures don't stop directory processing
- **Variable resolution**: All `Value::Variable` references are resolved to final string values

### Scanner Integration Notes

- **No semantic interpretation**: Parser extracts all object fields without filtering or categorization
- **CTN type preservation**: No normalization applied - scanner builds own routing registry
- **Platform agnostic**: Parser output requires scanner interpretation for platform-specific execution
- **Reference resolution**: All `STATE_REF` and `OBJECT_REF` elements are resolved to complete definitions

This FFI interface provides complete access to the ICS parser functionality while maintaining memory safety and clear architectural boundaries between parsing and scanning responsibilities.