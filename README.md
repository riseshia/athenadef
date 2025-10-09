# athenadef

Schema management for AWS Athena - a CLI tool for managing Athena table schemas as code.

## Features

- **Infrastructure as Code**: Manage Athena table definitions using SQL files in a Git-friendly directory structure
- **Change Preview**: See exactly what will change before applying (similar to Terraform plan)
- **Safe Deployments**: Interactive approval with detailed diff display
- **Export Capability**: Export existing tables to local SQL files
- **Target Filtering**: Apply changes to specific tables or databases using flexible patterns
- **AWS Managed Storage**: Works without S3 bucket configuration (uses AWS managed storage by default)
- **Parallel Execution**: Fast operations with concurrent query execution
- **CI/CD Ready**: GitHub Action available for automated deployments

## Installation

### Homebrew

```bash
brew install rieshia/x/athenadef
```

### Binary Download

Download pre-compiled binaries from the [release page](https://github.com/riseshia/athenadef/releases).

### From Source

```bash
cargo install --git https://github.com/riseshia/athenadef
```

### GitHub Action

```yaml
jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: riseshia/athenadef@v0
        with:
          version: v0.1.0 # or latest
```

## Quick Start

1. **Export existing tables** (optional, to get started with existing infrastructure):

```bash
athenadef export
```

This creates SQL files for all your existing tables.

2. **Review changes**:

```bash
athenadef plan
```

3. **Apply changes**:

```bash
athenadef apply
```

## Commands

### Global Options

Available for all commands:

```
-c, --config <FILE>      Config file path [default: athenadef.yaml]
-t, --target <TABLES>    Filter tables using <database>.<table> format
    --debug              Enable debug logging
-h, --help               Print help information
-V, --version            Print version information
```

### `plan` - Preview Changes

Show what changes will be made to match your local configuration:

```bash
athenadef plan [OPTIONS]
```

**Options:**
- `--show-unchanged`: Show tables with no changes

**Example output:**
```
Plan: 2 to add, 1 to change, 0 to destroy.

+ salesdb.new_customers
  Will create table

~ marketingdb.leads
  Will update table
--- remote: marketingdb.leads
+++ local:  marketingdb.leads
 CREATE EXTERNAL TABLE leads (
-    score int,
+    score double,
+    created_at timestamp,
     email string
 )
```

### `apply` - Apply Changes

Apply the changes to make your Athena tables match your local configuration:

```bash
athenadef apply [OPTIONS]
```

**Options:**
- `-a, --auto-approve`: Skip interactive approval
- `--dry-run`: Show what would be done without executing

**Example output:**
```
Plan: 2 to add, 1 to change, 0 to destroy.

Do you want to perform these actions? (yes/no): yes

salesdb.new_customers: Creating...
salesdb.new_customers: Creation complete

marketingdb.leads: Modifying...
marketingdb.leads: Modification complete

Apply complete! Resources: 1 added, 1 changed, 0 destroyed.
```

### `export` - Export Table Definitions

Export existing Athena table definitions to local SQL files:

```bash
athenadef export [OPTIONS]
```

**Options:**
- `--overwrite`: Overwrite existing files

**Example output:**
```
Exporting table definitions...

salesdb.customers: Exported to salesdb/customers.sql
salesdb.orders: Exported to salesdb/orders.sql

Export complete! 2 tables exported.
```

### Target Filtering

Use `--target` to filter operations to specific tables or databases:

```bash
# Specific table
athenadef plan --target salesdb.customers

# Multiple tables
athenadef plan --target salesdb.customers --target marketingdb.leads

# All tables in a database
athenadef plan --target salesdb.*

# Tables with same name across databases
athenadef plan --target *.customers
```

## Configuration

### Directory Structure

Organize your SQL files in a directory structure that mirrors your databases and tables:

```
project-root/
├── athenadef.yaml        # Configuration file
├── salesdb/             # Database name
│   ├── customers.sql    # Table definition
│   └── orders.sql
└── marketingdb/
    ├── leads.sql
    └── campaigns.sql
```

### SQL Files

Each `.sql` file should contain a complete `CREATE EXTERNAL TABLE` statement:

```sql
-- customers.sql
CREATE EXTERNAL TABLE customers (
    customer_id bigint,
    name string,
    email string COMMENT 'Customer email address',
    registration_date date
)
PARTITIONED BY (
    year string,
    month string
)
STORED AS PARQUET
LOCATION 's3://your-data-bucket/customers/'
TBLPROPERTIES (
    'projection.enabled' = 'true',
    'projection.year.type' = 'integer',
    'projection.year.range' = '2020,2030',
    'projection.month.type' = 'integer',
    'projection.month.range' = '1,12',
    'projection.month.digits' = '2'
);
```

### Configuration File

Create an `athenadef.yaml` file in your project root:

```yaml
# athenadef.yaml

# Optional: Athena workgroup (default: "primary")
workgroup: "primary"

# Optional: S3 location for query results
# If not specified, uses workgroup's default output location (recommended)
# output_location: "s3://athena-results-bucket/athenadef/"

# Optional: List of databases to manage
# If specified and --target is not provided, only these databases will be processed
# This is useful to avoid scanning all databases in your account
# databases:
#   - salesdb
#   - marketingdb

# Optional: AWS region (uses default from AWS config if not specified)
# region: "us-west-2"

# Optional: Query timeout in seconds (default: 300)
# query_timeout_seconds: 600

# Optional: Maximum concurrent queries (default: 5)
# max_concurrent_queries: 10
```

## Examples

See the [examples](./examples) directory for complete sample projects:

- [examples/basic](./examples/basic) - Simple setup with a few tables
- [examples/partitioned](./examples/partitioned) - Tables with partitions and partition projection
- [examples/multi-database](./examples/multi-database) - Multiple databases with many tables

## IAM Permissions

### Minimum Permissions (with AWS Managed Storage)

When using AWS managed storage (default, no `output_location` specified), you only need these permissions:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "athena:StartQueryExecution",
        "athena:GetQueryExecution",
        "athena:GetQueryResults",
        "athena:StopQueryExecution"
      ],
      "Resource": "arn:aws:athena:*:*:workgroup/*"
    },
    {
      "Effect": "Allow",
      "Action": [
        "glue:GetDatabase",
        "glue:GetDatabases",
        "glue:GetTable",
        "glue:GetTables",
        "glue:CreateTable",
        "glue:UpdateTable",
        "glue:DeleteTable"
      ],
      "Resource": "*"
    }
  ]
}
```

### Additional S3 Permissions

Only required when specifying `output_location` in your configuration:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "s3:GetBucketLocation",
        "s3:GetObject",
        "s3:ListBucket",
        "s3:PutObject"
      ],
      "Resource": [
        "arn:aws:s3:::your-query-results-bucket",
        "arn:aws:s3:::your-query-results-bucket/*"
      ]
    }
  ]
}
```

**References:**
- [Amazon Athena IAM Actions](https://docs.aws.amazon.com/service-authorization/latest/reference/list_amazonathena.html)
- [AWS Glue IAM Actions](https://docs.aws.amazon.com/service-authorization/latest/reference/list_awsglue.html)
- [Amazon S3 IAM Actions](https://docs.aws.amazon.com/service-authorization/latest/reference/list_amazons3.html)

## How It Works

athenadef uses a simple but effective approach:

1. **Reads local SQL files** organized in a `database/table.sql` structure
2. **Fetches current state** from AWS Athena using `SHOW CREATE TABLE`
3. **Compares definitions** using text-based diff (like git diff)
4. **Delegates SQL validation** to AWS Athena (no local parsing)
5. **Applies changes** by executing DDL statements through Athena

This design ensures:
- **Simplicity**: No complex SQL parsing
- **Compatibility**: Supports all Athena features automatically
- **Reliability**: SQL validation by AWS Athena itself

## Troubleshooting

### Common Issues

**Configuration file not found:**
```bash
# Specify config file explicitly
athenadef plan --config path/to/athenadef.yaml
```

**AWS authentication errors:**
```bash
# Check AWS credentials
aws sts get-caller-identity

# Set AWS profile
export AWS_PROFILE=your-profile
```

**SQL syntax errors:**
SQL errors are reported by Athena and include the file name and query that failed. Check the SQL syntax in your `.sql` files.

### Debug Mode

Enable debug logging to see detailed execution information:

```bash
athenadef plan --debug
```

For more detailed troubleshooting help, see the [Troubleshooting Guide](docs/troubleshooting.md).

## Documentation

### Guides

- **[Troubleshooting Guide](docs/troubleshooting.md)** - Detailed solutions for common issues
- **[AWS Permissions Guide](docs/aws-permissions.md)** - Complete IAM permissions reference and examples
- **[Migration Guide](docs/migration-guide.md)** - Migrate from Terraform, CDK, CloudFormation, or manual management
- **[Advanced Usage](docs/advanced-usage.md)** - Advanced patterns, CI/CD integration, multi-environment setup
- **[FAQ](docs/faq.md)** - Frequently asked questions

### Technical Documentation

- **[Architecture](docs/architecture.md)** - System architecture and design
- **[Specification](docs/specification.md)** - Complete technical specification
- **[Technical Design](docs/technical-design.md)** - Core algorithms and implementation details
- **[JSON Output](docs/json-output.md)** - JSON output format specification

### Examples

- **[Basic Example](examples/basic/)** - Simple setup with a few tables
- **[Partitioned Tables](examples/partitioned/)** - Partition projection and time-series data
- **[Multi-Database](examples/multi-database/)** - Managing multiple databases

## Contributing

Contributions are welcome! Please see our [contributing guidelines](CONTRIBUTING.md) for details.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

This project includes software developed by:
- aws-sdk-config: Licensed under the Apache License, Version 2.0
- aws-sdk-athena: Licensed under the Apache License, Version 2.0
- aws-sdk-s3: Licensed under the Apache License, Version 2.0
