# Contributing

Thanks for your interest in `sparcli`. This is a small, focused library, so
contributions should keep it lightweight and cross-platform.

## Getting started

See [`DEVELOPMENT.md`](DEVELOPMENT.md) for building, running the examples,
testing, linting and the project layout. The full coding conventions live in
[`CLAUDE.md`](CLAUDE.md); please skim it before a larger change.

## Scope

`sparcli` is deliberately minimal: styled output widgets and single interactive
input widgets, built on `crossterm` with no `ratatui` or async. Full-screen /
retained TUIs, form or app frameworks, and heavy dependencies are out of scope.
For anything beyond a small fix, please open an issue first to discuss it.

## Before opening a pull request

- `cargo fmt` and `cargo fmt --check` are clean.
- `cargo clippy --all-targets --all-features -- -D warnings` is clean.
- `cargo test` and `cargo test --all-features` pass.
- New behavior has tests; output widgets render to a buffer, input widgets are
  driven by the scripted fake event source (headless, no TTY needed).
- Documentation is updated where relevant: rustdoc on public items, plus
  `README.md` / `API.md` / `CHANGELOG.md`.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/) (e.g.
`feat: …`, `fix: …`, `docs: …`, `refactor: …`). Mark breaking changes with a
`!` (e.g. `refactor(api)!: …`) and note them in `CHANGELOG.md`.
