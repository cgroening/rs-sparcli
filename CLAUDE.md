# CLAUDE.md – sparcli (Rust)

Anforderungen für alle künftigen Sessions in diesem Projekt. Bei Konflikt geht
diese Datei vor Konventionen, aber nicht vor expliziten Nutzeranweisungen.

## Was ist das?

`sparcli` ist ein **leichtes, plattformunabhängiges** Toolkit (macOS, Windows,
Linux) für **styled CLI-Ausgabe** und **interaktive Einzel-Input-Widgets** –
ein nativer Rust-Port der C-Library `sparcli`. Leitbild: schlank, für kleine
CLI-Tools. Kein async, kein ratatui, minimaler Footprint.

- **Fundament:** eigener Renderer auf `crossterm` (kein ratatui).
- **API-Gefühl:** ratatui-vertrautes Vokabular (`Style`, `Color`, `Span`,
  `Line`, `Text`, `Modifier`), fluentes Builder-API **und** Options-Struct.
- **Scope:** Output komplett; Input nur Einzel-Widgets (kein Form/App/Args/
  Serde). Fuzzy nur als inline Select.

## Architektur (Schichten strikt trennen, §2.6/§7.2)

- `core/` – Fundament: style, text, markup, theme, border, geometry, width,
  terminal, render. Keine Widget-Logik.
- `output/` – druckbare Widgets, implementieren `Renderable`.
- `input/` – interaktive Prompts über `EventSource` (DI) + `frame`-Redraw +
  `line_edit` (SSOT für Texteingabe).
- Abhängigkeitsrichtung: `output`/`input` → `core`. Niemals zyklisch.
- **Einheitliches Theme** in `core/theme.rs` gilt für Input UND Output.

## Dependencies & Feature-Flags

- Default (immer): `crossterm`, `unicode-width`, `thiserror`, `log`.
- Opt-in: `markup`, `fuzzy` (`nucleo-matcher`), `pager`.
- Neue Dependencies **vorher mit dem Nutzer abstimmen** (§7.7).
- Etablierte Crates mit `// https://crates.io/crates/<name>` über dem `use`.
- **Logging:** nur die `log`-Facade und nur als `warn!`/`debug!` an Stellen, wo
  ein `Result` sonst still verschluckt würde (z. B. Terminal-Restore im
  `TerminalGuard`, History-Save/Load, Temp-Cleanup). Keine `error!`-Logs – echte
  Fehler kommen über `SparcliError` zurück (kein Doppel-Logging); kein Logger/
  Backend mitliefern (entscheidet die App); nichts in Hot Paths/Render-Schleifen.

## Fehlerbehandlung (§7.3) – sehr wichtig, robust & langlebig

- Kein `unwrap()`; `expect()` nur an beweisbar unfehlbaren Stellen mit
  Begründung. Kein `panic!` im Normalbetrieb.
- Fehler über `Result<T, E>` + `?`. Lib-Fehler als `thiserror`-Enum
  (`SparcliError`), Fremdfehler via `#[from]`.
- Input-Prompts geben `Outcome<T>` zurück (`Submitted` / `Cancelled`).
- `TerminalGuard` (RAII) stellt das Terminal bei Drop/Fehler/Panik wieder her.
- Defensiv: Eingaben/Grenzfälle absichern, lieber kontrolliert fehlschlagen.

## Clean Code (§1, §2.5) – Kernpunkte

- SRP für Funktionen/Structs; Funktionen klein halten.
- **≤ 3 Funktionsparameter**; mehr → in Struct/Opts bündeln.
- Frühe Returns (Guard Clauses), max. 2 Verschachtelungsebenen.
- SLAP (eine Abstraktionsebene pro Funktion); keine Flag-Argumente.
- Keine Magic Numbers/Strings → benannte Konstanten.
- Starke Typisierung: `enum`s statt magischer Strings, `struct`s statt Tupel.
- Lookup-Tables statt großer `match` für konstante Zuordnungen.

## Stil & Tooling

- Edition 2024. `cargo fmt` (rustfmt.toml: max_width 80).
- `cargo clippy --all-targets -- -D warnings` muss sauber sein.
- 80-Zeichen-Codezeilen; gerade Anführungszeichen.
- **Kein Geviertstrich** als Gedankenstrich: in Code-Dateien den
  Bindestrich, in `.md`-Dateien den Halbgeviertstrich `–` verwenden.
- rustdoc auf jedem öffentlichen Item, Modul-`//!`; `# Examples`/`# Errors`/
  `# Panics` wo zutreffend; `#![warn(missing_docs)]`.

## Optik (§7.10)

- Gedämpfte Optik, ein Akzentton, `dim` für Sekundärtext.
- Default-Border `Rounded`; Überlauf mit `…` kürzen.
- Glyphen in zwei Stufen (Unicode + ASCII-Fallback), übers Theme wählbar.
- Auswahllisten navigieren zyklisch; `NO_COLOR`/Nicht-TTY respektieren.

## Tests (§2.8/§7.8) – Pflicht

- Unit-Tests in `#[cfg(test)] mod tests` je Datei; Integration in `tests/`.
- Testnamen beschreiben das erwartete Verhalten; Fakes vor Mocks.
- Output: zu `Rendered` rendern und Inhalt/Style prüfen (kein TTY).
- Input: über `EventSource`-Fake (gescriptete Keys) headless treiben.
- **Nach jeder Änderung alle Tests laufen lassen** (`cargo test`).
- Doctests in `# Examples` zählen als Tests und müssen laufen.

## Doku/Wartung

- Bei Änderungen README/rustdoc und Tests mitziehen.
- Toten/auskommentierten Code entfernen; Ursache statt Symptom beheben.
