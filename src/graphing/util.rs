use std::{collections::BTreeMap, ops::Range};

use chrono::{Date, Duration, Utc};
use plotters::style::RGBColor;
use serenity::model::id::UserId;

pub fn max_time<'a>(
    stats: impl IntoIterator<Item = (&'a UserId, &'a BTreeMap<Date<Utc>, Duration>)>,
) -> Option<Duration> {
    stats
        .into_iter()
        .flat_map(|(_, stats)| stats.iter().map(|(_, ontime)| *ontime))
        .max()
}

pub fn uid_to_color(uid: &UserId) -> RGBColor {
    const D: u64 = 256;

    let uid = uid.0;
    let c = 10u64.pow(18) / (D.pow(3) + 1);
    let n = uid / c;
    let r = n / (D * D);
    let g = (n - (r * (D * D))) / D;
    let b = n - (r * (D * D)) - (g * D);

    RGBColor(r as u8, g as u8, b as u8)
}

pub fn fill_with_hold(
    date_range: &Range<Date<Utc>>,
    mut stats: BTreeMap<UserId, BTreeMap<Date<Utc>, Duration>>,
) -> BTreeMap<UserId, BTreeMap<Date<Utc>, Duration>> {
    for (_, stat) in &mut stats {
        let mut buf = BTreeMap::new();

        let mut cur_date = date_range.start;
        let mut last_value = Duration::zero();

        for (date, dur) in stat.iter() {
            while cur_date < *date {
                buf.insert(cur_date, last_value);
                cur_date = cur_date + Duration::days(1);
            }

            buf.insert(*date, *dur);
            cur_date = cur_date + Duration::days(1);
            last_value = *dur;
        }

        while cur_date < date_range.end {
            buf.insert(cur_date, last_value);
            cur_date = cur_date + Duration::days(1);
        }

        *stat = buf;
    }

    stats
}

pub fn fill_with_zero(
    date_range: &Range<Date<Utc>>,
    mut stats: BTreeMap<UserId, BTreeMap<Date<Utc>, Duration>>,
) -> BTreeMap<UserId, BTreeMap<Date<Utc>, Duration>> {
    for (_, stat) in &mut stats {
        let mut buf = BTreeMap::new();

        let mut cur_date = date_range.start;

        for (date, dur) in stat.iter() {
            while cur_date < *date {
                buf.insert(cur_date, Duration::zero());
                cur_date = cur_date + Duration::days(1);
            }

            buf.insert(*date, *dur);
            cur_date = cur_date + Duration::days(1);
        }

        while cur_date < date_range.end {
            buf.insert(cur_date, Duration::zero());
            cur_date = cur_date + Duration::days(1);
        }

        *stat = buf;
    }

    stats
}
