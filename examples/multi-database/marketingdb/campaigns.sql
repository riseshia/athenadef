CREATE EXTERNAL TABLE campaigns (
    campaign_id bigint,
    name string,
    start_date date,
    end_date date,
    budget decimal(10,2),
    status string,
    channel string
)
STORED AS PARQUET
LOCATION 's3://your-data-bucket/marketingdb/campaigns/'
TBLPROPERTIES (
    'parquet.compression' = 'SNAPPY'
);
