use crate::calendar::{Calendar, Holiday, NthWeek};
use chrono::{NaiveDate, Weekday};
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct Market {
    calendars: BTreeMap<String, Calendar>,
}

impl Market {
    pub fn new() -> Market {
        Market {
            calendars: generate_calendars(),
        }
    }
    pub fn print_calendars(&self) {
        println!("{:?}", self.calendars);
    }
}

/// Generate fixed set of some calendars for testing purposes only
pub fn generate_calendars() -> BTreeMap<String, Calendar> {
    let mut calendars = BTreeMap::new();
    let target_holidays = vec![
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
        },
        // MLK, 3rd Monday of January
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
        },
        // Juneteenth National Independence Day
        Holiday::MovableYearlyDay {
            month: 6,
            day: 19,
            first: Some(2022),
            last: None,
        },
        // Independence Day
        Holiday::MovableYearlyDay {
            month: 7,
            day: 4,
            first: None,
            last: None,
        },
        // Labour Day
        Holiday::MonthWeekday {
            month: 9,
            weekday: Weekday::Mon,
            nth: NthWeek::First,
            first: None,
            last: None,
        },
        // Thanksgiving Day
        Holiday::MonthWeekday {
            month: 11,
            weekday: Weekday::Thu,
            nth: NthWeek::Fourth,
            first: None,
            last: None,
        },
        // Chrismas Day
        Holiday::MovableYearlyDay {
            month: 12,
            day: 25,
            first: None,
            last: None,
        },
        Holiday::SingularDay(NaiveDate::from_ymd(2001, 9, 11)),
    ];
    let target_cal = Calendar::calc_calendar(&target_holidays, 2000, 2050);
    calendars.insert("US_EXCHANGES".to_string(), target_cal);

    calendars
}
