use chrono::{NaiveDate, NaiveTime};

use crate::schema::online_time_log;

#[derive(Queryable, Identifiable)]
#[table_name = "online_time_log"]
pub struct LogEntry {
    pub id: i32,
    pub user_id: String,
    pub day: NaiveDate,
    pub online_time_start: NaiveTime,
    pub online_time_end: NaiveTime,
}

#[derive(Insertable)]
#[table_name = "online_time_log"]
pub struct NewLogEntry<'query> {
    pub user_id: &'query str,
    pub day: NaiveDate,
    pub online_time_start: NaiveTime,
    pub online_time_end: NaiveTime,
}

#[derive(Insertable)]
#[table_name = "online_time_log"]
pub struct NewLogEntryOwned {
    pub user_id: String,
    pub day: NaiveDate,
    pub online_time_start: NaiveTime,
    pub online_time_end: NaiveTime,
}

#[derive(AsChangeset)]
#[table_name = "online_time_log"]
pub struct ChangeLogEntry<'query> {
    pub user_id: Option<&'query str>,
    pub day: Option<NaiveDate>,
    pub online_time_start: Option<NaiveTime>,
    pub online_time_end: Option<NaiveTime>,
}
