//! Time-based and interactive output widgets: spinner, progress, multi-progress
//! live display and pager. Run in a real terminal:
//!
//! `cargo run --example output_dynamic --features pager`
//!
//! Off-terminal (piped/redirected) the animations become no-ops and only the
//! final state is printed; the pager prints its content directly.

use std::thread::sleep;
use std::time::Duration;

use sparcli::prelude::*;
use sparcli::{Live, MultiProgress, ProgressBar, Spinner};

fn main() -> Result<()> {
    spinner()?;
    progress()?;
    multi_progress()?;
    live()?;
    pager()?;
    Ok(())
}

/// Animates a spinner for a moment, then finishes with a success marker.
fn spinner() -> Result<()> {
    let mut spinner = Spinner::new("working");
    for _ in 0..12 {
        spinner.tick()?;
        sleep(Duration::from_millis(80));
    }
    spinner.finish(true, "done")
}

/// Fills a single progress bar from 0 to 100 %.
fn progress() -> Result<()> {
    let mut bar = ProgressBar::new().label("download").width(30);
    for step in 0..=20 {
        bar.draw(f64::from(step), 20.0)?;
        sleep(Duration::from_millis(50));
    }
    bar.finish(20.0, 20.0)
}

/// Advances two bars together in a single block.
fn multi_progress() -> Result<()> {
    let mut multi = MultiProgress::new();
    let downloads = multi.add(ProgressBar::new().label("downloads").width(20));
    let installs = multi.add(ProgressBar::new().label("installs ").width(20));
    for step in 0..=20 {
        multi.update(downloads, f64::from(step), 20.0)?;
        multi.update(installs, f64::from(step) * 0.6, 20.0)?;
        sleep(Duration::from_millis(60));
    }
    multi.finish()
}

/// Redraws a panel in place a few times.
fn live() -> Result<()> {
    let mut live = Live::new();
    for tick in 1..=8 {
        let frame = Panel::new(format!("live tick {tick}"))
            .title(Title::new("Live"))
            .render(30);
        live.update(&frame)?;
        sleep(Duration::from_millis(120));
    }
    live.finish()
}

/// Pages a long block of lines (only with the `pager` feature).
#[cfg(feature = "pager")]
fn pager() -> Result<()> {
    use sparcli::Pager;
    let lines = (1..=200)
        .map(|n| Line::raw(format!("line {n}")))
        .collect::<Vec<_>>();
    Pager::new().page(&Rendered::new(lines))
}

/// No-op when the `pager` feature is disabled.
#[cfg(not(feature = "pager"))]
fn pager() -> Result<()> {
    Ok(())
}
