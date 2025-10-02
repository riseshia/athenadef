# AWS Permissions Guide

This document provides detailed information about AWS IAM permissions required for athenadef.

## Table of Contents

- [Overview](#overview)
- [Minimum Permissions](#minimum-permissions)
- [Permission Breakdown by Operation](#permission-breakdown-by-operation)
- [Example IAM Policies](#example-iam-policies)
- [Security Best Practices](#security-best-practices)
- [Troubleshooting Permissions](#troubleshooting-permissions)

## Overview

athenadef requires permissions to interact with three main AWS services:

1. **AWS Athena** - Execute queries and manage query lifecycle
2. **AWS Glue Data Catalog** - Read and modify table definitions
3. **Amazon S3** - Store query results (optional, only when using custom output location)

The specific permissions needed depend on:
- Which athenadef commands you use
- Whether you use AWS managed storage or custom S3 location
- Your organization's security requirements

## Minimum Permissions

### With AWS Managed Storage (Recommended)

When using AWS managed storage (the default, no `output_location` configured), you only need Athena and Glue permissions:

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

**Benefits of AWS Managed Storage:**
- No S3 bucket configuration needed
- No S3 permissions required
- Automatic encryption
- 24-hour retention
- Automatic cleanup

### With Custom S3 Output Location

If you specify `output_location` in your configuration, add these S3 permissions:

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

## Permission Breakdown by Operation

### All Commands (Required)

These permissions are required for all athenadef operations:

**Athena:**
- `athena:GetQueryExecution` - Check query status
- `athena:GetQueryResults` - Retrieve query results
- `athena:StartQueryExecution` - Execute SHOW CREATE TABLE queries

**Glue:**
- `glue:GetDatabase` - Verify database exists
- `glue:GetDatabases` - List available databases
- `glue:GetTable` - Fetch current table definitions
- `glue:GetTables` - List tables in a database

### `plan` Command

**Additional permissions:** None (uses only base permissions)

The plan command is read-only and doesn't modify anything.

### `export` Command

**Additional permissions:** None (uses only base permissions)

The export command is read-only and fetches table definitions from Glue.

### `apply` Command

**Additional permissions:**

**Glue:**
- `glue:CreateTable` - Create new tables
- `glue:UpdateTable` - Modify existing tables
- `glue:DeleteTable` - Remove tables (only when explicitly removing SQL files)

**Athena:**
- `athena:StopQueryExecution` - Cancel running queries (for cleanup)

## Example IAM Policies

### 1. Read-Only Access

For users who should only run `plan` and `export`:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "AthenaReadOnly",
      "Effect": "Allow",
      "Action": [
        "athena:StartQueryExecution",
        "athena:GetQueryExecution",
        "athena:GetQueryResults"
      ],
      "Resource": "arn:aws:athena:*:*:workgroup/primary"
    },
    {
      "Sid": "GlueReadOnly",
      "Effect": "Allow",
      "Action": [
        "glue:GetDatabase",
        "glue:GetDatabases",
        "glue:GetTable",
        "glue:GetTables"
      ],
      "Resource": "*"
    }
  ]
}
```

### 2. Full Access (All Operations)

For users who need to run all commands including `apply`:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "AthenaFullAccess",
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
      "Sid": "GlueFullAccess",
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

### 3. Scoped to Specific Databases

Limit access to specific databases only:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "AthenaAccess",
      "Effect": "Allow",
      "Action": [
        "athena:StartQueryExecution",
        "athena:GetQueryExecution",
        "athena:GetQueryResults",
        "athena:StopQueryExecution"
      ],
      "Resource": "arn:aws:athena:us-west-2:123456789012:workgroup/primary"
    },
    {
      "Sid": "GlueDatabaseAccess",
      "Effect": "Allow",
      "Action": [
        "glue:GetDatabase"
      ],
      "Resource": [
        "arn:aws:glue:us-west-2:123456789012:catalog",
        "arn:aws:glue:us-west-2:123456789012:database/salesdb",
        "arn:aws:glue:us-west-2:123456789012:database/marketingdb"
      ]
    },
    {
      "Sid": "GlueTableAccess",
      "Effect": "Allow",
      "Action": [
        "glue:GetTable",
        "glue:GetTables",
        "glue:CreateTable",
        "glue:UpdateTable",
        "glue:DeleteTable"
      ],
      "Resource": [
        "arn:aws:glue:us-west-2:123456789012:catalog",
        "arn:aws:glue:us-west-2:123456789012:database/salesdb",
        "arn:aws:glue:us-west-2:123456789012:table/salesdb/*",
        "arn:aws:glue:us-west-2:123456789012:database/marketingdb",
        "arn:aws:glue:us-west-2:123456789012:table/marketingdb/*"
      ]
    }
  ]
}
```

### 4. Scoped to Specific Workgroup

Limit access to a specific Athena workgroup:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "AthenaWorkgroupAccess",
      "Effect": "Allow",
      "Action": [
        "athena:StartQueryExecution",
        "athena:GetQueryExecution",
        "athena:GetQueryResults",
        "athena:StopQueryExecution"
      ],
      "Resource": "arn:aws:athena:us-west-2:123456789012:workgroup/athenadef-prod"
    },
    {
      "Sid": "GlueAccess",
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

### 5. CI/CD Pipeline (GitHub Actions)

Minimal permissions for automated deployments:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "CICDAthenaAccess",
      "Effect": "Allow",
      "Action": [
        "athena:StartQueryExecution",
        "athena:GetQueryExecution",
        "athena:GetQueryResults"
      ],
      "Resource": "arn:aws:athena:*:*:workgroup/primary"
    },
    {
      "Sid": "CICDGlueAccess",
      "Effect": "Allow",
      "Action": [
        "glue:GetDatabase",
        "glue:GetDatabases",
        "glue:GetTable",
        "glue:GetTables",
        "glue:CreateTable",
        "glue:UpdateTable"
      ],
      "Resource": "*"
    }
  ]
}
```

Note: This policy omits `glue:DeleteTable` for safety in CI/CD environments.

### 6. With Custom S3 Output Location

When using `output_location` in configuration:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "AthenaAccess",
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
      "Sid": "GlueAccess",
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
    },
    {
      "Sid": "S3QueryResultsAccess",
      "Effect": "Allow",
      "Action": [
        "s3:GetBucketLocation",
        "s3:GetObject",
        "s3:ListBucket",
        "s3:PutObject"
      ],
      "Resource": [
        "arn:aws:s3:::athena-results-bucket",
        "arn:aws:s3:::athena-results-bucket/*"
      ]
    }
  ]
}
```

## Security Best Practices

### 1. Use AWS Managed Storage

AWS managed storage is recommended because:
- Eliminates need for S3 permissions
- Reduces attack surface
- Automatic encryption and retention
- No bucket configuration needed

```yaml
# athenadef.yaml
workgroup: "primary"
# No output_location specified - uses managed storage
```

### 2. Principle of Least Privilege

Grant only the minimum permissions needed:

**For development:**
- Read-only policy for experimentation
- Scoped to test databases only

**For production:**
- Separate IAM roles for plan vs apply
- Require approval for apply operations
- Use specific database/table ARNs

### 3. Use IAM Roles (Not Users)

**For EC2/ECS:**
```bash
# Attach IAM role to instance
# No credentials in environment variables
```

**For GitHub Actions:**
```yaml
- name: Configure AWS Credentials
  uses: aws-actions/configure-aws-credentials@v4
  with:
    role-to-assume: arn:aws:iam::123456789012:role/athenadef-github-actions
    aws-region: us-west-2
```

### 4. Separate Workgroups

Use different workgroups for different environments:

```yaml
# Production
workgroup: "athenadef-prod"

# Staging
workgroup: "athenadef-staging"
```

Then use workgroup-specific IAM policies to control access.

### 5. Enable CloudTrail Logging

Monitor athenadef operations:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "cloudtrail:LookupEvents"
      ],
      "Resource": "*"
    }
  ]
}
```

Monitor for:
- `glue:CreateTable` - New tables
- `glue:UpdateTable` - Modified tables
- `glue:DeleteTable` - Deleted tables
- `athena:StartQueryExecution` - Query execution

### 6. Resource Tagging

Tag resources managed by athenadef:

```sql
-- In your SQL files
CREATE EXTERNAL TABLE customers (
  id bigint,
  name string
)
LOCATION 's3://bucket/customers/'
TBLPROPERTIES (
  'managed_by' = 'athenadef',
  'environment' = 'production'
);
```

Then use tag-based IAM policies:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "glue:CreateTable",
        "glue:UpdateTable",
        "glue:DeleteTable"
      ],
      "Resource": "*",
      "Condition": {
        "StringEquals": {
          "aws:ResourceTag/managed_by": "athenadef"
        }
      }
    }
  ]
}
```

### 7. Multi-Account Setup

For organizations with multiple AWS accounts:

**Centralized Catalog Account:**
```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "glue:GetTable",
        "glue:GetTables"
      ],
      "Resource": "arn:aws:glue:us-west-2:111111111111:*"
    }
  ]
}
```

**Cross-Account Access:**
```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {
        "AWS": "arn:aws:iam::222222222222:role/athenadef-role"
      },
      "Action": [
        "glue:GetTable",
        "glue:GetTables",
        "glue:CreateTable",
        "glue:UpdateTable"
      ],
      "Resource": "*"
    }
  ]
}
```

## Troubleshooting Permissions

### Testing Permissions

Verify permissions with AWS CLI:

```bash
# Test Athena access
aws athena start-query-execution \
  --query-string "SHOW DATABASES" \
  --work-group primary \
  --query-execution-context Database=default

# Test Glue access
aws glue get-databases

aws glue get-table \
  --database-name salesdb \
  --name customers
```

### Common Permission Errors

**Error: `AccessDeniedException: User: arn:aws:iam::123456789012:user/username is not authorized to perform: glue:GetTable`**

**Solution:** Add `glue:GetTable` permission to IAM policy

**Error: `InvalidRequestException: Insufficient permissions to execute the query`**

**Solution:** Ensure you have `athena:StartQueryExecution` permission for the workgroup

**Error: `Access Denied: Athena failed to create query results bucket`**

**Solution:** Use AWS managed storage (remove `output_location` from config) or add S3 permissions

### Debugging Permission Issues

1. **Enable debug mode:**
   ```bash
   athenadef plan --debug
   ```

2. **Check IAM policy simulator:**
   - Use AWS IAM Policy Simulator
   - Test specific API calls
   - Identify missing permissions

3. **Review CloudTrail logs:**
   ```bash
   aws cloudtrail lookup-events \
     --lookup-attributes AttributeKey=EventName,AttributeValue=GetTable
   ```

4. **Verify assume role:**
   ```bash
   # Check current identity
   aws sts get-caller-identity

   # Verify role can be assumed
   aws sts assume-role \
     --role-arn arn:aws:iam::123456789012:role/athenadef-role \
     --role-session-name test
   ```

## IAM Policy Templates

### Quick Start Template

Save as `athenadef-policy.json`:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "AthenaAccess",
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
      "Sid": "GlueAccess",
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

Apply with:
```bash
aws iam create-policy \
  --policy-name AthenadefFullAccess \
  --policy-document file://athenadef-policy.json

aws iam attach-user-policy \
  --user-name your-user \
  --policy-arn arn:aws:iam::123456789012:policy/AthenadefFullAccess
```

## References

- [Amazon Athena IAM Permissions](https://docs.aws.amazon.com/service-authorization/latest/reference/list_amazonathena.html)
- [AWS Glue IAM Permissions](https://docs.aws.amazon.com/service-authorization/latest/reference/list_awsglue.html)
- [Amazon S3 IAM Permissions](https://docs.aws.amazon.com/service-authorization/latest/reference/list_amazons3.html)
- [IAM Policy Simulator](https://policysim.aws.amazon.com/)
- [AWS IAM Best Practices](https://docs.aws.amazon.com/IAM/latest/UserGuide/best-practices.html)
