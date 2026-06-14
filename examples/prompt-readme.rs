//! The input hero collage for the README, mirroring the C sparcli demo in a
//! slim form: a titled hero panel and three rows of statically rendered prompt
//! frames (no boxes/hints, fuzzy as an inline list, default theme).
//!
//! `cargo run --example prompt-readme --features fuzzy`
//!
//! Each prompt's frame is produced by its `frame()` method (no TTY, no
//! interaction) and composed side by side.

use sparcli::prelude::*;
use sparcli::{
    Columns, Confirm, Date, DatePicker, NumberInput, Panel, PasswordInput,
    Select, TextInput, Textarea,
};

fn main() -> Result<()> {
    println!();
    hero()?;
    println!();
    choices()?;
    println!();
    fields()?;
    println!();
    rich()?;
    Ok(())
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

/// Row 1: confirm, single-select, multi-select.
fn choices() -> Result<()> {
    let confirm = Confirm::new("Deploy to production?").default_yes().frame();
    let environment = Select::new("Environment")
        .options(["staging", "production", "local"])
        .cursor(1)
        .frame();
    let targets = Select::new("Targets")
        .multi()
        .options(["web", "api", "worker", "db"])
        .checked([0, 2])
        .frame();
    Columns::new()
        .add_rendered(confirm)
        .add_rendered(environment)
        .add_rendered(targets)
        .gap(3)
        .separator(BorderType::Single)
        .print()
}

/// Row 2: text, password, number, placeholder and ghost-autocomplete fields.
fn fields() -> Result<()> {
    let service = TextInput::new("Service").initial("api-gateway").frame();
    let password = PasswordInput::new("Password").initial("hunter2").frame();
    let replicas = NumberInput::new("Replicas")
        .initial(3.0)
        .range(1.0, 10.0)
        .frame();
    let email = TextInput::new("Email")
        .placeholder("you@example.com")
        .frame();
    let region = TextInput::new("Region")
        .initial("eu-")
        .suggestions(["eu-central-1"])
        .frame();
    Columns::new()
        .add_rendered(service)
        .add_rendered(password)
        .add_rendered(replicas)
        .add_rendered(email)
        .add_rendered(region)
        .gap(3)
        .separator(BorderType::Single)
        .print()
}

/// Row 3: textarea, fuzzy finder (feature `fuzzy`) and a date picker.
fn rich() -> Result<()> {
    let notes = Textarea::new("Notes")
        .initial("first line\nsecond line\nthird line")
        .frame();
    let date = DatePicker::new("Release date")
        .initial(Date::new(2026, 5, 15))
        .frame();

    let columns = Columns::new()
        .gap(3)
        .separator(BorderType::Single)
        .add_rendered(notes);
    #[cfg(feature = "fuzzy")]
    let columns = columns.add_rendered(
        sparcli::FuzzySelect::new("Language")
            .options(["C", "C++", "Rust", "Zig"])
            .query("ru")
            .frame(),
    );
    columns.add_rendered(date).print()
}
