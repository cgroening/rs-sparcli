# Changelog

All notable changes to this project are documented here. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `Card`, a filled counterpart to `Panel`: a colored surface with its own title and footer rows instead of a title embedded in the border. A single `.accent(...)` derives the whole palette through HSL - the title keeps the accent saturated, the body text and both backgrounds become desaturated, darker shades of the same hue. The border is opt-in, the card fills the width it is rendered into, content wraps, and title, body and footer each take their own padding and alignment. The style setters patch the derived values rather than replacing them. Below truecolor support the backgrounds are dropped and the card renders as accented text, because the derived shades would collapse onto one ANSI-16 color.
- `BorderType::Tall`, a thin block border around a card's filled surface, following the geometry of Textual's `wide` border: the side bars ink a quarter of their cell's width and the top and bottom lines an eighth of their cell's height, which comes out equally thick because a terminal cell is about twice as tall as it is wide, and the horizontal lines run across the corner cells so the corners close. Only `Card` draws it natively - the bars need a filled surface to read against, all four edges use a different glyph, and the right-hand one is painted with foreground and background swapped, none of which `BorderChars` can express with its single `horizontal` and `vertical` and one uniform style. Every other widget receives the `BorderType::Thick` glyphs from `BorderType::chars`, as does a card without truecolor support; a theme with `unicode: false` degrades it to `BorderType::Ascii`. Note that `BorderType` is not `#[non_exhaustive]`, so the new variant breaks exhaustive `match` expressions over it.
- `Color::to_rgb` returns the 24-bit value of any color: named colors and palette indices resolve through the standard xterm palette (fixed table, 6x6x6 cube, grayscale ramp). `Color::Reset` has no fixed value and returns `None`.
- `width::wrap_line` and `width::truncate_line` are style-preserving counterparts to `wrap` and `truncate`. They keep each span's style and hyperlink, and a word straddling a span boundary stays whole instead of being wrapped apart.
- `core::command::split_command` splits a configured command line into an argument vector, honoring single and double quotes. Written by hand rather than pulling in a crate, keeping the dependency set unchanged.

### Fixed

- `$EDITOR`, `$VISUAL` and `$PAGER` are split with `split_command` instead of `split_whitespace`, so an editor or pager path containing spaces (`/Applications/Sublime Text/subl`) no longer breaks into invalid arguments. A command with an unbalanced quote now reports `SparcliError::Config` instead of being silently mangled. Mirrors the Python port.
- The history state directory is normalized to an absolute path, and an application name that is empty, a dot component, or contains a path separator is rejected instead of being written outside the state directory. Such a history stays in memory. Mirrors the Python port.

## [0.3.0] - 2026-07-11

### Added

- The terminal hardware cursor is now hidden during in-place redraws (spinner, progress, multi-progress, live) and interactive prompts, and restored on finish, on prompt exit, on drop and on panic. Mirrors the Python port.
- `Style::remove_modifier` clears one or more attributes from a style (the counterpart to `add_modifier`). Mirrors the Python port.

### Changed

- `Panel` and `Alert` now honour the render width: a fixed width is capped to the terminal, overflowing natural content shrinks the frame, and a title too wide for the interior is truncated (`…`) instead of widening the box.
- Inline markup matches attribute names and the `on` background keyword case-insensitively (`[BOLD]`, `[white ON blue]`), and a tag opened inside a backtick code span no longer defeats the closing backtick.
- Inline markup no longer swallows a closed bracket that names no known style or attribute (such as `array[0]`); such text is emitted literally. Mirrors the Python port.

### Fixed

- Control characters (C0, DEL and C1, except tab) are stripped from span content and OSC-8 link URLs before they reach the terminal, so untrusted text can no longer inject escape sequences or terminate a hyperlink early. Mirrors the Python port.
- History files are written atomically (temp file plus rename), so a crash or a concurrent writer can no longer truncate the file. Mirrors the Python port.
- `truncate` never exceeds `max_cols`: a width of `0` yields an empty string, and an ellipsis wider than `max_cols` is clamped to fit.
- `strip_ansi` recognises the full CSI final-byte range (`0x40..=0x7e`), so escape sequences ending in a non-letter byte are stripped correctly.
- `COLORTERM` is matched case-insensitively when detecting truecolor support.
- `terminal_size` honours the `COLUMNS`/`LINES` environment variables before querying the terminal.

### Notes

- `DatePicker`'s initial "today" remains UTC here (dependency-free), while the Python port uses the local date. This one-point divergence is intentional; near midnight the default day can differ by one between the two ports.

## [0.2.1] - 2026-07-10

### Added

- `Spinner::clear` stops a spinner and erases its line, for a transient spinner whose outcome is reported elsewhere. Mirrors `Live::clear`.

### Changed

- `Table` now honours the render width: a table that already fits is unchanged, while an overflowing one shrinks its flexible columns (wrapping columns reflow first, then the rest truncate) so its borders stay within the terminal. `fixed_width` columns never shrink and no column falls below its `min_width`.

## [0.2.0] - 2026-07-07

### Added

- Lightweight `log`-facade diagnostics (`warn!`/`debug!`) at previously swallowed error points (terminal restore, input-history save/load, temp-file cleanup, editor raw-mode toggles). No logger or backend is forced on consumers, and errors already surfaced via `SparcliError` are not double-logged.

### Removed

- The public `eval` expression evaluator from `input::number`. Arithmetic parsing is now an internal detail of `NumberInput::calculator` and reports a typed internal error instead of a `String`.
- `LineEditor` and `TerminalGuard` are no longer part of the public API; they are internal implementation details.

### Changed

- Internal reorganization of the largest modules into focused submodules – `output::table`, `input::text`, `input::number`, `input::datepicker` and `core::style` – with no change to the public API beyond the `eval` removal above.
- Curated the public surface: the internal layer tree (`core`/`input`/`output`) is no longer public. Types are used via the crate root (`sparcli::Table`, `sparcli::Style`, …); the free-function utilities moved from `sparcli::{core, input, output}::…` to dedicated modules `sparcli::{markup, validate, event, shortcut, width, terminal}`. The `prelude` is unchanged.

## [0.1.2] - 2026-06-14

### Added

- `frame()` on every prompt: a static, non-interactive render of the configured state, for previews and README screenshots.
- Per-instance styling for `Table` (`border_style`, `header_style`, `title_style`, `stripe_style`) and `Tree::dashes`.
- Preset state on prompts: `Select::cursor`/`Select::checked`, `FuzzySelect::query`, `PasswordInput::initial`.
- `output-readme` and `prompt-readme` example collages and README screenshots; docs.rs now builds with all features.

### Fixed

- Tree vertical guides now match the connector color.
- Prompts no longer draw to the terminal when driven by a non-interactive (scripted) event source.
- The external editor (Ctrl-G) no longer leaves the terminal in raw mode when it was not already enabled.
- The final prompt frame hides the input cursor after submit.

## [0.1.1] - 2026-06-14

### Added

- `LICENSE` file; the crate is licensed under MIT.

## [0.1.0] - 2026-06-14

### Added

- Initial release.
- Output widgets: styled text, `markup` (feature), tables (colspan/rowspan, striping, wrapping, titles), panels, alerts, rules, lists, trees, key-value lists, badges, columns, diff, progress bars, spinners, multi-progress, live display, pager (feature), and composition helpers (`align`, `pad`, `vstack`).
- Input widgets: confirm, text (validation, char filters, history, ghost and dropdown autocomplete, external editor), password, number (with calculator), textarea, single/multi select, inline fuzzy select (feature), and a date picker – all returning `Outcome<T>` and never panicking on input.
- Unified theme shared by output and input; `NO_COLOR` and non-terminal aware.
- Small default build with opt-in `markup`, `fuzzy` and `pager` features.
