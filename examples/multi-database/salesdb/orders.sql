CREATE EXTERNAL TABLE orders (
    order_id bigint,
    customer_id bigint,
    order_date date,
    total_amount decimal(10,2),
    status string
)
STORED AS PARQUET
LOCATION 's3://your-data-bucket/salesdb/orders/'
TBLPROPERTIES (
    'parquet.compression' = 'SNAPPY'
);
