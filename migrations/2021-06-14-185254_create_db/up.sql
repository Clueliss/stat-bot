CREATE TABLE online_time_log (
    id SERIAL NOT NULL,
    user_id TEXT NOT NULL,
    online_time_start TIMESTAMP NOT NULL,
    online_time_end TIMESTAMP NOT NULL,

    PRIMARY KEY (id)
);
