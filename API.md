# sparcli API reference

The complete public API, grouped by layer. This mirrors the rustdoc
(`cargo doc --all-features --open`), which remains the canonical source.

**Conventions**

- Most types use a **fluent builder**: methods take `self` and return `Self`.
- Every output widget implements [`Renderable`](#renderable): `.print()` writes
  to stdout, `.print_to(&mut w)` writes to any `Write`, `.render(width)` returns
  a [`Rendered`](#rendered) for composition.
- Every prompt returns `Result<Outcome<T>>`, where
  [`Outcome`](#outcomet) is `Submitted(T)` or `Cancelled`. Prompts need an
  interactive terminal and otherwise return `SparcliError::NoTerminal`.
- Feature-gated items are marked **(feature `x`)**.
- Import the common types via `use sparcli::prelude::*;`.

---

## Contents

- [Core](#core) — colors, text, geometry, theme, render
- [Errors](#errors)
- [Output widgets](#output-widgets)
- [Input widgets](#input-widgets)

---

## Core

### Color

```rust
pub enum Color {
    Reset, Black, Red, Green, Yellow, Blue, Magenta, Cyan, Gray,
    DarkGray, LightRed, LightGreen, LightYellow, LightBlue,
    LightMagenta, LightCyan, White,
    Rgb(u8, u8, u8),
    Indexed(u8),
}

impl Color {
    pub fn from_name(name: &str) -> Option<Self>; // "red", "lightblue", …
    pub fn from_hex(hex: &str) -> Option<Self>;   // "#ff8800"
}
```

### Attribute / Modifier

`Modifier` is an alias of `Attribute`.

```rust
pub struct Attribute(/* bitflags */);

impl Attribute {
    pub const NONE: Self;
    pub const BOLD: Self;
    pub const DIM: Self;
    pub const ITALIC: Self;
    pub const UNDERLINED: Self;
    pub const STRIKETHROUGH: Self;

    pub fn contains(self, other: Self) -> bool;
    pub fn is_empty(self) -> bool;
}
// `a | b` combines attributes.
```

### Style

```rust
pub struct Style { pub fg: Option<Color>, pub bg: Option<Color>, pub attrs: Attribute }

impl Style {
    pub fn new() -> Self;
    pub fn fg(self, color: Color) -> Self;
    pub fn bg(self, color: Color) -> Self;
    pub fn add_modifier(self, attr: Attribute) -> Self;
    pub fn bold(self) -> Self;
    pub fn dim(self) -> Self;
    pub fn italic(self) -> Self;
    pub fn underlined(self) -> Self;
    pub fn strikethrough(self) -> Self;
    pub fn patch(self, other: Style) -> Self; // overlay set fields of `other`
}
// `Color` converts into `Style` via `From`.
```

### Span / Line / Text

```rust
pub struct Span { pub content: String, pub style: Style, pub link: Option<String> }

impl Span {
    pub fn raw(content: impl Into<String>) -> Self;
    pub fn styled(content: impl Into<String>, style: Style) -> Self;
    pub fn link(self, url: impl Into<String>) -> Self; // OSC-8 hyperlink
    pub fn width(&self) -> usize;
}

pub struct Line { pub spans: Vec<Span> }

impl Line {
    pub fn new(spans: Vec<Span>) -> Self;
    pub fn raw(content: impl Into<String>) -> Self;
    pub fn styled(content: impl Into<String>, style: Style) -> Self;
    pub fn width(&self) -> usize;
    pub fn plain(&self) -> String;
}

pub struct Text { pub lines: Vec<Line> }

impl Text {
    pub fn new(lines: Vec<Line>) -> Self;
    pub fn raw(content: impl Into<String>) -> Self;   // splits on '\n'
    pub fn styled(content: impl Into<String>, style: Style) -> Self;
    pub fn push_line(&mut self, line: impl Into<Line>);
    pub fn width(&self) -> usize;
    pub fn height(&self) -> usize;
}
```

`&str`/`String` convert into `Span`/`Line`/`Text`; `Span`→`Line`→`Text` too.

### Geometry

```rust
pub enum Align { Left, Center, Right }
pub enum VAlign { Top, Middle, Bottom }
pub enum Position { Top, Bottom }

pub struct Edges { pub top: u16, pub right: u16, pub bottom: u16, pub left: u16 }

impl Edges {
    pub fn all(value: u16) -> Self;
    pub fn symmetric(vertical: u16, horizontal: u16) -> Self;
    pub fn horizontal(self) -> u16;
    pub fn vertical(self) -> u16;
}

pub struct Title { /* content, align, position, pad */ }

impl Title {
    pub fn new(content: impl Into<Text>) -> Self;
    pub fn align(self, align: Align) -> Self;
    pub fn position(self, position: Position) -> Self;
    pub fn pad(self, pad: u16) -> Self;
    pub fn style(self, style: Style) -> Self;
}
```

### Border

```rust
pub enum BorderType { None, Ascii, Single, Double, Rounded, Thick } // default: Rounded

impl BorderType {
    pub fn chars(self) -> BorderChars;
    pub fn is_none(self) -> bool;
}

pub struct BorderChars { /* corners, edges, junctions (chars) */ }
```

### Theme

A single theme drives both output and input.

```rust
pub struct Theme {
    pub accent: Color,
    pub title: Style, pub heading: Style, pub secondary: Style,
    pub success: Style, pub error: Style, pub warning: Style,
    pub info: Style, pub debug: Style, pub hint: Style,
    pub selection: Style, pub cursor: Style,
    pub border: BorderType,
    pub unicode: bool, // false → ASCII glyph fallbacks
}

impl Theme {
    pub fn bullet(&self) -> &'static str;
    pub fn cursor_marker(&self) -> &'static str;
    pub fn marker(&self) -> &'static str;
    pub fn checkbox_on(&self) -> &'static str;
    pub fn checkbox_off(&self) -> &'static str;
}

pub fn theme() -> Theme;          // current theme (clone)
pub fn set_theme(new_theme: Theme); // replace process-wide theme
```

### Width helpers

```rust
pub fn visible_width(text: &str) -> usize;       // ANSI-aware column width
pub fn strip_ansi(text: &str) -> String;
pub fn truncate(text: &str, max_cols: usize, ellipsis: &str) -> String;
pub fn wrap(text: &str, width: usize) -> Vec<String>;
```

### Terminal

```rust
pub enum ColorSupport { None, Ansi16, TrueColor }

pub fn terminal_size() -> (u16, u16); // (cols, rows), fallback 80x24
pub fn term_width() -> u16;
pub fn term_height() -> u16;
pub fn is_output_tty() -> bool;
pub fn is_input_tty() -> bool;
pub fn color_support() -> ColorSupport; // honors NO_COLOR / CLICOLOR_FORCE
```

### Rendered

```rust
pub struct Rendered { pub lines: Vec<Line> }

impl Rendered {
    pub fn new(lines: Vec<Line>) -> Self;
    pub fn empty() -> Self;
    pub fn push(&mut self, line: impl Into<Line>);
    pub fn width(&self) -> usize;
    pub fn height(&self) -> usize;
    pub fn plain(&self) -> String;
}
```

### Renderable

```rust
pub trait Renderable {
    fn render(&self, max_width: u16) -> Rendered;
    fn print(&self) -> Result<()>;                       // to stdout
    fn print_to<W: Write>(&self, writer: &mut W) -> Result<()>;
}

// Low-level flush, if you manage the writer yourself:
pub fn write_rendered<W: Write>(
    writer: &mut W, rendered: &Rendered, support: ColorSupport,
) -> std::io::Result<()>;
```

### Markup — feature `markup`

Rich-style `[bold red]…[/]`, `#rrggbb`, `on <color>`, backtick `` `code` ``.

```rust
pub fn parse(markup: &str) -> Text;
pub fn markup_print(markup: &str) -> Result<()>;
pub fn markup_println(markup: &str) -> Result<()>;
```

---

## Errors

```rust
pub enum SparcliError {
    Io(std::io::Error),
    NoTerminal,
    Config(String),
}

pub type Result<T> = std::result::Result<T, SparcliError>;
```

---

## Output widgets

All implement [`Renderable`](#renderable).

### Table / Column / Cell

```rust
pub struct Column { /* … */ }
impl Column {
    pub fn new(header: impl Into<Text>) -> Self;
    pub fn align(self, align: Align) -> Self;
    pub fn min_width(self, width: usize) -> Self;
    pub fn max_width(self, width: usize) -> Self;
    pub fn fixed_width(self, width: usize) -> Self;
    pub fn wrap(self) -> Self; // wrap instead of truncate
}

pub struct Cell { /* … */ }
impl Cell {
    pub fn new(content: impl Into<Text>) -> Self;
    pub fn align(self, align: Align) -> Self;
    pub fn colspan(self, columns: usize) -> Self;
    pub fn rowspan(self, rows: usize) -> Self;
}

pub struct Table { /* … */ }
impl Table {
    pub fn new() -> Self;
    pub fn column(self, column: impl Into<Column>) -> Self;
    pub fn columns<I, C>(self, columns: I) -> Self where I: IntoIterator<Item = C>, C: Into<Column>;
    pub fn row<I, C>(self, cells: I) -> Self where I: IntoIterator<Item = C>, C: Into<Cell>;
    pub fn footer_row<I, C>(self, cells: I) -> Self where I: IntoIterator<Item = C>, C: Into<Cell>;
    pub fn border(self, border: BorderType) -> Self;
    pub fn header(self, show: bool) -> Self;
    pub fn striped(self, striped: bool) -> Self;
    pub fn title(self, title: impl Into<Text>) -> Self;
    pub fn pad(self, pad: u16) -> Self;
    pub fn row_separators(self, on: bool) -> Self;
}
```

> Note: `colspan` and `rowspan` are supported. Pair `rowspan` with the default
> (no row separators) for the cleanest result.

### Panel

```rust
impl Panel {
    pub fn new(content: impl Into<Text>) -> Self;
    pub fn from_rendered(content: Rendered) -> Self;
    pub fn border(self, border: BorderType) -> Self;
    pub fn border_style(self, style: Style) -> Self;
    pub fn fill(self, style: Style) -> Self;
    pub fn padding(self, padding: Edges) -> Self;
    pub fn title(self, title: impl Into<Title>) -> Self;
    pub fn subtitle(self, subtitle: Title) -> Self;
    pub fn width(self, width: u16) -> Self;
    pub fn content_align(self, align: Align) -> Self;
}
```

### Alert

```rust
pub enum AlertKind { Info, Debug, Warning, Error, Success }

impl Alert {
    pub fn new(kind: AlertKind, content: impl Into<Text>) -> Self;
    pub fn info(content: impl Into<Text>) -> Self;
    pub fn debug(content: impl Into<Text>) -> Self;
    pub fn warning(content: impl Into<Text>) -> Self;
    pub fn error(content: impl Into<Text>) -> Self;
    pub fn success(content: impl Into<Text>) -> Self;
}
```

### Rule

```rust
impl Rule {
    pub fn new() -> Self;
    pub fn with_title(title: impl Into<Text>) -> Self;
    pub fn border(self, border: BorderType) -> Self;
    pub fn style(self, style: Style) -> Self;
    pub fn align(self, align: Align) -> Self;
    pub fn width(self, width: u16) -> Self;
    pub fn margin(self, margin: Edges) -> Self;
}
```

### List / Marker

```rust
pub enum Marker { Bullet, Number, AlphaLower, AlphaUpper, RomanLower, RomanUpper }

impl List {
    pub fn new() -> Self;                      // bulleted
    pub fn ordered(marker: Marker) -> Self;
    pub fn item(self, content: impl Into<Text>) -> Self;
    pub fn item_with(self, content: impl Into<Text>, children: List) -> Self;
    pub fn bullet(self, glyph: impl Into<String>) -> Self;
    pub fn marker_style(self, style: Style) -> Self;
    pub fn indent(self, indent: u16) -> Self;
    pub fn item_gap(self, gap: u16) -> Self;
    pub fn margin(self, margin: Edges) -> Self;
}
```

### Tree / TreeNode

```rust
impl TreeNode {
    pub fn new(content: impl Into<Text>) -> Self;
    pub fn child(self, child: TreeNode) -> Self;
}

impl Tree {
    pub fn new() -> Self;
    pub fn node(self, node: TreeNode) -> Self;
    pub fn border(self, border: BorderType) -> Self;
    pub fn connector_style(self, style: Style) -> Self;
    pub fn no_guides(self) -> Self;
}
```

### KeyValue

```rust
impl KeyValue {
    pub fn new() -> Self;
    pub fn add(self, key: impl Into<String>, value: impl Into<Text>) -> Self;
    pub fn separator(self, separator: impl Into<String>) -> Self;
    pub fn key_width(self, width: u16) -> Self;
    pub fn key_style(self, style: Style) -> Self;
    pub fn value_style(self, style: Style) -> Self;
    pub fn item_gap(self, gap: u16) -> Self;
    pub fn wrap_values(self, wrap: bool) -> Self;
    pub fn margin(self, margin: Edges) -> Self;
}
```

### Badge

```rust
impl Badge {
    pub fn new(text: impl Into<String>) -> Self;
    pub fn caps(self, left: impl Into<String>, right: impl Into<String>) -> Self;
    pub fn style(self, style: Style) -> Self;
    pub fn pad(self, pad: u16) -> Self;
    pub fn span(&self) -> Span; // for embedding inline
}
```

### Columns

```rust
impl Columns {
    pub fn new() -> Self;
    pub fn add(self, content: &impl Renderable, width: u16) -> Self;
    pub fn add_rendered(self, block: Rendered) -> Self;
    pub fn align(self, align: Align) -> Self;   // of the last column
    pub fn gap(self, gap: u16) -> Self;
    pub fn separator(self, border: BorderType) -> Self;
    pub fn valign(self, valign: VAlign) -> Self;
}
```

### Diff

```rust
impl Diff {
    pub fn new(old: impl Into<String>, new: impl Into<String>) -> Self;
    pub fn context(self, lines: usize) -> Self;
    pub fn no_header(self) -> Self;
    pub fn labels(self, old: impl Into<String>, new: impl Into<String>) -> Self;
}
```

### ProgressBar

```rust
pub enum ProgressStyle { Block, Ascii, Line, Shaded }

pub struct Thresholds {
    pub mid: f64, pub high: f64,
    pub low_color: Color, pub mid_color: Color, pub high_color: Color,
}

impl ProgressBar {
    pub fn new() -> Self;
    pub fn style(self, style: ProgressStyle) -> Self;
    pub fn caps(self, left: impl Into<String>, right: impl Into<String>) -> Self;
    pub fn fill_color(self, color: Color) -> Self;
    pub fn thresholds(self, thresholds: Thresholds) -> Self;
    pub fn show_percent(self, show: bool) -> Self;
    pub fn show_value(self, show: bool) -> Self;
    pub fn width(self, width: u16) -> Self;
    pub fn label(self, label: impl Into<String>) -> Self;

    pub fn bar(&self, value: f64, max: f64) -> Rendered;        // static frame
    pub fn draw(&mut self, value: f64, max: f64) -> Result<()>; // in place
    pub fn finish(self, value: f64, max: f64) -> Result<()>;
}
```

### Spinner

```rust
pub enum SpinnerStyle { Braille, Pipe, Dots, Arrow }

impl Spinner {
    pub fn new(label: impl Into<String>) -> Self;
    pub fn style(self, style: SpinnerStyle) -> Self;
    pub fn color(self, color: Color) -> Self;
    pub fn set_label(&mut self, label: impl Into<String>);
    pub fn frame(&self) -> Rendered;
    pub fn tick(&mut self) -> Result<()>;
    pub fn finish(self, success: bool, label: impl Into<String>) -> Result<()>;
}
```

### MultiProgress

```rust
impl MultiProgress {
    pub fn new() -> Self;
    pub fn transient(self) -> Self;            // erase on finish
    pub fn add(&mut self, bar: ProgressBar) -> usize; // returns index
    pub fn update(&mut self, index: usize, value: f64, max: f64) -> Result<()>;
    pub fn finish(self) -> Result<()>;
}
```

### Live

```rust
impl Live {
    pub fn new() -> Self;       // no-op redraws off-terminal
    pub fn always() -> Self;    // redraw even off-terminal
    pub fn update(&mut self, content: &impl Renderable) -> Result<()>;
    pub fn finish(self) -> Result<()>; // leave final frame
    pub fn clear(self) -> Result<()>;  // erase
}
```

### Pager — feature `pager`

```rust
impl Pager {
    pub fn new() -> Self;
    pub fn command(self, command: impl Into<String>) -> Self; // overrides $PAGER
    pub fn always(self) -> Self;
    pub fn page(&self, content: &impl Renderable) -> Result<()>;
}
```

### Composition helpers

```rust
pub fn align(block: &Rendered, width: u16, how: Align) -> Rendered;
pub fn pad(block: &Rendered, edges: Edges) -> Rendered;
pub fn vstack(parts: &[Rendered], gap: u16) -> Rendered;
```

---

## Input widgets

### Outcome\<T\>

```rust
pub enum Outcome<T> { Submitted(T), Cancelled, Shortcut(i32) }

impl<T> Outcome<T> {
    pub fn submitted(self) -> Option<T>;
    pub fn is_cancelled(&self) -> bool;
    pub fn shortcut_id(&self) -> Option<i32>; // Some when ended on a shortcut
}
```

### Confirm → `Outcome<bool>`

```rust
impl Confirm {
    pub fn new(question: impl Into<String>) -> Self;
    pub fn default_yes(self) -> Self;
    pub fn labels(self, yes: impl Into<String>, no: impl Into<String>) -> Self;
    pub fn shortcuts<I: IntoIterator<Item = Shortcut>>(self, s: I) -> Self;
    pub fn run(self) -> Result<Outcome<bool>>;
}
```

### TextInput → `Outcome<String>`

```rust
impl TextInput {
    pub fn new(prompt: impl Into<String>) -> Self;
    pub fn initial(self, value: impl Into<String>) -> Self;
    pub fn placeholder(self, value: impl Into<String>) -> Self;
    pub fn max_chars(self, max: usize) -> Self;
    pub fn validate(self, validator: Validator) -> Self;
    pub fn char_filter(self, filter: CharFilter) -> Self;
    pub fn suggestions<I, S>(self, suggestions: I) -> Self; // ghost completion
    pub fn dropdown(self) -> Self;          // navigable list instead of ghost
    pub fn fuzzy_suggestions(self) -> Self; // subsequence match (vs prefix)
    pub fn hide_char_count(self) -> Self;   // hides the (n/max) counter
    pub fn history<I, S>(self, entries: I) -> Self;         // Up/Down recall
    pub fn history_app(self, app: impl Into<String>) -> Self; // persisted
    pub fn editor(self) -> Self;            // Ctrl-G opens $EDITOR
    pub fn editor_command(self, command: impl Into<String>) -> Self;
    pub fn run(self) -> Result<Outcome<String>>;
}
```

### PasswordInput → `Outcome<String>`

```rust
impl PasswordInput {
    pub fn new(prompt: impl Into<String>) -> Self;
    pub fn mask(self, mask: impl Into<String>) -> Self; // empty hides length
    pub fn max_chars(self, max: usize) -> Self;
    pub fn validate(self, validator: Validator) -> Self;
    pub fn char_filter(self, filter: CharFilter) -> Self;
    pub fn run(self) -> Result<Outcome<String>>;
}
```

### NumberInput → `Outcome<f64>`

```rust
impl NumberInput {
    pub fn new(prompt: impl Into<String>) -> Self;
    pub fn initial(self, value: f64) -> Self;
    pub fn range(self, min: f64, max: f64) -> Self;
    pub fn step(self, step: f64) -> Self;
    pub fn decimals(self, decimals: usize) -> Self;
    pub fn calculator(self) -> Self; // accept `+ - * / ( )` expressions
    pub fn run(self) -> Result<Outcome<f64>>;
}

// Standalone expression evaluator (also used by calculator mode):
pub fn eval(expr: &str) -> Result<f64, String>;
```

### Textarea → `Outcome<String>`

```rust
impl Textarea {
    pub fn new(prompt: impl Into<String>) -> Self;
    pub fn initial(self, value: impl Into<String>) -> Self;
    pub fn editor(self) -> Self;            // Ctrl-G opens $EDITOR
    pub fn editor_command(self, command: impl Into<String>) -> Self;
    pub fn run(self) -> Result<Outcome<String>>; // Enter=newline, Ctrl-D=submit
}
```

### Select → `Outcome<usize>` / `Outcome<Vec<usize>>`

```rust
impl Select {
    pub fn new(prompt: impl Into<String>) -> Self;
    pub fn options<I, S>(self, options: I) -> Self;
    pub fn multi(self) -> Self;
    pub fn max_visible(self, rows: usize) -> Self;
    pub fn no_cycle(self) -> Self;
    pub fn shortcuts<I: IntoIterator<Item = Shortcut>>(self, s: I) -> Self;
    pub fn run(self) -> Result<Outcome<usize>>;
    pub fn run_multi(self) -> Result<Outcome<Vec<usize>>>;
}
```

`shortcuts` adds a footer hint and a `?` help overlay; a bound key ends the
prompt with `Outcome::Shortcut(id)`.

### FuzzySelect — feature `fuzzy` → `Outcome<usize>` / `Outcome<Vec<usize>>`

```rust
impl FuzzySelect {
    pub fn new(prompt: impl Into<String>) -> Self;
    pub fn options<I, S>(self, options: I) -> Self;
    pub fn multi(self) -> Self;
    pub fn max_visible(self, rows: usize) -> Self;
    pub fn shortcuts<I: IntoIterator<Item = Shortcut>>(self, s: I) -> Self;
    pub fn run(self) -> Result<Outcome<usize>>;
    pub fn run_multi(self) -> Result<Outcome<Vec<usize>>>;
}
```

### DatePicker / Date → `Outcome<Date>`

```rust
pub struct Date { pub year: i32, pub month: u32, pub day: u32 }

impl Date {
    pub fn new(year: i32, month: u32, day: u32) -> Self;
    pub fn empty() -> Self;          // "no date" sentinel
    pub fn is_empty(self) -> bool;
    pub fn today() -> Self;
    pub fn days_in_month(self) -> u32;
    pub fn weekday_monday0(self) -> u32; // 0 = Monday
    pub fn add_days(self, delta: i64) -> Self;
    pub fn add_months(self, delta: i32) -> Self;
}

impl DatePicker {
    pub fn new(prompt: impl Into<String>) -> Self;
    pub fn initial(self, date: Date) -> Self;
    pub fn allow_clear(self) -> Self; // Del/Backspace -> Date::empty()
    pub fn shortcuts<I: IntoIterator<Item = Shortcut>>(self, s: I) -> Self;
    pub fn run(self) -> Result<Outcome<Date>>;
}
```

`shortcuts` adds a footer hint and a `?` help overlay; a bound key ends the
prompt with `Outcome::Shortcut(id)`.

### Validation (`input::validate`)

```rust
pub type Validator = Box<dyn Fn(&str) -> Result<(), String>>;
pub type CharFilter = Box<dyn Fn(char) -> bool>;

pub fn non_empty() -> Validator;
pub fn min_len(min: usize) -> Validator;
pub fn digits() -> CharFilter;
pub fn decimal() -> CharFilter;
pub fn alpha() -> CharFilter;
pub fn alnum() -> CharFilter;
pub fn no_space() -> CharFilter;
```

### History (`input::history`)

```rust
impl History {
    pub fn new() -> Self;
    pub fn for_app(app: &str) -> Self;   // XDG state dir
    pub fn max_entries(self, max: usize) -> Self;
    pub fn keep_duplicates(self) -> Self;
    pub fn entries(&self) -> &[String];
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn add(&mut self, line: &str);
    pub fn load(&mut self) -> Result<()>;
    pub fn save(&self) -> Result<()>;
}
```

### Shortcuts (`input::shortcut`)

```rust
pub struct Shortcut { pub key: KeyPress, pub id: i32, pub label: String }
impl Shortcut { pub fn new(key: KeyPress, id: i32, label: impl Into<String>) -> Self; }

pub fn find(key: KeyPress, shortcuts: &[Shortcut]) -> Option<i32>;
pub fn hint_line(shortcuts: &[Shortcut]) -> Line;   // footer hint
pub fn help_overlay(shortcuts: &[Shortcut]) -> Vec<Line>; // `?` overlay lines
pub fn key_name(key: KeyPress) -> String;           // e.g. "Ctrl-S"
```

`Select` and `FuzzySelect` accept `.shortcuts(...)` directly; the standalone
helpers above are for building your own prompt loops.

### External editor (`input::editor`)

```rust
// Open an external editor ($VISUAL / $EDITOR, or an override) on a file:
pub fn edit_file(command: Option<&str>, path: &Path) -> Result<()>;
```

Text prompts call this internally for Ctrl-G; `edit_file` is exposed for
standalone use.

### Events (`input::event`)

```rust
pub enum KeyCode {
    Char(char), Enter, Esc, Tab, BackTab, Backspace, Delete,
    Up, Down, Left, Right, Home, End, PageUp, PageDown,
    Function(u8), Unknown,
}

pub struct KeyPress { pub code: KeyCode, pub ctrl: bool, pub alt: bool, pub shift: bool }
impl KeyPress {
    pub fn new(code: KeyCode) -> Self;
    pub fn ctrl(letter: char) -> Self;
    pub fn is_ctrl(&self, letter: char) -> bool;
}

pub enum InputEvent { Key(KeyPress), Paste(String), Resize }

pub trait EventSource { fn next_event(&mut self) -> Result<InputEvent>; }
pub struct CrosstermSource; // the real terminal source
```

### LineEditor (`input::line_edit`)

The shared single-/multi-line text-editing core (caret + selection +
in-process clipboard). Useful when building a custom prompt.

```rust
impl LineEditor {
    pub fn new(initial: &str, multiline: bool) -> Self;
    pub fn value(&self) -> String;
    pub fn set_value(&mut self, value: &str);
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn cursor(&self) -> usize;
    pub fn lines(&self) -> Vec<String>;
    pub fn cursor_line_col(&self) -> (usize, usize);
    pub fn has_selection(&self) -> bool;
    pub fn selection_range(&self) -> Option<(usize, usize)>;
    pub fn insert_char(&mut self, ch: char);
    pub fn insert_str(&mut self, text: &str);
    pub fn insert_newline(&mut self);
    pub fn backspace(&mut self);
    pub fn delete(&mut self);
    pub fn move_left(&mut self, select: bool);
    pub fn move_right(&mut self, select: bool);
    pub fn move_home(&mut self, select: bool);
    pub fn move_end(&mut self, select: bool);
    pub fn move_up(&mut self, select: bool);
    pub fn move_down(&mut self, select: bool);
    pub fn select_all(&mut self);
    pub fn delete_word_back(&mut self);
    pub fn kill_to_line_start(&mut self);
    pub fn kill_to_line_end(&mut self);
    pub fn copy(&mut self);
    pub fn cut(&mut self);
    pub fn paste(&mut self);
}
```

### TerminalGuard (`input::guard`)

RAII: enables raw mode + bracketed paste on `new`, restores both on drop
(even on early return or panic).

```rust
impl TerminalGuard {
    pub fn new() -> Result<Self>;
}
```
