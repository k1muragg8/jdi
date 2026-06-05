use crate::models::{MarketCacheEntry, MarketPrice};

/// Fill change / change_percent on a price when provider omitted them but previous close exists.
pub fn normalize_market_price(mut price: MarketPrice) -> MarketPrice {
    if price.change.is_none() || price.change_percent.is_none() {
        if let Some(prev) = price.previous_close {
            if prev.abs() > 1e-12 {
                let ch = price.price - prev;
                let ch_pct = (ch / prev) * 100.0;
                if price.change.is_none() {
                    price.change = Some(ch);
                }
                if price.change_percent.is_none() {
                    price.change_percent = Some(ch_pct);
                }
            }
        }
    }
    price
}

/// Apply normalized quote fields onto a cache entry.
pub fn apply_price_to_cache_entry(
    entry: &mut MarketCacheEntry,
    price: &MarketPrice,
    fetched_at: &str,
) {
    entry.price = price.price;
    entry.date = price.date.clone();
    entry.currency = price.currency.clone();
    entry.source = price.source.clone();
    entry.fetched_at = fetched_at.to_string();
    entry.previous_close = price.previous_close;
    entry.change = price.change;
    entry.change_percent = price.change_percent;
    entry.status = Some("ok".to_string());
    entry.error_message = None;
}

pub fn new_cache_entry_from_price(
    price: &MarketPrice,
    cache_symbol: &str,
    fetched_at: &str,
) -> MarketCacheEntry {
    MarketCacheEntry {
        symbol: cache_symbol.to_string(),
        price: price.price,
        date: price.date.clone(),
        currency: price.currency.clone(),
        source: price.source.clone(),
        fetched_at: fetched_at.to_string(),
        previous_close: price.previous_close,
        change: price.change,
        change_percent: price.change_percent,
        status: Some("ok".to_string()),
        error_message: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_calculates_change_from_previous_close() {
        let price = MarketPrice {
            symbol: "QQQ".to_string(),
            price: 450.0,
            date: "2026-06-05".to_string(),
            currency: "USD".to_string(),
            source: "yahoo".to_string(),
            is_stale: false,
            previous_close: Some(440.0),
            change: None,
            change_percent: None,
        };
        let n = normalize_market_price(price);
        assert!((n.change.unwrap() - 10.0).abs() < 0.01);
        assert!((n.change_percent.unwrap() - (10.0 / 440.0 * 100.0)).abs() < 0.01);
    }

    #[test]
    fn test_normalize_keeps_provider_change() {
        let price = MarketPrice {
            symbol: "SPY".to_string(),
            price: 500.0,
            date: "2026-06-05".to_string(),
            currency: "USD".to_string(),
            source: "yahoo".to_string(),
            is_stale: false,
            previous_close: Some(490.0),
            change: Some(10.0),
            change_percent: Some(2.04),
        };
        let n = normalize_market_price(price);
        assert_eq!(n.change, Some(10.0));
        assert_eq!(n.change_percent, Some(2.04));
    }
}
