//! The input hero collage for the README, mirroring the C sparcli demo in a
//! slim form: a titled hero panel and a balanced multi-column dashboard of
//! statically rendered prompt frames (no boxes/hints, default theme).
//!
//! `cargo run --example prompt-readme`
//!
//! Each prompt's frame is produced by its `frame()` method (no TTY, no
//! interaction); columns are stacked with `vstack` and joined with `Columns`.
//! The fuzzy column is composed directly so the example needs no extra
//! features.

use sparcli::prelude::*;
use sparcli::{
    Columns, Confirm, Date, DatePicker, NumberInput, Panel, PasswordInput,
    Select, TextInput, Textarea,
};

fn main() -> Result<()> {
    println!();
    let board = dashboard();
    hero(board.width() as u16)?;
    println!();
    board.print()?;
    Ok(())
}

/// A blank one-line spacer for `vstack`.
fn blank() -> Rendered {
    Rendered::new(vec![Line::default()])
}

/// The top hero panel, sized to match the dashboard width.
fn hero(width: u16) -> Result<()> {
    let theme = theme();
    let body = Text::new(vec![
        Line::raw("Interactive prompts — confirm · select · text · password ·"),
        Line::raw("number · textarea · fuzzy · date."),
    ]);
    Panel::new(body)
        .border(BorderType::Rounded)
        .border_style(Style::new().fg(theme.accent))
        .title(
            Title::new(" sparcli · input widgets ")
                .align(Align::Center)
                .style(theme.title),
        )
        .content_align(Align::Center)
        .width(width)
        .print()
}

/// The balanced three-column dashboard of prompt frames.
fn dashboard() -> Rendered {
    Columns::new()
        .add_rendered(left_column())
        .add_rendered(middle_column())
        .add_rendered(calendar_column())
        .gap(3)
        .separator(BorderType::Single)
        .render(0)
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

/// Right column: the date picker with the fuzzy finder beneath it.
fn calendar_column() -> Rendered {
    let date = DatePicker::new("Release date")
        .initial(Date::new(2026, 5, 15))
        .frame();
    vstack(&[date, blank(), fuzzy_block()], 0)
}

/// A composed snapshot of an inline fuzzy finder (no `fuzzy` feature needed
/// for this static screenshot).
fn fuzzy_block() -> Rendered {
    let theme = theme();
    Rendered::new(vec![
        Line::new(vec![
            Span::styled("Language ".to_string(), theme.title),
            Span::raw("ru"),
            Span::styled(" ".to_string(), theme.cursor),
        ]),
        Line::new(vec![
            Span::styled("‣ ".to_string(), theme.selection),
            Span::styled("Rust".to_string(), theme.selection),
        ]),
    ])
}
