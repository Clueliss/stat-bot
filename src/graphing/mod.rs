use std::path::Path;

use plotters::coord::Shift;
use plotters::prelude::{DrawingArea, DrawingBackend};
pub use crate::graphing::stats::{StatResult, StatReadError};

mod draw;
mod stats;

pub fn time_total_graph_from_dir<I: AsRef<Path>, DB: DrawingBackend>(input_dir: I, canvas: &mut DrawingArea<DB, Shift>) -> StatResult<()> {
    let trans_file_path = input_dir.as_ref().join("trans.json");

    let dates = stats::available_datapoint_range(&input_dir)?;
    let trans = stats::get_translations(&trans_file_path)?;
    let st = stats::get_stats(&input_dir, dates.clone())?;
    draw::time_total_graph(canvas, st, trans, dates);

    Ok(())
}
