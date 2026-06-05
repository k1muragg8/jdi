//! Market (市场) display model.

use crate::models::InstrumentConfig;
use crate::web::services::market_service::MarketPageData;
use crate::web::state::AppState;
use crate::{engine, models};
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct MarketPageVm {
    pub cleanup_confirm_msg: String,
    pub filter_tabs_html: String,
    pub auto_refresh_html: String,
    pub inst_rows_html: String,
    pub last_refresh: String,
    pub cache_depth: usize,
    pub mon_count: usize,
    pub fail_count: usize,
    pub fail_count_color: String,
    pub list_filter: engine::MarketListFilter,
}

pub async fn build_market_vm(
    state: &Arc<AppState>,
    page: MarketPageData,
    filter: Option<&str>,
) -> Result<MarketPageVm> {
    let ctx = &state.ctx;
    let instruments = page.instruments;
    let market_cache = page.market_cache;
    let cleanup_confirm_msg = page.cleanup_confirm_msg;
    let dup_ids = page.dup_ids;
    let list_filter = engine::MarketListFilter::from_query(filter);
    let filter_str = match list_filter {
        engine::MarketListFilter::All => "all",
        engine::MarketListFilter::Enabled => "enabled",
        engine::MarketListFilter::Disabled => "disabled",
        engine::MarketListFilter::Archived => "archived",
        engine::MarketListFilter::Test => "test",
        engine::MarketListFilter::Duplicate => "duplicate",
        _ => "active",
    };
    let filtered: Vec<&InstrumentConfig> = instruments
        .iter()
        .filter(|i| engine::matches_filter(i, list_filter, &dup_ids))
        .collect();

    let filter_tabs = format!(
        r#"<div class="market-filter-bar" style="display:flex;flex-wrap:wrap;gap:8px;margin-bottom:12px;align-items:center;">
            <span style="font-size:0.85rem;color:var(--text-muted);font-weight:600;">筛选:</span>
            <a href="/market?filter=active" class="btn btn-sm {}">监控中</a>
            <a href="/market?filter=all" class="btn btn-sm {}">全部</a>
            <a href="/market?filter=disabled" class="btn btn-sm {}">已禁用</a>
            <a href="/market?filter=archived" class="btn btn-sm {}">已归档</a>
            <a href="/market?filter=test" class="btn btn-sm {}">重复/测试</a>
        </div>"#,
        btn_class(list_filter == engine::MarketListFilter::Active),
        btn_class(list_filter == engine::MarketListFilter::All),
        btn_class(list_filter == engine::MarketListFilter::Disabled),
        btn_class(list_filter == engine::MarketListFilter::Archived),
        btn_class(list_filter == engine::MarketListFilter::Test),
    );

    let mut inst_rows = String::new();
    let cache_map: HashMap<String, &models::MarketCacheEntry> = market_cache
        .entries
        .iter()
        .map(|e| (e.symbol.clone(), e))
        .collect();

    for inst in &filtered {
        inst_rows.push_str(&render_instrument_row(
            inst, &cache_map, &dup_ids, filter_str,
        ));
    }

    if inst_rows.is_empty() {
        let empty_hint = match list_filter {
            engine::MarketListFilter::Archived => "当前筛选下没有已归档标的",
            engine::MarketListFilter::Test => "当前筛选下没有测试/重复标的",
            engine::MarketListFilter::Disabled => "当前筛选下没有已禁用标的",
            _ => "暂无监控标的，请先新增、恢复默认或切换筛选",
        };
        inst_rows = format!(
            "<tr><td colspan='10'><div class='empty-state'><span class='empty-state-icon'>📉</span><div class='empty-state-text'>{}</div><span style='font-size:0.7rem;'>使用上方表单新增，或点击「恢复默认标的」</span></div></td></tr>",
            empty_hint
        );
    }

    let last_refresh = market_cache
        .entries
        .iter()
        .map(|e| e.fetched_at.as_str())
        .max()
        .unwrap_or("从未刷新")
        .to_string();
    let auto_refresh_html = format!(
        r#"<div id="marketAutoRefreshBar" class="auto-refresh-bar">
            <span id="autoStatus">自动刷新：开启</span>
            <span id="nextRefresh">下次刷新：60 秒后</span>
            <span id="lastRefreshTime">最近刷新：{}</span>
            <button id="toggleAutoBtn" type="button" class="btn btn-sm btn-outline" onclick="toggleAutoRefresh(this)">暂停自动刷新</button>
        </div>"#,
        last_refresh
    );
    let cache_depth = market_cache.entries.len();
    let market_job_opt = state
        .repo
        .get_latest_job(ctx, "market_refresh")
        .await
        .unwrap_or(None);
    let fail_count = if let Some(j) = &market_job_opt {
        j.result_json
            .as_ref()
            .and_then(|v| v.get("failed_count").and_then(|x| x.as_u64()))
            .unwrap_or(0) as usize
    } else {
        0
    };
    let mon_count = instruments
        .iter()
        .filter(|i| engine::matches_filter(i, engine::MarketListFilter::Active, &dup_ids))
        .count();

    Ok(MarketPageVm {
        cleanup_confirm_msg,
        filter_tabs_html: filter_tabs,
        auto_refresh_html,
        inst_rows_html: inst_rows,
        last_refresh,
        cache_depth,
        mon_count,
        fail_count,
        fail_count_color: if fail_count > 0 {
            "var(--warn-color)".to_string()
        } else {
            "var(--text-muted)".to_string()
        },
        list_filter,
    })
}

fn btn_class(active: bool) -> &'static str {
    if active { "" } else { "btn-outline" }
}

fn render_instrument_row(
    inst: &InstrumentConfig,
    cache_map: &HashMap<String, &models::MarketCacheEntry>,
    dup_ids: &HashSet<String>,
    filter: &str,
) -> String {
    let quote = cache_map.get(&inst.symbol);
    let price_html = if let Some(q) = quote {
        if q.price > 0.0 || q.status.as_deref() == Some("ok") {
            format!("<span class='price-cell tabular'>{:.2}</span>", q.price)
        } else {
            "<span class='text-muted'>—</span>".to_string()
        }
    } else {
        "<span class='text-muted'>—</span>".to_string()
    };

    let is_dup = dup_ids.contains(&inst.instrument_id);
    let is_test = engine::is_test_instrument(inst);
    let is_arch = engine::is_instrument_archived(inst);
    let status_badge = if is_dup {
        "<span class='badge badge-orange'>重复标的</span>"
    } else if is_test {
        "<span class='badge badge-orange'>测试标的</span>"
    } else if is_arch {
        "<span class='badge badge-gray'>已归档</span>"
    } else if inst.enabled {
        "<span class='badge badge-blue'>监控中</span>"
    } else {
        "<span class='badge badge-gray'>未启用</span>"
    };

    let (chg_html, chg_pct_html) = if let Some(q) = quote {
        if let Some(ch) = q.change {
            let pct = q.change_percent.unwrap_or(0.0);
            let cls = if ch >= 0.0 { "text-up" } else { "text-down" };
            (
                format!("<span class='{} tabular'>{:+.2}</span>", cls, ch),
                format!("<span class='{} tabular'>{:+.2}%</span>", cls, pct),
            )
        } else if q.price > 0.0 {
            (
                "<span class='text-muted tabular'>暂无</span>".to_string(),
                "<span style='font-size:0.65rem;'>未返回昨收</span>".to_string(),
            )
        } else {
            ("-".to_string(), "-".to_string())
        }
    } else {
        ("-".to_string(), "-".to_string())
    };

    let row_status = if is_dup {
        status_badge.to_string()
    } else if let Some(q) = quote {
        if q.status.as_deref() == Some("ok") && q.price > 0.0 {
            "<span class='badge badge-green'>已更新</span>".to_string()
        } else if let Some(em) = &q.error_message {
            format!("<span class='badge badge-red' title='{}'>失败</span>", em)
        } else {
            "<span class='badge badge-gray'>暂无</span>".to_string()
        }
    } else {
        status_badge.to_string()
    };

    let display_nm = inst.name_zh.as_deref().unwrap_or(&inst.symbol);
    let prov = quote
        .map(|q| q.source.as_str())
        .unwrap_or(inst.provider.as_str());
    let id_for_form = &inst.instrument_id;
    let ac_str = format!("{:?}", inst.asset_class);

    let edit_btn = format!(
        r#"<button type="button" class="btn btn-sm btn-outline" onclick="openInstEdit(this)"
            data-id="{}" data-name="{}" data-symbol="{}" data-provider="{}" data-psym="{}" data-currency="{}" data-class="{}" data-filter="{}">编辑</button>"#,
        id_for_form,
        display_nm.replace('"', "&quot;"),
        inst.symbol,
        inst.provider,
        inst.provider_symbol,
        inst.currency,
        ac_str,
        filter
    );

    let refresh_btn = format!(
        r#"<button class='btn btn-sm market-refresh-btn' onclick='refreshOneSymbol(\"{}\", this)'>刷新</button>"#,
        inst.symbol
    );

    let action_buttons = if is_arch {
        let restore_form = format!(
            r#"<form action="/admin/instruments/restore" method="POST" style="display:inline;"><input type="hidden" name="instrument_id" value="{}"><input type="hidden" name="filter" value="{}"><button type="submit" class="btn btn-sm btn-outline">恢复</button></form>"#,
            id_for_form, filter
        );
        let delete_form = format!(
            r#"<form action="/admin/instruments/delete" method="POST" style="display:inline;" onsubmit="return confirm('确认永久删除该标的？此操作不可撤销。');"><input type="hidden" name="instrument_id" value="{}"><input type="hidden" name="filter" value="{}"><button type="submit" class="btn btn-sm btn-danger">删除</button></form>"#,
            id_for_form, filter
        );
        format!(
            "<div class='market-actions'>{}{}{}</div>",
            edit_btn, restore_form, delete_form
        )
    } else {
        let enable_disable_form = if inst.enabled {
            format!(
                r#"<form action="/admin/instruments/disable" method="POST" style="display:inline;" onsubmit="return confirm('禁用此标的?');"><input type="hidden" name="instrument_id" value="{}"><input type="hidden" name="filter" value="{}"><button type="submit" class="btn btn-sm btn-outline">禁用</button></form>"#,
                id_for_form, filter
            )
        } else {
            format!(
                r#"<form action="/admin/instruments/enable" method="POST" style="display:inline;"><input type="hidden" name="instrument_id" value="{}"><input type="hidden" name="filter" value="{}"><button type="submit" class="btn btn-sm btn-outline">启用</button></form>"#,
                id_for_form, filter
            )
        };
        let archive_form = format!(
            r#"<form action="/admin/instruments/archive" method="POST" style="display:inline;" onsubmit="return confirm('归档此标的?');"><input type="hidden" name="instrument_id" value="{}"><input type="hidden" name="filter" value="{}"><button type="submit" class="btn btn-sm btn-danger">归档</button></form>"#,
            id_for_form, filter
        );
        format!(
            "<div class='market-actions'>{}{}{}{}</div>",
            edit_btn, refresh_btn, enable_disable_form, archive_form
        )
    };

    format!(
        "<tr>
            <td><div style='font-weight:700;'>{}</div></td>
            <td><code style='font-size:0.75rem;'>{}</code></td>
            <td><span class='badge badge-outline' style='font-size:0.65rem;'>{:?}</span></td>
            <td class='tabular price-cell'>{}</td>
            <td class='tabular'>{}</td>
            <td class='tabular'>{}</td>
            <td>{}</td>
            <td style='font-size:0.75rem;'>{}</td>
            <td>{}</td>
            <td class='text-right actions-col'>
                {}
            </td>
        </tr>",
        display_nm,
        inst.symbol,
        inst.asset_class,
        price_html,
        chg_html,
        chg_pct_html,
        inst.currency,
        prov,
        row_status,
        action_buttons
    )
}
