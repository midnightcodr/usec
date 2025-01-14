//! Implementation of US stock exchange calendar with full and half-day holidays.
//! code borrowed heavily from
//! <https://github.com/xemwebe/cal-calc>

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::env;

/// Specifies the nth week of a month
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub enum NthWeek {
    First,
    Second,
    Third,
    Fourth,
    Last,
}
/// Do the half-day holiday check before or after the target date
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub enum HalfCheck {
    Before,
    After,
}

/// Types of days when US stocks exchanges are closed
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum Holiday {
    /// for US exchanges, `Sat` and `Sun`
    WeekDay(Weekday),
    /// `first` and `last` are the first and last year this day is a holiday (inclusively).
    MovableYearlyDay {
        month: u32,
        day: u32,
        first: Option<i32>,
        last: Option<i32>,
        half_check: Option<HalfCheck>,
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
        half_check: Option<HalfCheck>,
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
                // check if prior to 7/4 and 12/25
                Holiday::MovableYearlyDay {
                    month,
                    day,
                    first,
                    last,
                    half_check,
                } => {
                    let (first, last) = Self::calc_first_and_last(start, end, first, last);
                    for year in first..last + 1 {
                        let date = Calendar::from_ymd(year, *month, *day);
                        // if date falls on Saturday, use Friday, if date falls on Sunday, use Monday
                        let orig_wd = date.weekday();
                        let mut moved_already = false;
                        let date = match orig_wd {
                            Weekday::Sat => {
                                moved_already = true;
                                date.pred_opt().unwrap()
                            }
                            Weekday::Sun => {
                                moved_already = true;
                                date.succ_opt().unwrap()
                            }
                            _ => date,
                        };
                        let (last_date_of_month, last_date_of_year) = accounting_period_end(date);
                        // use the date only if it's not the end of a month or a year
                        if date != last_date_of_month && date != last_date_of_year {
                            holidays.insert(date);
                            if !moved_already {
                                do_halfday_check(&date, &mut halfdays, half_check);
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
                        let easter = Calendar::from_ymd(easter.year, easter.month, easter.day);
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
                    half_check,
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
                        let mut date = Calendar::from_ymd(year, *month, day);
                        while date.weekday() != *weekday {
                            date = match nth {
                                NthWeek::Last => date.pred_opt().unwrap(),
                                _ => date.succ_opt().unwrap(),
                            }
                        }
                        holidays.insert(date);
                        do_halfday_check(&date, &mut halfdays, half_check);
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
    pub fn next_biz_day(&self, date: NaiveDate) -> NaiveDate {
        let mut date = date.succ_opt().unwrap();
        while !self.is_business_day(date) {
            date = date.succ_opt().unwrap();
        }
        date
    }

    /// Calculate the previous business day
    pub fn prev_biz_day(&self, date: NaiveDate) -> NaiveDate {
        let mut date = date.pred_opt().unwrap();
        while !self.is_business_day(date) {
            date = date.pred_opt().unwrap();
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

    pub fn from_ymd(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }
}

/// Returns true if the specified year is a leap year (i.e. Feb 29th exists for this year)
pub fn is_leap_year(year: i32) -> bool {
    NaiveDate::from_ymd_opt(year, 2, 29).is_some()
}

/// Returns ending accounting period (end of month, end of year)
pub fn accounting_period_end(date: NaiveDate) -> (NaiveDate, NaiveDate) {
    let month = date.month();
    let year = date.year();
    let last_date_of_month = NaiveDate::from_ymd_opt(year, month + 1, 1)
        .unwrap_or_else(|| Calendar::from_ymd(year + 1, 1, 1))
        .pred_opt()
        .unwrap();
    let last_date_of_year = NaiveDate::from_ymd_opt(year, 12, 31).unwrap();
    return (last_date_of_month, last_date_of_year);
}

pub fn do_halfday_check(
    date: &NaiveDate,
    halfdays: &mut BTreeSet<NaiveDate>,
    half_check: &Option<HalfCheck>,
) {
    let weekday = date.weekday();
    match half_check {
        None => {}
        Some(HalfCheck::Before) => {
            if weekday == Weekday::Mon {
                return;
            }
            let prior = date.pred_opt().unwrap();
            halfdays.insert(prior);
        }
        Some(HalfCheck::After) => {
            if weekday == Weekday::Fri {
                return;
            }
            let next = date.succ_opt().unwrap();
            halfdays.insert(next);
        }
    }
}

/// Calculate the last day of a given month in a given year
pub fn last_day_of_month(year: i32, month: u32) -> u32 {
    NaiveDate::from_ymd_opt(year, month + 1, 1)
        .unwrap_or_else(|| Calendar::from_ymd(year + 1, 1, 1))
        .pred_opt()
        .unwrap()
        .day()
}

/// Calendar specific to US stock exchanges
#[derive(Debug, Clone)]
pub struct UsExchangeCalendar {
    cal: Calendar,
    holiday_rules: Vec<Holiday>,
}

impl UsExchangeCalendar {
    /// NYSE holiday calendar as of 2022
    /// create a new US Exchange calendar with default rules, populate the
    /// calendar with default range (2000-2050) if `populate` is set to `true`
    pub fn with_default_range(populate: bool) -> UsExchangeCalendar {
        let mut holiday_rules = vec![
            // Saturdays
            Holiday::WeekDay(Weekday::Sat),
            // Sundays
            Holiday::WeekDay(Weekday::Sun),
            // New Year's day
            Holiday::MovableYearlyDay {
                month: 1,
                day: 1,
                first: None,
                last: None,
                half_check: None,
            },
            // MLK, 3rd Monday of January
            Holiday::MonthWeekday {
                month: 1,
                weekday: Weekday::Mon,
                nth: NthWeek::Third,
                first: None,
                last: None,
                half_check: None,
            },
            // President's Day
            Holiday::MonthWeekday {
                month: 2,
                weekday: Weekday::Mon,
                nth: NthWeek::Third,
                first: None,
                last: None,
                half_check: None,
            },
            // Good Friday
            Holiday::EasterOffset {
                offset: -2,
                first: Some(2000),
                last: None,
            },
            // Memorial Day
            Holiday::MonthWeekday {
                month: 5,
                weekday: Weekday::Mon,
                nth: NthWeek::Last,
                first: None,
                last: None,
                half_check: None,
            },
            // Juneteenth National Independence Day
            Holiday::MovableYearlyDay {
                month: 6,
                day: 19,
                first: Some(2022),
                last: None,
                half_check: None,
            },
            // Independence Day
            Holiday::MovableYearlyDay {
                month: 7,
                day: 4,
                first: None,
                last: None,
                half_check: Some(HalfCheck::Before),
            },
            // Labour Day
            Holiday::MonthWeekday {
                month: 9,
                weekday: Weekday::Mon,
                nth: NthWeek::First,
                first: None,
                last: None,
                half_check: None,
            },
            // Thanksgiving Day
            Holiday::MonthWeekday {
                month: 11,
                weekday: Weekday::Thu,
                nth: NthWeek::Fourth,
                first: None,
                last: None,
                half_check: Some(HalfCheck::After),
            },
            // Chrismas Day
            Holiday::MovableYearlyDay {
                month: 12,
                day: 25,
                first: None,
                last: None,
                half_check: Some(HalfCheck::Before),
            },
            Holiday::SingularDay(Calendar::from_ymd(2001, 9, 11)),
        ];
        let additional_rules = env::var("ADDITIONAL_RULES");
        if additional_rules.is_ok() {
            let mut additional_rules: Vec<Holiday> =
                serde_json::from_str(&additional_rules.unwrap()).unwrap();
            holiday_rules.append(&mut additional_rules);
        }
        let cal = Calendar {
            holidays: BTreeSet::new(),
            halfdays: BTreeSet::new(),
            weekdays: Vec::new(),
        };
        let mut sc = UsExchangeCalendar { cal, holiday_rules };
        if populate {
            sc.populate_cal(None, None);
        }
        sc
    }

    /// add an ad-hoc holiday rule to the rule list
    pub fn add_holiday_rule(&mut self, holiday: Holiday) -> &mut Self {
        self.holiday_rules.push(holiday);
        self
    }

    /// populate calendar for given `start` and `end` years (inclusively, defaults to 2000 and 2050 if None,
    /// None are given)
    pub fn populate_cal(&mut self, start: Option<i32>, end: Option<i32>) -> &mut Self {
        let start = start.unwrap_or(2000);
        let end = end.unwrap_or(2050);
        self.cal = Calendar::calc_calendar(&self.holiday_rules, start, end);
        self
    }

    pub fn get_cal(&self) -> Calendar {
        self.cal.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cal() -> Calendar {
        let usec = UsExchangeCalendar::with_default_range(true);
        usec.get_cal()
    }

    #[test]
    fn fixed_dates_calendar() {
        let holidays = vec![
            Holiday::SingularDay(Calendar::from_ymd(2019, 11, 20)),
            Holiday::SingularDay(Calendar::from_ymd(2019, 11, 24)),
            Holiday::SingularDay(Calendar::from_ymd(2019, 11, 25)),
            Holiday::WeekDay(Weekday::Sat),
            Holiday::WeekDay(Weekday::Sun),
        ];
        let cal = Calendar::calc_calendar(&holidays, 2019, 2019);

        assert_eq!(false, cal.is_business_day(Calendar::from_ymd(2019, 11, 20)));
        assert_eq!(true, cal.is_business_day(Calendar::from_ymd(2019, 11, 21)));
        assert_eq!(true, cal.is_business_day(Calendar::from_ymd(2019, 11, 22)));
        // weekend
        assert_eq!(false, cal.is_business_day(Calendar::from_ymd(2019, 11, 23)));
        assert_eq!(true, cal.is_weekend(Calendar::from_ymd(2019, 11, 23)));
        assert_eq!(false, cal.is_holiday(Calendar::from_ymd(2019, 11, 23)));
        // weekend and holiday
        assert_eq!(false, cal.is_business_day(Calendar::from_ymd(2019, 11, 24)));
        assert_eq!(true, cal.is_weekend(Calendar::from_ymd(2019, 11, 24)));
        assert_eq!(true, cal.is_holiday(Calendar::from_ymd(2019, 11, 24)));
        assert_eq!(false, cal.is_business_day(Calendar::from_ymd(2019, 11, 25)));
        assert_eq!(true, cal.is_business_day(Calendar::from_ymd(2019, 11, 26)));
    }

    #[test]
    fn test_movable_yearly_day() {
        let holidays = vec![Holiday::MovableYearlyDay {
            month: 1,
            day: 1,
            first: None,
            last: None,
            half_check: None,
        }];
        let cal = Calendar::calc_calendar(&holidays, 2021, 2022);
        assert_eq!(false, cal.is_holiday(Calendar::from_ymd(2021, 12, 31)));
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
        assert_eq!(false, cal.is_business_day(Calendar::from_ymd(2021, 4, 2)));
        assert_eq!(false, cal.is_business_day(Calendar::from_ymd(2022, 4, 15)));
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
                half_check: None,
            },
            // President's Day
            Holiday::MonthWeekday {
                month: 2,
                weekday: Weekday::Mon,
                nth: NthWeek::Third,
                first: None,
                last: None,
                half_check: None,
            },
        ];
        let cal = Calendar::calc_calendar(&holidays, 2022, 2022);
        assert_eq!(true, cal.is_holiday(Calendar::from_ymd(2022, 1, 17)));
        assert_eq!(true, cal.is_holiday(Calendar::from_ymd(2022, 2, 21)));
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
                half_check: None,
            },
            Holiday::MovableYearlyDay {
                month: 11,
                day: 1,
                first: Some(2016),
                last: None,
                half_check: None,
            },
            Holiday::SingularDay(Calendar::from_ymd(2019, 11, 25)),
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
      "last": null,
      "half_check": null
    }
  },
  {
    "MovableYearlyDay": {
      "month": 11,
      "day": 1,
      "first": 2016,
      "last": null,
      "half_check": null
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

    #[test]
    fn test_usexchange_calendar_empty() {
        let sc = UsExchangeCalendar::with_default_range(false);
        let c = sc.get_cal();
        assert!(c.holidays.len() == 0);
        assert!(c.halfdays.len() == 0);
        assert!(c.weekdays.len() == 0);
    }

    #[test]
    fn test_usexchange_calendar_populated() {
        let sc = UsExchangeCalendar::with_default_range(true);
        let c = sc.get_cal();
        assert!(c.holidays.len() > 0);
        assert!(c.halfdays.len() > 0);
        assert!(c.weekdays.len() > 0);
        assert!(c.is_holiday(Calendar::from_ymd(2021, 1, 1)));
        assert_eq!(false, c.is_holiday(Calendar::from_ymd(2021, 12, 31)))
    }

    #[test]
    fn test_usexchange_calendar_with_new_rule() {
        // imaginary holiday, let's call it March Madness Day
        let mut sc = UsExchangeCalendar::with_default_range(false);
        let holiday = Holiday::MonthWeekday {
            month: 3,
            weekday: Weekday::Wed,
            nth: NthWeek::Third,
            first: None,
            last: None,
            half_check: None,
        };
        sc.add_holiday_rule(holiday).populate_cal(None, None);
        let c = sc.get_cal();
        assert_eq!(true, c.is_holiday(Calendar::from_ymd(2022, 3, 16)));
    }

    #[test]
    fn test_is_trading_date() {
        let cal = make_cal();
        assert_eq!(cal.is_business_day(Calendar::from_ymd(2021, 4, 18)), false);
        assert_eq!(cal.is_business_day(Calendar::from_ymd(2021, 4, 19)), true);
        assert_eq!(cal.is_business_day(Calendar::from_ymd(2021, 1, 1)), false);
    }

    #[test]
    fn test_is_partial_trading_date() {
        let cal = make_cal();
        assert_eq!(cal.is_half_holiday(Calendar::from_ymd(2027, 12, 23)), false);
        assert_eq!(cal.is_half_holiday(Calendar::from_ymd(2026, 7, 2)), false);
        assert_eq!(cal.is_half_holiday(Calendar::from_ymd(2021, 11, 26)), true);
        assert_eq!(cal.is_half_holiday(Calendar::from_ymd(2022, 5, 12)), false);
    }

    #[test]
    fn test_prev_biz_day() {
        let cal = make_cal();
        assert_eq!(
            cal.prev_biz_day(Calendar::from_ymd(2021, 1, 18)),
            Calendar::from_ymd(2021, 1, 15)
        );
        assert_eq!(
            cal.prev_biz_day(Calendar::from_ymd(2021, 4, 19)),
            Calendar::from_ymd(2021, 4, 16)
        );
        assert_eq!(
            cal.prev_biz_day(Calendar::from_ymd(2021, 8, 9)),
            Calendar::from_ymd(2021, 8, 6)
        );
    }

    #[test]
    fn test_next_biz_day() {
        let cal = make_cal();
        assert_eq!(
            cal.next_biz_day(Calendar::from_ymd(2021, 4, 16)),
            Calendar::from_ymd(2021, 4, 19)
        );
        assert_eq!(
            cal.next_biz_day(Calendar::from_ymd(2021, 4, 19)),
            Calendar::from_ymd(2021, 4, 20)
        );
        assert_eq!(
            cal.next_biz_day(Calendar::from_ymd(2021, 4, 2)),
            Calendar::from_ymd(2021, 4, 5)
        );
    }
}
