CREATE EXTERNAL TABLE page_views (
    page_view_id string,
    session_id string,
    user_id string,
    page_url string,
    page_title string,
    view_timestamp timestamp,
    referrer_url string,
    time_on_page_seconds int
)
STORED AS PARQUET
LOCATION 's3://your-data-bucket/analyticsdb/page_views/'
TBLPROPERTIES (
    'parquet.compression' = 'SNAPPY'
);
