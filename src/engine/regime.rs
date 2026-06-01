use crate::models::{Candle, CycleWindowStats, MarketRegimeResult, RegimeConfig};

pub fn calculate_market_regime(
    symbol: &str,
    candles: &[Candle],
    config: &RegimeConfig,
) -> MarketRegimeResult {
    let mut result = MarketRegimeResult {
        symbol: symbol.to_string(),
        latest_price: 0.0,
        latest_return: 0.0,
        latest_date: "N/A".to_string(),
        source: "unknown".to_string(),
        windows: Vec::new(),
        pendulum_score: 0.0,
        regime_label: "中性".to_string(),
        warning: None,
    };

    if candles.is_empty() {
        result.warning = Some("缺少行情数据".to_string());
        return result;
    }

    let latest_candle = &candles[0]; // candles are sorted newest first in our providers
    result.latest_price = latest_candle.close;
    result.latest_date = latest_candle.date.clone();
    result.source = latest_candle.source.clone();

    if candles.len() > 1 {
        let prev_candle = &candles[1];
        if prev_candle.close > 0.0 {
            result.latest_return = (latest_candle.close / prev_candle.close) - 1.0;
        }
    }

    let mut windows_stats = Vec::new();
    for &window in &config.default_windows {
        if let Some(stats) = calculate_window_stats(candles, window) {
            windows_stats.push(stats);
        }
    }

    if windows_stats.is_empty() {
        result.warning = Some("历史数据不足，无法计算周期指标".to_string());
        return result;
    }

    result.windows = windows_stats;

    // Calculate Pendulum Score
    let score = calculate_pendulum_score(&result.windows, config);
    result.pendulum_score = score.score;
    result.regime_label = score.label;

    result
}

fn calculate_window_stats(candles: &[Candle], window_days: usize) -> Option<CycleWindowStats> {
    if candles.is_empty() {
        return None;
    }

    let actual_window = std::cmp::min(window_days, candles.len());
    let subset = &candles[0..actual_window];

    let prices: Vec<f64> = subset.iter().map(|c| c.close).collect();
    let latest_price = prices[0];

    // Moving Average
    let ma: f64 = prices.iter().sum::<f64>() / prices.len() as f64;

    // Price Standard Deviation (Sample StdDev)
    let price_stddev = if prices.len() > 1 {
        let variance: f64 =
            prices.iter().map(|p| (p - ma).powi(2)).sum::<f64>() / (prices.len() - 1) as f64;
        variance.sqrt()
    } else {
        0.0
    };

    // Z-Score
    let z_score = if price_stddev > 0.000001 {
        Some((latest_price - ma) / price_stddev)
    } else {
        None
    };

    // Rolling High & Drawdown
    let rolling_high = prices.iter().fold(f64::MIN, |a, &b| a.max(b));
    let drawdown = (latest_price / rolling_high) - 1.0;

    // Cumulative Return
    let first_price = prices[prices.len() - 1];
    let cumulative_return = (latest_price / first_price) - 1.0;

    // Daily Returns & Volatility
    let mut daily_returns = Vec::new();
    for i in 0..subset.len() - 1 {
        // candles are newest first, so i is today, i+1 is yesterday
        let today = subset[i].close;
        let yesterday = subset[i + 1].close;
        if yesterday > 0.0 {
            daily_returns.push((today / yesterday) - 1.0);
        }
    }

    let (daily_return_stddev, annualized_volatility) = if daily_returns.len() > 1 {
        let mean_return: f64 = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
        let variance: f64 = daily_returns
            .iter()
            .map(|r| (r - mean_return).powi(2))
            .sum::<f64>()
            / (daily_returns.len() - 1) as f64;
        let stddev = variance.sqrt();
        let ann_vol = stddev * (252.0f64).sqrt();
        (stddev, ann_vol)
    } else {
        (0.0, 0.0)
    };

    Some(CycleWindowStats {
        window_days,
        moving_average: ma,
        price_stddev,
        daily_return_stddev,
        annualized_volatility,
        z_score,
        rolling_high,
        drawdown,
        cumulative_return,
    })
}

pub fn calculate_pendulum_score(
    windows: &[CycleWindowStats],
    config: &RegimeConfig,
) -> crate::models::PendulumScore {
    if windows.is_empty() {
        return crate::models::PendulumScore {
            score: 0.0,
            label: "未知".to_string(),
            explanation: "无可用的周期数据".to_string(),
        };
    }

    // A simple multi-factor model
    // 1. Average Z-Score (weighted towards short/medium term)
    // 2. Average Drawdown
    // 3. Volatility Adjustment (higher vol increases CAUTION, but score remains price-based)

    let mut z_score_sum = 0.0;
    let mut z_count = 0;

    for w in windows {
        if let Some(z) = w.z_score {
            // Give more weight to shorter windows for "pendulum" feel?
            // Actually let's just average them for now.
            z_score_sum += z;
            z_count += 1;
        }
    }

    let avg_z = if z_count > 0 {
        z_score_sum / z_count as f64
    } else {
        0.0
    };

    // Map avg_z to score: -2.0 -> -100, +2.0 -> +100
    // Score = avg_z * (100 / threshold)
    let z_threshold = config.hot_z_threshold; // assuming symmetrical
    let mut score = avg_z * (100.0 / z_threshold);

    // Drawdown contribution (optional: if avg_z is positive but drawdown is deep? unlikely)
    // Actually drawdown usually correlates with negative Z-score.

    // Cap score at [-100, 100]
    score = score.clamp(-100.0, 100.0);

    let label = if score >= 60.0 {
        "过热"
    } else if score >= 20.0 {
        "偏热"
    } else if score <= -60.0 {
        "极冷"
    } else if score <= -20.0 {
        "偏冷"
    } else {
        "中性"
    };

    let explanation = format!(
        "基于 {} 个时间周期的平均 Z-score ({:.2}) 计算。当前市场处于 {} 状态。",
        z_count, avg_z, label
    );

    crate::models::PendulumScore {
        score,
        label: label.to_string(),
        explanation,
    }
}
