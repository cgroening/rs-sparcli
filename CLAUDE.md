# CLAUDE.md – sparcli (Rust)

Requirements for all future sessions in this project. In case of conflict, this
file takes precedence over conventions, but not over explicit user instructions.

## What is this?

`sparcli` is a **lightweight, platform-independent** toolkit (macOS, Windows,
Linux) for **styled CLI output** and **interactive single-input widgets** –
a native Rust port of the C library `sparcli`. Guiding principle: lean, for
small CLI tools. No async, no ratatui, minimal footprint.

- **Foundation:** custom renderer on `crossterm` (no ratatui).
- **API feel:** ratatui-familiar vocabulary (`Style`, `Color`, `Span`,
  `Line`, `Text`, `Modifier`), fluent builder API **and** options struct.
- **Scope:** output complete; input only single widgets (no Form/App/Args/
  Serde). Fuzzy only as inline Select.

## Python twin – keep in sync

There is a parallel Python version of `sparcli` at
`/Users/cgroening/Developer/Python/libs/sparcli`. On **every change** to this
Rust crate, check whether the same change has to be applied to the Python
version too (behavior, API, docs, tests) – and **ask the user** whether it
should be ported before proceeding.

## Architecture (separate layers strictly, §2.6/§7.2)

- `core/` – foundation: style, text, markup, theme, border, geometry, width,
  terminal, render. No widget logic.
- `output/` – printable widgets, implement `Renderable`.
- `input/` – interactive prompts via `EventSource` (DI) + `frame` redraw +
  `line_edit` (SSOT for text input).
- Dependency direction: `output`/`input` → `core`. Never cyclic.
- **Unified theme** in `core/theme.rs` applies to input AND output.

## Dependencies & Feature Flags

- Default (always): `crossterm`, `unicode-width`, `thiserror`, `log`.
- Opt-in: `markup`, `fuzzy` (`nucleo-matcher`), `pager`.
- New dependencies **must be agreed with the user beforehand** (§7.7).
- Established crates with `// https://crates.io/crates/<name>` above the `use`.
- **Logging:** only the `log` facade and only as `warn!`/`debug!` at places where
  a `Result` would otherwise be silently swallowed (e.g. terminal restore in the
  `TerminalGuard`, history save/load, temp cleanup). No `error!` logs – real
  errors come back via `SparcliError` (no double logging); do not ship a logger/
  backend (the app decides); nothing in hot paths/render loops.

## Error Handling (§7.3) – very important, robust & long-lived

- No `unwrap()`; `expect()` only at provably infallible places with a
  justification. No `panic!` in normal operation.
- Errors via `Result<T, E>` + `?`. Library errors as a `thiserror` enum
  (`SparcliError`), foreign errors via `#[from]`.
- Input prompts return `Outcome<T>` (`Submitted` / `Cancelled`).
- `TerminalGuard` (RAII) restores the terminal on drop/error/panic.
- Defensive: safeguard inputs/edge cases, prefer to fail in a controlled way.

## Clean Code (§1, §2.5) – key points

- SRP for functions/structs; keep functions small.
- **≤ 3 function parameters**; more → bundle into a struct/opts.
- Early returns (guard clauses), max. 2 nesting levels.
- SLAP (one abstraction level per function); no flag arguments.
- No magic numbers/strings → named constants.
- Strong typing: `enum`s instead of magic strings, `struct`s instead of tuples.
- Lookup tables instead of large `match` for constant mappings.

## Style & Tooling

- Edition 2024. `cargo fmt` (rustfmt.toml: max_width 80).
- `cargo clippy --all-targets -- -D warnings` must be clean.
- 80-character code lines; straight quotes.
- **No em dash** as a dash: in code files use the hyphen, in `.md` files use
  the en dash `–`.
- rustdoc on every public item, module `//!`; `# Examples`/`# Errors`/
  `# Panics` where applicable; `#![warn(missing_docs)]`.

## Appearance (§7.10)

- Muted appearance, one accent tone, `dim` for secondary text.
- Default border `Rounded`; truncate overflow with `…`.
- Glyphs in two tiers (Unicode + ASCII fallback), selectable via the theme.
- Selection lists navigate cyclically; respect `NO_COLOR`/non-TTY.

## Tests (§2.8/§7.8) – mandatory

- Unit tests in `#[cfg(test)] mod tests` per file; integration in `tests/`.
- Test names describe the expected behavior; fakes over mocks.
- Output: render to `Rendered` and check content/style (no TTY).
- Input: drive headless via an `EventSource` fake (scripted keys).
- **Run all tests after every change** (`cargo test`).
- Doctests in `# Examples` count as tests and must run.

## Docs/Maintenance

- On changes, keep README/rustdoc and tests in sync.
- Remove dead/commented-out code; fix the cause instead of the symptom.
