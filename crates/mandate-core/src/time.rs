//! Simulation calendar (DEC-042 §4.1). The base unit is the day.
//!
//! The fictional calendar uses 12 months of fixed length (365 days, no leap
//! years) so date arithmetic is trivially deterministic. Day 0 is Monday,
//! 1 January of year 0 of the simulation epoch; worlds start at a configured
//! year offset.

use serde::{Deserialize, Serialize};

pub const DAYS_PER_YEAR: u32 = 365;
pub const MONTH_LENGTHS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// Days since the simulation epoch.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SimDate(pub u32);

/// Calendar breakdown of a date. `month` and `day` are 1-based.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CalendarDate {
    pub year: u32,
    pub month: u32,
    pub day: u32,
}

/// Day of week. The epoch day is a Monday.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl SimDate {
    pub fn from_ymd(year: u32, month: u32, day: u32) -> Self {
        let m = month.clamp(1, 12);
        let d = day.clamp(1, MONTH_LENGTHS[(m - 1) as usize]);
        let before: u32 = MONTH_LENGTHS[..(m - 1) as usize].iter().sum();
        SimDate(year * DAYS_PER_YEAR + before + d - 1)
    }

    pub fn calendar(self) -> CalendarDate {
        let year = self.0 / DAYS_PER_YEAR;
        let mut rem = self.0 % DAYS_PER_YEAR;
        let mut month = 1;
        for len in MONTH_LENGTHS {
            if rem < len {
                break;
            }
            rem -= len;
            month += 1;
        }
        CalendarDate {
            year,
            month,
            day: rem + 1,
        }
    }

    pub fn weekday(self) -> Weekday {
        match self.0 % 7 {
            0 => Weekday::Mon,
            1 => Weekday::Tue,
            2 => Weekday::Wed,
            3 => Weekday::Thu,
            4 => Weekday::Fri,
            5 => Weekday::Sat,
            _ => Weekday::Sun,
        }
    }

    pub fn add_days(self, days: u32) -> Self {
        SimDate(self.0.saturating_add(days))
    }

    pub fn days_until(self, later: SimDate) -> u32 {
        later.0.saturating_sub(self.0)
    }

    /// Whole years elapsed between two dates (used for ages).
    pub fn years_until(self, later: SimDate) -> u32 {
        self.days_until(later) / DAYS_PER_YEAR
    }
}

/// How often a system runs, in calendar terms.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Cadence {
    Daily,
    /// Mondays.
    Weekly,
    /// First of each month.
    Monthly,
    /// 1 Jan, 1 Apr, 1 Jul, 1 Oct.
    Quarterly,
    /// 1 January.
    Yearly,
}

impl Cadence {
    /// Does a system with this cadence run on `date`?
    pub fn hits(self, date: SimDate) -> bool {
        match self {
            Cadence::Daily => true,
            Cadence::Weekly => date.0.is_multiple_of(7),
            Cadence::Monthly => date.calendar().day == 1,
            Cadence::Quarterly => {
                let c = date.calendar();
                c.day == 1 && matches!(c.month, 1 | 4 | 7 | 10)
            }
            Cadence::Yearly => {
                let c = date.calendar();
                c.day == 1 && c.month == 1
            }
        }
    }

    /// Boundaries this cadence crosses in the half-open range `(from, to]`.
    pub fn boundaries(self, from: SimDate, to: SimDate) -> impl Iterator<Item = SimDate> {
        ((from.0 + 1)..=to.0)
            .map(SimDate)
            .filter(move |d| self.hits(*d))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn ymd_round_trip_known_dates() {
        let d = SimDate::from_ymd(2031, 3, 3);
        assert_eq!(
            d.calendar(),
            CalendarDate {
                year: 2031,
                month: 3,
                day: 3
            }
        );
        assert_eq!(SimDate(0).weekday(), Weekday::Mon);
    }

    #[test]
    fn cadence_counts_per_year() {
        let start = SimDate::from_ymd(10, 1, 1);
        let end = SimDate::from_ymd(11, 1, 1);
        assert_eq!(Cadence::Monthly.boundaries(start, end).count(), 12);
        assert_eq!(Cadence::Quarterly.boundaries(start, end).count(), 4);
        assert_eq!(Cadence::Yearly.boundaries(start, end).count(), 1);
        assert_eq!(Cadence::Daily.boundaries(start, end).count(), 365);
    }

    proptest! {
        #[test]
        fn calendar_round_trips(days in 0u32..2_000_000) {
            let d = SimDate(days);
            let c = d.calendar();
            prop_assert_eq!(SimDate::from_ymd(c.year, c.month, c.day), d);
            prop_assert!(c.month >= 1 && c.month <= 12);
        }

        #[test]
        fn calendar_is_monotonic(a in 0u32..1_000_000, step in 1u32..1000) {
            let x = SimDate(a).calendar();
            let y = SimDate(a + step).calendar();
            prop_assert!((y.year, y.month, y.day) > (x.year, x.month, x.day));
        }
    }
}
