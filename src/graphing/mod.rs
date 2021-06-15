use chrono::{Date, Duration, Utc};
use plotters::{
    coord::Shift,
    prelude::{DrawingArea, DrawingBackend},
};
use serenity::{client::Context, model::id::UserId};

use self::util::min_max;

mod draw;
mod util;

pub fn time_total_graph<DB: DrawingBackend>(
    stats: impl IntoIterator<Item = (UserId, Date<Utc>, Duration)>,
    ctx: &Context,
    canvas: &mut DrawingArea<DB, Shift>,
) {
    let times = stats.into_iter().collect::<Vec<_>>();

    let (&min_date, &max_date) = min_max(times.iter().map(|(_, date, _)| date)).unwrap();
    let date_range = min_date..max_date + Duration::days(1);

    let dates = util::group_by_users(times);

    draw::time_total_graph(canvas, ctx, dates, date_range);
}
