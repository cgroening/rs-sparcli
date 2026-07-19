# sparcli

[![Crates.io](https://img.shields.io/crates/v/sparcli.svg)](https://crates.io/crates/sparcli) [![Docs.rs](https://docs.rs/sparcli/badge.svg)](https://docs.rs/sparcli) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE) [![MSRV](https://img.shields.io/badge/MSRV-1.88-blue.svg)](https://www.rust-lang.org)

A lightweight, cross-platform toolkit for **styled CLI output** and **interactive input widgets** in Rust. It renders directly to the terminal via [`crossterm`](https://crates.io/crates/crossterm) (no `ratatui` dependency) but mirrors ratatui's familiar vocabulary (`Style`, `Color`, `Span`, `Line`, `Text`), so the API feels at home if you already use ratatui.

sparcli is meant for small, lightweight CLI tools: a single accent color, muted defaults, rounded borders, graceful `NO_COLOR` and non-terminal behavior. Heavy, full-screen/retained TUIs are out of scope (that is what ratatui is for).

![sparcli output widgets](https://raw.githubusercontent.com/cgroening/rs-sparcli/main/images/screenshot-1.png)

![sparcli input widgets](https://raw.githubusercontent.com/cgroening/rs-sparcli/main/images/screenshot-2.png)

## Highlights

- **Output**: styled text, markup, tables (colspan, striping, wrap, titles), panels, cards (a filled surface whose whole palette is derived from one accent color), alerts, rules, lists, trees, key-value lists, badges, progress bars, spinners, multi-progress, diffs, columns, live display, pager, and composition helpers (`align`, `pad`, `vstack`).
- **Input**: confirm, text (validation, char filters, history, ghost autocomplete), password, number (with a calculator), textarea, single/multi select, an inline fuzzy select, and a calendar date picker.
- **Unified theme** for input *and* output, set once and overridable per call.
- **Robust**: no panics on input, RAII terminal restore, `Result`-based errors.

## Install

```toml
[dependencies]
sparcli = "0.2"
# Opt-in features (the base stays small):
# sparcli = { version = "0.2", features = ["markup", "fuzzy", "pager"] }
```

MSRV: Rust 1.88 (edition 2024).

| Feature  | Adds                                              |
| -------- | ------------------------------------------------- |
| `markup` | `[bold red]…[/]` inline markup parsing            |
| `fuzzy`  | inline fuzzy-select (pulls in `nucleo-matcher`)   |
| `pager`  | paging via `$PAGER` / `less` / `more`             |
| `full`   | enables `markup`, `fuzzy` and `pager`             |

### From a local checkout or Git

To use an unpublished or local copy instead of the crates.io release, point Cargo at the source directory or repository:

```toml
[dependencies]
# Local path (absolute or relative to your Cargo.toml):
sparcli = { path = "../sparcli" }
# Straight from Git:
# sparcli = { git = "https://github.com/cgroening/rs-sparcli", branch = "main" }
```

## Output example

```rust
use sparcli::prelude::*;
use sparcli::{Alert, Table};

fn main() -> sparcli::Result<()> {
    Alert::success("Build finished.").print()?;

    Table::new()
        .columns(["Name", "Status"])
        .row(["web-1", "online"])
        .row(["db-1", "online"])
        .striped(true)
        .print()?;
    Ok(())
}
```

Every output widget implements `Renderable`: call `.print()` to write to stdout, or `.print_to(&mut writer)` to capture output. When stdout is not a terminal (a pipe, a file, or with `NO_COLOR`), no escape codes are emitted.

Widgets like `Panel` frame their content with a rounded border and an optional title:

```rust
use sparcli::{Panel, Renderable, Title};

Panel::new("All systems nominal.")
    .title(Title::new("Status"))
    .print()?;
```

```text
╭─ Status ─────────────╮
│ All systems nominal. │
╰──────────────────────╯
```

A left-aligned title reads as part of the frame: one connecting border glyph sits before it (`╭─ Status ─`), never a flush `╭ Status` - unless the title is too wide for the frame, in which case it is truncated into the border rather than widening the panel.

Where `Panel` draws a frame, `Card` fills a surface. It takes a single accent color and derives everything else from it - the title stays saturated, the body text and both backgrounds become desaturated, darker shades of the same hue:

```rust
use sparcli::{Card, Color, Renderable};

Card::new("Deployed to production.")
    .title("Release 1.4.0")
    .footer("3 minutes ago")
    .accent(Color::from_hex("#a6e3a1").unwrap_or(Color::Green))
    .print()?;
```

A card fills the width it is rendered into, wraps its content, and carries no border unless you add one with `.border(...)`; the title and footer sit on rows of their own, each with its own background. Padding is set separately for the title row, the body and the footer. Individual setters (`.title_style()`, `.fill()`, `.content_style()`, …) patch the derived values rather than replacing them, so adding an attribute keeps the derived colors.

Because the derived shades all collapse onto the same ANSI-16 color, a card drops its backgrounds below truecolor support and renders as accented text instead - readable everywhere, rather than an unreadable block.

`BorderType::Tall` is the one border a card draws natively - a thin block frame around the filled surface, following the geometry of Textual's `wide` border:

```rust
use sparcli::{BorderType, Card, Renderable};

Card::new("A thin block frame bounds the surface.")
    .title("Tall")
    .border(BorderType::Tall)
    .print()?;
```

The side bars ink a quarter of their cell's width and the top and bottom lines an eighth of their cell's height - the same number of pixels, since a terminal cell is about twice as tall as it is wide. The horizontal lines run across the corner cells too, so the corners close.

Only `Card` renders it that way, because the bars need a filled surface to read against; `Panel`, `Table` and the other framed widgets receive the heavy line glyphs instead. The same degradation applies to a card without truecolor support, and to `Ascii` when the theme disables Unicode.

## Input example

```rust
use sparcli::validate::non_empty;
use sparcli::{Confirm, Outcome, TextInput};

fn main() -> sparcli::Result<()> {
    if let Outcome::Submitted(name) =
        TextInput::new("Your name?").validate(non_empty()).run()?
    {
        if let Outcome::Submitted(true) = Confirm::new("Continue?").run()? {
            println!("Hello, {name}!");
        }
    }
    Ok(())
}
```

Prompts return `Outcome<T>` (`Submitted(value)`, `Cancelled`, or a fired `Shortcut(id)`) and never panic. They require an interactive terminal; without one they return `SparcliError::NoTerminal`.

## Theming

```rust
use sparcli::{Color, Theme, set_theme};

let theme = Theme {
    accent: Color::Rgb(180, 142, 173),
    unicode: true, // set false for ASCII-only glyphs
    ..Theme::default()
};
set_theme(theme);
```

The same theme drives both output widgets and input prompts.

## Examples

```sh
cargo run --example output_gallery --features markup,fuzzy,pager
cargo run --example output_dynamic --features pager   # spinner/progress/live
cargo run --example prompts --features fuzzy          # interactive; needs a TTY
cargo run --example output_readme --features markup   # screenshot collage
cargo run --example prompt_readme                     # non-interactive prompts
```

## Documentation

- [docs.rs/sparcli](https://docs.rs/sparcli) – the complete API reference (rustdoc, built with all features).
- [`DEVELOPMENT.md`](https://github.com/cgroening/rs-sparcli/blob/main/docs/DEVELOPMENT.md) – building, testing and contributing.
- [`CHANGELOG.md`](https://github.com/cgroening/rs-sparcli/blob/main/CHANGELOG.md) – release notes.
- `cargo doc --all-features --open` – build the same reference locally.

## License

MIT – see [`LICENSE`](LICENSE).
