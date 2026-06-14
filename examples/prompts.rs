//! Interactive prompt examples. Run with `cargo run --example prompts`
//! in a real terminal (needs a TTY).

use sparcli::input::validate::non_empty;
use sparcli::{
    Alert, Confirm, NumberInput, Outcome, PasswordInput, Renderable, Select,
    TextInput,
};

fn main() -> sparcli::Result<()> {
    let name = match TextInput::new("Your name?")
        .placeholder("e.g. Alice")
        .validate(non_empty())
        .run()?
    {
        Outcome::Submitted(name) => name,
        Outcome::Cancelled => return cancelled(),
    };

    let _password = match PasswordInput::new("Password?").run()? {
        Outcome::Submitted(value) => value,
        Outcome::Cancelled => return cancelled(),
    };

    let age = match NumberInput::new("Age?").range(0.0, 130.0).run()? {
        Outcome::Submitted(age) => age,
        Outcome::Cancelled => return cancelled(),
    };

    let color = match Select::new("Favorite color?")
        .options(["red", "green", "blue"])
        .run()?
    {
        Outcome::Submitted(index) => index,
        Outcome::Cancelled => return cancelled(),
    };

    if let Outcome::Submitted(true) =
        Confirm::new("Save?").default_yes().run()?
    {
        Alert::success(format!("Saved {name}, age {age:.0}, color #{color}."))
            .print()?;
    } else {
        cancelled()?;
    }
    Ok(())
}

/// Prints a cancellation notice.
fn cancelled() -> sparcli::Result<()> {
    Alert::warning("Cancelled.").print()
}
