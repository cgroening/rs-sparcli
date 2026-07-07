# Changelog

All notable changes to this project are documented here. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Removed

- The public `eval` expression evaluator from `input::number`. Arithmetic parsing is now an internal detail of `NumberInput::calculator` and reports a typed internal error instead of a `String`.

### Changed

- Internal reorganization of the largest modules into focused submodules – `output::table`, `input::text`, `input::number`, `input::datepicker` and `core::style` – with no change to the public API beyond the `eval` removal above.

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
