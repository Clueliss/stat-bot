use chrono::{Date, Utc};
use plotters::coord::Shift;
use plotters::prelude::*;
use plotters::series::LineSeries;
use std::collections::BTreeMap;
use std::ops::Range;

mod util;

pub fn time_total_graph<DB: DrawingBackend>(
    canvas: &mut DrawingArea<DB, Shift>,
    stats: Vec<(Date<Utc>, BTreeMap<String, u64>)>,
    trans: BTreeMap<String, String>,
    dates: Range<Date<Utc>>,
) {
    let max_time = util::max_time(&stats).unwrap_or(0);
    let stats = util::split_stats(stats);

    canvas.fill(&WHITE).unwrap();

    let mut chart = ChartBuilder::on(&canvas)
        .caption("Time total", (FontFamily::SansSerif, 50))
        .margin_left(30)
        .margin_right(30)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(dates, 0u64..(max_time / 60 / 60))
        .unwrap();

    chart.configure_mesh().draw().unwrap();

    for (user, stst) in stats {
        if stst
            .last()
            .map(|(_d, t)| *t < max_time / 100)
            .unwrap_or(true)
        {
            continue;
        }

        let color = util::uid_to_color(&user);

        chart
            .draw_series(LineSeries::new(
                stst.into_iter().map(|(date, ontime)| (date, ontime / 60 / 60)),
                color.stroke_width(2),
            ))
            .unwrap()
            .label(
                trans
                    .get(&user)
                    .map(String::as_str)
                    .unwrap_or("[[untranslatable]]"),
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
