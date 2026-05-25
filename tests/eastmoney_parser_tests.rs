use chrono::{TimeZone, Utc};
use serde_json::Value;

fn parse_variable(content: &str, var_name: &str) -> Option<String> {
    let pattern = format!("var {} = ", var_name);
    if let Some(start_idx) = content.find(&pattern) {
        let value_start = start_idx + pattern.len();
        if let Some(end_idx) = content[value_start..].find(';') {
            let mut value = content[value_start..value_start + end_idx].trim();
            if (value.starts_with('\"') && value.ends_with('\"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                value = &value[1..value.len() - 1];
            }
            return Some(value.to_string());
        }
    }
    None
}

#[test]
fn test_parse_pingzhongdata_success() {
    let js = r#"
        var fS_name = "纳斯达克100基金";
        var fS_code = "006327";
        var Data_netWorthTrend = [{"x":1716422400000,"y":5.38,"equityReturn":0,"unitMoney":""}];
        var Data_ACWorthTrend = [[1716422400000, 5.38]];
    "#;

    let fund_name = parse_variable(js, "fS_name").unwrap();
    assert_eq!(fund_name, "纳斯达克100基金");

    let fund_code = parse_variable(js, "fS_code").unwrap();
    assert_eq!(fund_code, "006327");

    let net_worth_json = parse_variable(js, "Data_netWorthTrend").unwrap();
    let net_worth_data: Value = serde_json::from_str(&net_worth_json).unwrap();
    let last_item = net_worth_data.as_array().and_then(|a| a.last()).unwrap();

    let timestamp = last_item["x"].as_i64().unwrap();
    let nav = last_item["y"].as_f64().unwrap();
    assert_eq!(nav, 5.38);

    let dt = Utc.timestamp_millis_opt(timestamp).unwrap();
    let nav_date = dt.format("%Y-%m-%d").to_string();
    assert_eq!(nav_date, "2024-05-23");

    let ac_worth_json = parse_variable(js, "Data_ACWorthTrend").unwrap();
    let ac_worth_data: Value = serde_json::from_str(&ac_worth_json).unwrap();
    let last_ac_item = ac_worth_data.as_array().and_then(|a| a.last()).unwrap();
    let ac_nav = last_ac_item
        .as_array()
        .and_then(|a| a.get(1))
        .and_then(|v| v.as_f64())
        .unwrap();
    assert_eq!(ac_nav, 5.38);
}

#[test]
fn test_parse_variable_not_found() {
    let js = "var other = 123;";
    assert!(parse_variable(js, "fS_name").is_none());
}
