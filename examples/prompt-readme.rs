//! The input hero collage for the README, mirroring the C sparcli demo in a
//! slim form: a titled hero panel and a balanced multi-column dashboard of
//! statically rendered prompt frames (no boxes/hints, fuzzy as an inline list,
//! default theme).
//!
//! `cargo run --example prompt-readme --features fuzzy`
//!
//! Each prompt's frame is produced by its `frame()` method (no TTY, no
//! interaction); columns are stacked with `vstack` and joined with `Columns`.

use sparcli::prelude::*;
use sparcli::{
    Columns, Confirm, Date, DatePicker, NumberInput, Panel, PasswordInput,
    Select, TextInput, Textarea,
};

fn main() -> Result<()> {
    println!();
    hero()?;
    println!();
    dashboard()?;
    Ok(())
}

/// A blank one-line spacer for `vstack`.
fn blank() -> Rendered {
    Rendered::new(vec![Line::default()])
}

/// The top hero panel.
fn hero() -> Result<()> {
    let theme = theme();
    let body = Text::from(
        "Interactive prompts — confirm · select · text · password · number · \
         textarea · fuzzy · date.",
    );
    Panel::new(body)
        .border(BorderType::Rounded)
        .border_style(Style::new().fg(theme.accent))
        .title(
            Title::new(" sparcli · input widgets ")
                .align(Align::Center)
                .style(theme.title),
        )
        .content_align(Align::Center)
        .width(96)
        .print()
}

/// The balanced three-column dashboard of prompt frames.
fn dashboard() -> Result<()> {
    Columns::new()
        .add_rendered(left_column())
        .add_rendered(middle_column())
        .add_rendered(right_column())
        .gap(3)
        .separator(BorderType::Single)
        .print()
}

/// Left column: the confirm prompt with the input fields stacked beneath it,
/// then the notes textarea.
fn left_column() -> Rendered {
    vstack(
        &[
            Confirm::new("Deploy to production?").default_yes().frame(),
            blank(),
            TextInput::new("Service").initial("api-gateway").frame(),
            PasswordInput::new("Password").initial("hunter2").frame(),
            NumberInput::new("Replicas")
                .initial(3.0)
                .range(1.0, 10.0)
                .frame(),
            TextInput::new("Email")
                .placeholder("you@example.com")
                .frame(),
            TextInput::new("Region")
                .initial("eu-")
                .suggestions(["eu-central-1"])
                .frame(),
            blank(),
            Textarea::new("Notes")
                .initial("first line\nsecond line\nthird line")
                .frame(),
        ],
        0,
    )
}

/// Middle column: the single- and multi-select prompts.
fn middle_column() -> Rendered {
    vstack(
        &[
            Select::new("Environment")
                .options(["staging", "production", "local"])
                .cursor(1)
                .frame(),
            blank(),
            Select::new("Targets")
                .multi()
                .options(["web", "api", "worker", "db"])
                .checked([0, 2])
                .frame(),
        ],
        0,
    )
}

/// Right column: the fuzzy finder (feature `fuzzy`) above the date picker.
fn right_column() -> Rendered {
    let date = DatePicker::new("Release date")
        .initial(Date::new(2026, 5, 15))
        .frame();
    #[cfg(feature = "fuzzy")]
    {
        let fuzzy = sparcli::FuzzySelect::new("Language")
            .options(["C", "C++", "Rust", "Zig"])
            .query("ru")
            .frame();
        vstack(&[fuzzy, blank(), date], 0)
    }
    #[cfg(not(feature = "fuzzy"))]
    {
        date
    }
}
