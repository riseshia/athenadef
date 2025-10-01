# Partitioned Tables Example

This example demonstrates how to manage partitioned tables with partition projection in Athena.

## Setup

This example contains:
- One database: `analyticsdb`
- Two partitioned tables: `events` and `metrics`

## Files

```
partitioned/
├── athenadef.yaml      # Configuration file
├── analyticsdb/
│   ├── events.sql      # Events table with date partitioning
│   └── metrics.sql     # Metrics table with date/hour partitioning
└── README.md
```

## Features Demonstrated

- **Partition Projection**: Automatically discover partitions without running `MSCK REPAIR TABLE`
- **Date-based Partitioning**: Common pattern for time-series data
- **Multi-level Partitioning**: Partitioning by year/month/day and hour

## Usage

1. **Preview changes:**
   ```bash
   cd examples/partitioned
   athenadef plan
   ```

2. **Apply changes:**
   ```bash
   athenadef apply
   ```

## Partition Projection Benefits

Partition projection eliminates the need to run `MSCK REPAIR TABLE` or manually add partitions:

- **Automatic discovery**: Athena automatically discovers partitions based on the projection configuration
- **Better performance**: Queries can use partition pruning without metadata lookups
- **Less maintenance**: No need to manage partition metadata in the Glue catalog

## Query Examples

Once tables are created, you can query them using partition filters:

```sql
-- Query events for a specific date
SELECT * FROM analyticsdb.events
WHERE year = '2024' AND month = '01' AND day = '15';

-- Query metrics for a specific hour
SELECT * FROM analyticsdb.metrics
WHERE year = '2024' AND month = '01' AND day = '15' AND hour = '10';
```

## Notes

- Update S3 location paths to match your data bucket
- Partition projection ranges should cover your data date range
- The projection configuration must match your S3 directory structure
