//! POST actions: templates

pub async fn template_transactions_handler() -> (axum::http::HeaderMap, String) {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        axum::http::HeaderValue::from_static("attachment; filename=transactions_template.csv"),
    );

    let content = "交易日期,交易类型,资产代码,资产名称,金额,份额,价格,手续费,币种,来源,备注\n\
        2024-01-01,buy,000216,华安黄金ETF联接A,1000.0,2.5,400.0,1.2,CNY,manual,示例买入\n\
        2024-01-02,sell,000216,华安黄金ETF联接A,500.0,1.25,400.0,0.6,CNY,manual,示例卖出"
        .to_string();

    (headers, content)
}

pub async fn template_alipay_holdings_handler() -> (axum::http::HeaderMap, String) {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        axum::http::HeaderValue::from_static("attachment; filename=alipay_holdings_template.csv"),
    );

    let content = "基金代码,基金名称,持有份额,持有金额,最新净值,净值日期,投入本金,持有收益,持有收益率,来源\n\
        000216,华安黄金ETF联接A,124.45,49782.36,1.23,2024-06-02,45000.0,4782.36,10.6,alipay_screenshot\n\
        000042,财通资管积极配置,5678.9,10234.56,1.80,2024-06-02,10000.0,234.56,2.3,alipay_screenshot"
        .to_string();

    (headers, content)
}
