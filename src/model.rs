use chrono::NaiveDateTime;

use crate::schema::online_time_log;

#[derive(Queryable, Identifiable)]
#[table_name = "online_time_log"]
pub struct LogEntry {
    pub id: i32,
    pub user_id: String,
    pub online_time_start: NaiveDateTime,
    pub online_time_end: NaiveDateTime,
}

#[derive(Insertable)]
#[table_name = "online_time_log"]
pub struct NewLogEntry<'query> {
    pub user_id: &'query str,
    pub online_time_start: NaiveDateTime,
    pub online_time_end: NaiveDateTime,
}

#[derive(Insertable)]
#[table_name = "online_time_log"]
pub struct NewLogEntryOwned {
    pub user_id: String,
    pub online_time_start: NaiveDateTime,
    pub online_time_end: NaiveDateTime,
}

#[derive(AsChangeset)]
#[table_name = "online_time_log"]
pub struct ChangeLogEntry<'query> {
    pub user_id: Option<&'query str>,
    pub online_time_start: Option<NaiveDateTime>,
    pub online_time_end: Option<NaiveDateTime>,
}
