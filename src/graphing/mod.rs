use std::{collections::BTreeMap, ops::Range};

use chrono::{Date, Duration, Utc};
use plotters::{
    coord::Shift,
    prelude::{DrawingArea, DrawingBackend},
};
use serenity::{client::Context, model::id::UserId};

mod draw;
mod util;

fn draw_graph<DB: DrawingBackend>(
    drawing_area: &mut DrawingArea<DB, Shift>,
    ctx: &Context,
    caption: impl AsRef<str>,
    date_range: Range<Date<Utc>>,
    stats: BTreeMap<UserId, BTreeMap<Date<Utc>, Duration>>,
) {
    let max_time = util::max_time(&stats).unwrap();

    let ctx = stats
        .keys()
        .map(|u| {
            let uname = u
                .to_user(ctx)
                .map(|u| u.name)
                .unwrap_or("[[invalid]]".to_owned());
            (*u, uname)
        })
        .collect::<BTreeMap<UserId, String>>();

    draw::draw_time_graph(drawing_area, &ctx, caption, date_range, max_time, stats);
}

pub fn draw_time_total_graph<DB: DrawingBackend>(
    drawing_area: &mut DrawingArea<DB, Shift>,
    ctx: &Context,
    caption: impl AsRef<str>,
    date_range: Range<Date<Utc>>,
    stats: BTreeMap<UserId, BTreeMap<Date<Utc>, Duration>>,
) {
    let stats = util::fill_with_hold(&date_range, stats);
    draw_graph(drawing_area, ctx, caption, date_range, stats);
}

pub fn draw_time_per_day_graph<DB: DrawingBackend>(
    drawing_area: &mut DrawingArea<DB, Shift>,
    ctx: &Context,
    caption: impl AsRef<str>,
    date_range: Range<Date<Utc>>,
    stats: BTreeMap<UserId, BTreeMap<Date<Utc>, Duration>>,
) {
    let stats = util::fill_with_zero(&date_range, stats);
    draw_graph(drawing_area, ctx, caption, date_range, stats);
}
