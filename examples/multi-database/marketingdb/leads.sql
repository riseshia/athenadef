CREATE EXTERNAL TABLE leads (
    lead_id bigint,
    email string,
    name string,
    phone string,
    source string,
    score double,
    created_at timestamp,
    status string
)
STORED AS PARQUET
LOCATION 's3://your-data-bucket/marketingdb/leads/'
TBLPROPERTIES (
    'parquet.compression' = 'SNAPPY'
);
