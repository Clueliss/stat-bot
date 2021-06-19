use std::{
    collections::{btree_map::Entry, BTreeMap},
    ops::Range,
    str::FromStr,
};

use chrono::{Date, Duration, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use diesel::{data_types::PgInterval, dsl::sql, prelude::*, sql_types::Interval};
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

    pub fn date_range(&self) -> QueryResult<Range<Date<Utc>>> {
        use crate::schema::online_time_log::dsl::*;

        let conn = self.make_conn().unwrap();

        let (min, max) = online_time_log
            .select((sql("MIN(online_time_start)"), sql("MAX(online_time_end)")))
            .first::<(NaiveDate, NaiveDate)>(&conn)?;

        Ok(Date::from_utc(min, Utc)..Date::from_utc(max, Utc) + Duration::days(1))
    }

    pub fn absolute_sum_time_iter(&self) -> QueryResult<impl Iterator<Item = (UserId, Duration)>> {
        use crate::schema::online_time_log::dsl::*;
        let conn = self.make_conn().unwrap();

        let logs = online_time_log
            .select((user_id, sql("SUM(online_time_end - online_time_start)")))
            .group_by(user_id)
            .load::<(String, PgInterval)>(&conn)?;

        Ok(logs.into_iter().map(|(uid, time)| {
            (
                UserId::from_str(&uid).unwrap(),
                Duration::microseconds(time.microseconds),
            )
        }))
    }

    fn collect_db_results(
        data: Vec<(String, NaiveDate, PgInterval)>,
    ) -> BTreeMap<UserId, Vec<(Date<Utc>, Duration)>> {
        let mut buf: BTreeMap<UserId, Vec<(Date<Utc>, Duration)>> = BTreeMap::new();

        for (uid, day, dur) in data {
            let val = (
                Date::from_utc(day, Utc),
                Duration::microseconds(dur.microseconds),
            );

            buf.entry(UserId::from_str(&uid).unwrap())
                .or_insert(Vec::new())
                .push(val);
        }

        buf
    }

    pub fn time_per_day_iter(&self) -> QueryResult<BTreeMap<UserId, Vec<(Date<Utc>, Duration)>>> {
        use crate::schema::online_time_log::dsl as ot;

        let results = {
            let conn = self.make_conn().unwrap();

            ot::online_time_log
                .select((
                    ot::user_id,
                    ot::day,
                    sql("SUM(online_time_end - online_time_start)"),
                ))
                .group_by((ot::user_id, ot::day))
                .load::<(String, NaiveDate, PgInterval)>(&conn)?
        };

        Ok(Self::collect_db_results(results))
    }

    pub fn absolute_sum_time_per_day_iter(
        &self,
    ) -> QueryResult<BTreeMap<UserId, Vec<(Date<Utc>, Duration)>>> {
        use crate::schema::online_time_log::dsl as ot;

        /*let q = sql_query("
            WITH data AS (
                SELECT user_id, DATE(online_time_start) AS day, SUM(online_time_end - online_time_start) as ontime
                FROM online_time_log
                GROUP BY user_id, DATE(online_time_start)
            )

            SELECT user_id, day, SUM(ontime) OVER (PARTITION BY day)
            FROM data
        ");*/

        let results = {
            let conn = self.make_conn().unwrap();

            ot::online_time_log
                .select((
                    ot::user_id,
                    ot::day,
                    sql::<Interval>(
                        r"
                        SUM(SUM(online_time_end - online_time_start))
                        OVER (PARTITION BY user_id ORDER BY day)
                    ",
                    ),
                ))
                .group_by((ot::user_id, ot::day))
                .load::<(String, NaiveDate, PgInterval)>(&conn)?
        };

        Ok(Self::collect_db_results(results))
    }

    pub fn flush_stats(&mut self) -> QueryResult<()> {
        use crate::model::NewLogEntryOwned;
        use crate::schema::online_time_log::dsl as ot;

        let conn = self.make_conn().unwrap();
        let now = Utc::now().naive_utc();
        let mut buf = Vec::new();

        for (uid, timestamp) in self.state.iter_mut() {
            let start = std::mem::replace(timestamp, now);

            let intervals = split_by_days(start, now)
                .into_iter()
                .map(|(day, start, end)| NewLogEntryOwned {
                    user_id: uid.to_string(),
                    day,
                    online_time_start: start,
                    online_time_end: end,
                });

            buf.extend(intervals);
        }

        diesel::insert_into(ot::online_time_log)
            .values(buf)
            .execute(&conn)?;

        Ok(())
    }

    pub fn user_now_offline(&mut self, uid: UserId) -> QueryResult<bool> {
        use crate::model::NewLogEntry;
        use crate::schema::online_time_log::dsl as ot;

        match self.state.remove(&uid) {
            None => Ok(false),
            Some(timestamp) => {
                let conn = self.make_conn().unwrap();
                let now = Utc::now().naive_utc();
                let uid = uid.to_string();

                let intervals = split_by_days(timestamp, now)
                    .into_iter()
                    .map(|(day, start, end)| NewLogEntry {
                        user_id: &uid,
                        day,
                        online_time_start: start,
                        online_time_end: end,
                    })
                    .collect::<Vec<_>>();

                diesel::insert_into(ot::online_time_log)
                    .values(intervals)
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

fn split_by_days(
    start: NaiveDateTime,
    end: NaiveDateTime,
) -> Vec<(NaiveDate, NaiveTime, NaiveTime)> {
    if start.date() == end.date() {
        vec![(start.date(), start.time(), end.time())]
    } else {
        let mut dbuf = vec![(start.date(), start.time(), NaiveTime::from_hms(23, 59, 59))];

        let mut d1 = NaiveDateTime::new(
            start.date() + Duration::days(1),
            NaiveTime::from_hms(0, 0, 0),
        );
        let mut d2 = NaiveDateTime::new(
            start.date() + Duration::days(1),
            NaiveTime::from_hms(23, 59, 59),
        );
        while d1.date() < end.date() {
            dbuf.push((d1.date(), d1.time(), d2.time()));
            d1 += Duration::days(1);
            d2 += Duration::days(1);
        }

        dbuf.push((d1.date(), d1.time(), end.time()));
        dbuf
    }
}

#[cfg(test)]
mod test {
    use super::split_by_days;
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
    use diesel::{debug_query, pg::Pg};

    #[test]
    fn query_test() {
        use super::*;
        use crate::schema::online_time_log::dsl::*;

        let q = online_time_log
            .select((
                user_id,
                day,
                sql::<PgInterval>(
                    r"
                    SUM(SUM(online_time_end - online_time_start))
                    OVER (PARTITION BY user_id ORDER BY day)
                ",
                ),
            ))
            .group_by((user_id, day));

        println!("{}", debug_query::<Pg, _>(&q));
    }

    #[test]
    fn test_dur_split() {
        let start = NaiveDateTime::new(
            NaiveDate::from_ymd(2021, 6, 16),
            NaiveTime::from_hms(12, 0, 0),
        );
        let end = NaiveDateTime::new(
            NaiveDate::from_ymd(2021, 6, 18),
            NaiveTime::from_hms(14, 0, 0),
        );

        let times = split_by_days(start, end);

        for (day, start, end) in times {
            println!(
                "day: {}; start: {}; end: {}",
                day.format("%Y.%m.%d"),
                start.format("%H:%M:%S"),
                end.format("%H:%M:%S")
            );
        }
    }
}
