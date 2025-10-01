CREATE EXTERNAL TABLE customers (
    customer_id bigint,
    name string,
    email string,
    phone string,
    registration_date date,
    status string
)
STORED AS PARQUET
LOCATION 's3://your-data-bucket/salesdb/customers/'
TBLPROPERTIES (
    'parquet.compression' = 'SNAPPY'
);
