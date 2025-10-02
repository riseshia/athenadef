# Frequently Asked Questions (FAQ)

Common questions and answers about athenadef.

## Table of Contents

- [General Questions](#general-questions)
- [Installation and Setup](#installation-and-setup)
- [Configuration](#configuration)
- [Usage and Operations](#usage-and-operations)
- [AWS and Permissions](#aws-and-permissions)
- [Performance](#performance)
- [Troubleshooting](#troubleshooting)
- [Comparisons](#comparisons)

## General Questions

### What is athenadef?

athenadef is a command-line tool for managing AWS Athena table schemas as code. It allows you to define table schemas in SQL files, track them in version control, and deploy them to AWS Athena with a plan-apply workflow similar to Terraform.

### Why should I use athenadef?

**Benefits:**
- Version control your table schemas
- Review schema changes before applying
- Enable CI/CD for schema deployments
- Simplify schema migrations
- Maintain consistency across environments
- Easy rollback capabilities

### How does athenadef work?

1. You define tables in SQL files organized by database/table structure
2. athenadef reads your local SQL files
3. It fetches current table definitions from AWS Athena
4. It compares local vs remote and shows differences
5. You approve changes and athenadef applies them to AWS

### Is athenadef production-ready?

Yes. athenadef is designed for production use with features like:
- Preview changes before applying (plan command)
- Interactive approval process
- Detailed error messages
- Support for all Athena table features
- No destructive operations without explicit confirmation

### What's the difference between athenadef and Terraform?

| Feature | athenadef | Terraform |
|---------|-----------|-----------|
| Focus | Athena tables only | All infrastructure |
| Definition format | SQL | HCL |
| State management | AWS is source of truth | Terraform state file |
| SQL validation | By AWS Athena | Limited |
| Learning curve | Low (just SQL) | Higher |

athenadef is specialized for Athena workflows, while Terraform is general-purpose infrastructure-as-code.

### Can I use athenadef alongside Terraform/CDK?

Yes! Common patterns:
- Terraform/CDK manages S3 buckets, IAM roles, databases
- athenadef manages table definitions

This separation keeps your SQL in native format and simplifies table management.

## Installation and Setup

### How do I install athenadef?

**Homebrew (macOS/Linux):**
```bash
brew install rieshia/x/athenadef
```

**Binary download:**
Download from [GitHub releases](https://github.com/riseshia/athenadef/releases)

**From source:**
```bash
cargo install --git https://github.com/riseshia/athenadef
```

### What are the system requirements?

- Operating system: Linux, macOS, or Windows
- AWS credentials configured
- Internet connection to reach AWS APIs
- (Optional) Git for version control

### How do I verify the installation?

```bash
athenadef --version
```

### Do I need to install anything else?

No. athenadef is a standalone binary with no additional dependencies.

## Configuration

### Is a configuration file required?

No. athenadef works with default settings. However, a configuration file is recommended for:
- Custom workgroup
- Custom query timeouts
- Performance tuning

### What's the minimum configuration?

An empty configuration file or just:
```yaml
workgroup: "primary"
```

### Do I need to configure S3 output location?

No. athenadef uses AWS managed storage by default, which:
- Requires no configuration
- Requires no S3 permissions
- Is automatically managed by AWS
- Has 24-hour retention

Only configure `output_location` if you have specific requirements.

### Can I use environment variables in configuration?

Not directly in the YAML file, but you can use environment variables for AWS configuration:
- `AWS_REGION`
- `AWS_PROFILE`
- `AWS_ACCESS_KEY_ID`
- `AWS_SECRET_ACCESS_KEY`

### How do I manage multiple environments?

Create separate directories with their own configurations:
```
environments/
├── dev/
│   └── athenadef.yaml
├── staging/
│   └── athenadef.yaml
└── prod/
    └── athenadef.yaml
```

## Usage and Operations

### How do I get started with an existing Athena setup?

1. Install athenadef
2. Create `athenadef.yaml`
3. Run `athenadef export` to export existing tables
4. Commit to git
5. Make changes and use `plan`/`apply`

### What's the difference between plan and apply?

- `plan`: Shows what changes would be made (read-only, safe)
- `apply`: Actually makes the changes (requires approval)

Always run `plan` before `apply`.

### Can I apply changes without interactive approval?

Yes, use the `--auto-approve` flag:
```bash
athenadef apply --auto-approve
```

This is useful for CI/CD pipelines.

### What happens if I delete a SQL file?

By default, athenadef will show that the table should be deleted but won't actually delete it unless you confirm. The table is only removed from AWS if you approve the deletion during apply.

### Can I rename a table?

Renaming in Athena is effectively delete + create. Steps:
1. Rename the SQL file
2. Run `athenadef plan` - you'll see one deletion and one creation
3. Run `athenadef apply` to execute

Note: This will lose the table's data location unless you update the LOCATION path.

### How do I rollback a change?

```bash
# With git
git revert HEAD
athenadef apply

# Or checkout previous version
git checkout HEAD~1 salesdb/customers.sql
athenadef apply --target salesdb.customers
```

### Does athenadef modify data in S3?

No. athenadef only modifies table metadata in the AWS Glue Data Catalog. Your actual data in S3 is never touched.

## AWS and Permissions

### What AWS permissions do I need?

**Minimum (with AWS managed storage):**
- Athena: StartQueryExecution, GetQueryExecution, GetQueryResults
- Glue: GetDatabase, GetTable, CreateTable, UpdateTable, DeleteTable

See [AWS Permissions Guide](./aws-permissions.md) for detailed policies.

### Do I need S3 permissions?

Not if you use AWS managed storage (the default). Only needed if you configure a custom `output_location`.

### Can I restrict permissions to specific databases?

Yes, use resource-based IAM policies to limit access to specific databases or tables. See the [AWS Permissions Guide](./aws-permissions.md#example-iam-policies).

### Does athenadef support cross-account access?

Yes, but you need to:
1. Configure cross-account IAM roles
2. Set up appropriate trust relationships
3. Use the role in your AWS credentials

### Which AWS regions are supported?

All regions where AWS Athena is available.

## Performance

### How fast is athenadef?

Performance depends on:
- Number of tables
- AWS API response times
- Network latency
- Concurrent query settings

Typical performance:
- 10 tables: ~5-10 seconds
- 100 tables: ~30-60 seconds
- 1000 tables: ~5-10 minutes

### How can I speed up operations?

1. **Increase concurrency:**
   ```yaml
   max_concurrent_queries: 10
   ```

2. **Use target filtering:**
   ```bash
   athenadef plan --target salesdb.*
   ```

3. **Use AWS managed storage** (no S3 round-trips)

4. **Run in AWS** (lower latency to AWS APIs)

### Does athenadef cache anything?

No. athenadef always fetches the current state from AWS to ensure accuracy.

## Troubleshooting

### Why do I see changes when I haven't modified anything?

Possible reasons:
1. Someone else modified the table in AWS
2. Athena adds default values (like compression settings)
3. SQL formatting differences (athenadef normalizes these)

Run `athenadef export --overwrite` to sync with AWS current state.

### Why is my SQL syntax error not caught locally?

athenadef delegates all SQL validation to AWS Athena. This ensures:
- Support for all Athena features
- Accurate validation
- No need to maintain SQL parser

Errors are reported when Athena processes the SQL.

### The plan shows no changes but I expected some

Check:
1. Are you in the right directory?
2. Did you save the file?
3. Is target filtering excluding your table?
4. Run with `--debug` to see what's being compared

### How do I enable verbose logging?

```bash
athenadef plan --debug
```

This shows detailed information about operations.

### Where can I get help?

1. Check this FAQ
2. Read [Troubleshooting Guide](./troubleshooting.md)
3. Search [GitHub Issues](https://github.com/riseshia/athenadef/issues)
4. Open a new issue with debug output

## Comparisons

### athenadef vs manual SQL scripts?

| Feature | Manual Scripts | athenadef |
|---------|---------------|-----------|
| Preview changes | ❌ | ✅ |
| Version control | ⚠️ Manual | ✅ Built-in |
| Idempotency | ⚠️ Manual | ✅ Automatic |
| Error handling | ⚠️ Manual | ✅ Built-in |
| CI/CD ready | ⚠️ Custom | ✅ Yes |

### athenadef vs AWS Console?

| Feature | AWS Console | athenadef |
|---------|-------------|-----------|
| Version control | ❌ | ✅ |
| Code review | ❌ | ✅ |
| Automation | ❌ | ✅ |
| Audit trail | ⚠️ CloudTrail only | ✅ Git history |
| Bulk operations | ❌ | ✅ |

### athenadef vs AWS CDK?

Both are valid choices:

**Choose athenadef if:**
- You only need to manage Athena tables
- You prefer SQL over TypeScript/Python
- You want simpler workflows

**Choose CDK if:**
- You're managing many AWS resources together
- You have complex cross-resource dependencies
- Your team already uses CDK extensively

**Or use both:** CDK for infrastructure, athenadef for tables.

### athenadef vs Terraform aws_glue_catalog_table?

**athenadef advantages:**
- Native SQL format
- Simpler for Athena-only workflows
- No state file management
- Better SQL validation

**Terraform advantages:**
- Manage all resources together
- Rich ecosystem
- Cross-cloud support

## Advanced Questions

### Can I use athenadef with AWS Lake Formation?

Yes, but ensure your IAM permissions include Lake Formation permissions if tables are registered with Lake Formation.

### Does athenadef support table locking?

No. athenadef doesn't implement locking. Use external orchestration if you need to prevent concurrent modifications.

### Can I extend athenadef with custom logic?

athenadef is a CLI tool. You can:
- Wrap it in scripts
- Use it in CI/CD pipelines
- Parse its output programmatically

### Is there an API or library?

Currently athenadef is CLI-only. The code is open source if you want to build on it.

### Can I manage Athena views with athenadef?

athenadef focuses on tables (EXTERNAL TABLE). For views, use:
- Separate SQL scripts
- AWS Console
- Terraform/CDK

### Does athenadef support AWS Glue jobs?

No. athenadef only manages table metadata. Use AWS CDK or Terraform for Glue jobs.

### Can I use athenadef for Apache Iceberg tables?

Yes, if Athena supports the Iceberg features you're using. athenadef passes your SQL directly to Athena.

### Does athenadef support table statistics?

athenadef manages table definitions. For statistics, use Athena's `ANALYZE TABLE` command separately.

## Contributing and Development

### How can I contribute?

See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

### Is athenadef open source?

Yes, athenadef is open source under the MIT license.

### Can I report bugs or request features?

Yes! Open an issue on [GitHub](https://github.com/riseshia/athenadef/issues).

### How often is athenadef updated?

Check the [releases page](https://github.com/riseshia/athenadef/releases) for the latest updates.

## Still Have Questions?

If your question isn't answered here:

1. Check the [main documentation](../README.md)
2. Read the [troubleshooting guide](./troubleshooting.md)
3. Search [existing issues](https://github.com/riseshia/athenadef/issues)
4. Open a new issue with the `question` label
