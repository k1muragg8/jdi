use crate::api::MarketDataProvider;
use crate::models::{ConfigRoot, PortfolioState, ProxyValuationResult};

pub fn calculate_proxy_valuations(
    config: &ConfigRoot,
    state: &PortfolioState,
    market_provider: &dyn MarketDataProvider,
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

        if asset.reference_index_symbol.is_none() {
            res.status = "缺少参考指数".to_string();
            results.push(res);
            continue;
        }

        if holding.latest_nav.is_none() || holding.latest_nav_date.is_none() {
            res.status = "缺少基金净值".to_string();
            results.push(res);
            continue;
        }

        let symbol = asset.reference_index_symbol.as_ref().unwrap();
        let nav_date = holding.latest_nav_date.as_ref().unwrap();

        // 1. Get latest price
        let latest_price = match market_provider.fetch_latest_price(symbol) {
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
        // We look back 30 days to find a candle on or before nav_date
        let candles = match market_provider.fetch_daily_candles(symbol, 30) {
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
            res.proxy_return = (res.reference_latest_price / res.reference_price_on_nav_date) - 1.0;
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
