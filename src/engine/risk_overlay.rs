use crate::api::{FxProvider, MarketDataProvider};
use crate::models::{GlobalRiskOverlay, RegimeConfig, RiskConfig, RiskFactorSnapshot};

pub fn calculate_risk_overlay(
    config: &RiskConfig,
    regime_config: &RegimeConfig,
    market_provider: &dyn MarketDataProvider,
    fx_provider: &dyn FxProvider,
) -> GlobalRiskOverlay {
    let mut factors = Vec::new();
    let mut warnings = Vec::new();

    // 1. VIX
    let vix = fetch_and_calculate_factor("VIX", &config.vix_symbol, config, market_provider);
    factors.push(vix);

    // 2. US 30Y
    let us30y = fetch_and_calculate_factor("US30Y", &config.us30y_symbol, config, market_provider);
    factors.push(us30y);

    // 3. Cryptos
    for symbol in &config.crypto_symbols {
        let name = match symbol.as_str() {
            "BTC-USD" => "BTC",
            "ETH-USD" => "ETH",
            "SOL-USD" => "SOL",
            _ => symbol.as_str(),
        };
        factors.push(fetch_and_calculate_factor(
            name,
            symbol,
            config,
            market_provider,
        ));
    }

    // 4. Equities
    for symbol in &config.equity_symbols {
        factors.push(fetch_and_calculate_factor(
            symbol,
            symbol,
            config,
            market_provider,
        ));
    }

    // 5. USD/CNH
    let usd_cnh = fetch_usd_cnh_factor(config, fx_provider);
    factors.push(usd_cnh);

    // Calculate score
    let (score, label, explanation) =
        calculate_aggregate_score(config, &factors, regime_config, &mut warnings);

    GlobalRiskOverlay {
        risk_score: score,
        risk_label: label,
        factor_results: factors,
        warnings,
        explanation,
    }
}

fn fetch_and_calculate_factor(
    name: &str,
    symbol: &str,
    config: &RiskConfig,
    market_provider: &dyn MarketDataProvider,
) -> RiskFactorSnapshot {
    let mut snapshot = RiskFactorSnapshot {
        name: name.to_string(),
        symbol: symbol.to_string(),
        latest_value: 0.0,
        latest_date: "N/A".to_string(),
        source: "unknown".to_string(),
        status: "正常".to_string(),
        short_return: 0.0,
        medium_return: 0.0,
        z_score: None,
        drawdown: 0.0,
        warning: None,
    };

    match market_provider.fetch_daily_candles(symbol, config.lookback_days) {
        Ok(candles) if !candles.is_empty() => {
            let latest = &candles[0];
            snapshot.latest_value = latest.close;
            snapshot.latest_date = latest.date.clone();
            snapshot.source = latest.source.clone();

            // Returns
            if candles.len() >= config.short_window_days {
                let prev = candles[config.short_window_days - 1].close;
                if prev > 0.0 {
                    snapshot.short_return = (latest.close / prev) - 1.0;
                }
            }
            if candles.len() >= config.medium_window_days {
                let prev = candles[config.medium_window_days - 1].close;
                if prev > 0.0 {
                    snapshot.medium_return = (latest.close / prev) - 1.0;
                }
            }

            // Drawdown (250d)
            let rolling_high = candles.iter().map(|c| c.close).fold(f64::MIN, f64::max);
            if rolling_high > 0.0 {
                snapshot.drawdown = (latest.close / rolling_high) - 1.0;
            }

            // Z-Score (250d)
            let prices: Vec<f64> = candles.iter().map(|c| c.close).collect();
            let mean: f64 = prices.iter().sum::<f64>() / prices.len() as f64;
            if prices.len() > 1 {
                let variance: f64 = prices.iter().map(|p| (p - mean).powi(2)).sum::<f64>()
                    / (prices.len() - 1) as f64;
                let stddev = variance.sqrt();
                if stddev > 0.000001 {
                    snapshot.z_score = Some((latest.close - mean) / stddev);
                }
            }

            // Specific Warnings
            if name == "VIX" {
                if snapshot.latest_value >= config.extreme_vix_threshold {
                    snapshot.warning = Some("VIX 极高，恐慌情绪蔓延".to_string());
                    snapshot.status = "极高".to_string();
                } else if snapshot.latest_value >= config.high_vix_threshold {
                    snapshot.warning = Some("VIX 偏高，波动加剧".to_string());
                    snapshot.status = "偏高".to_string();
                }
            }
        }
        _ => {
            snapshot.status = "查询失败".to_string();
        }
    }

    snapshot
}

fn fetch_usd_cnh_factor(config: &RiskConfig, fx_provider: &dyn FxProvider) -> RiskFactorSnapshot {
    let mut snapshot = RiskFactorSnapshot {
        name: "USD/CNH".to_string(),
        symbol: "USDCNH=X".to_string(), // Use configurable symbol if needed, but for now hardcoded
        latest_value: 0.0,
        latest_date: "N/A".to_string(),
        source: "unknown".to_string(),
        status: "正常".to_string(),
        short_return: 0.0,
        medium_return: 0.0,
        z_score: None,
        drawdown: 0.0,
        warning: None,
    };

    match fx_provider.fetch_daily_rates("USDCNH=X", config.lookback_days) {
        Ok(candles) if !candles.is_empty() => {
            let latest = &candles[0];
            snapshot.latest_value = latest.close;
            snapshot.latest_date = latest.date.clone();
            snapshot.source = latest.source.clone();

            if candles.len() >= config.short_window_days {
                let prev = candles[config.short_window_days - 1].close;
                if prev > 0.0 {
                    snapshot.short_return = (latest.close / prev) - 1.0;
                }
            }
            if candles.len() >= config.medium_window_days {
                let prev = candles[config.medium_window_days - 1].close;
                if prev > 0.0 {
                    snapshot.medium_return = (latest.close / prev) - 1.0;
                }
            }
        }
        _ => {
            snapshot.status = "查询失败".to_string();
        }
    }

    snapshot
}

fn calculate_aggregate_score(
    config: &RiskConfig,
    factors: &[RiskFactorSnapshot],
    _regime_config: &RegimeConfig,
    warnings: &mut Vec<String>,
) -> (f64, String, String) {
    let mut total_score = 0.0;
    let mut explanations = Vec::new();

    // 1. VIX Contribution (0-30)
    if let Some(vix) = factors.iter().find(|f| f.name == "VIX") {
        if vix.status != "查询失败" {
            let mut vix_score = 0.0;
            // Absolute level
            if vix.latest_value > 15.0 {
                vix_score += (vix.latest_value - 15.0).min(20.0);
            }
            // Rise (VIX rising is usually risk-off)
            if vix.short_return > 0.10 {
                vix_score += 5.0;
            }
            if vix.short_return > 0.25 {
                vix_score += 5.0;
            }
            let score = vix_score.min(30.0);
            total_score += score;
            if score > 15.0 {
                explanations.push(format!(
                    "VIX 指数 ({:.2}) 贡献了 {:.1} 分风险",
                    vix.latest_value, score
                ));
            }
        } else {
            warnings.push("缺失 VIX 数据，风险评分不完整。".to_string());
        }
    }

    // 2. US30Y Contribution (0-20)
    if let Some(us30y) = factors.iter().find(|f| f.name == "US30Y") {
        if us30y.status != "查询失败" {
            let mut yield_score: f64 = 0.0;
            // 60-day rise in bps
            // Actually us30y.medium_return is (latest / prev) - 1
            // prev = latest / (1 + medium_return)
            // diff = latest - prev
            let prev = if (1.0 + us30y.medium_return).abs() > 0.00001 {
                us30y.latest_value / (1.0 + us30y.medium_return)
            } else {
                us30y.latest_value
            };
            let bps_diff = (us30y.latest_value - prev) * 100.0;

            if bps_diff > config.us30y_fast_rise_bps_60d {
                yield_score += 10.0;
                if bps_diff > config.us30y_fast_rise_bps_60d * 2.0 {
                    yield_score += 10.0;
                }
            }
            let score = yield_score.min(20.0);
            total_score += score;
            if score > 0.0 {
                explanations.push(format!(
                    "美债收益率快速上升 ({:.0} bps / 60d) 增加了市场压力",
                    bps_diff
                ));
            }
        }
    }

    // 3. Crypto Contribution (0-20)
    let crypto_drawdowns: Vec<f64> = factors
        .iter()
        .filter(|f| ["BTC", "ETH", "SOL"].contains(&f.name.as_str()) && f.status == "正常")
        .map(|f| f.drawdown)
        .collect();

    if !crypto_drawdowns.is_empty() {
        let avg_drawdown = crypto_drawdowns.iter().sum::<f64>() / crypto_drawdowns.len() as f64;
        if avg_drawdown < config.crypto_drawdown_warning {
            let score = ((-avg_drawdown - 0.20) * 50.0).clamp(0.0, 20.0);
            total_score += score;
            if score > 5.0 {
                explanations.push(format!(
                    "加密货币平均回撤 ({:.1}%) 释放了风险信号",
                    avg_drawdown * 100.0
                ));
            }
        }
    }

    // 4. Equity Regime Contribution (0-20)
    let equity_factors: Vec<&RiskFactorSnapshot> = factors
        .iter()
        .filter(|f| ["QQQ", "SPY"].contains(&f.name.as_str()) && f.status == "正常")
        .collect();

    if !equity_factors.is_empty() {
        let mut equity_score = 0.0;
        for f in &equity_factors {
            if let Some(z) = f.z_score {
                if z > 2.0 {
                    equity_score += 10.0; // Overheat
                } else if z < -2.0 {
                    // equity_score += 5.0; // panic selloff? usually VIX covers this
                }
            }
        }
        let score = (equity_score / equity_factors.len() as f64).min(20.0);
        total_score += score;
        if score > 0.0 {
            explanations.push("权益市场处于均值偏离高位 (过热)".to_string());
        }
    }

    // 5. USD/CNH Contribution (0-10)
    if let Some(fx) = factors.iter().find(|f| f.name == "USD/CNH") {
        if fx.status != "查询失败" {
            if fx.short_return > 0.02 {
                // 2% CNH depreciation in 20 days
                total_score += 10.0;
                explanations.push("离岸人民币快速贬值增加了波动风险".to_string());
            }
        }
    }

    total_score = total_score.clamp(0.0, 100.0);

    let label = if total_score >= 80.0 {
        "极高风险"
    } else if total_score >= 60.0 {
        "高风险"
    } else if total_score >= 40.0 {
        "偏高"
    } else if total_score >= 20.0 {
        "正常"
    } else {
        "低风险"
    };

    let explanation = if explanations.is_empty() {
        "当前市场各项指标相对平稳，未见明显全球性风险信号。".to_string()
    } else {
        explanations.join("；") + "。"
    };

    (total_score, label.to_string(), explanation)
}
