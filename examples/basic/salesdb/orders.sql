CREATE EXTERNAL TABLE orders (
    order_id bigint COMMENT 'Unique order identifier',
    customer_id bigint COMMENT 'Customer who placed the order',
    order_date date COMMENT 'Date order was placed',
    total_amount decimal(10,2) COMMENT 'Total order amount in USD',
    status string COMMENT 'Order status: pending, completed, cancelled, refunded',
    shipping_address string COMMENT 'Shipping address',
    payment_method string COMMENT 'Payment method used'
)
STORED AS PARQUET
LOCATION 's3://your-data-bucket/orders/'
TBLPROPERTIES (
    'parquet.compression' = 'SNAPPY',
    'has_encrypted_data' = 'false'
);
