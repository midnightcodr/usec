use chrono::{Duration, NaiveDate};
use fin::calendar::UsExchangeCalendar;
/// example to show holidays as well as half trading days
use std::env::args;
fn main() {
    let args: Vec<String> = args().collect();
    let len = args.len();
    if len < 2 {
        panic!("Usage: {} first [last]", args[0]);
    }
    let first: i32 = (&args[1]).parse().unwrap();
    let last: i32 = if len > 2 {
        (&args[2]).parse().unwrap()
    } else {
        first
    };
    let mut usec = UsExchangeCalendar::with_default_rules(false);
    let usec = usec.populate_cal(Some(first), Some(last));
    let cal = usec.get_cal();
    let mut first_date = NaiveDate::from_ymd(first, 1, 1);
    let last_date = NaiveDate::from_ymd(last, 12, 31);
    let mut holidays: Vec<NaiveDate> = Vec::new();
    let mut halfdays: Vec<NaiveDate> = Vec::new();
    while first_date < last_date {
        if cal.is_holiday(first_date) {
            holidays.push(first_date);
        } else if cal.is_half_holiday(first_date) {
            halfdays.push(first_date);
        }
        first_date = first_date + Duration::days(1);
    }
    println!("holidays: {:?}", holidays);
    println!("half days: {:?}", halfdays);
}
