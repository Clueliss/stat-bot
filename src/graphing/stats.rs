use std::collections::BTreeMap;
use std::fs::File;
use std::ops::Range;
use std::path::Path;

use chrono::{Date, NaiveDate, NaiveDateTime, Utc};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StatReadError {
    #[error("json parse error")]
    JsonParseError(#[from] serde_json::Error),
    #[error("io error")]
    IOError(#[from] std::io::Error),
}

pub type StatResult<T> = Result<T, StatReadError>;

pub fn available_datapoint_range<P: AsRef<Path>>(path: P) -> std::io::Result<Range<Date<Utc>>> {
    let mut min_date = Utc::today();
    let mut max_date = Date::from_utc(NaiveDateTime::from_timestamp(0, 0).date(), Utc);

    for e in std::fs::read_dir(path)? {
        let path = e?.path();

        if !path.is_file() {
            continue;
        }

        let filestem = path.file_stem().unwrap().to_string_lossy().into_owned();

        if !filestem.starts_with("stats_") {
            continue;
        }

        match NaiveDate::parse_from_str(&filestem[6..], "%Y-%m-%d").map(|d| Date::from_utc(d, Utc))
        {
            Ok(date) => {
                if date < min_date {
                    min_date = date.clone();
                }

                if date > max_date {
                    max_date = date;
                }
            }
            Err(_) => (),
        }
    }

    Ok(min_date..max_date.succ())
}

pub fn get_translations<P: AsRef<Path>>(path: P) -> StatResult<BTreeMap<String, String>> {
    let f = File::open(path)?;
    serde_json::from_reader(f).map_err(Into::into)
}

fn get_stat<P: AsRef<Path>>(path: P) -> StatResult<BTreeMap<String, u64>> {
    let f = File::open(path)?;
    serde_json::from_reader(f).map_err(Into::into)
}

pub fn get_stats<P: AsRef<Path>>(
    path: P,
    dates: Range<Date<Utc>>,
) -> StatResult<Vec<(Date<Utc>, BTreeMap<String, u64>)>> {
    let date_iter = std::iter::successors(Some(dates.start), |date| {
        if date < &dates.end {
            Some(date.succ())
        } else {
            None
        }
    });

    let mut buf = Vec::new();

    for date in date_iter {
        let filepath = path
            .as_ref()
            .join(format!("stats_{}.json", date.format("%Y-%m-%d")));

        match get_stat(&filepath).map(|s| (date, s)) {
            Ok(s) => buf.push(s),
            Err(StatReadError::IOError(e)) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e),
        }
    }

    Ok(buf)
}
