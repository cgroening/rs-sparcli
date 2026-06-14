//! A non-interactive collage of input prompts for a README screenshot.
//!
//! `cargo run --example prompt-readme --features fuzzy`
//!
//! Mirrors the C sparcli `readme_screenshots_input` demo: each prompt's static
//! frame is rendered via its `frame()` method (no TTY, no interaction) and the
//! frames are composed side by side.

use sparcli::prelude::*;
use sparcli::{
    Columns, Confirm, Date, DatePicker, NumberInput, PasswordInput, Select,
    TextInput, Textarea,
};

fn main() -> Result<()> {
    println!();
    choices()?;
    println!();
    fields()?;
    println!();
    rich()?;
    Ok(())
}

/// Row 1: confirm, single-select, multi-select.
fn choices() -> Result<()> {
    let confirm = Confirm::new("Deploy to production?").default_yes().frame();
    let single = Select::new("Environment")
        .options(["staging", "production", "local"])
        .cursor(1)
        .frame();
    let multi = Select::new("Targets")
        .multi()
        .options(["web", "api", "worker", "db"])
        .checked([0, 2])
        .frame();
    Columns::new()
        .add_rendered(confirm)
        .add_rendered(single)
        .add_rendered(multi)
        .gap(3)
        .separator(BorderType::Single)
        .print()
}

/// Row 2: text, password, number and a ghost-autocomplete field.
fn fields() -> Result<()> {
    let text = TextInput::new("Service").initial("api-gateway").frame();
    let password = PasswordInput::new("Password").initial("hunter2").frame();
    let number = NumberInput::new("Replicas").initial(3.0).frame();
    let region = TextInput::new("Region")
        .initial("eu-")
        .suggestions(["eu-central-1"])
        .frame();
    Columns::new()
        .add_rendered(text)
        .add_rendered(password)
        .add_rendered(number)
        .add_rendered(region)
        .gap(3)
        .separator(BorderType::Single)
        .print()
}

/// Row 3: textarea, fuzzy finder (feature `fuzzy`) and a date picker.
fn rich() -> Result<()> {
    let textarea = Textarea::new("Notes")
        .initial("first line\nsecond line")
        .frame();
    let date = DatePicker::new("Release date")
        .initial(Date::new(2026, 5, 15))
        .frame();

    let columns = Columns::new()
        .gap(3)
        .separator(BorderType::Single)
        .add_rendered(textarea);
    #[cfg(feature = "fuzzy")]
    let columns = columns.add_rendered(
        sparcli::FuzzySelect::new("Language")
            .options(["C", "C++", "Rust", "Zig"])
            .query("ru")
            .frame(),
    );
    columns.add_rendered(date).print()
}
