use crate::api::{FxProvider, MarketDataProvider};
use crate::models::{ConfigRoot, PortfolioState, ProxyValuationResult};

pub fn calculate_proxy_valuations(
    config: &ConfigRoot,
    state: &PortfolioState,
    market_provider: &dyn MarketDataProvider,
    fx_provider: &dyn FxProvider,
) -> Vec<ProxyValuationResult> {
    let mut results = Vec::new();

    for asset in &config.assets {
        if !asset.enabled {
            continue;
        }

        let holding = match state
            .asset_holdings
            .iter()
            .find(|h| h.asset_id == asset.asset_id)
        {
            Some(h) => h,
            None => continue,
        };

        let mut res = ProxyValuationResult {
            asset_id: asset.asset_id.clone(),
            fund_code: asset.fund_code.clone(),
            fund_name: asset.fund_name.clone(),
            sector: asset.sector.clone(),
            units: holding.units,
            official_nav: holding.latest_nav.unwrap_or(0.0),
            official_nav_date: holding
                .latest_nav_date
                .clone()
                .unwrap_or_else(|| "N/A".to_string()),
            official_market_value: holding.last_market_value,
            reference_index_name: asset
                .reference_index_name
                .clone()
                .unwrap_or_else(|| "-".to_string()),
            reference_index_symbol: asset
                .reference_index_symbol
                .clone()
                .unwrap_or_else(|| "-".to_string()),
            reference_price_on_nav_date: 0.0,
            reference_latest_price: 0.0,
            reference_latest_date: "N/A".to_string(),
            proxy_return: 0.0,
            index_return: 0.0,
            fx_return: 0.0,
            combined_proxy_return: 0.0,
            use_fx_adjustment: asset.use_fx_adjustment.unwrap_or(false),
            estimated_nav: 0.0,
            estimated_market_value: 0.0,
            estimated_pnl: 0.0,
            data_source: asset
                .market_data_provider
                .clone()
                .unwrap_or_else(|| config.market.default_market_provider.clone()),
            status: "正常".to_string(),
            warning: None,
        };

        let symbol_opt = asset
            .reference_instrument_symbol
            .clone()
            .or(asset.reference_index_symbol.clone());

        if symbol_opt.is_none() {
            res.status = "缺少参考指数".to_string();
            results.push(res);
            continue;
        }

        let symbol = symbol_opt.unwrap();
        res.reference_index_symbol = symbol.clone();

        if holding.latest_nav.is_none() || holding.latest_nav_date.is_none() {
            res.status = "缺少基金净值".to_string();
            results.push(res);
            continue;
        }

        let nav_date = holding.latest_nav_date.as_ref().unwrap();

        // 1. Get latest price
        let latest_price = match market_provider.fetch_latest_price(&symbol) {
            Ok(p) => p,
            Err(e) => {
                res.status = "行情查询失败".to_string();
                res.warning = Some(format!("Error: {}", e));
                results.push(res);
                continue;
            }
        };
        res.reference_latest_price = latest_price.price;
        res.reference_latest_date = latest_price.date.clone();

        // 2. Get historical price on nav_date
        let candles = match market_provider.fetch_daily_candles(&symbol, 30) {
            Ok(c) => c,
            Err(e) => {
                res.status = "缺少指数历史数据".to_string();
                res.warning = Some(format!("Error: {}", e));
                results.push(res);
                continue;
            }
        };

        let base_candle = candles
            .iter()
            .filter(|c| c.date <= *nav_date)
            .max_by_key(|c| c.date.clone());

        if let Some(candle) = base_candle {
            res.reference_price_on_nav_date = candle.close;
            res.index_return = (res.reference_latest_price / res.reference_price_on_nav_date) - 1.0;

            let mut fx_adj_success = false;
            let mut fx_warning = None;

            if res.use_fx_adjustment {
                if let Some(pair) = &asset.proxy_fx_pair {
                    // Try to get FX rates
                    let latest_fx_res = fx_provider.fetch_latest_rate(pair);
                    let hist_fx_res = fx_provider.fetch_daily_rates(pair, 30);

                    match (latest_fx_res, hist_fx_res) {
                        (Ok(latest), Ok(hist)) => {
                            let base_fx = hist
                                .iter()
                                .filter(|c| c.date <= *nav_date)
                                .max_by_key(|c| &c.date);

                            if let Some(b_fx) = base_fx {
                                res.fx_return = (latest.rate / b_fx.close) - 1.0;
                                res.combined_proxy_return = (res.reference_latest_price
                                    / res.reference_price_on_nav_date)
                                    * (latest.rate / b_fx.close)
                                    - 1.0;
                                fx_adj_success = true;
                            } else {
                                fx_warning = Some(
                                    "缺少汇率历史数据 (nav_date 当日或之前无记录)".to_string(),
                                );
                            }
                        }
                        (Err(e), _) => {
                            fx_warning = Some(format!("汇率查询失败: {}", e));
                        }
                        (_, Err(e)) => {
                            fx_warning = Some(format!("汇率历史数据查询失败: {}", e));
                        }
                    }
                }
            }

            if res.use_fx_adjustment && !fx_adj_success {
                let msg = fx_warning.unwrap_or_else(|| "缺少汇率数据".to_string());
                res.warning = Some(format!("{}, 已退回指数-only 估算。", msg));
                res.proxy_return = res.index_return;
                res.combined_proxy_return = res.index_return;
            } else if fx_adj_success {
                res.proxy_return = res.combined_proxy_return;
            } else {
                res.proxy_return = res.index_return;
                res.combined_proxy_return = res.index_return;
            }

            res.estimated_nav = res.official_nav * (1.0 + res.proxy_return);
            res.estimated_market_value = res.units * res.estimated_nav;
            res.estimated_pnl = res.estimated_market_value - holding.cost_basis;
        } else {
            res.status = "缺少指数历史数据".to_string();
            res.warning = Some(format!("未找到 {} 或之前的历史价格", nav_date));
        }

        results.push(res);
    }

    results
}
