# JSON Output Format

The `plan` command supports a `--json` flag to output structured JSON data instead of human-readable text. This is useful for programmatic processing, CI/CD integrations, or building custom tooling around athenadef.

## Usage

```bash
athenadef plan --json
athenadef plan --json > changes.json
athenadef plan --json --target salesdb.* | jq '.summary'
```

## JSON Structure

The JSON output follows this structure:

```typescript
{
  "no_change": boolean,           // true if no changes detected
  "summary": {
    "to_add": number,             // count of tables to create
    "to_change": number,          // count of tables to update
    "to_destroy": number          // count of tables to delete
  },
  "table_diffs": [
    {
      "database_name": string,    // database name
      "table_name": string,       // table name
      "operation": string,        // "Create" | "Update" | "Delete" | "NoChange"
      "text_diff": string | null, // unified diff text (only for updates)
      "change_details": {         // structured change information (optional)
        "column_changes": [
          {
            "change_type": string,     // "Added" | "Removed" | "TypeChanged"
            "column_name": string,
            "old_type": string | null,
            "new_type": string | null
          }
        ],
        "property_changes": [
          {
            "property_name": string,
            "old_value": string | null,
            "new_value": string | null
          }
        ]
      } | null
    }
  ]
}
```

## Example Outputs

### No Changes

When there are no differences between local and remote schemas:

```json
{
  "no_change": true,
  "summary": {
    "to_add": 0,
    "to_change": 0,
    "to_destroy": 0
  },
  "table_diffs": []
}
```

### Create Tables

When new tables are detected in local files that don't exist remotely:

```json
{
  "no_change": false,
  "summary": {
    "to_add": 2,
    "to_change": 0,
    "to_destroy": 0
  },
  "table_diffs": [
    {
      "database_name": "salesdb",
      "table_name": "customers",
      "operation": "Create",
      "text_diff": null,
      "change_details": null
    },
    {
      "database_name": "salesdb",
      "table_name": "orders",
      "operation": "Create",
      "text_diff": null,
      "change_details": null
    }
  ]
}
```

### Update Tables

When existing tables have schema changes:

```json
{
  "no_change": false,
  "summary": {
    "to_add": 0,
    "to_change": 1,
    "to_destroy": 0
  },
  "table_diffs": [
    {
      "database_name": "marketingdb",
      "table_name": "leads",
      "operation": "Update",
      "text_diff": "--- remote: marketingdb.leads\n+++ local:  marketingdb.leads\n CREATE EXTERNAL TABLE leads (\n-    score int,\n+    score double,\n+    created_at timestamp,\n     email string\n )\n STORED AS PARQUET\n LOCATION 's3://data-bucket/leads/'\n TBLPROPERTIES (\n-    'projection.enabled' = 'false'\n+    'projection.enabled' = 'true'\n );",
      "change_details": {
        "column_changes": [
          {
            "change_type": "TypeChanged",
            "column_name": "score",
            "old_type": "int",
            "new_type": "double"
          },
          {
            "change_type": "Added",
            "column_name": "created_at",
            "old_type": null,
            "new_type": "timestamp"
          }
        ],
        "property_changes": [
          {
            "property_name": "projection.enabled",
            "old_value": "false",
            "new_value": "true"
          }
        ]
      }
    }
  ]
}
```

### Delete Tables

When remote tables don't exist in local files:

```json
{
  "no_change": false,
  "summary": {
    "to_add": 0,
    "to_change": 0,
    "to_destroy": 1
  },
  "table_diffs": [
    {
      "database_name": "salesdb",
      "table_name": "old_orders",
      "operation": "Delete",
      "text_diff": null,
      "change_details": null
    }
  ]
}
```

### Mixed Operations

When multiple types of changes are detected:

```json
{
  "no_change": false,
  "summary": {
    "to_add": 1,
    "to_change": 1,
    "to_destroy": 1
  },
  "table_diffs": [
    {
      "database_name": "salesdb",
      "table_name": "new_customers",
      "operation": "Create",
      "text_diff": null,
      "change_details": null
    },
    {
      "database_name": "marketingdb",
      "table_name": "leads",
      "operation": "Update",
      "text_diff": "--- remote: marketingdb.leads\n+++ local:  marketingdb.leads\n CREATE EXTERNAL TABLE leads (\n-    score int,\n+    score double,\n     email string\n )\n STORED AS PARQUET\n LOCATION 's3://data-bucket/leads/';",
      "change_details": {
        "column_changes": [
          {
            "change_type": "TypeChanged",
            "column_name": "score",
            "old_type": "int",
            "new_type": "double"
          }
        ],
        "property_changes": []
      }
    },
    {
      "database_name": "salesdb",
      "table_name": "old_orders",
      "operation": "Delete",
      "text_diff": null,
      "change_details": null
    }
  ]
}
```

## Processing JSON Output

### Using jq

Extract summary information:

```bash
athenadef plan --json | jq '.summary'
```

Get list of tables to create:

```bash
athenadef plan --json | jq '.table_diffs[] | select(.operation == "Create") | "\(.database_name).\(.table_name)"'
```

Check if there are any changes:

```bash
if [ "$(athenadef plan --json | jq -r '.no_change')" == "true" ]; then
  echo "No changes detected"
else
  echo "Changes detected"
fi
```

Count total changes:

```bash
athenadef plan --json | jq '.summary | .to_add + .to_change + .to_destroy'
```

### Using Python

```python
import json
import subprocess

# Run plan command and get JSON output
result = subprocess.run(
    ["athenadef", "plan", "--json"],
    capture_output=True,
    text=True
)

diff = json.loads(result.stdout)

# Check for changes
if diff["no_change"]:
    print("No changes detected")
else:
    print(f"Changes: {diff['summary']['to_add']} to add, "
          f"{diff['summary']['to_change']} to change, "
          f"{diff['summary']['to_destroy']} to destroy")

    # Process each table diff
    for table_diff in diff["table_diffs"]:
        qualified_name = f"{table_diff['database_name']}.{table_diff['table_name']}"
        operation = table_diff["operation"]

        if operation == "Update" and table_diff["change_details"]:
            details = table_diff["change_details"]
            print(f"\n{qualified_name}:")

            for col_change in details["column_changes"]:
                print(f"  - Column {col_change['column_name']}: {col_change['change_type']}")
```

### Using JavaScript/Node.js

```javascript
const { execSync } = require('child_process');

// Run plan command and get JSON output
const output = execSync('athenadef plan --json', { encoding: 'utf-8' });
const diff = JSON.parse(output);

// Check for changes
if (diff.no_change) {
  console.log('No changes detected');
} else {
  console.log(`Changes detected:`);
  console.log(`  - ${diff.summary.to_add} to add`);
  console.log(`  - ${diff.summary.to_change} to change`);
  console.log(`  - ${diff.summary.to_destroy} to destroy`);

  // Get list of all changed tables
  const changedTables = diff.table_diffs
    .filter(d => d.operation !== 'NoChange')
    .map(d => `${d.database_name}.${d.table_name}`);

  console.log('\nAffected tables:', changedTables.join(', '));
}
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Check Athena Schema Changes

on: [pull_request]

jobs:
  check-schema:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2

      - name: Install athenadef
        run: |
          brew install rieshia/x/athenadef

      - name: Check for schema changes
        id: plan
        run: |
          athenadef plan --json > plan.json
          echo "::set-output name=has_changes::$(jq -r '.no_change == false' plan.json)"

      - name: Post plan as comment
        if: steps.plan.outputs.has_changes == 'true'
        uses: actions/github-script@v6
        with:
          script: |
            const fs = require('fs');
            const plan = JSON.parse(fs.readFileSync('plan.json', 'utf8'));

            const summary = `## Athena Schema Changes

            - ${plan.summary.to_add} table(s) to create
            - ${plan.summary.to_change} table(s) to update
            - ${plan.summary.to_destroy} table(s) to delete

            <details>
            <summary>Affected Tables</summary>

            ${plan.table_diffs.map(d =>
              `- ${d.operation}: ${d.database_name}.${d.table_name}`
            ).join('\n')}

            </details>`;

            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: summary
            });
```

### GitLab CI Example

```yaml
schema-check:
  stage: validate
  script:
    - athenadef plan --json > plan.json
    - |
      if [ "$(jq -r '.no_change' plan.json)" == "false" ]; then
        echo "Schema changes detected:"
        jq '.summary' plan.json
        jq -r '.table_diffs[] | "\(.operation): \(.database_name).\(.table_name)"' plan.json
      else
        echo "No schema changes"
      fi
  artifacts:
    reports:
      dotenv: plan.json
```

## Field Descriptions

### Top-Level Fields

- **`no_change`**: Boolean indicating whether any changes were detected. When `true`, `table_diffs` will be empty.
- **`summary`**: Object containing counts of different operation types.
- **`table_diffs`**: Array of individual table differences. May be empty when `no_change` is true.

### Summary Fields

- **`to_add`**: Number of tables that will be created (exist locally but not remotely).
- **`to_change`**: Number of tables that will be updated (exist both locally and remotely with differences).
- **`to_destroy`**: Number of tables that will be deleted (exist remotely but not locally).

### TableDiff Fields

- **`database_name`**: Name of the database containing the table.
- **`table_name`**: Name of the table.
- **`operation`**: Type of operation. One of:
  - `"Create"`: Table will be created
  - `"Update"`: Table will be updated
  - `"Delete"`: Table will be deleted
  - `"NoChange"`: No changes detected (only shown with `--show-unchanged` flag)
- **`text_diff`**: Unified diff showing the SQL changes. Only present for `Update` operations. Contains the complete diff including context lines, added lines (prefixed with `+`), and removed lines (prefixed with `-`).
- **`change_details`**: Structured information about specific changes. Currently `null` in the implementation but reserved for future use. Will contain:
  - `column_changes`: Array of column-level changes (additions, removals, type changes)
  - `property_changes`: Array of property changes (location, format, partitions, etc.)

### ChangeDetails Fields (Future Enhancement)

While the structure exists in the data types, `change_details` is currently not populated. In future versions, this will contain:

- **`column_changes`**: Array of column modifications
  - `change_type`: `"Added"`, `"Removed"`, or `"TypeChanged"`
  - `column_name`: Name of the affected column
  - `old_type`: Previous data type (null for additions)
  - `new_type`: New data type (null for removals)

- **`property_changes`**: Array of table property modifications
  - `property_name`: Name of the property (e.g., "location", "projection.enabled")
  - `old_value`: Previous value (null for new properties)
  - `new_value`: New value (null for removed properties)

## Notes

- The JSON output is deterministic and suitable for diffing between runs.
- All strings are properly escaped according to JSON standards.
- The `text_diff` field preserves newlines and special characters.
- When using `--target` filters, only matching tables appear in the output.
- The `--show-unchanged` flag is not relevant with `--json` as unchanged tables are always included with `NoChange` operation in the JSON output if they match the filter criteria.
