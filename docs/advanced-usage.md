# Advanced Usage Guide

This guide covers advanced usage patterns and techniques for athenadef.

## Table of Contents

- [Target Filtering Patterns](#target-filtering-patterns)
- [Performance Optimization](#performance-optimization)
- [CI/CD Integration](#cicd-integration)
- [Multi-Environment Setup](#multi-environment-setup)
- [Large Scale Deployments](#large-scale-deployments)
- [Security and Compliance](#security-and-compliance)
- [Monitoring and Observability](#monitoring-and-observability)
- [Advanced SQL Features](#advanced-sql-features)
- [Disaster Recovery](#disaster-recovery)

## Target Filtering Patterns

### Basic Filtering

```bash
# Single table
athenadef plan --target salesdb.customers

# Multiple specific tables
athenadef plan --target salesdb.customers --target salesdb.orders

# All tables in a database
athenadef plan --target salesdb.*

# Tables with same name across databases
athenadef plan --target *.customers
```

### Advanced Filtering Strategies

**Incremental Deployment:**

```bash
# Deploy in phases
athenadef apply --target salesdb.customers  # Test with one table
athenadef apply --target salesdb.*          # Then entire database
athenadef apply                              # Finally all databases
```

**Database-by-Database:**

```bash
# Deploy databases independently
for db in salesdb marketingdb analyticsdb; do
  echo "Deploying $db..."
  athenadef apply --target "$db.*" --auto-approve
done
```

**Pattern-Based Deployment:**

```bash
# All fact tables
athenadef apply --target *.fact_*

# All dimension tables
athenadef apply --target *.dim_*

# All staging tables
athenadef apply --target staging.*
```

## Performance Optimization

### Concurrent Query Configuration

Optimize for your workload:

```yaml
# athenadef.yaml

# For small projects (default)
max_concurrent_queries: 5

# For large projects with many tables
max_concurrent_queries: 10

# For very large projects (check AWS limits)
max_concurrent_queries: 20
```

**Finding the right value:**

```bash
# Monitor with debug mode
time athenadef plan --debug 2>&1 | grep "concurrent"

# Increase gradually and measure
for n in 5 10 15 20; do
  echo "Testing with $n concurrent queries"
  time athenadef plan --config config-$n.yaml
done
```

### Query Timeout Optimization

```yaml
# For simple schemas
query_timeout_seconds: 60

# For complex queries (default)
query_timeout_seconds: 300

# For very large tables
query_timeout_seconds: 600
```

### Reducing Network Overhead

**Use AWS managed storage:**

```yaml
# Faster, no S3 round-trips needed
workgroup: "primary"
# No output_location specified
```

**Minimize table scans:**

```bash
# Use specific targets
athenadef plan --target salesdb.customers

# Instead of scanning all
athenadef plan
```

## CI/CD Integration

### GitHub Actions

**Basic workflow:**

```yaml
# .github/workflows/athena-plan.yml
name: Athena Schema Plan
on:
  pull_request:
    paths:
      - 'athena-definitions/**'
      - '.github/workflows/athena-plan.yml'

jobs:
  plan:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: write
    steps:
      - uses: actions/checkout@v5

      - uses: riseshia/athenadef@v0
        with:
          version: latest

      - name: Configure AWS
        uses: aws-actions/configure-aws-credentials@v4
        with:
          role-to-assume: arn:aws:iam::${{ secrets.AWS_ACCOUNT_ID }}:role/athenadef-ci
          aws-region: us-west-2

      - name: Run plan
        id: plan
        run: |
          cd athena-definitions
          athenadef plan --debug > plan-output.txt 2>&1
          echo "exit_code=$?" >> $GITHUB_OUTPUT
        continue-on-error: true

      - name: Comment PR
        uses: actions/github-script@v7
        if: github.event_name == 'pull_request'
        with:
          script: |
            const fs = require('fs');
            const output = fs.readFileSync('athena-definitions/plan-output.txt', 'utf8');

            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: `## Athena Schema Plan\n\n\`\`\`\n${output}\n\`\`\`\n`
            });

      - name: Check plan status
        if: steps.plan.outputs.exit_code != '0'
        run: exit 1
```

**Deployment workflow:**

```yaml
# .github/workflows/athena-deploy.yml
name: Athena Schema Deploy
on:
  push:
    branches: [main]
    paths:
      - 'athena-definitions/**'

jobs:
  deploy:
    runs-on: ubuntu-latest
    environment: production
    steps:
      - uses: actions/checkout@v5

      - uses: riseshia/athenadef@v0

      - name: Configure AWS
        uses: aws-actions/configure-aws-credentials@v4
        with:
          role-to-assume: arn:aws:iam::${{ secrets.AWS_ACCOUNT_ID }}:role/athenadef-deploy
          aws-region: us-west-2

      - name: Deploy changes
        run: |
          cd athena-definitions
          athenadef apply --auto-approve --debug

      - name: Notify on failure
        if: failure()
        uses: slackapi/slack-github-action@v1
        with:
          payload: |
            {
              "text": "Athena deployment failed!",
              "blocks": [
                {
                  "type": "section",
                  "text": {
                    "type": "mrkdwn",
                    "text": "Athena deployment failed for ${{ github.repository }}"
                  }
                }
              ]
            }
        env:
          SLACK_WEBHOOK_URL: ${{ secrets.SLACK_WEBHOOK_URL }}
```

### GitLab CI

```yaml
# .gitlab-ci.yml
stages:
  - plan
  - deploy

variables:
  AWS_REGION: us-west-2
  ATHENADEF_VERSION: latest

.athenadef_setup:
  before_script:
    - wget https://github.com/riseshia/athenadef/releases/download/${ATHENADEF_VERSION}/athenadef-linux-amd64
    - chmod +x athenadef-linux-amd64
    - mv athenadef-linux-amd64 /usr/local/bin/athenadef

plan:
  stage: plan
  extends: .athenadef_setup
  script:
    - cd athena-definitions
    - athenadef plan
  only:
    - merge_requests
    - main

deploy:
  stage: deploy
  extends: .athenadef_setup
  script:
    - cd athena-definitions
    - athenadef apply --auto-approve
  only:
    - main
  environment:
    name: production
```

### Jenkins

```groovy
// Jenkinsfile
pipeline {
    agent any

    environment {
        AWS_REGION = 'us-west-2'
        AWS_CREDENTIALS = credentials('aws-athenadef')
    }

    stages {
        stage('Install athenadef') {
            steps {
                sh '''
                    wget https://github.com/riseshia/athenadef/releases/latest/download/athenadef-linux-amd64
                    chmod +x athenadef-linux-amd64
                    mv athenadef-linux-amd64 /usr/local/bin/athenadef
                '''
            }
        }

        stage('Plan') {
            steps {
                dir('athena-definitions') {
                    sh 'athenadef plan'
                }
            }
        }

        stage('Deploy') {
            when {
                branch 'main'
            }
            steps {
                dir('athena-definitions') {
                    sh 'athenadef apply --auto-approve'
                }
            }
        }
    }

    post {
        failure {
            emailext(
                subject: "Athena Deployment Failed: ${env.JOB_NAME}",
                body: "Check console output at ${env.BUILD_URL}",
                to: "team@example.com"
            )
        }
    }
}
```

## Multi-Environment Setup

### Directory Structure

```
project/
├── environments/
│   ├── dev/
│   │   ├── athenadef.yaml
│   │   ├── salesdb/
│   │   │   └── customers.sql
│   │   └── analyticsdb/
│   │       └── events.sql
│   ├── staging/
│   │   ├── athenadef.yaml
│   │   ├── salesdb/
│   │   │   └── customers.sql
│   │   └── analyticsdb/
│   │       └── events.sql
│   └── prod/
│       ├── athenadef.yaml
│       ├── salesdb/
│       │   └── customers.sql
│       └── analyticsdb/
│           └── events.sql
```

### Environment-Specific Configuration

**Development:**

```yaml
# environments/dev/athenadef.yaml
workgroup: "dev-workgroup"
query_timeout_seconds: 60
max_concurrent_queries: 3
```

**Production:**

```yaml
# environments/prod/athenadef.yaml
workgroup: "prod-workgroup"
query_timeout_seconds: 300
max_concurrent_queries: 10
```

### Deployment Script

```bash
#!/bin/bash
# deploy.sh

set -e

ENV=${1:-dev}
ACTION=${2:-plan}

if [[ ! -d "environments/$ENV" ]]; then
  echo "Environment $ENV not found"
  exit 1
fi

cd "environments/$ENV"

echo "Running athenadef $ACTION for $ENV environment..."

case $ACTION in
  plan)
    athenadef plan --debug
    ;;
  apply)
    athenadef apply --auto-approve
    ;;
  export)
    athenadef export --overwrite
    ;;
  *)
    echo "Unknown action: $ACTION"
    exit 1
    ;;
esac
```

Usage:

```bash
# Plan changes in dev
./deploy.sh dev plan

# Apply to staging
./deploy.sh staging apply

# Export from production
./deploy.sh prod export
```

### Shared Configuration

For common table definitions:

```
project/
├── shared/
│   └── common_tables.sql
├── environments/
│   ├── dev/
│   │   ├── athenadef.yaml
│   │   └── salesdb/
│   │       └── customers.sql -> ../../shared/customers.sql
│   └── prod/
│       ├── athenadef.yaml
│       └── salesdb/
│           └── customers.sql -> ../../shared/customers.sql
```

## Large Scale Deployments

### Managing Hundreds of Tables

**1. Organize by domain:**

```
athena-definitions/
├── sales/
│   ├── salesdb/
│   │   ├── customers.sql
│   │   ├── orders.sql
│   │   └── ...
├── marketing/
│   ├── marketingdb/
│   │   ├── campaigns.sql
│   │   ├── leads.sql
│   │   └── ...
└── analytics/
    ├── analyticsdb/
        ├── events.sql
        ├── sessions.sql
        └── ...
```

**2. Deploy by domain:**

```bash
#!/bin/bash
# deploy-domain.sh

DOMAIN=$1

if [[ -z "$DOMAIN" ]]; then
  echo "Usage: $0 <domain>"
  exit 1
fi

cd "athena-definitions/$DOMAIN"

# Get all databases in this domain
DATABASES=$(ls -d */ | tr -d '/')

for db in $DATABASES; do
  echo "Deploying $db..."
  athenadef apply --target "$db.*" --auto-approve

  if [ $? -ne 0 ]; then
    echo "Failed to deploy $db"
    exit 1
  fi
done
```

**3. Parallel deployments:**

```bash
#!/bin/bash
# parallel-deploy.sh

DOMAINS=(sales marketing analytics)

for domain in "${DOMAINS[@]}"; do
  (
    echo "Starting deployment for $domain"
    ./deploy-domain.sh "$domain"
  ) &
done

wait
echo "All deployments complete"
```

### Batching Strategy

For very large numbers of tables:

```bash
#!/bin/bash
# batch-deploy.sh

BATCH_SIZE=50
DATABASES=(salesdb marketingdb analyticsdb)

for db in "${DATABASES[@]}"; do
  # Get all tables in database
  TABLES=$(ls "$db"/*.sql | sed "s|$db/||g" | sed 's/.sql//g')

  # Process in batches
  BATCH=()
  for table in $TABLES; do
    BATCH+=("$table")

    if [ ${#BATCH[@]} -eq $BATCH_SIZE ]; then
      # Deploy batch
      TARGETS=$(printf -- "--target $db.%s " "${BATCH[@]}")
      athenadef apply $TARGETS --auto-approve
      BATCH=()
    fi
  done

  # Deploy remaining
  if [ ${#BATCH[@]} -gt 0 ]; then
    TARGETS=$(printf -- "--target $db.%s " "${BATCH[@]}")
    athenadef apply $TARGETS --auto-approve
  fi
done
```

## Security and Compliance

### Sensitive Data Handling

**Never commit sensitive data:**

```bash
# .gitignore
*.env
*secrets*
credentials.json

# But do track structure
!athenadef.yaml.example
```

**Use environment variables:**

```yaml
# athenadef.yaml
workgroup: "${ATHENA_WORKGROUP:-primary}"
region: "${AWS_REGION:-us-west-2}"
```

### Audit Trail

**Enable CloudTrail logging:**

```bash
# Create CloudTrail for Glue/Athena operations
aws cloudtrail create-trail \
  --name athenadef-audit \
  --s3-bucket-name my-cloudtrail-bucket

aws cloudtrail start-logging --name athenadef-audit
```

**Query audit logs:**

```bash
# Find all table modifications
aws cloudtrail lookup-events \
  --lookup-attributes AttributeKey=EventName,AttributeValue=UpdateTable \
  --max-results 50
```

### Approval Workflows

Require manual approval in production:

```yaml
# GitHub Actions
deploy:
  environment:
    name: production
    url: https://console.aws.amazon.com/athena
  steps:
    - name: Deploy
      run: athenadef apply --auto-approve
```

## Monitoring and Observability

### Logging

**Enable debug logging:**

```bash
# Save debug output
athenadef apply --debug > deploy.log 2>&1

# Filter specific operations
athenadef apply --debug 2>&1 | grep -i "query"
```

### Metrics Collection

**Track deployment metrics:**

```bash
#!/bin/bash
# deploy-with-metrics.sh

START=$(date +%s)

athenadef apply --auto-approve > output.txt 2>&1
EXIT_CODE=$?

END=$(date +%s)
DURATION=$((END - START))

# Extract metrics from output
ADDED=$(grep -c "to add" output.txt)
CHANGED=$(grep -c "to change" output.txt)
DESTROYED=$(grep -c "to destroy" output.txt)

# Send to monitoring system
curl -X POST https://metrics.example.com/api/v1/metrics \
  -d "deployment.duration=$DURATION" \
  -d "deployment.added=$ADDED" \
  -d "deployment.changed=$CHANGED" \
  -d "deployment.destroyed=$DESTROYED" \
  -d "deployment.status=$EXIT_CODE"
```

### Health Checks

**Verify deployments:**

```bash
#!/bin/bash
# verify-deployment.sh

TABLES=(
  "salesdb.customers"
  "salesdb.orders"
  "analyticsdb.events"
)

for table in "${TABLES[@]}"; do
  echo "Checking $table..."

  # Run test query
  aws athena start-query-execution \
    --query-string "SELECT COUNT(*) FROM $table LIMIT 1" \
    --work-group primary \
    --query-execution-context Database=$(echo $table | cut -d. -f1) \
    --output json > query.json

  QUERY_ID=$(jq -r '.QueryExecutionId' query.json)

  # Wait for completion
  aws athena get-query-execution \
    --query-execution-id "$QUERY_ID" \
    --output json | jq -r '.QueryExecution.Status.State'
done
```

## Advanced SQL Features

### Partition Projection

```sql
-- Advanced partition projection with multiple dimensions
CREATE EXTERNAL TABLE events (
    event_id string,
    user_id bigint,
    event_type string,
    timestamp bigint
)
PARTITIONED BY (
    year string,
    month string,
    day string,
    hour string
)
STORED AS PARQUET
LOCATION 's3://data-bucket/events/'
TBLPROPERTIES (
    'projection.enabled' = 'true',

    'projection.year.type' = 'integer',
    'projection.year.range' = '2020,2030',

    'projection.month.type' = 'integer',
    'projection.month.range' = '1,12',
    'projection.month.digits' = '2',

    'projection.day.type' = 'integer',
    'projection.day.range' = '1,31',
    'projection.day.digits' = '2',

    'projection.hour.type' = 'integer',
    'projection.hour.range' = '0,23',
    'projection.hour.digits' = '2',

    'storage.location.template' = 's3://data-bucket/events/year=${year}/month=${month}/day=${day}/hour=${hour}'
);
```

### Views as Code

While athenadef focuses on tables, you can manage views:

```sql
-- Create view alongside table
-- views/salesdb/customer_summary.sql
CREATE OR REPLACE VIEW customer_summary AS
SELECT
    customer_id,
    COUNT(order_id) as order_count,
    SUM(amount) as total_spent
FROM customers
LEFT JOIN orders USING (customer_id)
GROUP BY customer_id;
```

Deploy views with a script:

```bash
#!/bin/bash
# deploy-views.sh

for view_file in views/*/*.sql; do
  db=$(basename $(dirname $view_file))

  aws athena start-query-execution \
    --query-string "$(cat $view_file)" \
    --query-execution-context Database=$db \
    --work-group primary
done
```

## Disaster Recovery

### Backup Strategy

**1. Git is your primary backup:**

```bash
# All definitions are in git
git log --all -- salesdb/customers.sql
```

**2. Regular exports:**

```bash
#!/bin/bash
# backup-schemas.sh

DATE=$(date +%Y%m%d-%H%M%S)
BACKUP_DIR="backups/$DATE"

mkdir -p "$BACKUP_DIR"

athenadef export --overwrite
cp -r */. "$BACKUP_DIR/"

git add "$BACKUP_DIR"
git commit -m "Backup: $DATE"
```

**3. Cross-region replication:**

```bash
# Export from primary region
AWS_REGION=us-west-2 athenadef export --overwrite

# Apply to secondary region
AWS_REGION=us-east-1 athenadef apply --auto-approve
```

### Recovery Procedures

**Recover single table:**

```bash
# Find previous version
git log -- salesdb/customers.sql

# Checkout specific version
git checkout <commit-hash> -- salesdb/customers.sql

# Apply
athenadef apply --target salesdb.customers
```

**Recover entire database:**

```bash
# Checkout previous state
git checkout <commit-hash> -- salesdb/

# Review changes
athenadef plan --target salesdb.*

# Apply
athenadef apply --target salesdb.* --auto-approve
```

**Full disaster recovery:**

```bash
# 1. Export current state
athenadef export --overwrite
git commit -m "Backup before recovery"

# 2. Checkout known good state
git checkout production-release-tag

# 3. Review
athenadef plan

# 4. Recover
athenadef apply --auto-approve
```

## Additional Resources

- [Main Documentation](../README.md)
- [Troubleshooting Guide](./troubleshooting.md)
- [AWS Permissions](./aws-permissions.md)
- [Examples](../examples/)
