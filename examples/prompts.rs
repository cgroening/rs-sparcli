//! Interactive prompt examples covering every input widget. Run in a real
//! terminal: `cargo run --example prompts --features fuzzy`.
//!
//! Each prompt runs in turn; pressing Esc (or Ctrl-C) cancels and ends the
//! demo cleanly.

use sparcli::event::{KeyCode, KeyPress};
use sparcli::shortcut::{self, Shortcut};
use sparcli::validate::{alnum, non_empty};
use sparcli::{
    Alert, Confirm, DatePicker, NumberInput, Outcome, PasswordInput,
    Renderable, Rendered, Select, TextInput, Textarea,
};

#[cfg(feature = "fuzzy")]
use sparcli::FuzzySelect;

/// Unwraps a submitted value or ends the demo on cancellation.
macro_rules! get {
    ($prompt:expr) => {
        match $prompt {
            Outcome::Submitted(value) => value,
            _ => return cancelled(),
        }
    };
}

fn main() -> sparcli::Result<()> {
    footer_hint()?;

    let name = get!(
        TextInput::new("Your name?")
            .placeholder("e.g. Alice")
            .validate(non_empty())
            .run()?
    );

    let _username = get!(
        TextInput::new("Username?")
            .char_filter(alnum())
            .max_chars(16)
            .suggestions(["alice", "albert", "bob", "carol"])
            .history(["alice", "bob"])
            .run()?
    );

    let _password = get!(PasswordInput::new("Password?").mask("•").run()?);

    let age = get!(
        NumberInput::new("Age? (try `= 20 + 2`)")
            .range(0.0, 130.0)
            .calculator()
            .run()?
    );

    let _bio = get!(Textarea::new("Short bio (Ctrl-D to submit):").run()?);

    let color = get!(
        Select::new("Favorite color?")
            .options(["red", "green", "blue"])
            .run()?
    );

    let toppings = get!(
        Select::new("Pick toppings (Space to toggle):")
            .options(["cheese", "mushroom", "olive", "onion"])
            .multi()
            .run_multi()?
    );

    let _fruit = get!(pick_fruit()?);

    let date = get!(DatePicker::new("Pick a date:").run()?);

    let confirmed = matches!(
        Confirm::new("Save everything?")
            .default_yes()
            .labels("Save", "Discard")
            .run()?,
        Outcome::Submitted(true)
    );

    if confirmed {
        Alert::success(format!(
            "Saved {name}, age {age:.0}, color #{color}, {} toppings, \
             date {}-{:02}-{:02}.",
            toppings.len(),
            date.year,
            date.month,
            date.day,
        ))
        .print()?;
    } else {
        cancelled()?;
    }
    Ok(())
}

/// Runs the fuzzy picker when the feature is enabled.
#[cfg(feature = "fuzzy")]
fn pick_fruit() -> sparcli::Result<Outcome<usize>> {
    FuzzySelect::new("Find a fruit:")
        .options(["apple", "apricot", "banana", "cherry", "grape"])
        .run()
}

/// Stand-in that submits a default when the `fuzzy` feature is disabled.
#[cfg(not(feature = "fuzzy"))]
fn pick_fruit() -> sparcli::Result<Outcome<usize>> {
    Ok(Outcome::Submitted(0))
}

/// Prints a footer-style key hint line above the prompts.
fn footer_hint() -> sparcli::Result<()> {
    let shortcuts = [
        Shortcut::new(KeyPress::new(KeyCode::Enter), 1, "submit"),
        Shortcut::new(KeyPress::new(KeyCode::Esc), 2, "cancel"),
    ];
    Rendered::new(vec![shortcut::hint_line(&shortcuts)]).print()
}

/// Prints a cancellation notice.
fn cancelled() -> sparcli::Result<()> {
    Alert::warning("Cancelled.").print()
}
