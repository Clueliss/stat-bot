use std::collections::BTreeMap;

use chrono::{Date, Duration, Utc};
use plotters::style::RGBColor;
use serenity::model::id::UserId;

pub fn max_time<'a>(
    stats: impl IntoIterator<Item = (&'a UserId, &'a Vec<(Date<Utc>, Duration)>)>,
) -> Option<i64> {
    stats
        .into_iter()
        .flat_map(|(_, stats)| stats.iter().map(|(_, ontime)| ontime))
        .max()
        .map(|d| d.num_seconds())
}

pub fn group_by_users(
    mut stats: Vec<(UserId, Date<Utc>, Duration)>,
) -> BTreeMap<UserId, Vec<(Date<Utc>, Duration)>> {
    stats.sort_by(|(uid1, _, _), (uid2, _, _)| uid1.cmp(uid2));

    let by_users = stats.group_by(|(u1, _, _), (u2, _, _)| u1 == u2);

    by_users
        .map(|x| {
            (
                x[0].0,
                x.iter().map(|(_, date, dur)| (*date, *dur)).collect(),
            )
        })
        .collect::<BTreeMap<UserId, Vec<(Date<Utc>, Duration)>>>()
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

pub fn min_max<T: Ord + Copy>(mut i: impl Iterator<Item = T>) -> Option<(T, T)> {
    let first = i.next()?;

    Some(i.fold((first, first), |(min, max), x| {
        let min = std::cmp::min(min, x);
        let max = std::cmp::max(max, x);

        (min, max)
    }))
}
