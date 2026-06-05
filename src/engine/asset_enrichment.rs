//! Fund metadata lookup and sector/asset inference (no formula changes).

use crate::api::FundProvider;
use crate::models::{AssetConfig, FundInfo, FundNav};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FundLookupResult {
    pub success: bool,
    pub fund_code: String,
    pub fund_name: Option<String>,
    pub fund_type: Option<String>,
    pub currency: Option<String>,
    pub source: Option<String>,
    pub inferred_sector: Option<String>,
    pub nav: Option<f64>,
    pub nav_date: Option<String>,
    pub warnings: Vec<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EnrichApplyResult {
    pub changed_fields: Vec<String>,
    pub warnings: Vec<String>,
}

/// Infer sector/category from fund name, type, and code (heuristic; user may override).
pub fn infer_sector_from_text(name: &str, fund_type: &str, fund_code: &str) -> Option<String> {
    let name = name.to_lowercase();
    let ft = fund_type.to_lowercase();
    let code = fund_code.trim();

    if name.contains("纳斯达克科技")
        || name.contains("nasdaq tech")
        || name.contains("nasdaq100")
        || name.contains("纳斯达克100")
        || name.contains("nasdaq")
        || name.contains("qqq")
    {
        return Some("美国科技".to_string());
    }
    if name.contains("标普500")
        || name.contains("s&p 500")
        || name.contains("s&p500")
        || name.contains("spy")
        || name.contains("ivv")
        || name.contains("voo")
        || name.contains("161125")
    {
        return Some("美国大盘".to_string());
    }
    if name.contains("生物科技")
        || name.contains("创新药")
        || name.contains("医疗")
        || name.contains("biotech")
        || name.contains("医药")
    {
        return Some("生物科技".to_string());
    }
    if name.contains("日经") || name.contains("日本") || name.contains("nikkei") {
        return Some("日本".to_string());
    }
    if name.contains("越南") || name.contains("vietnam") {
        return Some("越南".to_string());
    }
    if name.contains("印度") || name.contains("india") {
        return Some("印度".to_string());
    }
    if name.contains("黄金") || name.contains("gold") || code == "000216" {
        return Some("黄金".to_string());
    }
    if name.contains("债")
        || name.contains("国开")
        || name.contains("同业存单")
        || name.contains("中短债")
        || name.contains("美元债")
        || name.contains("bond")
        || ft.contains("债")
    {
        return Some("债券".to_string());
    }
    if name.contains("dax")
        || name.contains("德国")
        || name.contains("cac40")
        || name.contains("法国")
        || name.contains("欧洲")
        || name.contains("euro")
        || name.contains("富时100")
        || name.contains("英国")
        || name.contains("ftse")
    {
        return Some("欧洲".to_string());
    }
    if name.contains("商品") || name.contains("抗通胀") || name.contains("commodity") {
        return Some("商品".to_string());
    }
    if name.contains("沪深300") || name.contains("中证") || name.contains("a股") {
        return Some("A股".to_string());
    }
    if name.contains("货币") || name.contains("现金") || ft.contains("货币") {
        return Some("货币基金".to_string());
    }
    None
}

pub fn is_asset_archived(asset: &AssetConfig) -> bool {
    !asset.enabled || asset.sector.contains("已归档")
}

fn needs_sector_assignment(sector: &str) -> bool {
    sector.is_empty() || sector == "未分类" || sector == "待确认"
}

/// Assign sectors for assets that are empty / 未分类 / 待确认. Returns change count.
pub fn classify_unassigned_assets(assets: &mut [AssetConfig]) -> usize {
    let mut changed = 0usize;
    for asset in assets.iter_mut() {
        if !needs_sector_assignment(&asset.sector) {
            continue;
        }
        let name = asset.fund_name.as_str();
        let ft = "";
        let code = asset.fund_code.as_str();
        if let Some(s) = infer_sector_from_text(name, ft, code) {
            if asset.sector != s {
                asset.sector = s;
                changed += 1;
            }
        } else if needs_sector_assignment(&asset.sector) {
            asset.sector = "待确认".to_string();
            changed += 1;
        }
    }
    changed
}

pub fn lookup_fund(provider: &dyn FundProvider, fund_code: &str) -> FundLookupResult {
    let code = fund_code.trim();
    if code.is_empty() {
        return FundLookupResult {
            fund_code: code.to_string(),
            message: Some("基金代码不能为空".to_string()),
            ..Default::default()
        };
    }

    let mut warnings = Vec::new();
    match provider.search_fund_by_code(code) {
        Ok(info) => {
            let inferred =
                infer_sector_from_text(&info.fund_name, &info.fund_type, &info.fund_code);
            let (nav, nav_date) = match provider.fetch_latest_nav(code) {
                Ok(n) => (Some(n.nav), Some(n.nav_date)),
                Err(e) => {
                    warnings.push(format!("净值获取失败: {}", e));
                    (None, None)
                }
            };
            if info.source == "mock" {
                warnings.push("当前使用模拟基金数据源".to_string());
            }
            FundLookupResult {
                success: true,
                fund_code: info.fund_code,
                fund_name: Some(info.fund_name),
                fund_type: Some(info.fund_type),
                currency: Some(info.currency),
                source: Some(info.source),
                inferred_sector: inferred,
                nav,
                nav_date,
                warnings,
                message: None,
            }
        }
        Err(e) => FundLookupResult {
            success: false,
            fund_code: code.to_string(),
            warnings: vec![e.to_string()],
            message: Some(format!("基金查询失败: {}", e)),
            ..Default::default()
        },
    }
}

pub fn apply_fund_info_to_asset(
    asset: &mut AssetConfig,
    info: &FundInfo,
    nav: Option<&FundNav>,
) -> EnrichApplyResult {
    let mut changed = Vec::new();
    let mut warnings = Vec::new();

    if !info.fund_name.is_empty() && asset.fund_name != info.fund_name {
        asset.fund_name = info.fund_name.clone();
        changed.push("fund_name".into());
    }
    if asset.fund_code.is_empty() {
        asset.fund_code = info.fund_code.clone();
        changed.push("fund_code".into());
    }
    if !info.currency.is_empty() && asset.currency != info.currency {
        asset.currency = info.currency.clone();
        changed.push("currency".into());
    }

    let needs_sector =
        asset.sector.is_empty() || asset.sector == "未分类" || asset.sector == "待确认";
    if needs_sector {
        if let Some(s) = infer_sector_from_text(&info.fund_name, &info.fund_type, &info.fund_code) {
            asset.sector = s;
            changed.push("sector".into());
        }
    }

    if asset.market_data_provider.is_none() && info.source == "eastmoney" {
        asset.market_data_provider = Some("eastmoney".to_string());
        changed.push("market_data_provider".into());
    }

    if let Some(n) = nav {
        if n.is_stale {
            warnings.push(format!("净值可能过期 (日期 {})", n.nav_date));
        }
    }

    EnrichApplyResult {
        changed_fields: changed,
        warnings,
    }
}
