use chrono::{Date, Duration, Utc};
use plotters::{coord::Shift, prelude::*, series::LineSeries};
use serenity::{client::Context, model::id::UserId};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::ops::Range;

pub fn time_total_graph<DB: DrawingBackend>(
    canvas: &mut DrawingArea<DB, Shift>,
    ctx: &Context,
    stats: BTreeMap<UserId, Vec<(Date<Utc>, Duration)>>,
    date_range: Range<Date<Utc>>,
) {
    let max_time = super::util::max_time(&stats).unwrap_or(0);

    canvas.fill(&WHITE).unwrap();

    let mut chart = ChartBuilder::on(canvas)
        .caption("Time total", (FontFamily::SansSerif, 50))
        .margin_left(30)
        .margin_right(30)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(date_range, 0i64..(max_time / 60 / 60))
        .unwrap();

    chart.configure_mesh().draw().unwrap();

    for (user, stst) in stats {
        if stst
            .last()
            .map(|(_d, t)| t.num_seconds() < max_time / 100)
            .unwrap_or(true)
        {
            continue;
        }

        let color = super::util::uid_to_color(&user);

        chart
            .draw_series(LineSeries::new(
                stst.into_iter()
                    .map(|(date, ontime)| (date, ontime.num_seconds() / 60 / 60)),
                color.stroke_width(2),
            ))
            .unwrap()
            .label(
                user.to_user(ctx)
                    .map(|u| Cow::Owned(u.name))
                    .unwrap_or(Cow::Borrowed("[[invalid]]")),
            )
            .legend(move |(x, y)| {
                PathElement::new(vec![(x, y), (x + 20, y)], color.stroke_width(2))
            });
    }

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()
        .unwrap();
}
