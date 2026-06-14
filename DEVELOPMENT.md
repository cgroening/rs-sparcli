# Development

How to build, test and work on `sparcli`. Coding conventions live in
[`CLAUDE.md`](CLAUDE.md).

## Prerequisites

- Rust (edition 2024; toolchain 1.85+).
- No system dependencies beyond a terminal.

Build artifacts go to `target.nosync/` (configured in `.cargo/config.toml` so
iCloud does not sync them).

## Build

```sh
cargo build                       # default features
cargo build --all-features        # markup + fuzzy + pager
```

## Run the examples

```sh
# Static output gallery (tables, panels, lists, progress, diff, columns, …)
cargo run --example output_gallery --features markup,fuzzy,pager

# Time-based / interactive output (spinner, progress, multi-progress, live,
# pager) — best in a real terminal
cargo run --example output_dynamic --features pager

# Interactive prompts (needs a real terminal / TTY)
cargo run --example prompts --features fuzzy
```

The gallery also works without features (those widgets are simply omitted).
Piping the gallery (`| cat`, `> file`) or setting `NO_COLOR=1` yields plain
text with no escape codes. The `output_dynamic` animations become no-ops
off-terminal (only the final state prints).

## Run the tests

```sh
cargo test                        # unit + integration + doctests (default)
cargo test --all-features         # also tests markup, fuzzy, pager code
```

More targeted runs:

```sh
cargo test --lib                          # unit tests only
cargo test --test integration             # integration tests only (tests/)
cargo test --doc                          # doctests only
cargo test --all-features fuzzy           # tests whose name contains "fuzzy"
cargo test --all-features -- --nocapture  # show println! output
```

Tests never require a real terminal: output widgets are rendered to an
in-memory buffer and asserted on their visible text, and input prompts are
driven by a scripted fake event source (`input::event::ScriptedSource`). So the
whole suite is safe to run headless / in CI.

## Lint and format

```sh
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt
cargo fmt --check
```

clippy must be warning-free. `cargo fmt` may print two warnings about
`group_imports`/`imports_granularity` being nightly-only; these are harmless on
stable (the rules are followed manually, see `rustfmt.toml`).

## Feature flags

| Feature  | Effect                                            |
| -------- | ------------------------------------------------- |
| (none)   | core + output + simple input prompts              |
| `markup` | `[bold red]…[/]` inline markup parsing            |
| `fuzzy`  | inline fuzzy-select (pulls in `nucleo-matcher`)   |
| `pager`  | paging via `$PAGER` / `less` / `more`             |
| `full`   | enables `markup`, `fuzzy` and `pager`             |

## Project layout

```
src/
  core/     Foundation: style, text, markup, theme, border, geometry,
            width, terminal, render. No widget logic.
  output/   Printable widgets implementing `Renderable`.
  input/    Interactive prompts over an `EventSource`, plus the shared
            line editor, terminal guard and prompt driver.
examples/   Runnable demos (output_gallery, prompts).
tests/      End-to-end tests over the public API.
```

Dependency direction is one-way: `output` and `input` depend on `core`, never
the reverse. `sparcli` never depends on `ratatui`.

## Useful environment variables

- `NO_COLOR=1` — disable all color output.
- `CLICOLOR_FORCE=1` — force color even when output is not a terminal.
- `SPARCLI_NO_TTY=1` — force "no terminal" behavior (used in tests).

## Adding a widget

1. Add the module under `src/output/` or `src/input/`.
2. Implement `Renderable` (output) or a `run`/`run_with` pair (input) using the
   shared `prompt::run_prompt` driver.
3. Add unit tests in a `#[cfg(test)] mod tests` block.
4. Re-export the public type in `src/lib.rs` (and `prelude` if commonly used).
5. Run `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt`.
6. Update `README.md` and, if relevant, the example gallery.
