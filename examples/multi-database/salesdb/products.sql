CREATE EXTERNAL TABLE products (
    product_id bigint,
    name string,
    description string,
    price decimal(10,2),
    category string,
    created_at timestamp
)
STORED AS PARQUET
LOCATION 's3://your-data-bucket/salesdb/products/'
TBLPROPERTIES (
    'parquet.compression' = 'SNAPPY'
);
