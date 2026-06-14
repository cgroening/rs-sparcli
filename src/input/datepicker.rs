//! Month-grid calendar date picker.
//!
//! Uses a small built-in civil-date implementation (no external date crate).

use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::render::Rendered;
use crate::core::style::Style;
use crate::core::terminal::is_input_tty;
use crate::core::text::{Line, Span};
use crate::core::theme::{Theme, theme};
use crate::error::{Result, SparcliError};
use crate::input::Outcome;
use crate::input::event::{
    CrosstermSource, EventSource, InputEvent, KeyCode, KeyPress,
};
use crate::input::guard::TerminalGuard;
use crate::input::prompt::{Flow, run_prompt};
use crate::input::shortcut::{self, Shortcut};

/// Seconds in a day.
const SECONDS_PER_DAY: u64 = 86_400;

/// A calendar date (proleptic Gregorian).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    /// The year.
    pub year: i32,
    /// The month, 1-12.
    pub month: u32,
    /// The day of month, 1-31.
    pub day: u32,
}

impl Date {
    /// Creates a date (values are not validated here).
    pub fn new(year: i32, month: u32, day: u32) -> Self {
        Self { year, month, day }
    }

    /// The "no date" sentinel (all fields zero).
    pub fn empty() -> Self {
        Self {
            year: 0,
            month: 0,
            day: 0,
        }
    }

    /// Returns `true` if this is the "no date" sentinel.
    pub fn is_empty(self) -> bool {
        self.year == 0 && self.month == 0 && self.day == 0
    }

    /// Returns today's local-free (UTC) date, or 1970-01-01 on failure.
    pub fn today() -> Self {
        let days = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| (d.as_secs() / SECONDS_PER_DAY) as i64)
            .unwrap_or(0);
        Self::from_epoch_days(days)
    }

    /// Returns the number of days in this date's month.
    pub fn days_in_month(self) -> u32 {
        match self.month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if is_leap(self.year) => 29,
            2 => 28,
            _ => 30,
        }
    }

    /// Returns the weekday, 0 = Monday .. 6 = Sunday.
    pub fn weekday_monday0(self) -> u32 {
        let days = self.to_epoch_days();
        // Day 0 (1970-01-01) was a Thursday (= 3 with Monday=0).
        (((days % 7) + 7 + 3) % 7) as u32
    }

    /// Returns this date shifted by `delta` days.
    pub fn add_days(self, delta: i64) -> Self {
        Self::from_epoch_days(self.to_epoch_days() + delta)
    }

    /// Returns this date shifted by `delta` months, clamping the day.
    pub fn add_months(self, delta: i32) -> Self {
        let zero_based = self.month as i32 - 1 + delta;
        let year = self.year + zero_based.div_euclid(12);
        let month = zero_based.rem_euclid(12) as u32 + 1;
        let mut date = Self::new(year, month, 1);
        date.day = self.day.min(date.days_in_month());
        date
    }

    /// Converts to days since the Unix epoch.
    fn to_epoch_days(self) -> i64 {
        let year = if self.month <= 2 {
            self.year - 1
        } else {
            self.year
        } as i64;
        let era = year.div_euclid(400);
        let yoe = year - era * 400;
        let month = self.month as i64;
        let mp = if month > 2 { month - 3 } else { month + 9 };
        let doy = (153 * mp + 2) / 5 + self.day as i64 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// Converts from days since the Unix epoch.
    fn from_epoch_days(days: i64) -> Self {
        let z = days + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
        let year = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = if month <= 2 { year + 1 } else { year } as i32;
        Self { year, month, day }
    }
}

/// Returns whether `year` is a leap year.
fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Mutable state of a running date picker.
struct State {
    date: Date,
    help: bool,
}

/// A month-grid date picker prompt.
pub struct DatePicker {
    prompt: String,
    initial: Date,
    allow_clear: bool,
    shortcuts: Vec<Shortcut>,
}

impl DatePicker {
    /// Creates a date picker starting at today.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            initial: Date::today(),
            allow_clear: false,
            shortcuts: Vec::new(),
        }
    }

    /// Sets the initially selected date.
    #[must_use]
    pub fn initial(mut self, date: Date) -> Self {
        self.initial = date;
        self
    }

    /// Allows Delete/Backspace to clear the selection to "no date".
    ///
    /// A cleared selection submits [`Date::empty`]; check it with
    /// [`Date::is_empty`].
    #[must_use]
    pub fn allow_clear(mut self) -> Self {
        self.allow_clear = true;
        self
    }

    /// Registers shortcuts shown in a footer hint and the `?` help overlay.
    ///
    /// Pressing a bound key ends the prompt with [`Outcome::Shortcut`].
    #[must_use]
    pub fn shortcuts<I>(mut self, shortcuts: I) -> Self
    where
        I: IntoIterator<Item = Shortcut>,
    {
        self.shortcuts = shortcuts.into_iter().collect();
        self
    }

    /// Runs the picker on the real terminal.
    ///
    /// # Errors
    /// Returns [`SparcliError::NoTerminal`] without an interactive terminal,
    /// or [`SparcliError::Io`] on a terminal failure.
    pub fn run(self) -> Result<Outcome<Date>> {
        if !is_input_tty() {
            return Err(SparcliError::NoTerminal);
        }
        let _guard = TerminalGuard::new()?;
        let mut source = CrosstermSource;
        self.run_with(&mut source)
    }

    /// Runs the picker against any event source (used for tests).
    fn run_with(&self, source: &mut impl EventSource) -> Result<Outcome<Date>> {
        let mut state = State {
            date: self.initial,
            help: false,
        };
        run_prompt(
            source,
            &mut state,
            |state, _| self.render(state),
            |state, event| self.handle(state, event),
        )
    }

    /// Builds the calendar frame for the selected date.
    fn render(&self, state: &State) -> Rendered {
        let theme = theme();
        if state.help {
            return Rendered::new(shortcut::help_overlay(&self.shortcuts));
        }
        let mut lines = vec![Line::styled(self.prompt.clone(), theme.title)];
        if state.date.is_empty() {
            lines.push(Line::styled(
                "(no date) — press an arrow to choose".to_string(),
                theme.secondary,
            ));
        } else {
            lines.push(header_line(state.date, &theme));
            lines.push(weekday_header(&theme));
            lines.extend(month_grid(state.date, &theme));
        }
        if !self.shortcuts.is_empty() {
            lines.push(shortcut::hint_line(&self.shortcuts));
        }
        Rendered::new(lines)
    }

    /// Handles one event for the picker.
    fn handle(&self, state: &mut State, event: InputEvent) -> Flow<Date> {
        let InputEvent::Key(key) = event else {
            return Flow::Continue;
        };
        if state.help {
            state.help = false;
            return Flow::Continue;
        }
        if key.code == KeyCode::Char('?') && !self.shortcuts.is_empty() {
            state.help = true;
            return Flow::Continue;
        }
        if let Some(id) = shortcut::find(key, &self.shortcuts) {
            return Flow::Shortcut(id);
        }
        if matches!(key.code, KeyCode::Esc) {
            return Flow::Cancel;
        }
        if matches!(key.code, KeyCode::Enter) {
            return Flow::Submit(state.date);
        }
        if self.allow_clear
            && matches!(key.code, KeyCode::Delete | KeyCode::Backspace)
        {
            state.date = Date::empty();
            return Flow::Continue;
        }
        if state.date.is_empty() {
            // Any navigation key starts editing from today.
            state.date = Date::today();
            return Flow::Continue;
        }
        apply_key(&mut state.date, key)
    }
}

/// Builds the "Month Year" header line.
fn header_line(date: Date, theme: &Theme) -> Line {
    let label = format!("{} {}", month_name(date.month), date.year);
    Line::styled(label, theme.heading)
}

/// Builds the weekday column header (Monday-first).
fn weekday_header(theme: &Theme) -> Line {
    Line::styled("Mo Tu We Th Fr Sa Su".to_string(), theme.secondary)
}

/// Builds the day grid, highlighting the selected day.
fn month_grid(selected: Date, theme: &Theme) -> Vec<Line> {
    let first = Date::new(selected.year, selected.month, 1);
    let lead = first.weekday_monday0() as usize;
    let days = selected.days_in_month();
    let mut lines = Vec::new();
    let mut spans = vec![Span::raw("   ".repeat(lead))];
    let mut column = lead;
    for day in 1..=days {
        spans.push(day_span(day, day == selected.day, theme));
        column += 1;
        if column == 7 {
            lines.push(Line::new(std::mem::take(&mut spans)));
            column = 0;
        }
    }
    if !spans.is_empty() {
        lines.push(Line::new(spans));
    }
    lines
}

/// Renders one day cell, highlighting the selection.
fn day_span(day: u32, selected: bool, theme: &Theme) -> Span {
    let label = format!("{day:>2} ");
    let style = if selected {
        theme.selection
    } else {
        Style::new()
    };
    Span::styled(label, style)
}

/// Returns the English month name.
fn month_name(month: u32) -> &'static str {
    const NAMES: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    NAMES
        .get((month as usize).saturating_sub(1))
        .copied()
        .unwrap_or("?")
}

/// Applies a navigation key press to the selected (non-empty) date.
fn apply_key(date: &mut Date, key: KeyPress) -> Flow<Date> {
    match key.code {
        KeyCode::Left => *date = date.add_days(-1),
        KeyCode::Right => *date = date.add_days(1),
        KeyCode::Up => *date = date.add_days(-7),
        KeyCode::Down => *date = date.add_days(7),
        KeyCode::PageUp if key.shift => *date = date.add_months(-12),
        KeyCode::PageDown if key.shift => *date = date.add_months(12),
        KeyCode::PageUp => *date = date.add_months(-1),
        KeyCode::PageDown => *date = date.add_months(1),
        _ => {}
    }
    Flow::Continue
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::event::ScriptedSource;

    #[test]
    fn epoch_round_trips() {
        let date = Date::new(2026, 6, 14);
        assert_eq!(Date::from_epoch_days(date.to_epoch_days()), date);
    }

    #[test]
    fn known_weekday_is_correct() {
        // 2026-06-14 is a Sunday (= 6 with Monday=0).
        assert_eq!(Date::new(2026, 6, 14).weekday_monday0(), 6);
    }

    #[test]
    fn days_in_february_handles_leap_years() {
        assert_eq!(Date::new(2024, 2, 1).days_in_month(), 29);
        assert_eq!(Date::new(2025, 2, 1).days_in_month(), 28);
    }

    #[test]
    fn add_months_clamps_day() {
        let date = Date::new(2026, 1, 31).add_months(1);
        assert_eq!(date, Date::new(2026, 2, 28));
    }

    #[test]
    fn arrow_navigation_changes_day() {
        let outcome = DatePicker::new("when")
            .initial(Date::new(2026, 6, 14))
            .run_with(&mut ScriptedSource::keys([
                KeyCode::Right,
                KeyCode::Enter,
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(Date::new(2026, 6, 15)));
    }

    #[test]
    fn esc_cancels() {
        let outcome = DatePicker::new("when")
            .run_with(&mut ScriptedSource::keys([KeyCode::Esc]))
            .unwrap();
        assert_eq!(outcome, Outcome::Cancelled);
    }

    #[test]
    fn allow_clear_submits_empty_date() {
        let outcome = DatePicker::new("when")
            .allow_clear()
            .run_with(&mut ScriptedSource::keys([
                KeyCode::Delete,
                KeyCode::Enter,
            ]))
            .unwrap();
        match outcome {
            Outcome::Submitted(date) => assert!(date.is_empty()),
            _ => panic!("expected an empty date"),
        }
    }

    #[test]
    fn delete_is_ignored_without_allow_clear() {
        let outcome = DatePicker::new("when")
            .initial(Date::new(2026, 6, 14))
            .run_with(&mut ScriptedSource::keys([
                KeyCode::Delete,
                KeyCode::Enter,
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(Date::new(2026, 6, 14)));
    }

    #[test]
    fn shortcut_ends_with_its_id() {
        use crate::input::event::{InputEvent, KeyPress};
        use crate::input::shortcut::Shortcut;
        let outcome = DatePicker::new("when")
            .shortcuts([Shortcut::new(KeyPress::ctrl('t'), 9, "today")])
            .run_with(&mut ScriptedSource::events([InputEvent::Key(
                KeyPress::ctrl('t'),
            )]))
            .unwrap();
        assert_eq!(outcome, Outcome::Shortcut(9));
    }

    #[test]
    fn help_overlay_opens_and_closes() {
        use crate::input::event::KeyPress;
        use crate::input::shortcut::Shortcut;
        let outcome = DatePicker::new("when")
            .initial(Date::new(2026, 6, 14))
            .shortcuts([Shortcut::new(KeyPress::ctrl('t'), 1, "today")])
            .run_with(&mut ScriptedSource::keys([
                KeyCode::Char('?'),
                KeyCode::Char('x'),
                KeyCode::Enter,
            ]))
            .unwrap();
        assert_eq!(outcome, Outcome::Submitted(Date::new(2026, 6, 14)));
    }

    #[test]
    fn arrow_resumes_from_today_after_clear() {
        let outcome = DatePicker::new("when")
            .allow_clear()
            .run_with(&mut ScriptedSource::keys([
                KeyCode::Delete,
                KeyCode::Right,
                KeyCode::Enter,
            ]))
            .unwrap();
        match outcome {
            Outcome::Submitted(date) => assert!(!date.is_empty()),
            _ => panic!("expected a date"),
        }
    }
}
