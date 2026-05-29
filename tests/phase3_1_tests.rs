use pendulum_kelly_cli::engine::regime::{calculate_market_regime, calculate_pendulum_score};
use pendulum_kelly_cli::models::{Candle, RegimeConfig};

#[test]
fn test_regime_calculations() {
    let mut candles = Vec::new();
    // Create 300 days of data, rising from 100 to 400
    for i in 0..300 {
        candles.push(Candle {
            symbol: "TEST".to_string(),
            date: format!("2024-01-{:02}", i % 30 + 1), // dummy date
            open: 0.0,
            high: 100.0 + i as f64 + 5.0,
            low: 100.0 + i as f64 - 5.0,
            close: 100.0 + i as f64,
            volume: 1000,
            source: "test".to_string(),
        });
    }
    // candles are sorted newest first, so index 0 is 399.0
    // Actually the loop creates 100, 101, ... 399.
    // We want newest first, so we should reverse or push in reverse.
    candles.reverse();

    let config = RegimeConfig::default();
    let result = calculate_market_regime("TEST", &candles, &config);

    assert_eq!(result.symbol, "TEST");
    assert_eq!(result.latest_price, 399.0);
    assert_eq!(result.windows.len(), 4);

    // 20-day window: latest 399, MA is avg(380..399) = 389.5
    let w20 = result.windows.iter().find(|w| w.window_days == 20).unwrap();
    assert!(w20.moving_average < 399.0);
    assert!(w20.z_score.unwrap() > 0.0);
    assert_eq!(w20.drawdown, 0.0); // rising price, rolling high is latest
}

#[test]
fn test_pendulum_score_labels() {
    let config = RegimeConfig::default();

    // Neutral case: avg Z is 0
    let w_neutral = vec![create_mock_window(20, 100.0, Some(0.0))];
    let score_neutral = calculate_pendulum_score(&w_neutral, &config);
    assert_eq!(score_neutral.label, "中性");

    // Hot case: avg Z is 2.5
    let w_hot = vec![create_mock_window(20, 100.0, Some(2.5))];
    let score_hot = calculate_pendulum_score(&w_hot, &config);
    assert_eq!(score_hot.label, "过热");
    assert!(score_hot.score > 60.0);

    // Cold case: avg Z is -2.5
    let w_cold = vec![create_mock_window(20, 100.0, Some(-2.5))];
    let score_cold = calculate_pendulum_score(&w_cold, &config);
    assert_eq!(score_cold.label, "极冷");
    assert!(score_cold.score < -60.0);
}

fn create_mock_window(
    days: usize,
    ma: f64,
    z: Option<f64>,
) -> pendulum_kelly_cli::models::CycleWindowStats {
    pendulum_kelly_cli::models::CycleWindowStats {
        window_days: days,
        moving_average: ma,
        price_stddev: 10.0,
        daily_return_stddev: 0.01,
        annualized_volatility: 0.15,
        z_score: z,
        rolling_high: 110.0,
        drawdown: -0.05,
        cumulative_return: 0.1,
    }
}

#[test]
fn test_insufficient_data_regime() {
    let candles = vec![Candle {
        symbol: "T".to_string(),
        date: "2024".to_string(),
        open: 0.0,
        high: 0.0,
        low: 0.0,
        close: 100.0,
        volume: 0,
        source: "s".to_string(),
    }];
    let config = RegimeConfig::default();
    let result = calculate_market_regime("T", &candles, &config);
    // Should still calculate some windows (with 1 day) or at least not panic
    assert!(!result.windows.is_empty());
    // Standard deviation of 1 point is 0.0, so Z-score is None
    assert!(result.windows[0].z_score.is_none());
}
