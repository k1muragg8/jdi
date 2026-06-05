use crate::models::InstrumentConfig;
use crate::models::instrument::AssetClass;
use crate::storage::instrument_store::get_default_instruments;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketListFilter {
    /// Default watchlist: enabled, not archived, not test/demo
    Active,
    All,
    Enabled,
    Disabled,
    Archived,
    Test,
    Duplicate,
}

impl MarketListFilter {
    pub fn from_query(s: Option<&str>) -> Self {
        match s.unwrap_or("active") {
            "all" => Self::All,
            "enabled" | "监控中" => Self::Enabled,
            "disabled" | "已禁用" => Self::Disabled,
            "archived" | "已归档" => Self::Archived,
            "test" | "重复/测试" | "duplicate" => Self::Test,
            "dup" | "duplicate_only" => Self::Duplicate,
            _ => Self::Active,
        }
    }
}

pub fn normalize_provider_symbol(symbol: &str) -> String {
    symbol.trim().to_uppercase()
}

pub fn is_instrument_archived(inst: &InstrumentConfig) -> bool {
    if inst.archived {
        return true;
    }
    if inst
        .note
        .as_deref()
        .is_some_and(|n| n.to_lowercase().contains("archived"))
    {
        return true;
    }
    inst.tags.iter().any(|t| t.eq_ignore_ascii_case("archived"))
}

pub fn is_test_instrument(inst: &InstrumentConfig) -> bool {
    let sym = inst.symbol.trim().to_uppercase();
    if sym == "TEST" || sym.starts_with("TEST") {
        return true;
    }
    if inst.instrument_id.to_uppercase().contains("TEST") {
        return true;
    }
    if inst
        .name_zh
        .as_deref()
        .is_some_and(|n| n.contains("测试") || n.contains("演示") || n.contains("demo"))
    {
        return true;
    }
    if inst.name.to_lowercase().contains("test") {
        return true;
    }
    inst.tags.iter().any(|t| {
        let tl = t.to_lowercase();
        tl == "test" || tl == "demo" || tl == "测试"
    })
}

pub fn provider_symbol_key(inst: &InstrumentConfig) -> (String, String) {
    (
        inst.provider.trim().to_lowercase(),
        normalize_provider_symbol(if inst.provider_symbol.is_empty() {
            &inst.symbol
        } else {
            &inst.provider_symbol
        }),
    )
}

/// First enabled non-archived instrument per provider+symbol wins; others are duplicates.
pub fn duplicate_instrument_ids(
    instruments: &[InstrumentConfig],
) -> std::collections::HashSet<String> {
    let mut seen: HashMap<(String, String), String> = HashMap::new();
    let mut dups = std::collections::HashSet::new();
    let mut ordered: Vec<_> = instruments.iter().collect();
    ordered.sort_by_key(|i| (-i.priority, i.symbol.as_str()));
    for inst in ordered {
        let key = provider_symbol_key(inst);
        if let Some(first_id) = seen.get(&key) {
            dups.insert(inst.instrument_id.clone());
            dups.insert(first_id.clone());
        } else {
            seen.insert(key, inst.instrument_id.clone());
        }
    }
    dups
}

pub fn matches_filter(
    inst: &InstrumentConfig,
    filter: MarketListFilter,
    dups: &std::collections::HashSet<String>,
) -> bool {
    let archived = is_instrument_archived(inst);
    let test = is_test_instrument(inst);
    let dup = dups.contains(&inst.instrument_id);
    match filter {
        MarketListFilter::Active => inst.enabled && !archived && !test,
        MarketListFilter::All => true,
        MarketListFilter::Enabled => inst.enabled && !archived,
        MarketListFilter::Disabled => !inst.enabled && !archived,
        MarketListFilter::Archived => archived,
        MarketListFilter::Test => test || dup,
        MarketListFilter::Duplicate => dup,
    }
}

/// AU9999 must use Eastmoney (118.AU9999), not Yahoo/manual.
pub fn migrate_au9999_provider(inst: &mut InstrumentConfig) {
    if !inst.symbol.eq_ignore_ascii_case("AU9999") {
        return;
    }
    let prov = inst.provider.trim().to_lowercase();
    if prov == "yahoo" || prov == "manual" || prov.is_empty() {
        inst.provider = "eastmoney".to_string();
    }
    let ps = inst.provider_symbol.trim();
    if ps.is_empty() || ps.eq_ignore_ascii_case("AU9999") || !ps.contains('.') {
        inst.provider_symbol = "118.AU9999".to_string();
    }
    if inst.display_symbol.is_none() {
        inst.display_symbol = Some("AU9999".to_string());
    }
    inst.currency = "CNY".to_string();
    inst.asset_class = AssetClass::SpotCommodity;
}

pub fn migrate_instrument_flags(instruments: &mut [InstrumentConfig]) {
    for inst in instruments.iter_mut() {
        migrate_au9999_provider(inst);
        if !inst.archived && is_instrument_archived(inst) {
            inst.archived = true;
        }
        if inst.archived {
            inst.enabled = false;
        }
    }
}

fn apply_default_fields(existing: &mut InstrumentConfig, def: &InstrumentConfig) {
    existing.provider = def.provider.clone();
    existing.provider_symbol = def.provider_symbol.clone();
    existing.currency = def.currency.clone();
    existing.asset_class = def.asset_class.clone();
    if existing.display_symbol.is_none() {
        existing.display_symbol = def.display_symbol.clone();
    }
    if existing.name_zh.is_none() {
        existing.name_zh = def.name_zh.clone();
    }
    if existing.category_zh.is_none() {
        existing.category_zh = def.category_zh.clone();
    }
}

pub fn archive_instrument(inst: &mut InstrumentConfig) {
    inst.archived = true;
    inst.enabled = false;
    if !inst.tags.iter().any(|t| t == "archived") {
        inst.tags.push("archived".to_string());
    }
    let note = inst.note.clone().unwrap_or_default();
    if !note.to_lowercase().contains("archived") {
        inst.note = Some(format!(
            "{} [archived {}]",
            note.trim(),
            chrono::Local::now().format("%Y-%m-%d")
        ));
    }
}

pub fn restore_instrument(inst: &mut InstrumentConfig) {
    inst.archived = false;
    // do not auto enable, user can enable separately if wanted
    inst.tags.retain(|t| !t.eq_ignore_ascii_case("archived"));
    if let Some(n) = &inst.note {
        let cleaned = n
            .replace("[archived", "[restored")
            .replace("archived", "restored");
        inst.note = Some(cleaned);
    }
}

/// Idempotent restore of default watchlist instruments.
pub fn restore_default_instruments(
    instruments: &mut Vec<InstrumentConfig>,
    also_cleanup_test: bool,
) -> (usize, usize) {
    migrate_instrument_flags(instruments);
    let mut added = 0usize;
    let mut reactivated = 0usize;

    if also_cleanup_test {
        let _ = cleanup_test_instruments(instruments, false);
    }

    for def in get_default_instruments() {
        let key = provider_symbol_key(&def);
        if let Some(existing) = instruments.iter_mut().find(|i| {
            i.instrument_id == def.instrument_id
                || i.symbol.eq_ignore_ascii_case(&def.symbol)
                || provider_symbol_key(i) == key
        }) {
            apply_default_fields(existing, &def);
            migrate_au9999_provider(existing);
            let needs_reactivate =
                is_instrument_archived(existing) || (!existing.enabled && def.enabled);
            if needs_reactivate {
                existing.archived = false;
                existing.enabled = def.enabled;
                existing.tags.retain(|t| t != "archived");
                reactivated += 1;
            }
            continue;
        }
        instruments.push(def);
        added += 1;
    }
    (added, reactivated)
}

/// Archive test/demo instruments. Returns count archived.
pub fn cleanup_test_instruments(instruments: &mut [InstrumentConfig], preview_only: bool) -> usize {
    let ids: Vec<String> = instruments
        .iter()
        .filter(|i| is_test_instrument(i) && !is_instrument_archived(i))
        .map(|i| i.instrument_id.clone())
        .collect();
    if preview_only {
        return ids.len();
    }
    for inst in instruments.iter_mut() {
        if ids.contains(&inst.instrument_id) {
            archive_instrument(inst);
            if !inst.tags.iter().any(|t| t == "test") {
                inst.tags.push("test".to_string());
            }
        }
    }
    ids.len()
}

pub fn upsert_instrument(
    instruments: &mut Vec<InstrumentConfig>,
    mut new_inst: InstrumentConfig,
) -> Result<()> {
    new_inst.archived = false;
    let key = provider_symbol_key(&new_inst);
    if let Some(existing) = instruments
        .iter_mut()
        .find(|i| provider_symbol_key(i) == key)
    {
        if is_instrument_archived(existing) {
            anyhow::bail!("该 provider+symbol 已存在（已归档），请先在「已归档」中恢复或删除");
        }
        existing.enabled = true;
        if let Some(n) = &new_inst.name_zh {
            if !n.trim().is_empty() {
                existing.name_zh = Some(n.trim().to_string());
            }
        }
        if !new_inst.provider.is_empty() {
            existing.provider = new_inst.provider.clone();
        }
        if !new_inst.currency.is_empty() {
            existing.currency = new_inst.currency.clone();
        }
        return Ok(());
    }
    instruments.push(new_inst);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AssetClass;

    fn sample(inst_id: &str, sym: &str, enabled: bool, archived: bool) -> InstrumentConfig {
        InstrumentConfig {
            instrument_id: inst_id.to_string(),
            symbol: sym.to_string(),
            display_symbol: None,
            name: sym.to_string(),
            name_zh: None,
            name_en: None,
            description_zh: None,
            category_zh: None,
            display_label: None,
            asset_class: AssetClass::Etf,
            provider: "yahoo".to_string(),
            provider_symbol: sym.to_string(),
            market: None,
            exchange: None,
            currency: "USD".to_string(),
            quote_unit: "1".to_string(),
            price_unit: "1".to_string(),
            timezone: None,
            enabled,
            archived,
            priority: 0,
            tags: vec![],
            note: None,
        }
    }

    #[test]
    fn test_archive_hides_from_active_filter() {
        let mut i = sample("a", "QQQ", true, false);
        archive_instrument(&mut i);
        let dups = duplicate_instrument_ids(&[i.clone()]);
        assert!(!matches_filter(&i, MarketListFilter::Active, &dups));
        assert!(matches_filter(&i, MarketListFilter::Archived, &dups));
    }

    #[test]
    fn test_test_instrument_detection() {
        let t = sample("x", "TEST", true, false);
        assert!(is_test_instrument(&t));
        let dups = duplicate_instrument_ids(std::slice::from_ref(&t));
        assert!(!matches_filter(&t, MarketListFilter::Active, &dups));
    }

    #[test]
    fn test_restore_defaults_idempotent() {
        let mut list = get_default_instruments();
        let count = list.len();
        let (a1, _r1) = restore_default_instruments(&mut list, false);
        assert_eq!(a1, 0);
        assert_eq!(list.len(), count);
        let (a2, r2) = restore_default_instruments(&mut list, false);
        assert_eq!(a2, 0);
        assert_eq!(r2, 0);
        let qqq_count = list.iter().filter(|i| i.symbol == "QQQ").count();
        assert_eq!(qqq_count, 1);
    }

    #[test]
    fn test_migrate_au9999_to_eastmoney() {
        let mut inst = sample("au9999", "AU9999", true, false);
        inst.provider = "yahoo".to_string();
        inst.provider_symbol = "AU9999".to_string();
        migrate_au9999_provider(&mut inst);
        assert_eq!(inst.provider, "eastmoney");
        assert_eq!(inst.provider_symbol, "118.AU9999");
    }

    #[test]
    fn test_cleanup_test_archives_test_rows() {
        let mut list = vec![
            sample("t1", "TEST", true, false),
            sample("nasdaq_qqq", "QQQ", true, false),
        ];
        let n = cleanup_test_instruments(&mut list, false);
        assert_eq!(n, 1);
        assert!(is_instrument_archived(
            list.iter().find(|i| i.symbol == "TEST").unwrap()
        ));
    }
}
