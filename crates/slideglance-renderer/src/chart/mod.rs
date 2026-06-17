//! Chart element rendering.
//!
//! Direct port of. Covers
//! all 11 OOXML chart types — `bar` / `column`, `line`, `pie`,
//! `doughnut`, `area`, `scatter`, `bubble`, `radar`, `stock`,
//! `surface`, `ofPie` — plus combo charts and the six trendline
//! flavors (linear / exp / log / poly / power / movingAvg).

mod cartesian;
mod common;
mod radial;
mod render;
mod specialty;
mod trendline;

pub use render::{render_chart, ChartRenderResult};
