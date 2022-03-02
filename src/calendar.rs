//! Implementation of US stock exchange holidays.

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Specifies the nth week of a month
#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub enum NthWeek {
    First,
    Second,
    Third,
    Fourth,
    Last,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub enum Holiday {
    /// Though weekends are no holidays, they need to be specified in the calendar. Weekends are assumed to be non-business days.
    /// In most countries, weekends include Saturday (`Sat`) and Sunday (`Sun`). Unfortunately, there are a few exceptions.
    WeekDay(Weekday),
    /// Occurs every year, but is moved to next non-weekend day if it falls on a weekday.
    /// Note that Saturday and Sunday here assumed to be weekend days, even if these days
    /// are not defined as weekends in this calendar. If the next Monday is already a holiday,
    /// the date will be moved to the next available business day.
    /// `first` and `last` are the first and last year this day is a holiday (inclusively).
    MovableYearlyDay {
        month: u32,
        day: u32,
        first: Option<i32>,
        last: Option<i32>,
    },
    /// A single holiday which is valid only once in time.
    SingularDay(NaiveDate),
    /// A holiday that is defined in relative days (e.g. -2 for Good Friday) to Easter (Sunday).
    EasterOffset {
        offset: i32,
        first: Option<i32>,
        last: Option<i32>,
    },
    /// A holiday that falls on the nth (or last) weekday of a specific month, e.g. the first Monday in May.
    /// `first` and `last` are the first and last year this day is a holiday (inclusively).
    MonthWeekday {
        month: u32,
        weekday: Weekday,
        nth: NthWeek,
        first: Option<i32>,
        last: Option<i32>,
    },
}

/// Calendar for arbitrary complex holiday rules
#[derive(Debug, Clone)]
pub struct Calendar {
    holidays: BTreeSet<NaiveDate>,
    halfdays: BTreeSet<NaiveDate>,
    weekdays: Vec<Weekday>,
}

impl Calendar {
    /// Calculate all holidays and recognize weekend days for a given range of years
    /// from `start` to `end` (inclusively). The calculation is performed on the basis
    /// of a vector of holiday rules.
    pub fn calc_calendar(holiday_rules: &[Holiday], start: i32, end: i32) -> Calendar {
        let mut holidays = BTreeSet::new();
        let mut halfdays = BTreeSet::new();
        let mut weekdays = Vec::new();

        for rule in holiday_rules {
            match rule {
                Holiday::SingularDay(date) => {
                    let year = date.year();
                    if year >= start && year <= end {
                        holidays.insert(*date);
                    }
                }
                Holiday::WeekDay(weekday) => {
                    weekdays.push(*weekday);
                }
                // prior to 7/4 and 12/25, if
                Holiday::MovableYearlyDay {
                    month,
                    day,
                    first,
                    last,
                } => {
                    let (first, last) = Self::calc_first_and_last(start, end, first, last);
                    for year in first..last + 1 {
                        let date = NaiveDate::from_ymd(year, *month, *day);
                        // if date falls on Saturday, use Friday, if date falls on Sunday, use Monday
                        let orig_wd = date.weekday();
                        let date = match orig_wd {
                            Weekday::Sat => date.pred(),
                            Weekday::Sun => date.succ(),
                            _ => date,
                        };
                        let wd = date.weekday();
                        let last_date_of_month = NaiveDate::from_ymd_opt(year, month + 1, 1)
                            .unwrap_or_else(|| NaiveDate::from_ymd(year + 1, 1, 1))
                            .pred();
                        let yr = if wd == Weekday::Fri && *month == 1 {
                            year - 1
                        } else {
                            year
                        };
                        let last_date_of_year = NaiveDate::from_ymd_opt(yr, 12, 31).unwrap();
                        // use the date only if it's not the end of a month or a year
                        if date != last_date_of_month && date != last_date_of_year {
                            holidays.insert(date);
                            // determine half days
                            if *month == 7 && *day == 4 || *month == 12 && *day == 25 {
                                if wd != Weekday::Mon {
                                    halfdays.insert(date.pred());
                                }
                            }
                        }
                    }
                }
                Holiday::EasterOffset {
                    offset,
                    first,
                    last,
                } => {
                    let (first, last) = Self::calc_first_and_last(start, end, first, last);
                    for year in first..last + 1 {
                        let easter = computus::gregorian(year).unwrap();
                        let easter = NaiveDate::from_ymd(easter.year, easter.month, easter.day);
                        let date = easter
                            .checked_add_signed(Duration::days(*offset as i64))
                            .unwrap();
                        holidays.insert(date);
                    }
                }
                Holiday::MonthWeekday {
                    month,
                    weekday,
                    nth,
                    first,
                    last,
                } => {
                    let (first, last) = Self::calc_first_and_last(start, end, first, last);
                    for year in first..last + 1 {
                        let day = match nth {
                            NthWeek::First => 1,
                            NthWeek::Second => 8,
                            NthWeek::Third => 15,
                            NthWeek::Fourth => 22,
                            NthWeek::Last => last_day_of_month(year, *month),
                        };
                        let mut date = NaiveDate::from_ymd(year, *month, day);
                        while date.weekday() != *weekday {
                            date = match nth {
                                NthWeek::Last => date.pred(),
                                _ => date.succ(),
                            }
                        }
                        holidays.insert(date);
                        // Black Friday
                        if *month == 11 && *weekday == Weekday::Thu && *nth == NthWeek::Fourth {
                            halfdays.insert(date.succ());
                        }
                    }
                }
            }
        }
        Calendar {
            holidays,
            halfdays,
            weekdays,
        }
    }

    /// Calculate the next business day
    pub fn next_bday(&self, date: NaiveDate) -> NaiveDate {
        let mut date = date.succ();
        while !self.is_business_day(date) {
            date = date.succ();
        }
        date
    }

    /// Calculate the previous business day
    pub fn prev_bday(&self, date: NaiveDate) -> NaiveDate {
        let mut date = date.pred();
        while !self.is_business_day(date) {
            date = date.pred();
        }
        date
    }

    fn calc_first_and_last(
        start: i32,
        end: i32,
        first: &Option<i32>,
        last: &Option<i32>,
    ) -> (i32, i32) {
        let first = match first {
            Some(year) => std::cmp::max(start, *year),
            _ => start,
        };
        let last = match last {
            Some(year) => std::cmp::min(end, *year),
            _ => end,
        };
        (first, last)
    }

    /// Returns true if the date falls on a weekend
    pub fn is_weekend(&self, day: NaiveDate) -> bool {
        let weekday = day.weekday();
        for w_day in &self.weekdays {
            if weekday == *w_day {
                return true;
            }
        }
        false
    }

    /// Returns true if the specified day is a full-day holiday
    pub fn is_holiday(&self, date: NaiveDate) -> bool {
        self.holidays.get(&date).is_some()
    }

    /// Returns true if the specified day is a half-day holiday
    pub fn is_half_holiday(&self, date: NaiveDate) -> bool {
        self.halfdays.get(&date).is_some()
    }

    /// Returns true if the specified day is a business day
    pub fn is_business_day(&self, date: NaiveDate) -> bool {
        !self.is_weekend(date) && !self.is_holiday(date)
    }
}

pub struct CalendarNotFound {}

pub trait CalendarProvider {
    fn get_calendar(&self, calendar_name: &str) -> Result<&Calendar, CalendarNotFound>;
}

/// Returns true if the specified year is a leap year (i.e. Feb 29th exists for this year)
pub fn is_leap_year(year: i32) -> bool {
    NaiveDate::from_ymd_opt(year, 2, 29).is_some()
}

/// Calculate the last day of a given month in a given year
pub fn last_day_of_month(year: i32, month: u32) -> u32 {
    NaiveDate::from_ymd_opt(year, month + 1, 1)
        .unwrap_or_else(|| NaiveDate::from_ymd(year + 1, 1, 1))
        .pred()
        .day()
}

pub struct SimpleCalendar {
    cal: Calendar,
}

impl SimpleCalendar {
    pub fn new(cal: &Calendar) -> SimpleCalendar {
        SimpleCalendar { cal: cal.clone() }
    }
}

impl CalendarProvider for SimpleCalendar {
    fn get_calendar(&self, _calendar_name: &str) -> Result<&Calendar, CalendarNotFound> {
        Ok(&self.cal)
    }
}

impl Default for SimpleCalendar {
    fn default() -> SimpleCalendar {
        SimpleCalendar {
            cal: Calendar::calc_calendar(&[], 2000, 2050),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_dates_calendar() {
        let holidays = vec![
            Holiday::SingularDay(NaiveDate::from_ymd(2019, 11, 20)),
            Holiday::SingularDay(NaiveDate::from_ymd(2019, 11, 24)),
            Holiday::SingularDay(NaiveDate::from_ymd(2019, 11, 25)),
            Holiday::WeekDay(Weekday::Sat),
            Holiday::WeekDay(Weekday::Sun),
        ];
        let cal = Calendar::calc_calendar(&holidays, 2019, 2019);

        assert_eq!(
            false,
            cal.is_business_day(NaiveDate::from_ymd(2019, 11, 20))
        );
        assert_eq!(true, cal.is_business_day(NaiveDate::from_ymd(2019, 11, 21)));
        assert_eq!(true, cal.is_business_day(NaiveDate::from_ymd(2019, 11, 22)));
        // weekend
        assert_eq!(
            false,
            cal.is_business_day(NaiveDate::from_ymd(2019, 11, 23))
        );
        assert_eq!(true, cal.is_weekend(NaiveDate::from_ymd(2019, 11, 23)));
        assert_eq!(false, cal.is_holiday(NaiveDate::from_ymd(2019, 11, 23)));
        // weekend and holiday
        assert_eq!(
            false,
            cal.is_business_day(NaiveDate::from_ymd(2019, 11, 24))
        );
        assert_eq!(true, cal.is_weekend(NaiveDate::from_ymd(2019, 11, 24)));
        assert_eq!(true, cal.is_holiday(NaiveDate::from_ymd(2019, 11, 24)));
        assert_eq!(
            false,
            cal.is_business_day(NaiveDate::from_ymd(2019, 11, 25))
        );
        assert_eq!(true, cal.is_business_day(NaiveDate::from_ymd(2019, 11, 26)));
    }

    #[test]
    fn test_movable_yearly_day() {
        let holidays = vec![Holiday::MovableYearlyDay {
            month: 1,
            day: 1,
            first: None,
            last: None,
        }];
        let cal = Calendar::calc_calendar(&holidays, 2021, 2022);
        assert_eq!(false, cal.is_holiday(NaiveDate::from_ymd(2021, 12, 31)));
    }

    #[test]
    /// Good Friday example
    fn test_easter_offset() {
        let holidays = vec![Holiday::EasterOffset {
            offset: -2,
            first: None,
            last: None,
        }];
        let cal = Calendar::calc_calendar(&holidays, 2021, 2022);
        assert_eq!(false, cal.is_business_day(NaiveDate::from_ymd(2021, 4, 2)));
        assert_eq!(false, cal.is_business_day(NaiveDate::from_ymd(2022, 4, 15)));
    }

    #[test]
    fn test_month_weekday() {
        let holidays = vec![
            // MLK
            Holiday::MonthWeekday {
                month: 1,
                weekday: Weekday::Mon,
                nth: NthWeek::Third,
                first: None,
                last: None,
            },
            // President's Day
            Holiday::MonthWeekday {
                month: 2,
                weekday: Weekday::Mon,
                nth: NthWeek::Third,
                first: None,
                last: None,
            },
        ];
        let cal = Calendar::calc_calendar(&holidays, 2022, 2022);
        assert_eq!(true, cal.is_holiday(NaiveDate::from_ymd(2022, 1, 17)));
        assert_eq!(true, cal.is_holiday(NaiveDate::from_ymd(2022, 2, 21)));
    }

    #[test]
    /// Testing serialization and deserialization of holidays definitions
    fn serialize_cal_definition() {
        let holidays = vec![
            Holiday::MonthWeekday {
                month: 11,
                weekday: Weekday::Mon,
                nth: NthWeek::First,
                first: None,
                last: None,
            },
            Holiday::MovableYearlyDay {
                month: 11,
                day: 1,
                first: Some(2016),
                last: None,
            },
            Holiday::SingularDay(NaiveDate::from_ymd(2019, 11, 25)),
            Holiday::WeekDay(Weekday::Sat),
            Holiday::EasterOffset {
                offset: -2,
                first: None,
                last: None,
            },
        ];
        let json = serde_json::to_string_pretty(&holidays).unwrap();
        assert_eq!(
            json,
            r#"[
  {
    "MonthWeekday": {
      "month": 11,
      "weekday": "Mon",
      "nth": "First",
      "first": null,
      "last": null
    }
  },
  {
    "MovableYearlyDay": {
      "month": 11,
      "day": 1,
      "first": 2016,
      "last": null
    }
  },
  {
    "SingularDay": "2019-11-25"
  },
  {
    "WeekDay": "Sat"
  },
  {
    "EasterOffset": {
      "offset": -2,
      "first": null,
      "last": null
    }
  }
]"#
        );
        let holidays2: Vec<Holiday> = serde_json::from_str(&json).unwrap();
        assert_eq!(holidays.len(), holidays2.len());
        for i in 0..holidays.len() {
            assert_eq!(holidays[i], holidays2[i]);
        }
    }
}
