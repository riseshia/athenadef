# Multi-Database Example

This example demonstrates how to manage multiple databases with multiple tables using athenadef.

## Setup

This example contains:
- Three databases: `salesdb`, `marketingdb`, and `analyticsdb`
- Multiple tables across databases

## Files

```
multi-database/
├── athenadef.yaml      # Configuration file
├── salesdb/
│   ├── customers.sql
│   ├── orders.sql
│   └── products.sql
├── marketingdb/
│   ├── campaigns.sql
│   └── leads.sql
└── analyticsdb/
    ├── user_sessions.sql
    └── page_views.sql
```

## Features Demonstrated

- **Multiple Databases**: Managing schemas across different databases
- **Target Filtering**: Using `--target` to filter by database or table
- **Organization**: How to structure a larger project

## Usage

### Apply All Changes

```bash
cd examples/multi-database
athenadef plan
athenadef apply
```

### Target Specific Database

Apply changes only to the sales database:

```bash
athenadef plan --target salesdb.*
athenadef apply --target salesdb.*
```

### Target Specific Table

Apply changes to a single table:

```bash
athenadef plan --target salesdb.customers
athenadef apply --target salesdb.customers
```

### Target Multiple Tables

Apply changes to specific tables across databases:

```bash
athenadef plan --target salesdb.customers --target marketingdb.leads
athenadef apply --target salesdb.customers --target marketingdb.leads
```

### Target Pattern Match

Apply changes to all tables named "customers" across all databases:

```bash
athenadef plan --target *.customers
athenadef apply --target *.customers
```

## Use Cases

This structure is useful for:

- **Microservices**: Each service has its own database
- **Environments**: Separate databases for different data types
- **Team Organization**: Different teams manage different databases
- **Data Lifecycle**: Separate raw, processed, and analytics databases

## Best Practices

1. **Consistent Naming**: Use consistent naming conventions across databases
2. **Documentation**: Add README files in each database directory
3. **Access Control**: Use different IAM policies for different databases
4. **Testing**: Use `--target` to test changes on a single table first

## Notes

- Update S3 location paths to match your data bucket structure
- Consider using database prefixes for different environments (e.g., `prod_salesdb`, `dev_salesdb`)
- Use `--dry-run` flag to test changes without applying them
