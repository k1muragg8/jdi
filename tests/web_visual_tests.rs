use pendulum_kelly_cli::web;

#[test]
fn test_fmt_pct() {
    assert_eq!(web::fmt_pct(0.12345), "12.35%");
    assert_eq!(web::fmt_pct(-0.01), "-1.00%");
}

#[test]
fn test_safe_div() {
    assert_eq!(web::safe_div(1.0, 4.0), "25.00%");
    assert_eq!(web::safe_div(1.0, 0.0), "N/A");
}
