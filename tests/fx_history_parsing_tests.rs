use chrono::{TimeZone, Utc};
use serde_json::Value;

#[test]
fn test_parse_yahoo_fx_sparse_history_logic() {
    // This test simulates the logic in YahooFxProvider::fetch_daily_rates

    let json_1d = r#"{
        "chart": {
            "result": [
                {
                    "timestamp": [1779701808],
                    "indicators": {
                        "quote": [{"close": [6.7838]}]
                    }
                }
            ]
        }
    }"#;

    let json_60m = r#"{
        "chart": {
            "result": [
                {
                    "timestamp": [1779148800, 1779152400, 1779235200],
                    "indicators": {
                        "quote": [{"close": [6.7922, 6.7838, 6.7900]}]
                    }
                }
            ]
        }
    }"#;

    let data_1d: Value = serde_json::from_str(json_1d).unwrap();
    let data_60m: Value = serde_json::from_str(json_60m).unwrap();

    // Logic: if 1d has <= 1 timestamp, use 60m
    let timestamps_1d = data_1d["chart"]["result"][0]["timestamp"]
        .as_array()
        .unwrap();
    let chosen_data = if timestamps_1d.len() <= 1 {
        &data_60m
    } else {
        &data_1d
    };

    let result = &chosen_data["chart"]["result"][0];
    let timestamps = result["timestamp"].as_array().unwrap();
    let _closes = result["indicators"]["quote"][0]["close"]
        .as_array()
        .unwrap();

    let mut dates = Vec::new();
    let mut last_date = String::new();

    for i in (0..timestamps.len()).rev() {
        let ts = timestamps[i].as_i64().unwrap();
        let dt = Utc.timestamp_opt(ts, 0).unwrap();
        let date = dt.format("%Y-%m-%d").to_string();

        if date == last_date {
            continue;
        }
        dates.push(date.clone());
        last_date = date;
    }

    assert_eq!(dates.len(), 2);
    assert_eq!(dates[0], "2026-05-20");
    assert_eq!(dates[1], "2026-05-19");
}
