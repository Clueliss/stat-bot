use std::{collections::BTreeMap, ops::Range};

use chrono::{Date, Duration, Utc};
use plotters::{
    coord::Shift,
    prelude::{DrawingArea, DrawingBackend},
};
use serenity::{client::Context, model::id::UserId};

mod draw;
mod util;

pub fn draw_graph<DB: DrawingBackend>(
    drawing_area: &mut DrawingArea<DB, Shift>,
    ctx: &Context,
    caption: impl AsRef<str>,
    date_range: Range<Date<Utc>>,
    stats: BTreeMap<UserId, Vec<(Date<Utc>, Duration)>>,
) {
    let max_time = util::max_time(&stats).unwrap();

    draw::draw_time_graph(drawing_area, ctx, caption, date_range, max_time, stats);
}
