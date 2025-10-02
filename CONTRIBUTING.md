# Contributing to athenadef

Thank you for your interest in contributing to athenadef! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Submitting Changes](#submitting-changes)
- [Coding Standards](#coding-standards)
- [Documentation](#documentation)
- [Release Process](#release-process)

## Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). Please be respectful and constructive in all interactions.

## Getting Started

### Finding Issues to Work On

- Check the [Issues](https://github.com/riseshia/athenadef/issues) page
- Look for issues tagged with `good first issue` or `help wanted`
- If you have an idea for a new feature, open an issue to discuss it first

### Reporting Bugs

When reporting bugs, please include:

1. **Description**: Clear description of the bug
2. **Steps to reproduce**: Minimal steps to reproduce the issue
3. **Expected behavior**: What you expected to happen
4. **Actual behavior**: What actually happened
5. **Environment**:
   - athenadef version: `athenadef --version`
   - Operating system
   - Rust version (if building from source): `rustc --version`
6. **Debug output**: Run with `--debug` flag and include relevant output
7. **Configuration**: Your `athenadef.yaml` (remove sensitive information)

### Suggesting Features

When suggesting features:

1. Open an issue with the `enhancement` label
2. Describe the use case and problem you're trying to solve
3. Explain your proposed solution
4. Include examples if applicable
5. Consider backward compatibility

## Development Setup

### Prerequisites

- **Rust**: 1.70 or later
- **AWS Account**: For testing (optional but recommended)
- **Git**: For version control

### Initial Setup

1. **Fork the repository:**

   Click the "Fork" button on GitHub

2. **Clone your fork:**

   ```bash
   git clone https://github.com/YOUR_USERNAME/athenadef.git
   cd athenadef
   ```

3. **Add upstream remote:**

   ```bash
   git remote add upstream https://github.com/riseshia/athenadef.git
   ```

4. **Build the project:**

   ```bash
   cargo build
   ```

5. **Run tests:**

   ```bash
   cargo test
   ```

6. **Run the CLI locally:**

   ```bash
   cargo run -- --help
   cargo run -- plan --debug
   ```

### Development Tools

Install recommended tools:

```bash
# Format checker
rustup component add rustfmt

# Linter
rustup component add clippy

# Code coverage (optional)
cargo install cargo-tarpaulin
```

## Making Changes

### Branching Strategy

1. **Create a feature branch:**

   ```bash
   git checkout -b feature/my-new-feature
   ```

   Branch naming conventions:
   - Features: `feature/description`
   - Bug fixes: `fix/description`
   - Documentation: `docs/description`
   - Refactoring: `refactor/description`

2. **Keep your branch updated:**

   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

### Development Workflow

1. **Make your changes**

2. **Test your changes:**

   ```bash
   # Run all tests
   cargo test

   # Run specific test
   cargo test test_name

   # Run with logging
   RUST_LOG=debug cargo test
   ```

3. **Check formatting:**

   ```bash
   cargo fmt --check
   ```

4. **Run linter:**

   ```bash
   cargo clippy --all-targets --all-features --workspace -- -D warnings
   ```

5. **Build documentation:**

   ```bash
   cargo doc --open
   ```

### Code Organization

```
athenadef/
├── src/
│   ├── main.rs           # CLI entry point
│   ├── config.rs         # Configuration handling
│   ├── commands/         # Command implementations
│   │   ├── apply.rs
│   │   ├── plan.rs
│   │   └── export.rs
│   ├── athena/           # AWS Athena integration
│   ├── glue/             # AWS Glue integration
│   ├── diff/             # Diff computation
│   ├── parser/           # SQL parsing (if needed)
│   └── lib.rs            # Library root
├── tests/                # Integration tests
├── examples/             # Example configurations
├── docs/                 # Documentation
└── Cargo.toml            # Dependencies
```

## Testing

### Unit Tests

Place unit tests in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        assert_eq!(2 + 2, 4);
    }
}
```

### Integration Tests

Place integration tests in `tests/`:

```rust
// tests/integration_test.rs
use athenadef::config::Config;

#[test]
fn test_config_loading() {
    let config = Config::from_file("examples/basic/athenadef.yaml");
    assert!(config.is_ok());
}
```

### Testing with AWS

For tests requiring AWS:

1. **Use test fixtures** when possible
2. **Mock AWS calls** for unit tests
3. **Optional integration tests** that use real AWS (gated behind feature flag):

   ```bash
   # Run with AWS integration tests
   cargo test --features aws-integration
   ```

4. **Use test databases** with names like `athenadef_test_*`

### Test Coverage

Check test coverage:

```bash
cargo tarpaulin --out Html
```

## Submitting Changes

### Commit Guidelines

Write clear, descriptive commit messages:

**Format:**
```
<type>: <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `test`: Test additions or changes
- `refactor`: Code refactoring
- `style`: Formatting changes
- `chore`: Build, CI, or tooling changes

**Example:**
```
feat: Add support for partition projection

Implement partition projection for date-based partitions.
This allows queries on partitioned tables without running
MSCK REPAIR TABLE.

Fixes #123
```

### Pull Request Process

1. **Update your branch:**

   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Push your changes:**

   ```bash
   git push origin feature/my-new-feature
   ```

3. **Create Pull Request:**

   - Go to GitHub and click "New Pull Request"
   - Fill in the PR template
   - Link related issues

4. **PR Description should include:**

   - Summary of changes
   - Motivation and context
   - Type of change (bug fix, feature, etc.)
   - Testing done
   - Checklist completion

5. **Wait for review:**

   - Address reviewer feedback
   - Update PR as needed
   - Rebase if requested

6. **After approval:**

   - Maintainer will merge your PR
   - You can delete your branch

### Pull Request Checklist

- [ ] Code follows project style guidelines
- [ ] All tests pass: `cargo test`
- [ ] Code is formatted: `cargo fmt`
- [ ] Clippy is happy: `cargo clippy`
- [ ] Documentation is updated (if applicable)
- [ ] Tests are added (if applicable)
- [ ] Commit messages follow guidelines
- [ ] PR description is clear and complete

## Coding Standards

### Rust Style

Follow the [Rust Style Guide](https://doc.rust-lang.org/1.0.0/style/):

- Use `rustfmt` for formatting
- Use `clippy` for linting
- Write idiomatic Rust code
- Use descriptive variable names
- Add comments for complex logic

### Error Handling

- Use `Result` types for operations that can fail
- Provide clear error messages
- Use custom error types when appropriate
- Don't panic in library code

```rust
// Good
pub fn do_something() -> Result<(), Error> {
    let value = fetch_value()
        .context("Failed to fetch value")?;
    Ok(())
}

// Avoid
pub fn do_something() {
    let value = fetch_value().unwrap();
}
```

### Documentation

Document public APIs:

```rust
/// Loads configuration from a file.
///
/// # Arguments
///
/// * `path` - Path to the configuration file
///
/// # Returns
///
/// Returns `Ok(Config)` on success, `Err` on failure.
///
/// # Examples
///
/// ```
/// use athenadef::config::Config;
///
/// let config = Config::from_file("athenadef.yaml")?;
/// ```
pub fn from_file(path: &str) -> Result<Config> {
    // Implementation
}
```

### Naming Conventions

- Types: `PascalCase`
- Functions: `snake_case`
- Constants: `SCREAMING_SNAKE_CASE`
- Modules: `snake_case`

### Code Review Checklist

When reviewing code:

- [ ] Code is clear and maintainable
- [ ] Logic is correct
- [ ] Edge cases are handled
- [ ] Tests are comprehensive
- [ ] Documentation is clear
- [ ] Performance is reasonable
- [ ] Security considerations addressed

## Documentation

### Types of Documentation

1. **Code documentation** (Rustdoc)
   - Document public APIs
   - Include examples
   - Explain complex logic

2. **User documentation** (Markdown)
   - README.md - Getting started
   - docs/ - Detailed guides
   - examples/ - Working examples

3. **Architecture documentation**
   - docs/architecture.md
   - docs/technical-design.md
   - docs/specification.md

### Updating Documentation

When making changes:

- Update relevant documentation
- Add examples if introducing new features
- Update README.md if user-facing changes
- Add migration notes for breaking changes

### Building Documentation

```bash
# Build Rust documentation
cargo doc --open

# Preview README changes
# Use a Markdown viewer or GitHub preview
```

## Release Process

(For maintainers)

1. **Update version:**

   ```bash
   # In Cargo.toml
   version = "0.2.0"
   ```

2. **Update CHANGELOG:**

   ```markdown
   ## [0.2.0] - 2025-01-15

   ### Added
   - New feature X

   ### Changed
   - Improved feature Y

   ### Fixed
   - Bug Z
   ```

3. **Create release commit:**

   ```bash
   git add Cargo.toml CHANGELOG.md
   git commit -m "chore: Release v0.2.0"
   ```

4. **Tag release:**

   ```bash
   git tag -a v0.2.0 -m "Release v0.2.0"
   git push origin main --tags
   ```

5. **GitHub Actions will:**
   - Build binaries
   - Create GitHub release
   - Publish to crates.io (if configured)

## Questions?

If you have questions:

1. Check existing [documentation](docs/)
2. Search [Issues](https://github.com/riseshia/athenadef/issues)
3. Open a new issue with the `question` label
4. Ask in discussions (if enabled)

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT License).

## Thank You!

Thank you for contributing to athenadef! Your contributions help make this tool better for everyone.
