use usec::calendar::UsExchangeCalendar;
fn main() {
    let mut sc = UsExchangeCalendar::with_default_range(false);
    sc.populate_cal(Some(2021), Some(2024));
    let c = sc.get_cal();
    println!("{:?}", c);
}
