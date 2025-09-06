use chrono::{Datelike, Utc};
use usec::calendar::UsExchangeCalendar;
fn main() {
    let mut sc = UsExchangeCalendar::with_default_range(false);
    let current_year = Utc::now().year();
    let last_year = current_year -1;
    let next_year = current_year + 1;
    sc.populate_cal(Some(last_year), Some(next_year));
    let c = sc.get_cal();
    println!("{:?}", c);
}
