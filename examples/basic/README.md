# Basic Example

This is a simple example showing how to manage a single database with a few tables using athenadef.

## Setup

This example contains:
- One database: `salesdb`
- Two tables: `customers` and `orders`

## Files

```
basic/
├── athenadef.yaml      # Configuration file
├── salesdb/
│   ├── customers.sql   # Customer table definition
│   └── orders.sql      # Orders table definition
└── README.md
```

## Usage

1. **Preview changes:**
   ```bash
   cd examples/basic
   athenadef plan
   ```

2. **Apply changes:**
   ```bash
   athenadef apply
   ```

3. **Export existing tables:**
   ```bash
   athenadef export
   ```

## Expected Output

When running `athenadef plan`, you should see:

```
Plan: 2 to add, 0 to change, 0 to destroy.

+ salesdb.customers
  Will create table

+ salesdb.orders
  Will create table
```

When running `athenadef apply`, you'll be prompted to confirm:

```
Do you want to perform these actions? (yes/no): yes

salesdb.customers: Creating...
salesdb.customers: Creation complete

salesdb.orders: Creating...
salesdb.orders: Creation complete

Apply complete! Resources: 2 added, 0 changed, 0 destroyed.
```

## Notes

- This example uses AWS managed storage (no S3 bucket configuration needed)
- The `workgroup` defaults to "primary" if not specified
- Table data locations should be updated to match your S3 bucket paths
