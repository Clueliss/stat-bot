use std::{
    collections::{btree_map::Entry, BTreeMap},
    str::FromStr,
};

use chrono::{Date, Duration, NaiveDate, NaiveDateTime, Utc};
use diesel::{data_types::PgInterval, prelude::*};
use serenity::model::id::UserId;

pub struct StatManager {
    db_url: String,
    state: BTreeMap<UserId, NaiveDateTime>,
}

impl StatManager {
    fn make_conn(&self) -> Result<PgConnection, ConnectionError> {
        PgConnection::establish(&self.db_url)
    }

    pub fn new<S: AsRef<str>>(db_url: S) -> Self {
        Self {
            db_url: db_url.as_ref().to_owned(),
            state: BTreeMap::new(),
        }
    }

    pub fn absolute_time_iter(&self) -> QueryResult<impl Iterator<Item = (UserId, Duration)>> {
        use crate::schema::online_time_log::dsl::*;
        use diesel::dsl::sql;

        let conn = self.make_conn().unwrap();

        let logs = online_time_log
            .group_by(user_id)
            .select((user_id, sql("SUM(online_time_end - online_time_start)")))
            .load::<(String, PgInterval)>(&conn)?;

        Ok(logs.into_iter().map(|(uid, time)| {
            (
                UserId::from_str(&uid).unwrap(),
                Duration::microseconds(time.microseconds),
            )
        }))
    }

    pub fn time_per_day_iter(
        &self,
    ) -> QueryResult<impl Iterator<Item = (UserId, Date<Utc>, Duration)>> {
        use crate::schema::online_time_log::dsl::*;
        use diesel::dsl::{date, sql};

        let conn = self.make_conn().unwrap();
        let start_date = date(online_time_start);

        let results = online_time_log
            .group_by((user_id, start_date))
            .select((
                user_id,
                start_date,
                sql("SUM(online_time_end - online_time_start)"),
                sql("SUM((DATE(NOW()) + INTERVAL '1 day') - online_time_start)"),
            ))
            .order_by(start_date)
            .load::<(String, NaiveDate, PgInterval, PgInterval)>(&conn)?;

        Ok(results.into_iter().map(|(uid, date, fsum, ssum)| {
            (
                UserId::from_str(&uid).unwrap(),
                Date::from_utc(date, Utc),
                std::cmp::min(
                    Duration::microseconds(fsum.microseconds),
                    Duration::microseconds(ssum.microseconds),
                ),
            )
        }))
    }

    pub fn flush_stats(&mut self) -> QueryResult<()> {
        use crate::model::NewLogEntryOwned;
        use crate::schema::online_time_log::dsl::*;

        let conn = self.make_conn().unwrap();

        let now = Utc::now().naive_utc();
        let mut buf = Vec::new();

        for (uid, timestamp) in self.state.iter_mut() {
            let start = std::mem::replace(timestamp, now);
            buf.push(NewLogEntryOwned {
                user_id: uid.to_string(),
                online_time_start: start,
                online_time_end: now,
            });
        }

        diesel::insert_into(online_time_log)
            .values(buf)
            .execute(&conn)?;

        Ok(())
    }

    pub fn user_now_offline(&mut self, uid: UserId) -> QueryResult<bool> {
        use crate::model::NewLogEntryOwned;
        use crate::schema::online_time_log::dsl::*;

        match self.state.remove(&uid) {
            None => Ok(false),
            Some(timestamp) => {
                let conn = self.make_conn().unwrap();
                let now = Utc::now().naive_utc();

                diesel::insert_into(online_time_log)
                    .values(NewLogEntryOwned {
                        user_id: uid.to_string(),
                        online_time_start: timestamp,
                        online_time_end: now,
                    })
                    .execute(&conn)?;

                Ok(true)
            }
        }
    }

    pub fn user_now_online(&mut self, uid: UserId) -> bool {
        match self.state.entry(uid) {
            Entry::Occupied(_) => false,
            Entry::Vacant(ve) => {
                ve.insert(Utc::now().naive_utc());
                true
            }
        }
    }
}
