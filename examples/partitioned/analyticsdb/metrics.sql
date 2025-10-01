CREATE EXTERNAL TABLE metrics (
    metric_name string COMMENT 'Name of the metric',
    metric_value double COMMENT 'Metric value',
    tags map<string,string> COMMENT 'Metric tags as key-value pairs',
    timestamp bigint COMMENT 'Unix timestamp of the metric'
)
PARTITIONED BY (
    year string COMMENT 'Year partition',
    month string COMMENT 'Month partition (01-12)',
    day string COMMENT 'Day partition (01-31)',
    hour string COMMENT 'Hour partition (00-23)'
)
STORED AS PARQUET
LOCATION 's3://your-data-bucket/metrics/'
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
    'projection.hour.type' = 'integer',
    'projection.hour.range' = '0,23',
    'projection.hour.digits' = '2',
    'storage.location.template' = 's3://your-data-bucket/metrics/year=${year}/month=${month}/day=${day}/hour=${hour}',
    'parquet.compression' = 'SNAPPY'
);
