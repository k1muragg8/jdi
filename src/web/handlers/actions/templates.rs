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
