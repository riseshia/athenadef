# Architecture Design

## Overview

athenadef is a Rust CLI tool for AWS Athena schema management.
It applies the concept of declarative infrastructure management like Terraform to Athena table schemas, managing table definitions using SQL DDL files.

## Design Goals

- **Declarative Management**: Managing table definitions using SQL files
- **Diff Detection**: Display differences between current state and desired state
- **Safe Application**: Safe schema changes through plan -> apply workflow
- **Type Safety**: Error prevention leveraging Rust's type system

## Core Components

### 1. CLI Layer (`src/cli.rs`)
- Command-line argument parsing using `clap`
- Definition of `apply`, `plan`, `export` commands
- Management of common options (config, debug, target)

### 2. Command Layer (`src/commands/`)
- `apply.rs`: Apply schema changes
- `plan.rs`: Display change plans
- `export.rs`: Export existing table definitions

### 3. Core Logic Layer
- `differ.rs`: Calculate differences in table definitions
- `athena.rs`: Integration with AWS Athena API
- `s3.rs`: Query result retrieval and storage
- `context.rs`: Application context

### 4. Types Layer (`src/types/`)
- `config.rs`: Configuration file type definitions
- `table_definition.rs`: Table definition types
- `diff_result.rs`: Diff result types
- `query_execution.rs`: Query execution result types

### 5. Utilities
- `file_utils.rs`: File operation utilities (SQL string reading only)

## Data Flow

```
Local SQL Files (database/table.sql)
    ↓
File Reader (read as string)
    ↓
                    Remote SQL (from SHOW CREATE TABLE)
                            ↓
Differ Engine ← Text Comparison (using similar crate)
    ↓
Diff Result (with unified diff text)
    ↓
Command Executor → Athena API (DDL execution)
```

**Key Design Philosophy:**
- Simple text-based diff (no complex schema parsing)
- Read local SQL files as-is
- Get remote DDL via `SHOW CREATE TABLE`
- Compare normalized SQL text
- Display unified diff (like git diff)

## Directory Structure

```
src/
├── cli.rs                    # CLI definition
├── main.rs                   # Entry point
├── lib.rs                    # Library root
├── context.rs                # Application context
├── differ.rs                 # Diff calculation engine
├── file_utils.rs            # File operations (SQL string reading)
├── commands/
│   ├── mod.rs
│   ├── apply.rs             # Schema application
│   ├── plan.rs              # Change plan display
│   └── export.rs            # Table definition export
├── types/
│   ├── mod.rs
│   ├── config.rs            # Configuration types
│   ├── table_definition.rs  # Table definition types
│   ├── diff_result.rs       # Diff result types
│   └── query_execution.rs   # Query execution types
└── aws/
    ├── mod.rs
    ├── athena.rs            # Athena API
    ├── s3.rs                # S3 API
    └── sts.rs               # STS API
```

## Configuration

YAML-based configuration file (`athenadef.yaml`):

```yaml
workgroup: "primary"
output_location: "s3://your-athena-results-bucket/prefix/"  # Optional: omit to use workgroup's default
region: "us-west-2"  # Optional
```

**Query Result Storage:**
- When `output_location` is **not specified**: Uses the workgroup's default output location setting
- When `output_location` is **specified**: Overrides workgroup settings with custom S3 location

## Error Handling

- Comprehensive error handling using `anyhow`
- Proper conversion and display of AWS API errors
- User-friendly error messages

## Testing Strategy

- Unit tests: Functional testing of each module
- Integration tests: AWS API integration testing (using mockall)
- E2E tests: Verification in actual Athena environment

## Performance Considerations

- Parallel processing for table information retrieval
- Reduction of unnecessary API calls
