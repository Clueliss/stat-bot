CREATE TABLE online_time_log (
    id SERIAL PRIMARY KEY,
    user_id TEXT NOT NULL,
    day DATE NOT NULL,
    online_time_start TIME NOT NULL,
    online_time_end TIME NOT NULL CHECK (online_time_start <= online_time_end)
);

INSERT INTO online_time_log (user_id, day, online_time_start, online_time_end) VALUES ('1', '2021-06-15', '10:00', '11:00');
INSERT INTO online_time_log (user_id, day, online_time_start, online_time_end) VALUES ('1', '2021-06-15', '12:00', '14:00');
INSERT INTO online_time_log (user_id, day, online_time_start, online_time_end) VALUES ('1', '2021-06-16', '10:00', '11:00');

INSERT INTO online_time_log (user_id, day, online_time_start, online_time_end) VALUES ('2', '2021-06-15', '10:00', '11:00');
INSERT INTO online_time_log (user_id, day, online_time_start, online_time_end) VALUES ('2', '2021-06-15', '12:00', '18:00');
INSERT INTO online_time_log (user_id, day, online_time_start, online_time_end) VALUES ('2', '2021-06-17', '10:00', '11:00');
