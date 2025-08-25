# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

athenadef is a Rust CLI tool for AWS Athena schema management. It allows users to manage Athena table schemas through SQL files organized in a directory structure that mirrors database/table relationships.

## Architecture

- **Language**: Rust
- **CLI Framework**: Expected to use clap or similar for command parsing
- **AWS Integration**: Uses AWS SDK for Athena and S3 operations
- **File Structure**: SQL files organized as `database_name/table_name.sql`
- **Configuration**: YAML-based configuration (`athenadef.yaml`)

## Commands Structure

The tool provides these main commands:
- `apply`: Apply configuration changes to Athena
- `plan`: Preview configuration changes  
- `export`: Export existing table definitions to local files
- `help`: Display help information

## Development Commands

Since this is a Rust project, these commands will likely be used:

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run the CLI locally
cargo run -- <command>

# Build for release
cargo build --release

# Check code formatting
cargo fmt --check

# Run clippy for linting
cargo clippy
```

## Configuration File

The tool expects an `athenadef.yaml` configuration file with:
- `workgroup`: Athena workgroup (optional, defaults to "primary")
- `output_location`: S3 location for query results (optional)

## Directory Structure for Schema Files

Schema files should be organized as:
```
database_name/
  table_name.sql
```

Each SQL file contains DDL for the corresponding table.

## AWS Permissions

The tool requires specific IAM permissions for:
- Athena query operations (StartQueryExecution, GetQueryExecution, etc.)
- S3 bucket access for query results storage

## Distribution

- Available via Homebrew: `brew install rieshia/x/athenadef`
- GitHub releases with compiled binaries
- GitHub Action available: `riseshia/athenadef@v0`