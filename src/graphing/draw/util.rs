use std::collections::BTreeMap;

use chrono::{Date, Utc};
use plotters::style::RGBColor;

pub fn split_stats(
    stats: Vec<(Date<Utc>, BTreeMap<String, u64>)>,
) -> BTreeMap<String, Vec<(Date<Utc>, u64)>> {
    let mut buf: BTreeMap<String, Vec<(Date<Utc>, u64)>> = BTreeMap::new();

    for (date, mpsd) in stats {
        for (user, time) in mpsd {
            match buf.get_mut(&user) {
                Some(entry) => entry.push((date, time)),
                None => {
                    buf.insert(user, vec![(date, time)]);
                }
            }
        }
    }

    buf
}

pub fn max_time(stats: &[(Date<Utc>, BTreeMap<String, u64>)]) -> Option<u64> {
    stats
        .iter()
        .flat_map(|(_date, stats)| stats.iter().map(|(_user, ontime)| ontime))
        .max()
        .cloned()
}

pub fn uid_to_color(uid: &str) -> RGBColor {
    const D: u64 = 256;

    let uid = uid.parse::<u64>().unwrap();
    let c = 10u64.pow(18) / (D.pow(3) + 1);
    let n = uid / c;
    let r = n / (D * D);
    let g = (n - (r * (D * D))) / D;
    let b = n - (r * (D * D)) - (g * D);

    RGBColor(r as u8, g as u8, b as u8)
}
