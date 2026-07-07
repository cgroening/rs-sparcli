# Code-Walkthrough & Aufräumen (Checkliste zum Abhaken)

> Archiv-Dokument. Diese Checkliste ist abgeschlossen und beschreibt den
> damaligen Stand. Die seinerzeit flachen Dateien `core/style.rs`,
> `output/table.rs`, `input/text.rs`, `input/number.rs` und
> `input/datepicker.rs` sind inzwischen Verzeichnis-Module; danach folgten die
> API-Kuratierung (interner Modulbaum `pub(crate)`, Facade-Module) und das
> `log`-Facade-Logging. Der aktuelle Aufbau steht in `../DEVELOPMENT.md`.

## Status: abgeschlossen (2026-07-07)

Alle Phasen durchgearbeitet. Endzustand grün: `cargo fmt --check`, `cargo clippy
--all-targets --all-features -- -D warnings`, `cargo test` (default sowie einzeln
`markup`/`fuzzy`/`pager`/`--all-features`), `cargo build --examples --all-features`
und `RUSTDOCFLAGS="-D warnings" cargo doc --all-features` – alles sauber. 171
Tests bei `--all-features` (vorher 165), Verhalten unverändert.

Durchgeführte Änderungen:
- `input/number.rs` → `number/{mod,calc}.rs`; `eval`/`Calc` privat (`pub(crate)`),
  `Result<f64, String>` durch interne `CalcError` (thiserror) ersetzt, Doctest in
  Unit-Tests überführt.
- `output/table.rs` (813) → `table/{mod,plan,render}.rs`.
- `input/text.rs` (712) → `text/{mod,render,keys,suggest}.rs`.
- `input/datepicker.rs` (488) → `datepicker/{mod,date}.rs`.
- `core/style.rs` (384) → `style/{mod,color}.rs`.
- 28 rustdoc-Verstöße (fehlende Leerzeile nach `# Errors`/`# Examples`, §8.4) in
  16 Dateien behoben.
- Stale `#[allow(dead_code)]` in `input/event.rs` entfernt (wird von Tests genutzt).
- `API.md` und `CHANGELOG.md` synchronisiert (eval-Entfernung).

Bewusste Nicht-Änderungen (KISS, „im Zweifel lassen"):
- `panel` (326), `list` (309), `select` (402), `fuzzy` (385): je eine kohärente
  Verantwortung knapp über der ~300-Signalgrenze – kein Klarheitsgewinn durch
  weiteres Splitten.
- Der vermeintliche `spinner.rs`-80-Spalten-Befund war ein Byte-vs-Zeichen-
  Artefakt (rustfmt zählt Zeichen; keine echte Verletzung).
- Render-Helfer mit 4-7 Parametern bleiben – situative Werte (Puffer, Breiten,
  Styles), die §2.5 explizit als explizite Parameter erlaubt, kein Objekt-Zustand.
- `pub mod`-Breite der Widget-Module unverändert gelassen (Verengung auf
  `pub(crate)` würde dokumentierte Modulpfade brechen – offener Punkt für später).

## Context

Das Repo ist nach mehreren Feature-Runden stabil und sauber (`cargo fmt`
grün, `cargo clippy --all-targets --all-features -- -D warnings` grün, ~165
Unit-Tests + Integration, `#![warn(missing_docs)]` crate-weit, jedes Modul hat
einen `//!`-Doc, keine TODOs, nur ein begründetes lokales `#[allow(dead_code)]`
in `input/event.rs`). `sparcli` ist eine **eigenständige** Library ohne
ratatui/async: eigener Renderer auf `crossterm`, `unicode-width`, `thiserror`
als Default-Deps; `markup`, `fuzzy` (`nucleo-matcher`) und `pager` als opt-in
Feature-Flags. Es gibt keine ausgelagerten Crates – alles liegt hier.

Reihenfolge-Prinzip: zuerst Baseline herstellen, dann Schicht für Schicht von
`core` (dem Fundament) nach außen zu `output`/`input` (so baut sich das
Verständnis bottom-up auf und jede Schicht wird nach ihren Abhängigkeiten
geprüft), zum Schluss Crate-Root und ein Querschnitts-Durchlauf. Die
Abhängigkeitsrichtung ist strikt `output`/`input` → `core`, niemals zyklisch
(CLAUDE.md §2.6/§7.2).

## Generische Prüfpunkte (gelten bei JEDEM Modul)

Beim Durchgehen jeder Datei jeweils prüfen (CLAUDE.md §1, §2, §7):
- **Namen:** Prädikate `is_/has_/can_`; Methoden = Verben, Typen = Substantive;
  keine `Manager/Helper/Data`-Sammelnamen; keine negativen Booleans.
- **Funktionen:** SLAP (eine Abstraktionsebene), max. 2 Verschachtelungen mit
  frühem Return, ≤ 3 Parameter (sonst Struct/Opts), keine Flag-Argumente,
  Command-Query-Separation.
- **Sichtbarkeit:** so privat wie möglich; `pub` nur, wo wirklich genutzt
  (Re-Exports über `mod.rs`/`lib.rs` steuern die öffentliche Fläche).
- **Fehler:** `Result`/`?`, kein `unwrap/expect/panic` im Normalfluss; jedes
  `expect` an beweisbar unfehlbarer Stelle begründet (CLAUDE.md §7.3).
- **Magic Numbers/Strings:** durch benannte Konstanten/`enum`s ersetzt;
  Lookup-Tables statt großer `match` für konstante Zuordnungen.
- **Hygiene:** kein toter/auskommentierter Code; Kommentare erklären das
  *Warum*; rustdoc auf jedem öffentlichen Item aktuell (`# Examples`/`# Errors`/
  `# Panics` wo zutreffend); 80-Spalten; gerade Anführungszeichen; in Code kein
  Geviertstrich (Bindestrich), in `.md` der Halbgeviertstrich `–`.
- **Tests:** logiktragender Code hat Tests in `#[cfg(test)] mod tests`;
  Testnamen beschreiben das erwartete Verhalten; Doctests in `# Examples` laufen.

---

## Orientierung – Lesedurchgang (vor Phase 0, ohne Änderungen)

Top-down nur *lesen*, um die mentale Landkarte aufzubauen, bevor von innen nach
außen aufgeräumt wird. Hier wird nichts geändert – nur Verdrahtung und
Modulstruktur erfassen.

- [x] `lib.rs`: Modulbaum (`core`/`output`/`input`/`error`), öffentliche
  Re-Exports und `prelude` überfliegen – was ist nach außen sichtbar, was liegt
  hinter `#[cfg(feature = ...)]` (`pager`, `fuzzy`)?
- [x] `core/mod.rs`, `output/mod.rs`, `input/mod.rs`: Sub-Modulstruktur und
  Re-Exports erfassen; `Renderable`/`Rendered`-Vertrag (`core/render.rs`) und
  den `Outcome<T>`-Typ (`input`) verorten.
- [x] Den Abhängigkeiten von `output`/`input` nach `core` folgen, bis die
  Schichtengrenzen klar sind (kein `output`↔`input`, kein Rückgriff von `core`
  nach außen). Auffälligkeiten notieren, aber noch nicht anfassen – das passiert
  bottom-up ab Phase 1.

## Phase 0 – Baseline & Scope

- [x] `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D
  warnings`, `cargo test --all-features` laufen lassen – grüner
  Ausgangszustand bestätigt (auch ohne Features: `cargo test` mit
  Default-Feature-Set).
- [x] Sauberen Branch nutzen (`clean-up`, kein Commit auf `main`); Arbeitsstand
  sichern.
- [x] Entscheiden: reiner Review (nur lesen + Mini-Fixes) vs. echte Refactors –
  Umfang abstecken. KISS/YAGNI vor Stil-Umbau.

## Phase 1 – core: Primitive (`src/core/`)

Zuerst die Bausteine ohne Widget-Logik, in Abhängigkeitsreihenfolge:

- [x] `geometry.rs` (`Align`, `VAlign`, `Edges`, `Position`, `Title`): reine
  Wertetypen – generische Checks, Invarianten dokumentiert?
- [x] `width.rs`: Unicode-Breitenberechnung (`unicode-width`) – Grenzfälle
  (Wide-Chars, Zero-Width, Kürzen mit `…`) getestet.
- [x] `style.rs` (`Style`, `Color`, `Modifier`, `Attribute`): ratatui-vertrautes
  Vokabular; `NO_COLOR`/Nicht-TTY-Verhalten; keine Magic-ANSI-Codes.
- [x] `text.rs` (`Span`, `Line`, `Text`): Builder-API konsistent; SLAP in
  Umbruch-/Zusammenführungslogik.
- [x] `border.rs` (`BorderType`, Default `Rounded`): Glyphen in zwei Stufen
  (Unicode + ASCII-Fallback) übers Theme wählbar.

## Phase 2 – core: markup, theme, terminal, render (`src/core/`)

- [x] `markup.rs` (Feature `markup`): `[bold red]…[/]`-Parser – defensive
  Fehlerpfade bei kaputter Markup, keine Panics; hinter `#[cfg(feature)]`
  korrekt abgetrennt.
- [x] `theme.rs` (`Theme`, `theme()`, `set_theme()`): **einheitliches Theme**
  für Input UND Output (SSOT, CLAUDE.md-Architektur); gedämpfte Optik, ein
  Akzentton, `dim` für Sekundärtext; globaler Zustand thread-sicher/dokumentiert.
- [x] `terminal.rs`: `crossterm`-Kapselung, TTY-Erkennung, `NO_COLOR`; keine
  Widget-Logik durchgesickert.
- [x] `render.rs` (`Renderable`, `Rendered`): der Render-Vertrag als
  Testschnittstelle (zu `Rendered` rendern, ohne TTY prüfen) – Kern-Contract
  klar und minimal.
- [x] `mod.rs`: Re-Exports minimal; nur was `output`/`input`/`lib` wirklich
  brauchen.

## Phase 3 – output: Widgets (`src/output/`)

Alle implementieren `Renderable`; je Datei generische Checks + Render-Tests zu
`Rendered` (kein TTY). Überlauf mit `…` kürzen, Default-Border `Rounded`.

- [x] Primitive: `rule.rs`, `badge.rs`, `alert.rs` (`AlertKind`), `kv.rs`,
  `list.rs` (`Marker`).
- [x] Container/Layout: `panel.rs`, `columns.rs`, `layout.rs`, `compose.rs`
  (`align`/`pad`/`vstack`) – SLAP in dichten Render-Funktionen, keine
  Magic-Strings für Labels/Glyphen.
- [x] Tabellarisch/strukturiert: `table.rs` (`Cell`, `Column`), `tree.rs`
  (`TreeNode`), `diff.rs`.
- [x] Dynamisch: `progress.rs` (`ProgressStyle`, `Thresholds`),
  `multiprogress.rs`, `spinner.rs` (`SpinnerStyle`), `live.rs` – Redraw-Logik,
  keine Ressourcen-Leaks, Nicht-TTY-Fallback.
- [x] `pager.rs` (Feature `pager`): `$PAGER`/`less`/`more`-Integration,
  Fehlerpfade wenn kein Pager verfügbar; sauber hinter `#[cfg(feature)]`.
- [x] `mod.rs`: Re-Exports minimal; feature-gated Items (`pager`) korrekt
  abgetrennt.

## Phase 4 – input: Fundament (`src/input/`)

Die geteilte Infrastruktur zuerst – sie trägt alle Prompts:

- [x] `event.rs` (`EventSource`, DI): Fake für headless-Tests (gescriptete
  Keys); das eine `#[allow(dead_code)]` prüfen – noch nötig oder entfernbar?
- [x] `line_edit.rs`: **SSOT für Texteingabe** – Cursor/Editing-Logik, von den
  Text-Prompts geteilt; generische Checks, keine Duplikate in den Widgets.
- [x] `guard.rs` (`TerminalGuard`, RAII): stellt das Terminal bei
  Drop/Fehler/Panik wieder her – Wiederherstellungspfad und `Drop`-Impl prüfen.
- [x] `prompt.rs`, `field.rs`, `validate.rs`, `history.rs`: gemeinsame
  Bausteine (Redraw-`frame`, Validierung, Verlauf) – klare, minimale
  Schnittstellen; `Outcome<T>` (`Submitted`/`Cancelled`) konsistent.

## Phase 5 – input: Prompts (`src/input/`)

Je Widget: `EventSource`-Fake-Test (headless), `Outcome<T>`-Rückgabe, geteilte
`line_edit`/`validate`-Nutzung statt Eigenlogik.

- [x] Text: `text.rs` (`TextInput`), `password.rs` (`PasswordInput`),
  `textarea.rs`, `number.rs` (`NumberInput`) – teilen sie `line_edit` sauber?
- [x] Auswahl: `confirm.rs`, `select.rs` (`Select`), `datepicker.rs`
  (`Date`/`DatePicker`), `shortcut.rs` – zyklische Navigation der Listen.
- [x] `fuzzy.rs` (Feature `fuzzy`): inline Fuzzy-Select über `nucleo-matcher`;
  sauber hinter `#[cfg(feature)]`, Fallback/Fehlerpfade.
- [x] `editor.rs` (`edit_file`): externer `$EDITOR`-Aufruf – Fehlerpfade,
  Temp-Datei-Handling.
- [x] `mod.rs`: `Outcome`-Re-Export + feature-gated Items (`fuzzy`) korrekt;
  Re-Exports minimal.

## Phase 6 – Crate-Root (`src/lib.rs`, `src/error.rs`)

Zuletzt, weil hier alle Schichten zusammenlaufen (öffentliche API):

- [x] `error.rs` (`SparcliError`, `Result`): `#[error(...)]`-Texte
  aussagekräftig, Fremdfehler via `#[from]`, ein kohärenter Lib-Fehlertyp; keine
  Infrastruktur-Leaks.
- [x] `lib.rs`: Modul-Deklarationen vollständig/konsistent; öffentliche
  Re-Exports und `prelude` minimal & konsistent (was gehört wirklich in
  `prelude`?); feature-gated `pub use` (`Pager`, `FuzzySelect`) korrekt;
  crate-weite `#![warn(missing_docs)]`-Begründung aktuell; Modul-`//!`-Doc und
  Intro-Beispiele stimmen.

## Phase 7 – Querschnitt & Abschluss

- [x] **`#[allow]`-Inventur:** das lokale `#[allow(dead_code)]` in
  `input/event.rs` bewusst bestätigen oder entfernen; keine weiteren Allows
  eingeschlichen.
- [x] **Feature-Matrix:** je Feature-Kombination baut & testet sauber –
  `cargo test` (default), `--features markup`, `--features fuzzy`,
  `--features pager`, `--all-features`; `cargo hack`/manuell gegenchecken, dass
  kein Item versehentlich ungated ist.
- [x] **Doku-Sync:** `README.md`, `API.md`, `DEVELOPMENT.md`, `CHANGELOG.md`
  gegen den aufgeräumten Stand; rustdoc-Beispiele/Doctests konsistent;
  Feature-Flags korrekt dokumentiert.
- [x] **Tests:** durch Refactors berührte Pfade getestet; alle grün (Unit +
  `tests/integration.rs` + Doctests).
- [x] **Abschluss-Gates:** `cargo fmt --check`, `cargo clippy --all-targets
  --all-features -- -D warnings`, `cargo test --all-features` – alles grün.
- [x] Commit-Nachricht(en) im Conventional-Commits-Stil vorschlagen (kein
  Auto-Commit gemäß CLAUDE.md).

## Verifikation

Nach jeder Schicht und am Ende: `cargo fmt --check` + `cargo clippy
--all-targets --all-features -- -D warnings` + `cargo test --all-features`
grün. Reine Refactorings dürfen das Verhalten nicht ändern – Render-Tests
(`Rendered`-Inhalt/Style) und Integrationstests (`tests/`) müssen ohne
Neu-Generierung bestehen; nur bei bewusster Verhaltens-/Layout-Änderung Tests
gezielt anpassen.

## Hinweise / Nicht-Ziele

- **Scope-Grenzen (CLAUDE.md):** Output komplett, Input nur Einzel-Widgets –
  kein Form/App/Args/Serde/Logging, Fuzzy nur als inline Select. Nichts davon
  im Aufräumen „nachrüsten“.
- **Kein async, kein ratatui:** schlanker Footprint ist Leitbild – keine neuen
  Dependencies ohne vorherige Abstimmung (CLAUDE.md §7.7).
- **Einheitliches Theme:** `core/theme.rs` gilt für Input UND Output – nicht
  aufspalten.
- KISS/YAGNI vor „mein Stil“: lokalen Stil respektieren, nur anfassen was die
  Aufgabe erfordert, Refactoring von Verhalten trennen (CLAUDE.md §3).
