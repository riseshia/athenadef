# Troubleshooting Guide

This guide helps you diagnose and resolve common issues with athenadef.

## Table of Contents

- [Getting Started](#getting-started)
- [Configuration Issues](#configuration-issues)
- [AWS Authentication and Permissions](#aws-authentication-and-permissions)
- [SQL and Schema Issues](#sql-and-schema-issues)
- [Performance Issues](#performance-issues)
- [Export Issues](#export-issues)
- [Apply and Plan Issues](#apply-and-plan-issues)
- [Debug Mode](#debug-mode)
- [Getting Help](#getting-help)

## Getting Started

### Configuration File Not Found

**Error:**
```
Error: Configuration file not found: athenadef.yaml
```

**Solutions:**
1. Create an `athenadef.yaml` file in your project root:
   ```yaml
   workgroup: "primary"
   ```

2. Specify the config file path explicitly:
   ```bash
   athenadef plan --config path/to/athenadef.yaml
   ```

3. Check your current directory:
   ```bash
   pwd
   ls -la athenadef.yaml
   ```

### Invalid Configuration Format

**Error:**
```
Error: Failed to parse configuration file
```

**Solutions:**
1. Validate your YAML syntax:
   ```bash
   # Check for syntax errors
   cat athenadef.yaml
   ```

2. Ensure proper YAML formatting (use spaces, not tabs):
   ```yaml
   # Correct
   workgroup: "primary"
   query_timeout_seconds: 300

   # Incorrect (tabs)
   workgroup:	"primary"
   ```

3. Check for supported configuration keys (see [Configuration](../README.md#configuration))

## Configuration Issues

### Workgroup Not Found

**Error:**
```
Error: Workgroup 'myworkgroup' does not exist
```

**Solutions:**
1. Verify the workgroup exists:
   ```bash
   aws athena list-work-groups
   ```

2. Check your configuration:
   ```yaml
   workgroup: "primary"  # Use correct workgroup name
   ```

3. Ensure you have permissions to use the workgroup

### Invalid S3 Output Location

**Error:**
```
Error: Invalid output location
```

**Solutions:**
1. Use AWS managed storage (recommended):
   ```yaml
   # Remove or comment out output_location
   workgroup: "primary"
   ```

2. If using custom S3 location, ensure correct format:
   ```yaml
   output_location: "s3://bucket-name/path/"  # Must end with /
   ```

3. Verify the bucket exists and you have permissions:
   ```bash
   aws s3 ls s3://bucket-name/path/
   ```

## AWS Authentication and Permissions

### AWS Credentials Not Found

**Error:**
```
Error: No credentials found
```

**Solutions:**
1. Configure AWS credentials:
   ```bash
   aws configure
   ```

2. Use environment variables:
   ```bash
   export AWS_ACCESS_KEY_ID=your-access-key
   export AWS_SECRET_ACCESS_KEY=your-secret-key
   export AWS_REGION=us-west-2
   ```

3. Use AWS profile:
   ```bash
   export AWS_PROFILE=your-profile
   athenadef plan
   ```

4. Verify credentials work:
   ```bash
   aws sts get-caller-identity
   ```

### Insufficient Permissions

**Error:**
```
Error: Access denied for operation: glue:GetTable
```

**Solutions:**
1. Verify you have the required IAM permissions (see [IAM Permissions](../README.md#iam-permissions))

2. Check specific permission errors:
   - `glue:GetTable` - Need read access to Glue catalog
   - `glue:CreateTable` - Need write access for new tables
   - `athena:StartQueryExecution` - Need Athena execution permissions
   - `s3:PutObject` - Need S3 write permissions (only if using custom output_location)

3. Test permissions:
   ```bash
   # Test Glue access
   aws glue get-databases

   # Test Athena access
   aws athena list-work-groups
   ```

4. Apply the minimum required IAM policy from the README

### Cross-Region Issues

**Error:**
```
Error: Region mismatch
```

**Solutions:**
1. Specify region in configuration:
   ```yaml
   region: "us-west-2"
   ```

2. Set AWS region environment variable:
   ```bash
   export AWS_REGION=us-west-2
   ```

3. Ensure workgroup, databases, and S3 buckets are in the same region

## SQL and Schema Issues

### SQL Syntax Errors

**Error:**
```
Error: SYNTAX_ERROR: line 1:8: mismatched input 'TABLE'
```

**Solutions:**
1. Validate SQL syntax in your `.sql` files

2. Use `CREATE EXTERNAL TABLE` (not `CREATE TABLE`):
   ```sql
   -- Correct
   CREATE EXTERNAL TABLE customers (
     id bigint,
     name string
   )
   LOCATION 's3://bucket/path/'
   ```

3. Test SQL directly in Athena console

4. Check for common mistakes:
   - Missing semicolon at end
   - Incorrect column type names
   - Invalid partition syntax
   - Missing LOCATION clause

### Table Already Exists

**Error:**
```
Error: Table already exists: salesdb.customers
```

**Solutions:**
1. Export existing table first:
   ```bash
   athenadef export --target salesdb.customers
   ```

2. Use plan to see differences:
   ```bash
   athenadef plan --target salesdb.customers
   ```

3. Delete table manually if needed (careful!):
   ```bash
   aws glue delete-table --database-name salesdb --name customers
   ```

### Schema Mismatch

**Error:**
```
Error: Column type mismatch
```

**Solutions:**
1. Athena doesn't support in-place column type changes. You must:
   - Drop and recreate the table (data loss risk)
   - Create a new table with different name
   - Use `ALTER TABLE` for supported changes only

2. Check what changes are supported:
   - Adding columns: ✅ Supported
   - Removing columns: ✅ Supported (column is hidden, not deleted)
   - Changing column type: ❌ Not supported
   - Changing partition columns: ❌ Not supported

3. For unsupported changes, consider migration strategy:
   ```bash
   # Export current definition
   athenadef export --target salesdb.customers

   # Rename table in Athena
   aws glue update-table --database-name salesdb \
     --table-input file://new-table-def.json

   # Apply new definition
   athenadef apply --target salesdb.customers
   ```

### File Not Found

**Error:**
```
Error: No SQL file found for table: salesdb.customers
```

**Solutions:**
1. Check directory structure:
   ```bash
   ls -la salesdb/customers.sql
   ```

2. Ensure correct naming:
   - Database folder: `salesdb/`
   - Table file: `customers.sql` (not `customers.txt` or other extension)

3. Verify file path matches database and table names exactly

## Performance Issues

### Slow Plan/Apply Operations

**Symptoms:**
- Commands take a long time to complete
- Many tables being processed

**Solutions:**
1. Use target filtering to limit scope:
   ```bash
   athenadef plan --target salesdb.*
   ```

2. Increase concurrent queries in config:
   ```yaml
   max_concurrent_queries: 10  # Default is 5
   ```

3. Check Athena query limits in your account

4. Use `--debug` to see which operations are slow:
   ```bash
   athenadef plan --debug
   ```

### Query Timeout

**Error:**
```
Error: Query execution timed out
```

**Solutions:**
1. Increase timeout in configuration:
   ```yaml
   query_timeout_seconds: 600  # Default is 300 (5 minutes)
   ```

2. Check Athena workgroup settings

3. Verify complex queries aren't stuck

### Rate Limiting

**Error:**
```
Error: Rate exceeded
```

**Solutions:**
1. Reduce concurrent queries:
   ```yaml
   max_concurrent_queries: 3
   ```

2. Add delay between operations

3. Check AWS Athena service quotas:
   ```bash
   aws service-quotas list-service-quotas \
     --service-code athena
   ```

## Export Issues

### Export Creates Empty Files

**Symptoms:**
- Files are created but contain no content
- Files have only comments

**Solutions:**
1. Verify tables exist in Athena:
   ```bash
   aws glue get-tables --database-name salesdb
   ```

2. Check permissions to read table metadata

3. Use debug mode to see what's happening:
   ```bash
   athenadef export --debug
   ```

### Export Fails for Specific Tables

**Error:**
```
Error: Failed to export table: salesdb.customers
```

**Solutions:**
1. Check table exists and is accessible:
   ```bash
   aws glue get-table --database-name salesdb --name customers
   ```

2. Verify you have `glue:GetTable` permission

3. Try exporting a single table:
   ```bash
   athenadef export --target salesdb.customers --debug
   ```

### Overwrite Protection

**Error:**
```
Error: File already exists: salesdb/customers.sql
```

**Solutions:**
1. Use `--overwrite` flag:
   ```bash
   athenadef export --overwrite
   ```

2. Backup existing files first:
   ```bash
   cp -r salesdb salesdb.backup
   athenadef export --overwrite
   ```

## Apply and Plan Issues

### No Changes Detected When Changes Expected

**Symptoms:**
- You modified SQL files but plan shows no changes
- Changes appear in git diff but not in athenadef plan

**Solutions:**
1. Check SQL normalization - athenadef normalizes SQL for comparison:
   - Whitespace differences are ignored
   - Comment differences are ignored
   - Case differences in keywords are ignored

2. Verify file was saved:
   ```bash
   cat salesdb/customers.sql
   ```

3. Check target filtering isn't excluding the table:
   ```bash
   athenadef plan  # No target filter
   ```

4. Use debug mode to see what's being compared:
   ```bash
   athenadef plan --debug
   ```

### Unexpected Changes Detected

**Symptoms:**
- Plan shows changes but you didn't modify anything
- Remote definition looks different from what you expect

**Solutions:**
1. Export current remote state to compare:
   ```bash
   athenadef export --target salesdb.customers --overwrite
   git diff salesdb/customers.sql
   ```

2. Check for:
   - Default values added by Athena
   - Property normalization
   - Compression codec defaults
   - Partition projection defaults

3. Update your local SQL to match remote format

### Apply Fails After Plan Succeeds

**Error:**
```
Error: Failed to apply changes
```

**Solutions:**
1. Run plan again to see current state:
   ```bash
   athenadef plan
   ```

2. Check if someone else modified tables:
   ```bash
   athenadef export --overwrite
   git diff
   ```

3. Verify permissions haven't changed

4. Check Athena service health

### Dry Run Shows Different Results

**Symptoms:**
- `apply --dry-run` shows different output than `plan`

**Note:** `apply --dry-run` and `plan` should show the same results. If they don't:

1. Report this as a bug with:
   ```bash
   athenadef plan --debug > plan.log 2>&1
   athenadef apply --dry-run --debug > apply-dry-run.log 2>&1
   ```

2. Share both log files in a GitHub issue

## Debug Mode

Enable debug logging for detailed troubleshooting:

```bash
athenadef plan --debug
```

Debug mode shows:
- Configuration loading details
- SQL file discovery
- Remote table fetching
- SQL comparison details
- Query execution details
- AWS API calls

### Saving Debug Output

```bash
# Save to file
athenadef plan --debug > debug.log 2>&1

# Filter for specific information
athenadef plan --debug 2>&1 | grep "SQL"
```

## Common Patterns and Solutions

### Testing Changes Safely

```bash
# 1. Preview changes
athenadef plan

# 2. Test with dry run
athenadef apply --dry-run

# 3. Apply to single table first
athenadef apply --target salesdb.test_table

# 4. Verify result
athenadef plan --target salesdb.test_table

# 5. Apply to database
athenadef apply --target salesdb.*
```

### Rolling Back Changes

If you applied incorrect changes:

1. **Recreate from backup:**
   ```bash
   # If you have previous SQL files
   git checkout HEAD~1 salesdb/customers.sql
   athenadef apply --target salesdb.customers
   ```

2. **Export and revert:**
   ```bash
   # Export current (wrong) state
   athenadef export --overwrite

   # Revert to previous version in git
   git checkout HEAD~1 salesdb/

   # Apply correct version
   athenadef apply --target salesdb.*
   ```

3. **Manual fix in Athena:**
   - Use AWS Console or CLI to fix the table
   - Export to get correct SQL
   - Update local files

### Handling Large Projects

For projects with many tables:

1. **Use target filtering:**
   ```bash
   # Work on one database at a time
   athenadef apply --target salesdb.*
   athenadef apply --target marketingdb.*
   ```

2. **Increase concurrency:**
   ```yaml
   max_concurrent_queries: 10
   ```

3. **Split into multiple config files:**
   ```bash
   # Different configs for different environments
   athenadef plan --config athenadef.prod.yaml
   athenadef plan --config athenadef.staging.yaml
   ```

## Environment-Specific Issues

### CI/CD Pipeline Failures

**Common issues in CI/CD:**

1. **Credentials not configured:**
   ```yaml
   # GitHub Actions
   env:
     AWS_ACCESS_KEY_ID: ${{ secrets.AWS_ACCESS_KEY_ID }}
     AWS_SECRET_ACCESS_KEY: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
     AWS_REGION: us-west-2
   ```

2. **Binary not found:**
   ```yaml
   # Ensure athenadef is installed
   - uses: riseshia/athenadef@v0
     with:
       version: latest
   ```

3. **Auto-approve needed:**
   ```bash
   athenadef apply --auto-approve
   ```

### Docker Container Issues

If running in Docker:

1. **AWS credentials:**
   ```bash
   docker run -v ~/.aws:/root/.aws \
     -v $(pwd):/workspace \
     athenadef plan
   ```

2. **Working directory:**
   ```bash
   docker run -w /workspace \
     -v $(pwd):/workspace \
     athenadef plan
   ```

### Windows-Specific Issues

**Line ending issues:**
```bash
# Convert to Unix line endings
dos2unix salesdb/*.sql

# Or configure git
git config --global core.autocrlf true
```

## Getting Help

If you're still stuck:

1. **Search existing issues:**
   - [GitHub Issues](https://github.com/riseshia/athenadef/issues)

2. **Collect debug information:**
   ```bash
   athenadef --version
   athenadef plan --debug > debug.log 2>&1
   ```

3. **Create a minimal reproduction:**
   - Single table that demonstrates the issue
   - Anonymized SQL if needed
   - Steps to reproduce

4. **Open an issue:**
   - Include version, OS, error message
   - Attach debug log (remove sensitive info)
   - Describe expected vs actual behavior

5. **Check AWS service health:**
   - [AWS Health Dashboard](https://health.aws.amazon.com/health/status)

## Additional Resources

- [Main Documentation](../README.md)
- [Configuration Reference](../README.md#configuration)
- [IAM Permissions](../README.md#iam-permissions)
- [Examples](../examples/)
- [AWS Athena Troubleshooting](https://docs.aws.amazon.com/athena/latest/ug/troubleshooting-athena.html)
- [AWS Glue Troubleshooting](https://docs.aws.amazon.com/glue/latest/dg/troubleshooting.html)
