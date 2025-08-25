# athenadef

Schema management for AWS Athena

## Installation

Custom tap is available for Homebrew:

```
brew install rieshia/x/athenadef
```

or you can download the code from the [release page](https://github.com/riseshia/athenadef/releases) and compile from it.

### GitHub Action

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: riseshia/athenadef@v0
        with:
          version: v0.1.0 # or latest
```

## How to use

```
Usage: athenadef <COMMAND>

Commands:
  apply   apply config
  plan    plan config
  export  export table def to local
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### Directory structure

Let's say you have following databases and tables in Athena.

- Database: `salesdb`
  - Table: `customers`
  - Table: `orders`
- Database: `marketingdb`
  - Table: `leads`
  - Table: `campaigns`

This will be mapped to following directory structure:

```
- salesdb/
 - customers.sql
 - orders.sql
- marketingdb/
 - leads.sql
 - campaigns.sql
```

Each `.sql` file should contain DDL for target table.

## Configuration

```yaml
# athenadef.yaml
- workgroup: "primary" # Optional
  output_location: "s3://your-athena-results-bucket/prefix/" # Optional
```

You can specify config file path with `--config` option on each subcommand. Default is `athenadef.yaml`.

## Required IAM permissions

If you want to allow athenadef fine-grained permissions, you can start with following policy.

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "AllowAthenaQuery",
      "Effect": "Allow",
      "Action": [
        "athena:StartQueryExecution",
        "athena:GetQueryExecution",
        "athena:GetQueryResults",
        "athena:StopQueryExecution"
      ],
      "Resource": "arn:aws:athena:*:<your-account>:workgroup/<your-workgroup>"
    },
    {
      "Effect": "Allow",
      "Action": [
        "s3:GetBucketLocation",
        "s3:GetObject",
        "s3:ListBucket",
        "s3:ListBucketMultipartUploads",
        "s3:ListMultipartUploadParts",
        "s3:AbortMultipartUpload",
        "s3:PutObject"
      ],
      "Resource": [
        "arn:aws:s3:::your-athena-results-bucket",
        "arn:aws:s3:::your-athena-results-bucket/*"
      ]
    }
  ]
}
```

Reference:

- [Actions, resources, and condition keys for Amazon Athena](https://docs.aws.amazon.com/service-authorization/latest/reference/list_amazonathena.html)
- [Actions, resources, and condition keys for Amazon S3](https://docs.aws.amazon.com/service-authorization/latest/reference/list_amazons3.html)

## License

This project is licensed under MIT License.

And, this project includes software developed by:
- aws-sdk-config: Licensed under the Apache License, Version 2.0.
- aws-sdk-athena: Licensed under the Apache License, Version 2.0.
- aws-sdk-sts: Licensed under the Apache License, Version 2.0.
