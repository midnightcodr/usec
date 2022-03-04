use fin::calendar::UsExchangeCalendar;
fn main() {
    println!("let's do it");
    let mut sc = UsExchangeCalendar::with_default_rules(false);
    sc.populate_cal(Some(2021), Some(2024));
    let c = sc.get_cal();
    println!("{:?}", c);
}
