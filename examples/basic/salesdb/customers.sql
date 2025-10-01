CREATE EXTERNAL TABLE customers (
    customer_id bigint COMMENT 'Unique customer identifier',
    name string COMMENT 'Customer full name',
    email string COMMENT 'Customer email address',
    phone string COMMENT 'Customer phone number',
    registration_date date COMMENT 'Date customer registered',
    status string COMMENT 'Customer status: active, inactive, suspended'
)
STORED AS PARQUET
LOCATION 's3://your-data-bucket/customers/'
TBLPROPERTIES (
    'parquet.compression' = 'SNAPPY',
    'has_encrypted_data' = 'false'
);
