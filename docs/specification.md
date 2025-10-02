# Specification

## 1. Command Line Interface

### 1.1 Global Options

All commands support these global options:

```
GLOBAL OPTIONS:
    -c, --config <FILE>      Sets the config file [default: athenadef.yaml]
    -t, --target <TABLES>    Filter target tables in <database>.<table> format (can be used multiple times)
        --debug              Enable debug logging
    -h, --help               Print help information
    -V, --version            Print version information
```

### 1.2 Target Filtering Examples

The `--target` option accepts `<database>.<table>` format and can be used multiple times:

```bash
# Target specific table
athenadef plan --target salesdb.customers

# Target multiple tables
athenadef plan --target salesdb.customers --target marketingdb.leads

# Target all tables in a database using wildcard
athenadef plan --target salesdb.*

# Target tables matching pattern
athenadef plan --target *.customers
```

## 2. Commands

### 2.1 Plan Command

Show changes required by the current configuration.

```bash
athenadef plan [OPTIONS]
```

**Options:**
```
OPTIONS:
    -c, --config <FILE>          Config file path [default: athenadef.yaml]
    -t, --target <TABLES>        Filter target tables in <database>.<table> format
        --show-unchanged         Show tables with no changes
        --debug                  Enable debug logging
    -h, --help                   Print help information
```

**Output Format:**
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
 STORED AS PARQUET
 LOCATION 's3://data-bucket/leads/'
 TBLPROPERTIES (
-    'projection.enabled' = 'false'
+    'projection.enabled' = 'true'
 );

- salesdb.old_orders
  Will destroy table
```

### 2.2 Apply Command

Apply the changes required by the current configuration.

```bash
athenadef apply [OPTIONS]
```

**Options:**
```
OPTIONS:
    -c, --config <FILE>          Config file path [default: athenadef.yaml]
    -t, --target <TABLES>        Filter target tables in <database>.<table> format
    -a, --auto-approve           Skip interactive approval
        --dry-run                Show what would be done without executing
        --debug                  Enable debug logging
    -h, --help                   Print help information
```

**Interactive Flow:**
```
Plan: 2 to add, 1 to change, 0 to destroy.

+ salesdb.new_customers
~ marketingdb.leads

Do you want to perform these actions?
  athenadef will perform the actions described above.
  Only 'yes' will be accepted to approve.

  Enter a value: yes

salesdb.new_customers: Creating...
salesdb.new_customers: Creation complete

marketingdb.leads: Modifying...
marketingdb.leads: Modification complete

Apply complete! Resources: 1 added, 1 changed, 0 destroyed.
```

### 2.3 Export Command

Export existing table definitions to local files.

```bash
athenadef export [OPTIONS]
```

**Options:**
```
OPTIONS:
    -c, --config <FILE>          Config file path [default: athenadef.yaml]
    -t, --target <TABLES>        Filter target tables in <database>.<table> format
        --overwrite              Overwrite existing files
        --debug                  Enable debug logging
    -h, --help                   Print help information
```

**Output:**
```
Exporting table definitions...

salesdb.customers: Exported to salesdb/customers.sql
salesdb.orders: Exported to salesdb/orders.sql
marketingdb.leads: Exported to marketingdb/leads.sql
marketingdb.campaigns: Exported to marketingdb/campaigns.sql

Export complete! 4 tables exported.
```

## 3. File Structure

### 3.1 Directory Layout

Schema files should be organized as:
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

### 3.2 SQL File Format

Each SQL file contains a single CREATE TABLE statement:

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

## 4. Configuration

### 4.1 YAML Configuration File

The tool expects an `athenadef.yaml` configuration file:

```yaml
# athenadef.yaml

# Required: Athena workgroup to use
workgroup: "primary"

# Optional: S3 location for query results
# If not specified, uses AWS managed storage (recommended for simplicity)
# Managed storage: automatically managed, 24-hour retention, encrypted
# S3 bucket: full control, custom retention, requires S3 permissions
output_location: "s3://athena-results-bucket/athenadef/"

# Optional: AWS region (uses default from AWS config if not specified)
region: "us-west-2"

# Optional: Query timeout in seconds (default: 300)
query_timeout_seconds: 600

# Optional: Maximum concurrent queries (default: 5)
max_concurrent_queries: 10
```

## 5. Diff Detection

### 5.1 Detection Strategy

athenadef uses a simple text-based diff approach:

1. **Table Existence**: Detect table creation and deletion by comparing local SQL files with remote tables
2. **SQL Comparison**: For existing tables, compare the normalized SQL DDL between remote (from `SHOW CREATE TABLE`) and local (from SQL files)
3. **Text Diff**: Display unified diff of SQL statements for changes

### 5.2 Diff Display Format

```
+ New addition (table will be created)
~ Change (SQL diff shown below)
- Deletion (table will be destroyed)
  (unchanged) No changes (only shown with --show-unchanged)
```

For updates (`~`), a unified diff of the SQL DDL is displayed:
- Lines starting with `-` indicate removed content (from remote)
- Lines starting with `+` indicate added content (from local)
- Lines starting with ` ` indicate unchanged content

### 5.3 Implementation Notes

- No complex schema parsing or field-by-field comparison
- Delegates all SQL validation to AWS Athena
- Simple and maintainable approach inspired by git diff
- SQL normalization may be applied for consistent comparison

## 6. Error Handling

### 6.1 Error Categories

1. **Configuration Errors**: Invalid config file, AWS authentication failures
2. **AWS Errors**: Athena API, S3 API errors (including SQL syntax errors)
3. **File System Errors**: SQL file read/write errors

### 6.2 Error Output Format

**Configuration Errors:**
```
Error: Configuration error
  ↳ Missing required field 'workgroup' in athenadef.yaml
  ↳ Hint: Add 'workgroup: "primary"' to your config

Error: Configuration error
  ↳ Invalid S3 path: 'invalid-s3-path'
  ↳ Hint: S3 paths must start with 's3://' (or omit to use managed storage)
```

**SQL Syntax Errors (from Athena):**
```
Error: Query execution failed
  ↳ SYNTAX_ERROR: line 5:23: mismatched input ')' expecting ','
  ↳ Query: CREATE EXTERNAL TABLE salesdb.customers (...)
  ↳ File: salesdb/customers.sql
  ↳ Hint: Check the CREATE TABLE syntax

Error: Query execution failed  
  ↳ INVALID_TABLE_PROPERTY: Unsupported table property 'temporary'
  ↳ Query: CREATE TEMPORARY TABLE marketingdb.leads (...)
  ↳ File: marketingdb/leads.sql
  ↳ Hint: Remove unsupported properties from CREATE TABLE statement
```

**AWS Errors:**
```
Error: AWS API error (AccessDenied)
  ↳ Insufficient permissions for operation: athena:StartQueryExecution
  ↳ Hint: Check your IAM permissions

Error: Query execution failed
  ↳ SYNTAX_ERROR: line 1:23: Table 'database.table' does not exist
  ↳ Query: DESCRIBE database.table
  ↳ Hint: The table may have been deleted outside of athenadef
```

**File System Errors:**
```
Error: File system error
  ↳ Permission denied: salesdb/customers.sql
  ↳ Hint: Check file permissions

Error: File not found
  ↳ Configuration file not found: athenadef.yaml
  ↳ Hint: Run 'athenadef init' to create a default configuration
```

**Note:** SQL parsing and validation are delegated to AWS Athena, so syntax errors appear as Athena error messages. This ensures consistency and supports all Athena features automatically.

## 7. Performance Specifications

### 7.1 Performance Goals

- 100 tables diff calculation: within 30 seconds
- Single table application: within 10 seconds  
- Export processing: 1 table/second

### 7.2 Optimization Features

- Parallel query execution (max 5 concurrent)
- Minimize unnecessary DESCRIBE TABLE calls

## 8. Security

### 8.1 Required IAM Permissions

**Minimum permissions (using managed storage):**

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
      "Resource": [
        "arn:aws:athena:*:*:workgroup/*"
      ]
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

**Additional permissions (when using S3 bucket for query results):**

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

**Note:** S3 permissions are only required when `output_location` is specified in the configuration. When using AWS managed storage (default), S3 permissions for query results are not needed.

### 8.2 Security Considerations

- Proper handling of AWS credentials
- S3 path validation  
- SQL injection prevention (parameterized queries)

## 9. Exit Codes

```
0  - Success
1  - Error
```

## 10. Logging and Debug Output

### 10.1 Standard Logging

```
INFO  Starting athenadef plan
INFO  Loading configuration from athenadef.yaml
INFO  Discovered 15 SQL files in 3 databases
INFO  Fetching current table definitions from Athena
INFO  Calculating differences...
INFO  Plan: 2 to add, 1 to change, 0 to destroy
```

### 10.2 Debug Logging (--debug)

```
DEBUG Loading AWS configuration
DEBUG Using region: us-west-2
DEBUG Athena workgroup: primary
DEBUG S3 output location: s3://results/
DEBUG Scanning directory: salesdb/
DEBUG Found SQL file: salesdb/customers.sql
DEBUG Executing query: DESCRIBE salesdb.customers
DEBUG Query execution ID: 12345678-1234-1234-1234-123456789012
DEBUG Query completed successfully
DEBUG Comparing local vs current definition
DEBUG Found difference: column type change (int -> double)
```
