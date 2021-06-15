table! {
    online_time_log (id) {
        id -> Int4,
        user_id -> Text,
        online_time_start -> Timestamp,
        online_time_end -> Timestamp,
    }
}
