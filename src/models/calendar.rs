use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// A date in the Gregorian calendar, independent of the clock and time zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl Date {
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        if !(1..=9999).contains(&year)
            || !(1..=12).contains(&month)
            || day == 0
            || day > Self::days_in_month(year, month)
        {
            return None;
        }
        Some(Self { year, month, day })
    }

    pub fn is_leap_year(year: i32) -> bool {
        year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
    }

    pub fn days_in_month(year: i32, month: u8) -> u8 {
        match month {
            2 if Self::is_leap_year(year) => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            _ => 0,
        }
    }

    pub fn today_utc() -> Self {
        let days = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(elapsed) => (elapsed.as_secs() / 86_400) as i64,
            Err(error) => -((error.duration().as_secs() as i64 + 86_399) / 86_400),
        };
        Self::from_unix_days(days)
    }

    /// Sunday is 0 and Saturday is 6.
    pub fn weekday(self) -> u8 {
        (self.unix_days() + 4).rem_euclid(7) as u8
    }

    pub fn first_of_month(self) -> Self {
        Self { day: 1, ..self }
    }

    pub fn add_days(self, days: i32) -> Self {
        Self::from_unix_days(self.unix_days() + i64::from(days))
    }

    /// Keep the day where possible; otherwise choose the last day of the month.
    pub fn add_months(self, months: i32) -> Self {
        let month = (i64::from(self.year - 1) * 12 + i64::from(self.month - 1) + i64::from(months))
            .clamp(0, 9999 * 12 - 1);
        let year = (month / 12 + 1) as i32;
        let month = (month % 12 + 1) as u8;
        Self {
            year,
            month,
            day: self.day.min(Self::days_in_month(year, month)),
        }
    }

    pub fn unix_days(self) -> i64 {
        // March starts the arithmetic year, leaving the leap day at its end.
        let year = i64::from(self.year) - i64::from(self.month <= 2);
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let month = i64::from(self.month) + if self.month > 2 { -3 } else { 9 };
        let day_of_year = (153 * month + 2) / 5 + i64::from(self.day) - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era - 719_468
    }

    pub fn from_unix_days(days: i64) -> Self {
        let days = days.clamp(-719_162, 2_932_896) + 719_468;
        let era = days.div_euclid(146_097);
        let day_of_era = days - era * 146_097;
        let year_of_era =
            (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let year = year_of_era + era * 400;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * month + 2) / 5 + 1;
        let month = month + if month < 10 { 3 } else { -9 };
        Self {
            year: (year + i64::from(month <= 2)) as i32,
            month: month as u8,
            day: day as u8,
        }
    }
}

impl fmt::Display for Date {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        )
    }
}

/// The chosen day and the month currently visible in a calendar.
#[derive(Clone, Debug)]
pub struct Calendar {
    pub selected: Date,
    pub month: Date,
    pub first_weekday: u8,
}

impl Default for Calendar {
    fn default() -> Self {
        Self::new(Date::today_utc())
    }
}

impl Calendar {
    pub fn new(selected: Date) -> Self {
        Self {
            selected,
            month: selected.first_of_month(),
            first_weekday: 0,
        }
    }

    pub fn grid(&self) -> [Date; 42] {
        let month = self.month.first_of_month();
        let offset = (month.weekday() + 7 - self.first_weekday % 7) % 7;
        let start = month.add_days(-i32::from(offset));
        std::array::from_fn(|index| start.add_days(index as i32))
    }

    pub fn select(&mut self, date: Date) {
        self.selected = date;
        self.month = date.first_of_month();
    }

    /// Browse months without changing the selected day.
    pub fn change_month(&mut self, months: i32) {
        self.month = self.month.first_of_month().add_months(months);
    }

    pub fn move_days(&mut self, days: i32) {
        self.select(self.selected.add_days(days));
    }

    pub fn move_months(&mut self, months: i32) {
        self.select(self.selected.add_months(months));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::new(year, month, day).unwrap()
    }

    #[test]
    fn leap_days_follow_the_gregorian_century_rule() {
        assert!(Date::new(2000, 2, 29).is_some());
        assert!(Date::new(1900, 2, 29).is_none());
        assert!(Date::new(2024, 2, 29).is_some());
        assert!(Date::new(2026, 2, 29).is_none());
        assert_eq!(date(2024, 2, 28).add_days(1), date(2024, 2, 29));
        assert_eq!(date(2024, 2, 29).add_days(1), date(2024, 3, 1));
    }

    #[test]
    fn dates_round_trip_across_years_and_epoch() {
        for year in [1, 1600, 1900, 1969, 1970, 2000, 2026, 9999] {
            for month in 1..=12 {
                let day = date(year, month, Date::days_in_month(year, month));
                assert_eq!(Date::from_unix_days(day.unix_days()), day);
            }
        }
        assert_eq!(date(1970, 1, 1).unix_days(), 0);
        assert_eq!(date(1970, 1, 1).weekday(), 4);
        assert_eq!(date(2026, 10, 15).weekday(), 4);
        assert_eq!(date(2026, 1, 1).add_days(-1), date(2025, 12, 31));
    }

    #[test]
    fn month_navigation_clamps_the_day_and_keeps_selection_while_browsing() {
        let mut calendar = Calendar::new(date(2024, 1, 31));
        calendar.move_months(1);
        assert_eq!(calendar.selected, date(2024, 2, 29));
        calendar.change_month(-2);
        assert_eq!(calendar.month, date(2023, 12, 1));
        assert_eq!(calendar.selected, date(2024, 2, 29));
        calendar.move_days(1);
        assert_eq!(calendar.month, date(2024, 3, 1));
    }

    #[test]
    fn grid_respects_the_first_weekday_and_includes_adjacent_months() {
        let mut calendar = Calendar::new(date(2026, 10, 15));
        assert_eq!(calendar.grid()[0], date(2026, 9, 27));
        assert_eq!(calendar.grid()[41], date(2026, 11, 7));
        calendar.first_weekday = 1;
        assert_eq!(calendar.grid()[0], date(2026, 9, 28));
    }
}
