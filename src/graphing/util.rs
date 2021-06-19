use chrono::{Date, Duration, Utc};
use plotters::style::RGBColor;
use serenity::model::id::UserId;

pub fn max_time<'a>(
    stats: impl IntoIterator<Item = (&'a UserId, &'a Vec<(Date<Utc>, Duration)>)>,
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
