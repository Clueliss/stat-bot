use chrono::{Date, Duration, Utc};
use plotters::{coord::Shift, prelude::*, series::LineSeries};
use serenity::model::id::UserId;
use std::collections::BTreeMap;
use std::ops::Range;

pub fn draw_time_graph<DB: DrawingBackend>(
    drawing_area: &mut DrawingArea<DB, Shift>,
    ctx: &BTreeMap<UserId, String>,
    caption: impl AsRef<str>,
    date_range: Range<Date<Utc>>,
    max_time: Duration,
    stats: BTreeMap<UserId, BTreeMap<Date<Utc>, Duration>>,
) {
    drawing_area.fill(&WHITE).unwrap();

    let mut chart = ChartBuilder::on(drawing_area)
        .caption(caption, (FontFamily::SansSerif, 50))
        .margin_left(30)
        .margin_right(30)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(
            date_range,
            0i64..max_time.num_hours(), /*(max_time / 60 / 60)*/
        )
        .unwrap();

    chart.configure_mesh().draw().unwrap();

    for (user, stst) in stats {
        if stst
            .last_key_value()
            .map(|(_d, t)| t.num_seconds() < max_time.num_hours() / 100)
            .unwrap_or(true)
        {
            continue;
        }

        let color = super::util::uid_to_color(&user);

        chart
            .draw_series(LineSeries::new(
                stst.into_iter()
                    .map(|(date, ontime)| (date, ontime.num_hours())),
                color.stroke_width(2),
            ))
            .unwrap()
            .label(ctx.get(&user).map(String::as_str).unwrap_or("[[invalid]]"))
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use plotters::prelude::{BitMapBackend, IntoDrawingArea};
    use crate::{graphing::util, stats::StatManager};

    #[test]
    fn draw_time_total_graph() {
        dotenv::dotenv().ok();
        let st = StatManager::new(std::env::var("DATABASE_URL").unwrap());

        let temppath = tempfile::Builder::new()
            .suffix(".png")
            .tempfile()
            .unwrap()
            .into_temp_path()
            .keep()
            .unwrap();

        let date_range = st.date_range().unwrap();
        let ctx = BTreeMap::new();

        let stats = st.absolute_sum_time_per_day().unwrap();
        let stats = util::fill_with_hold(&date_range, stats);

        let max_time = crate::graphing::util::max_time(&stats).unwrap();

        let mut drawing_area = BitMapBackend::new(&temppath, (1920, 1080)).into_drawing_area();

        super::draw_time_graph(
            &mut drawing_area,
            &ctx,
            "Time total",
            date_range,
            max_time,
            stats,
        );

        println!("Saved to: {}", &temppath.display());
    }

    #[test]
    fn draw_time_per_day_graph() {
        dotenv::dotenv().ok();
        let st = StatManager::new(std::env::var("DATABASE_URL").unwrap());

        let temppath = tempfile::Builder::new()
            .suffix(".png")
            .tempfile()
            .unwrap()
            .into_temp_path()
            .keep()
            .unwrap();

        let date_range = st.date_range().unwrap();

        let stats = st.time_per_day().unwrap();
        let stats = util::fill_with_zero(&date_range, stats);

        let max_time = crate::graphing::util::max_time(&stats).unwrap();
        let ctx = BTreeMap::new();

        let mut drawing_area = BitMapBackend::new(&temppath, (3840, 2160)).into_drawing_area();

        super::draw_time_graph(
            &mut drawing_area,
            &ctx,
            "Time per Day",
            date_range,
            max_time,
            stats,
        );

        println!("Saved to: {}", &temppath.display());
    }
}
