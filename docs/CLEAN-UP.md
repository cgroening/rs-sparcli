# Code Walkthrough & Cleanup (checklist to tick off)

> Archive document. This checklist is complete and describes the state at that time. The then-flat files `core/style.rs`, `output/table.rs`, `input/text.rs`, `input/number.rs` and `input/datepicker.rs` are now directory modules; these were followed by the API curation (internal module tree `pub(crate)`, facade modules) and the `log` facade logging. The current structure is described in `../DEVELOPMENT.md`.

## Status: complete (2026-07-07)

All phases worked through. Final state green: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test` (default as well as individually `markup`/`fuzzy`/`pager`/`--all-features`), `cargo build --examples --all-features` and `RUSTDOCFLAGS="-D warnings" cargo doc --all-features` – all clean. 171 tests with `--all-features` (previously 165), behavior unchanged.

Changes made:
- `input/number.rs` → `number/{mod,calc}.rs`; `eval`/`Calc` private (`pub(crate)`), `Result<f64, String>` replaced by internal `CalcError` (thiserror), doctest moved into unit tests.
- `output/table.rs` (813) → `table/{mod,plan,render}.rs`.
- `input/text.rs` (712) → `text/{mod,render,keys,suggest}.rs`.
- `input/datepicker.rs` (488) → `datepicker/{mod,date}.rs`.
- `core/style.rs` (384) → `style/{mod,color}.rs`.
- 28 rustdoc violations (missing blank line after `# Errors`/`# Examples`, §8.4) in 16 files fixed.
- Stale `#[allow(dead_code)]` in `input/event.rs` removed (used by tests).
- `API.md` and `CHANGELOG.md` synchronized (eval removal).

Deliberate non-changes (KISS, "when in doubt, leave it"):
- `panel` (326), `list` (309), `select` (402), `fuzzy` (385): each one coherent responsibility just above the ~300 signal threshold – no clarity gain from further splitting.
- The supposed `spinner.rs` 80-column finding was a byte-vs-character artifact (rustfmt counts characters; no real violation).
- Render helpers with 4-7 parameters remain – situational values (buffers, widths, styles), which §2.5 explicitly allows as explicit parameters, no object state.
- `pub mod` breadth of the widget modules left unchanged (narrowing to `pub(crate)` would break documented module paths – open item for later).

## Context

The repo is stable and clean after several feature rounds (`cargo fmt` green, `cargo clippy --all-targets --all-features -- -D warnings` green, ~165 unit tests + integration, `#![warn(missing_docs)]` crate-wide, every module has a `//!` doc, no TODOs, only one justified local `#[allow(dead_code)]` in `input/event.rs`). `sparcli` is a **standalone** library without ratatui/async: its own renderer on `crossterm`, `unicode-width`, `thiserror` as default deps; `markup`, `fuzzy` (`nucleo-matcher`) and `pager` as opt-in feature flags. There are no extracted crates – everything lives here.

Ordering principle: first establish a baseline, then layer by layer from `core` (the foundation) outward to `output`/`input` (this way understanding builds up bottom-up and each layer is checked according to its dependencies), finally the crate root and a cross-cutting pass. The dependency direction is strictly `output`/`input` → `core`, never cyclic (CLAUDE.md §2.6/§7.2).

## Generic checkpoints (apply to EVERY module)

When going through each file, check each time (CLAUDE.md §1, §2, §7):
- **Names:** predicates `is_/has_/can_`; methods = verbs, types = nouns; no `Manager/Helper/Data` catch-all names; no negative booleans.
- **Functions:** SLAP (one abstraction level), max. 2 nesting levels with early return, ≤ 3 parameters (otherwise struct/opts), no flag arguments, command-query separation.
- **Visibility:** as private as possible; `pub` only where actually used (re-exports via `mod.rs`/`lib.rs` control the public surface).
- **Errors:** `Result`/`?`, no `unwrap/expect/panic` in the normal flow; every `expect` at a provably infallible place justified (CLAUDE.md §7.3).
- **Magic numbers/strings:** replaced by named constants/`enum`s; lookup tables instead of large `match` for constant mappings.
- **Hygiene:** no dead/commented-out code; comments explain the *why*; rustdoc up to date on every public item (`# Examples`/`# Errors`/ `# Panics` where applicable); 80 columns; straight quotes; in code no em dash (hyphen), in `.md` the en dash `–`.
- **Tests:** logic-bearing code has tests in `#[cfg(test)] mod tests`; test names describe the expected behavior; doctests in `# Examples` run.

---

## Orientation – reading pass (before Phase 0, without changes)

Top-down *reading* only, to build the mental map before cleaning up from inside out. Nothing is changed here – only capture the wiring and module structure.

- [x] `lib.rs`: skim the module tree (`core`/`output`/`input`/`error`), public re-exports and `prelude` – what is visible to the outside, what lies behind `#[cfg(feature = ...)]` (`pager`, `fuzzy`)?
- [x] `core/mod.rs`, `output/mod.rs`, `input/mod.rs`: capture the sub-module structure and re-exports; locate the `Renderable`/`Rendered` contract (`core/render.rs`) and the `Outcome<T>` type (`input`).
- [x] Follow the dependencies from `output`/`input` to `core` until the layer boundaries are clear (no `output`↔`input`, no reach-back from `core` outward). Note anything conspicuous, but do not touch it yet – that happens bottom-up from Phase 1.

## Phase 0 – Baseline & Scope

- [x] Run `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features` – green starting state confirmed (also without features: `cargo test` with the default feature set).
- [x] Use a clean branch (`clean-up`, no commit on `main`); secure the working state.
- [x] Decide: pure review (read only + mini fixes) vs. real refactors – delimit the scope. KISS/YAGNI before style rework.

## Phase 1 – core: primitives (`src/core/`)

First the building blocks without widget logic, in dependency order:

- [x] `geometry.rs` (`Align`, `VAlign`, `Edges`, `Position`, `Title`): pure value types – generic checks, invariants documented?
- [x] `width.rs`: Unicode width calculation (`unicode-width`) – edge cases (wide chars, zero-width, truncation with `…`) tested.
- [x] `style.rs` (`Style`, `Color`, `Modifier`, `Attribute`): ratatui-familiar vocabulary; `NO_COLOR`/non-TTY behavior; no magic ANSI codes.
- [x] `text.rs` (`Span`, `Line`, `Text`): builder API consistent; SLAP in wrapping/merging logic.
- [x] `border.rs` (`BorderType`, default `Rounded`): glyphs in two tiers (Unicode + ASCII fallback) selectable via the theme.

## Phase 2 – core: markup, theme, terminal, render (`src/core/`)

- [x] `markup.rs` (feature `markup`): `[bold red]…[/]` parser – defensive error paths on broken markup, no panics; correctly separated behind `#[cfg(feature)]`.
- [x] `theme.rs` (`Theme`, `theme()`, `set_theme()`): **unified theme** for input AND output (SSOT, CLAUDE.md architecture); muted look, one accent tone, `dim` for secondary text; global state thread-safe/documented.
- [x] `terminal.rs`: `crossterm` encapsulation, TTY detection, `NO_COLOR`; no widget logic leaked through.
- [x] `render.rs` (`Renderable`, `Rendered`): the render contract as a test interface (render to `Rendered`, check without TTY) – core contract clear and minimal.
- [x] `mod.rs`: re-exports minimal; only what `output`/`input`/`lib` really need.

## Phase 3 – output: widgets (`src/output/`)

All implement `Renderable`; per file generic checks + render tests to `Rendered` (no TTY). Truncate overflow with `…`, default border `Rounded`.

- [x] Primitives: `rule.rs`, `badge.rs`, `alert.rs` (`AlertKind`), `kv.rs`, `list.rs` (`Marker`).
- [x] Container/layout: `panel.rs`, `columns.rs`, `layout.rs`, `compose.rs` (`align`/`pad`/`vstack`) – SLAP in dense render functions, no magic strings for labels/glyphs.
- [x] Tabular/structured: `table.rs` (`Cell`, `Column`), `tree.rs` (`TreeNode`), `diff.rs`.
- [x] Dynamic: `progress.rs` (`ProgressStyle`, `Thresholds`), `multiprogress.rs`, `spinner.rs` (`SpinnerStyle`), `live.rs` – redraw logic, no resource leaks, non-TTY fallback.
- [x] `pager.rs` (feature `pager`): `$PAGER`/`less`/`more` integration, error paths when no pager is available; cleanly behind `#[cfg(feature)]`.
- [x] `mod.rs`: re-exports minimal; feature-gated items (`pager`) correctly separated.

## Phase 4 – input: foundation (`src/input/`)

The shared infrastructure first – it carries all prompts:

- [x] `event.rs` (`EventSource`, DI): fake for headless tests (scripted keys); check the one `#[allow(dead_code)]` – still needed or removable?
- [x] `line_edit.rs`: **SSOT for text input** – cursor/editing logic, shared by the text prompts; generic checks, no duplicates in the widgets.
- [x] `guard.rs` (`TerminalGuard`, RAII): restores the terminal on drop/error/panic – check the recovery path and `Drop` impl.
- [x] `prompt.rs`, `field.rs`, `validate.rs`, `history.rs`: shared building blocks (redraw `frame`, validation, history) – clear, minimal interfaces; `Outcome<T>` (`Submitted`/`Cancelled`) consistent.

## Phase 5 – input: prompts (`src/input/`)

Per widget: `EventSource` fake test (headless), `Outcome<T>` return, shared `line_edit`/`validate` usage instead of custom logic.

- [x] Text: `text.rs` (`TextInput`), `password.rs` (`PasswordInput`), `textarea.rs`, `number.rs` (`NumberInput`) – do they share `line_edit` cleanly?
- [x] Selection: `confirm.rs`, `select.rs` (`Select`), `datepicker.rs` (`Date`/`DatePicker`), `shortcut.rs` – cyclic navigation of the lists.
- [x] `fuzzy.rs` (feature `fuzzy`): inline fuzzy select via `nucleo-matcher`; cleanly behind `#[cfg(feature)]`, fallback/error paths.
- [x] `editor.rs` (`edit_file`): external `$EDITOR` invocation – error paths, temp file handling.
- [x] `mod.rs`: `Outcome` re-export + feature-gated items (`fuzzy`) correct; re-exports minimal.

## Phase 6 – crate root (`src/lib.rs`, `src/error.rs`)

Last, because all layers converge here (public API):

- [x] `error.rs` (`SparcliError`, `Result`): `#[error(...)]` texts meaningful, foreign errors via `#[from]`, one coherent lib error type; no infrastructure leaks.
- [x] `lib.rs`: module declarations complete/consistent; public re-exports and `prelude` minimal & consistent (what really belongs in `prelude`?); feature-gated `pub use` (`Pager`, `FuzzySelect`) correct; crate-wide `#![warn(missing_docs)]` justification current; module `//!` doc and intro examples correct.

## Phase 7 – cross-cutting & wrap-up

- [x] **`#[allow]` inventory:** deliberately confirm or remove the local `#[allow(dead_code)]` in `input/event.rs`; no further allows crept in.
- [x] **Feature matrix:** each feature combination builds & tests cleanly – `cargo test` (default), `--features markup`, `--features fuzzy`, `--features pager`, `--all-features`; cross-check with `cargo hack`/manually that no item is accidentally ungated.
- [x] **Docs sync:** `README.md`, `API.md`, `DEVELOPMENT.md`, `CHANGELOG.md` against the cleaned-up state; rustdoc examples/doctests consistent; feature flags correctly documented.
- [x] **Tests:** paths touched by refactors tested; all green (unit + `tests/integration.rs` + doctests).
- [x] **Final gates:** `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features` – all green.
- [x] Propose commit message(s) in Conventional Commits style (no auto-commit per CLAUDE.md).

## Verification

After each layer and at the end: `cargo fmt --check` + `cargo clippy --all-targets --all-features -- -D warnings` + `cargo test --all-features` green. Pure refactorings must not change behavior – render tests (`Rendered` content/style) and integration tests (`tests/`) must pass without regeneration; only adjust tests deliberately on an intentional behavior/layout change.

## Notes / non-goals

- **Scope boundaries (CLAUDE.md):** output complete, input only single widgets – no Form/App/Args/Serde/logging, fuzzy only as inline select. None of this to be "retrofitted" during cleanup.
- **No async, no ratatui:** a lean footprint is the guiding vision – no new dependencies without prior agreement (CLAUDE.md §7.7).
- **Unified theme:** `core/theme.rs` applies to input AND output – do not split.
- KISS/YAGNI before "my style": respect the local style, only touch what the task requires, separate refactoring from behavior (CLAUDE.md §3).
