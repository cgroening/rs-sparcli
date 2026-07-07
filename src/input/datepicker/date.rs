//! A small proleptic-Gregorian civil-date type (no external date crate).

use std::time::{SystemTime, UNIX_EPOCH};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
