# athenadef Examples

This directory contains example projects demonstrating various use cases of athenadef.

## Available Examples

### [basic/](./basic) - Simple Setup
A simple example with one database and two tables. Perfect for getting started.

**What you'll learn:**
- Basic directory structure
- Simple table definitions
- Basic configuration

### [partitioned/](./partitioned) - Partitioned Tables
Demonstrates partitioned tables with partition projection for time-series data.

**What you'll learn:**
- Partition projection configuration
- Date-based partitioning
- Multi-level partitioning (year/month/day/hour)
- Performance optimization for large datasets

### [multi-database/](./multi-database) - Multiple Databases
Shows how to manage multiple databases with many tables and use target filtering.

**What you'll learn:**
- Multiple database management
- Target filtering with `--target` option
- Organizing larger projects
- Parallel execution configuration

## Running Examples

Each example directory is self-contained and can be run independently:

```bash
# Navigate to an example directory
cd examples/basic

# Preview changes
athenadef plan

# Apply changes
athenadef apply

# Export existing tables
athenadef export
```

## Prerequisites

Before running examples:

1. **Install athenadef**: See the [main README](../README.md#installation)
2. **Configure AWS credentials**: Ensure you have AWS credentials configured
3. **Update S3 paths**: Change the S3 locations in SQL files to match your buckets
4. **Check permissions**: Ensure your AWS credentials have the required IAM permissions

## Customizing Examples

To use these examples with your own AWS environment:

1. **Copy the example** to your project directory
2. **Update S3 paths** in all `.sql` files to point to your S3 buckets
3. **Modify configuration** in `athenadef.yaml` if needed (workgroup, region, etc.)
4. **Run `athenadef plan`** to see what would be created
5. **Run `athenadef apply`** to create the tables

## Learning Path

Recommended order for learning:

1. Start with **basic/** to understand the fundamentals
2. Move to **partitioned/** to learn about partitioning strategies
3. Explore **multi-database/** for managing larger projects

## Common Patterns

### Testing Changes Safely

```bash
# Preview changes without applying
athenadef plan

# Dry run to see what would happen
athenadef apply --dry-run

# Apply to a single table first
athenadef apply --target salesdb.customers

# Then apply to the full database
athenadef apply --target salesdb.*
```

### Exporting Existing Infrastructure

If you already have tables in Athena:

```bash
# Export all tables
athenadef export

# Export specific database
athenadef export --target salesdb.*

# Overwrite existing files
athenadef export --overwrite
```

### CI/CD Integration

Use these examples as templates for your CI/CD pipeline:

```yaml
# .github/workflows/athena-deploy.yml
name: Deploy Athena Tables
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: riseshia/athenadef@v0
        with:
          version: latest
      - name: Apply changes
        run: |
          cd examples/basic
          athenadef apply --auto-approve
        env:
          AWS_ACCESS_KEY_ID: ${{ secrets.AWS_ACCESS_KEY_ID }}
          AWS_SECRET_ACCESS_KEY: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
          AWS_REGION: us-west-2
```

## Additional Resources

- [Main Documentation](../README.md)
- [Specification](../docs/specification.md)
- [Architecture](../docs/architecture.md)
- [AWS Athena Documentation](https://docs.aws.amazon.com/athena/)
- [AWS Glue Catalog Documentation](https://docs.aws.amazon.com/glue/latest/dg/catalog-and-crawler.html)

## Getting Help

If you have questions or run into issues:

1. Check the example's README file
2. Review the [main documentation](../README.md)
3. Enable debug mode: `athenadef plan --debug`
4. Open an issue on [GitHub](https://github.com/riseshia/athenadef/issues)
