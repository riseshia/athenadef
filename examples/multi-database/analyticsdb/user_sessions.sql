CREATE EXTERNAL TABLE user_sessions (
    session_id string,
    user_id string,
    start_time timestamp,
    end_time timestamp,
    duration_seconds int,
    pages_viewed int,
    device_type string,
    browser string
)
STORED AS PARQUET
LOCATION 's3://your-data-bucket/analyticsdb/user_sessions/'
TBLPROPERTIES (
    'parquet.compression' = 'SNAPPY'
);
