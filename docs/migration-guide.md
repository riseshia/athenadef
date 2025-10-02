# Migration Guide

This guide helps you migrate to athenadef from other schema management tools and approaches.

## Table of Contents

- [Migrating from Manual Management](#migrating-from-manual-management)
- [Migrating from Terraform](#migrating-from-terraform)
- [Migrating from AWS CDK](#migrating-from-aws-cdk)
- [Migrating from CloudFormation](#migrating-from-cloudformation)
- [Migrating from Custom Scripts](#migrating-from-custom-scripts)
- [Migration Best Practices](#migration-best-practices)

## Migrating from Manual Management

If you currently manage Athena tables manually through the AWS Console or CLI, here's how to migrate to athenadef.

### Current State

You're managing tables through:
- AWS Athena Console
- AWS CLI commands
- Manual SQL queries
- No version control for table definitions

### Migration Steps

**1. Set up athenadef:**

```bash
# Install athenadef
brew install rieshia/x/athenadef

# Or download from releases
# https://github.com/riseshia/athenadef/releases
```

**2. Create project structure:**

```bash
mkdir my-athena-project
cd my-athena-project

# Initialize git
git init

# Create configuration
cat > athenadef.yaml << EOF
workgroup: "primary"
EOF
```

**3. Export existing tables:**

```bash
# Export all tables
athenadef export

# Or export specific databases
athenadef export --target salesdb.*
athenadef export --target analyticsdb.*
```

This creates SQL files in the structure:
```
salesdb/
  customers.sql
  orders.sql
analyticsdb/
  events.sql
  sessions.sql
```

**4. Commit to version control:**

```bash
git add .
git commit -m "Initial import of Athena table definitions"
```

**5. Test the setup:**

```bash
# Verify no changes needed
athenadef plan

# Should show: "No changes. Your infrastructure matches the configuration."
```

**6. Make your first change:**

```bash
# Edit a table definition
vim salesdb/customers.sql

# Preview changes
athenadef plan

# Apply changes
athenadef apply
```

### Benefits After Migration

- ✅ Version control for all table definitions
- ✅ Code review process for schema changes
- ✅ Automated deployments via CI/CD
- ✅ Audit trail of all changes
- ✅ Easy rollback capabilities

## Migrating from Terraform

If you're currently using Terraform with `aws_glue_catalog_table` resources, here's how to migrate.

### Current State (Terraform)

```hcl
# terraform/athena.tf
resource "aws_glue_catalog_table" "customers" {
  name          = "customers"
  database_name = "salesdb"

  storage_descriptor {
    location      = "s3://my-data/customers/"
    input_format  = "org.apache.hadoop.hive.ql.io.parquet.MapredParquetInputFormat"
    output_format = "org.apache.hadoop.hive.ql.io.parquet.MapredParquetOutputFormat"

    ser_de_info {
      serialization_library = "org.apache.hadoop.hive.ql.io.parquet.serde.ParquetHiveSerDe"
    }

    columns {
      name = "customer_id"
      type = "bigint"
    }

    columns {
      name = "name"
      type = "string"
    }
  }
}
```

### Migration Steps

**1. Export tables to athenadef format:**

```bash
# Set up athenadef
mkdir athena-definitions
cd athena-definitions

cat > athenadef.yaml << EOF
workgroup: "primary"
EOF

# Export existing tables
athenadef export
```

**2. Remove Terraform resources:**

```bash
# Remove from Terraform state (doesn't delete resources)
terraform state rm aws_glue_catalog_table.customers
terraform state rm aws_glue_catalog_table.orders

# Or if managing in separate state:
# Remove the entire state file after backing it up
cp terraform.tfstate terraform.tfstate.backup
```

**3. Delete Terraform files:**

```bash
# Remove Terraform Athena definitions
rm terraform/athena.tf

# Or comment them out if you want to keep them for reference
```

**4. Set up CI/CD for athenadef:**

```yaml
# .github/workflows/athena.yml
name: Athena Schema Management
on:
  push:
    branches: [main]
    paths:
      - 'athena-definitions/**'

jobs:
  plan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: riseshia/athenadef@v0
      - name: Plan changes
        run: |
          cd athena-definitions
          athenadef plan

  apply:
    needs: plan
    runs-on: ubuntu-latest
    environment: production
    steps:
      - uses: actions/checkout@v5
      - uses: riseshia/athenadef@v0
      - name: Apply changes
        run: |
          cd athena-definitions
          athenadef apply --auto-approve
```

### Comparison

| Feature | Terraform | athenadef |
|---------|-----------|-----------|
| Table definition format | HCL | SQL (native) |
| Preview changes | `terraform plan` | `athenadef plan` |
| Apply changes | `terraform apply` | `athenadef apply` |
| State management | Terraform state | AWS Athena (source of truth) |
| SQL validation | Limited | Full (by AWS) |
| Learning curve | Terraform + AWS provider | Just SQL |
| File size | ~30-50 lines per table | ~10-20 lines per table |

### Why Migrate?

**Advantages of athenadef:**
- ✅ Native SQL format (copy-paste from Athena console)
- ✅ No state file management
- ✅ Simpler for Athena-specific workflows
- ✅ Better diffs (text-based SQL)
- ✅ Direct compatibility with Athena features

**Keep Terraform if:**
- ❌ You need to manage other AWS resources alongside tables
- ❌ You have complex cross-resource dependencies
- ❌ Your organization standardizes on Terraform for all infrastructure

## Migrating from AWS CDK

If you're using AWS CDK with `@aws-cdk/aws-glue` constructs, here's the migration path.

### Current State (CDK)

```typescript
// lib/athena-stack.ts
import * as glue from '@aws-cdk/aws-glue';
import * as cdk from '@aws-cdk/core';

export class AthenaStack extends cdk.Stack {
  constructor(scope: cdk.Construct, id: string) {
    super(scope, id);

    new glue.CfnTable(this, 'CustomersTable', {
      databaseName: 'salesdb',
      catalogId: this.account,
      tableInput: {
        name: 'customers',
        storageDescriptor: {
          columns: [
            { name: 'customer_id', type: 'bigint' },
            { name: 'name', type: 'string' },
          ],
          location: 's3://my-data/customers/',
          inputFormat: 'org.apache.hadoop.hive.ql.io.parquet.MapredParquetInputFormat',
          outputFormat: 'org.apache.hadoop.hive.ql.io.parquet.MapredParquetOutputFormat',
          serdeInfo: {
            serializationLibrary: 'org.apache.hadoop.hive.ql.io.parquet.serde.ParquetHiveSerDe',
          },
        },
      },
    });
  }
}
```

### Migration Steps

**1. Export existing tables:**

```bash
mkdir athena-definitions
cd athena-definitions

cat > athenadef.yaml << EOF
workgroup: "primary"
EOF

athenadef export
```

**2. Remove CDK resources:**

```typescript
// Comment out or remove Glue table definitions
// Keep other resources (S3 buckets, IAM roles, etc.)
```

**3. Deploy CDK changes:**

```bash
# Remove tables from CDK state
cdk destroy --force AthenaStack/CustomersTable
```

**4. Adopt athenadef:**

```bash
# Verify exported tables match
athenadef plan

# Should show no changes
```

### Hybrid Approach

You can use both CDK and athenadef together:

**CDK manages:**
- S3 buckets for data storage
- IAM roles and policies
- Athena workgroups
- Other AWS resources

**athenadef manages:**
- Athena table definitions
- Schema migrations
- Table structure changes

```typescript
// lib/athena-infrastructure-stack.ts
export class AthenaInfraStack extends cdk.Stack {
  constructor(scope: cdk.Construct, id: string) {
    super(scope, id);

    // CDK manages infrastructure
    const dataBucket = new s3.Bucket(this, 'DataBucket', {
      bucketName: 'my-data-bucket',
    });

    const athenaResultsBucket = new s3.Bucket(this, 'AthenaResultsBucket', {
      bucketName: 'my-athena-results',
    });

    new glue.CfnDatabase(this, 'SalesDB', {
      catalogId: this.account,
      databaseInput: {
        name: 'salesdb',
      },
    });

    // athenadef manages tables
    // (table definitions in athena-definitions/ directory)
  }
}
```

## Migrating from CloudFormation

If you're using CloudFormation with `AWS::Glue::Table` resources.

### Current State (CloudFormation)

```yaml
# cloudformation/athena-tables.yaml
Resources:
  CustomersTable:
    Type: AWS::Glue::Table
    Properties:
      DatabaseName: salesdb
      CatalogId: !Ref AWS::AccountId
      TableInput:
        Name: customers
        StorageDescriptor:
          Columns:
            - Name: customer_id
              Type: bigint
            - Name: name
              Type: string
          Location: s3://my-data/customers/
          InputFormat: org.apache.hadoop.hive.ql.io.parquet.MapredParquetInputFormat
          OutputFormat: org.apache.hadoop.hive.ql.io.parquet.MapredParquetOutputFormat
          SerdeInfo:
            SerializationLibrary: org.apache.hadoop.hive.ql.io.parquet.serde.ParquetHiveSerDe
```

### Migration Steps

**1. Export to athenadef:**

```bash
mkdir athena-definitions
cd athena-definitions

cat > athenadef.yaml << EOF
workgroup: "primary"
EOF

athenadef export
```

**2. Remove from CloudFormation:**

```bash
# Remove resources from stack
aws cloudformation update-stack \
  --stack-name athena-tables \
  --template-body file://cloudformation/athena-tables-empty.yaml \
  --retain-resources CustomersTable,OrdersTable
```

**3. Verify migration:**

```bash
cd athena-definitions
athenadef plan

# Should show no changes
```

## Migrating from Custom Scripts

If you have custom Python/Bash scripts that manage tables.

### Current State (Example)

```python
# scripts/create_tables.py
import boto3

glue = boto3.client('glue')

glue.create_table(
    DatabaseName='salesdb',
    TableInput={
        'Name': 'customers',
        'StorageDescriptor': {
            'Columns': [
                {'Name': 'customer_id', 'Type': 'bigint'},
                {'Name': 'name', 'Type': 'string'},
            ],
            'Location': 's3://my-data/customers/',
            'InputFormat': 'org.apache.hadoop.hive.ql.io.parquet.MapredParquetInputFormat',
            'OutputFormat': 'org.apache.hadoop.hive.ql.io.parquet.MapredParquetOutputFormat',
            'SerdeInfo': {
                'SerializationLibrary': 'org.apache.hadoop.hive.ql.io.parquet.serde.ParquetHiveSerDe'
            }
        }
    }
)
```

### Migration Steps

**1. Document current state:**

```bash
# Run existing scripts to create tables
python scripts/create_tables.py
```

**2. Export to athenadef:**

```bash
mkdir athena-definitions
cd athena-definitions

cat > athenadef.yaml << EOF
workgroup: "primary"
EOF

athenadef export
```

**3. Compare definitions:**

```bash
# Check exported SQL matches what scripts create
cat salesdb/customers.sql

# Verify with plan
athenadef plan
```

**4. Replace scripts:**

```bash
# Backup old scripts
mkdir scripts/legacy
mv scripts/*.py scripts/legacy/

# Use athenadef going forward
```

**5. Update documentation:**

```bash
# Update README with new process
cat >> README.md << EOF

## Table Management

Tables are managed using athenadef.

### Making changes:
1. Edit SQL files in athena-definitions/
2. Run: athenadef plan
3. Run: athenadef apply

### Exporting tables:
athenadef export --target salesdb.*
EOF
```

## Migration Best Practices

### 1. Start with Export

Always start by exporting existing tables:

```bash
athenadef export --overwrite
```

This ensures you capture the current state accurately.

### 2. Test in Non-Production First

```bash
# Export production tables
athenadef export --target production_*

# Test in staging
cp -r production_* staging_
# Edit staging tables as needed
athenadef apply --target staging_* --config staging.yaml
```

### 3. Use Version Control

```bash
git init
git add athenadef.yaml salesdb/ analyticsdb/
git commit -m "Initial migration to athenadef"
```

### 4. Gradual Migration

Don't migrate everything at once:

```bash
# Week 1: Export and verify
athenadef export
git add . && git commit -m "Export existing tables"

# Week 2: Migrate read-only tables
athenadef apply --target staging.*

# Week 3: Migrate low-risk tables
athenadef apply --target dev.*

# Week 4: Migrate production tables
athenadef apply --target prod.*
```

### 5. Keep Old Tool Temporarily

Keep your old infrastructure-as-code tool running alongside athenadef initially:

```bash
# Both can coexist
terraform plan  # Shows no changes
athenadef plan  # Shows no changes
```

After validating, remove the old tool.

### 6. Document the Change

Update team documentation:

```markdown
# Schema Management

**Old process:** Edit Terraform files
**New process:** Edit SQL files in athena-definitions/

Commands:
- Preview: `athenadef plan`
- Apply: `athenadef apply`
- Export: `athenadef export`
```

### 7. Train the Team

Hold a training session covering:
- Why we're migrating
- New workflow (edit SQL → plan → apply)
- How to handle rollbacks
- CI/CD integration

### 8. Monitor After Migration

```bash
# Check for drift
athenadef plan --debug

# Monitor CloudTrail for manual changes
aws cloudtrail lookup-events \
  --lookup-attributes AttributeKey=EventName,AttributeValue=UpdateTable
```

## Comparison Matrix

| Feature | Manual | Terraform | CDK | CloudFormation | Custom Scripts | athenadef |
|---------|--------|-----------|-----|----------------|----------------|-----------|
| Version Control | ❌ | ✅ | ✅ | ✅ | ⚠️  | ✅ |
| Native SQL | ✅ | ❌ | ❌ | ❌ | ⚠️  | ✅ |
| Preview Changes | ❌ | ✅ | ✅ | ✅ | ❌ | ✅ |
| Learning Curve | Low | High | High | Medium | Medium | Low |
| State Management | N/A | Complex | Complex | AWS | Custom | Simple |
| Multi-resource | N/A | ✅ | ✅ | ✅ | ✅ | ❌ |
| Athena-focused | ✅ | ❌ | ❌ | ❌ | ⚠️  | ✅ |

## Rollback Strategies

If you need to rollback after migration:

### Rollback with Git

```bash
# Revert to previous version
git revert HEAD
athenadef apply

# Or restore specific table
git checkout HEAD~1 salesdb/customers.sql
athenadef apply --target salesdb.customers
```

### Rollback with Export

```bash
# Export current state before making changes
athenadef export --overwrite
git commit -m "Backup before changes"

# If something goes wrong
git checkout HEAD~1
athenadef apply
```

### Emergency Rollback

```bash
# Manually recreate table in AWS Console
# Then export to get correct definition
athenadef export --target salesdb.customers --overwrite
git commit -m "Fix after manual intervention"
```

## Getting Help with Migration

If you encounter issues during migration:

1. **Check the troubleshooting guide:** [docs/troubleshooting.md](./troubleshooting.md)
2. **Enable debug mode:** `athenadef plan --debug`
3. **Compare exports:** Run export before and after migration
4. **Ask for help:** [GitHub Issues](https://github.com/riseshia/athenadef/issues)

## Additional Resources

- [Main Documentation](../README.md)
- [Examples](../examples/)
- [Troubleshooting Guide](./troubleshooting.md)
- [AWS Permissions Guide](./aws-permissions.md)
