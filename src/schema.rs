table! {
    online_time_log (id) {
        id -> Int4,
        user_id -> Text,
        day -> Date,
        online_time_start -> Time,
        online_time_end -> Time,
    }
}
