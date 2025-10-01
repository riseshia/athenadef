CREATE EXTERNAL TABLE events (
    event_id string COMMENT 'Unique event identifier',
    user_id string COMMENT 'User who triggered the event',
    event_type string COMMENT 'Type of event: click, view, purchase, etc.',
    event_data string COMMENT 'JSON string with event details',
    timestamp bigint COMMENT 'Unix timestamp of the event',
    session_id string COMMENT 'Session identifier'
)
PARTITIONED BY (
    year string COMMENT 'Year partition',
    month string COMMENT 'Month partition (01-12)',
    day string COMMENT 'Day partition (01-31)'
)
STORED AS PARQUET
LOCATION 's3://your-data-bucket/events/'
TBLPROPERTIES (
    'projection.enabled' = 'true',
    'projection.year.type' = 'integer',
    'projection.year.range' = '2020,2030',
    'projection.month.type' = 'integer',
    'projection.month.range' = '1,12',
    'projection.month.digits' = '2',
    'projection.day.type' = 'integer',
    'projection.day.range' = '1,31',
    'projection.day.digits' = '2',
    'storage.location.template' = 's3://your-data-bucket/events/year=${year}/month=${month}/day=${day}',
    'parquet.compression' = 'SNAPPY'
);
