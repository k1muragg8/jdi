use chrono::{TimeZone, Utc};
use serde_json::Value;

#[test]
fn test_parse_yahoo_chart_success() {
    let json = r#"{
        "chart": {
            "result": [
                {
                    "meta": {
                        "currency": "USD",
                        "symbol": "QQQ",
                        "regularMarketPrice": 445.67,
                        "regularMarketTime": 1716494400
                    },
                    "timestamp": [1716411600, 1716494400],
                    "indicators": {
                        "quote": [
                            {
                                "open": [440.1, 442.2],
                                "high": [443.5, 446.8],
                                "low": [439.8, 441.5],
                                "close": [442.1, 445.67],
                                "volume": [50000000, 52000000]
                            }
                        ]
                    }
                }
            ],
            "error": null
        }
    }"#;

    let data: Value = serde_json::from_str(json).unwrap();
    let result = &data["chart"]["result"][0];
    let meta = &result["meta"];

    assert_eq!(meta["regularMarketPrice"].as_f64().unwrap(), 445.67);
    assert_eq!(meta["currency"].as_str().unwrap(), "USD");

    let ts = meta["regularMarketTime"].as_i64().unwrap();
    let dt = Utc.timestamp_opt(ts, 0).unwrap();
    assert_eq!(dt.format("%Y-%m-%d").to_string(), "2024-05-23");

    let timestamps = result["timestamp"].as_array().unwrap();
    let indicators = &result["indicators"]["quote"][0];
    let closes = indicators["close"].as_array().unwrap();

    assert_eq!(timestamps.len(), 2);
    assert_eq!(closes[1].as_f64().unwrap(), 445.67);
}

#[test]
fn test_parse_yahoo_error() {
    let json = r#"{
        "chart": {
            "result": null,
            "error": {
                "code": "Not Found",
                "description": "No data found"
            }
        }
    }"#;
    let data: Value = serde_json::from_str(json).unwrap();
    assert!(data["chart"]["result"].is_null());
    assert!(!data["chart"]["error"].is_null());
}
