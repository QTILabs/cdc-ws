-- RisingWave example configuration for PostgreSQL CDC to OpenSearch
-- Keep this file copy-paste ready and aligned with SPECS.md

-- Inbound Users Replication Source
CREATE TABLE src_postgres_users (
    user_id INT,
    username VARCHAR,
    email VARCHAR,
    PRIMARY KEY (user_id)
) WITH (
    connector = 'postgres-cdc',
    hostname = 'postgres-primary.database.svc.cluster.local',
    port = '5432',
    username = 'rw_cdc_user',
    password = 'your_secure_postgres_password',
    database.name = 'production',
    schema.name = 'public',
    table.name = 'users'
);

-- Inbound Orders Replication Source
CREATE TABLE src_postgres_orders (
    order_id INT,
    user_id INT,
    total_amount NUMERIC,
    status VARCHAR,
    PRIMARY KEY (order_id)
) WITH (
    connector = 'postgres-cdc',
    hostname = 'postgres-primary.database.svc.cluster.local',
    port = '5432',
    username = 'rw_cdc_user',
    password = 'your_secure_postgres_password',
    database.name = 'production',
    schema.name = 'public',
    table.name = 'orders'
);

-- Materialized projections
CREATE MATERIALIZED VIEW mv_user_analytics AS
SELECT user_id, username, email FROM src_postgres_users;

CREATE MATERIALIZED VIEW mv_order_analytics AS
SELECT order_id, user_id, total_amount, status FROM src_postgres_orders;

-- Changelog stream subscriptions
CREATE SUBSCRIPTION sub_users FROM mv_user_analytics WITH (retention = '1D');
CREATE SUBSCRIPTION sub_orders FROM mv_order_analytics WITH (retention = '1D');
