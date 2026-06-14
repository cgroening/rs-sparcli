# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-06-14

### Added
- `LICENSE` file; the crate is licensed under MIT.

## [0.1.0] - 2026-06-14

### Added
- Initial release.
- Output widgets: styled text, `markup` (feature), tables (colspan/rowspan,
  striping, wrapping, titles), panels, alerts, rules, lists, trees, key-value
  lists, badges, columns, diff, progress bars, spinners, multi-progress, live
  display, pager (feature), and composition helpers (`align`, `pad`, `vstack`).
- Input widgets: confirm, text (validation, char filters, history, ghost and
  dropdown autocomplete, external editor), password, number (with calculator),
  textarea, single/multi select, inline fuzzy select (feature), and a date
  picker — all returning `Outcome<T>` and never panicking on input.
- Unified theme shared by output and input; `NO_COLOR` and non-terminal aware.
- Small default build with opt-in `markup`, `fuzzy` and `pager` features.
