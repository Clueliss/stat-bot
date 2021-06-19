use std::collections::btree_map::{BTreeMap, Entry};
use std::fs::File;
use std::io::Read;
use std::num::ParseIntError;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use chrono::{Date, Duration, NaiveDate, NaiveTime, Utc};
use diesel::{Connection, PgConnection, RunQueryDsl};
use serenity::model::id::UserId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StatParseError {
    #[error("failed to parse user id")]
    UserIdParseError(#[from] ParseIntError),
    #[error("failed to parse json")]
    JsonParseError(#[from] serde_json::Error),
    #[error("io error")]
    IOError(#[from] std::io::Error),
}

fn get_stat_impl<StatR: Read>(srdr: StatR) -> Result<BTreeMap<UserId, Duration>, StatParseError> {
    let stats: BTreeMap<String, u64> = serde_json::from_reader(srdr)?;

    stats
        .into_iter()
        .map(|(uid, secs)| {
            let parsed_uid = uid.parse::<u64>()?;
            Ok((UserId::from(parsed_uid), Duration::seconds(secs as i64)))
        })
        .collect()
}

pub fn read_stats(stat_dir: impl AsRef<Path>) -> BTreeMap<NaiveDate, BTreeMap<UserId, Duration>> {
    let mut collection: BTreeMap<NaiveDate, BTreeMap<UserId, Duration>> = BTreeMap::new();

    for entry in std::fs::read_dir(stat_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        let filename = path.file_name().unwrap().to_str().unwrap();

        if filename.starts_with("stats_") && path.extension().map(|ex| ex == "json").unwrap_or(false) {
            let date = NaiveDate::from_str(&filename[6..][..10]).unwrap();
            let stats = get_stat_impl(File::open(path).unwrap()).unwrap();
            
            collection.insert(date, stats);
        }
    }

    let coll2 = collection.clone();

    for (date, stat) in &mut collection {
        if let Some(entry) = coll2.get(&(*date - Duration::days(1))) {
            for (uid, dur) in stat {
                if let Some(old_dur) = entry.get(uid) {
                    *dur = *dur - *old_dur;
                }

                if *dur > Duration::days(1) {
                    panic!();
                }
            }
        }
    }

    collection
}

#[test]
fn test_stats() {
    let dir = Path::new("/home/liss/Netzwerk/lpf-nas/interchange/stats");
    let st = read_stats(dir);

    println!("{:#?}", st);
}

#[test]
fn run_import() {
    dotenv::dotenv().ok();
    let url = std::env::var("DATABASE_URL").unwrap();
    let stat_dir = Path::new("/home/liss/Netzwerk/lpf-nas/interchange/stats_new/data");

    import(&url, stat_dir);
}

fn import(db_url: &str, stat_dir: impl AsRef<Path>) {
    use crate::schema::online_time_log::dsl as ot;
    use crate::model::NewLogEntryOwned;

    let stats = read_stats(stat_dir);

    let stats = stats.into_iter()
        .flat_map(|(date, stat)| std::iter::repeat(date).zip(stat.into_iter()))
        .filter(|(_, (_, dur))| *dur > Duration::zero())
        .map(|(day, (uid, dur))| NewLogEntryOwned {
            user_id: uid.to_string(),
            day,
            online_time_start: NaiveTime::from_hms(0, 0, 0),
            online_time_end: NaiveTime::from_hms(0, 0, 0) + dur,
        })
        .collect::<Vec<_>>();

    let conn = PgConnection::establish(db_url).unwrap();

    diesel::insert_into(ot::online_time_log)
        .values(stats)
        .execute(&conn)
        .unwrap();
}
