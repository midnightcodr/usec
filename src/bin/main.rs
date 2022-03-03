use fin::calendar::SimpleCalendar;
fn main() {
    println!("let's do it");
    let mut sc = SimpleCalendar::with_default_rules(false);
    sc.populate_cal(Some(2021), Some(2022));
    let c = sc.get_cal();
    println!("{:?}", c);
}
