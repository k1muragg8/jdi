async fn daily_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let state_clone = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        let config = storage::load_config(&state_clone.config_path)?;
        let portfolio_state = storage::load_state(&state_clone.state_path)?;
        let date = Local::now().format("%Y-%m-%d").to_string();

        let dca_plans = storage::dca_store::load_dca_plans(&state_clone.dca_plans_path)?;
        let dca_preview = engine::dca::calculate_dca_preview(&config, &dca_plans, &date);

        let decision =
            engine::decision::generate_buy_suggestions(&config, &portfolio_state, date.clone());

        // Load caches for risk and regime
        let risk_cache = storage::risk_cache_store::load_risk_cache(&state_clone.risk_cache_path)
            .unwrap_or(None);
        let regime_cache =
            storage::regime_cache_store::load_regime_cache(&state_clone.regime_cache_path)
                .unwrap_or_default();

        // Default to low/safe values if no cache
        let risk_overlay = if let Some(rc) = risk_cache {
            rc.overlay
        } else {
            models::GlobalRiskOverlay {
                risk_score: 0.0,
                risk_label: "未知(未刷新)".to_string(),
                factor_results: vec![],
                warnings: vec!["请运行 data refresh --risk".to_string()],
                explanation: "请运行 data refresh --risk 以获取准确风险评估。".to_string(),
            }
        };

        let mut regimes = std::collections::HashMap::new();
        for entry in regime_cache.entries {
            for asset in &config.assets {
                let symbol_opt = asset
                    .reference_instrument_symbol
                    .clone()
                    .or(asset.reference_index_symbol.clone());
                if let Some(s) = symbol_opt {
                    if s == entry.symbol {
                        regimes.insert(asset.asset_id.clone(), entry.result.clone());
                    }
                }
            }
        }

        let adjusted = engine::adjusted_decision::calculate_adjusted_decision(
            &config,
            &portfolio_state,
            &decision,
            &risk_overlay,
            &regimes,
        );
        let kelly =
            engine::kelly::calculate_kelly_preview(&config, &decision, &risk_overlay, &regimes);

        let snapshots = storage::reconciliation_store::load_alipay_snapshots(
            &state_clone.alipay_snapshots_path,
        )?;
        let mut latest_snaps = std::collections::HashMap::new();
        for s in &snapshots {
            let entry = latest_snaps.entry(s.asset_id.clone()).or_insert(s.clone());
            if s.snapshot_date >= entry.snapshot_date {
                *entry = s.clone();
            }
        }
        let mut reconciliation_results = Vec::new();
        for asset in &config.assets {
            if let Some(s) = latest_snaps.get(&asset.asset_id) {
                reconciliation_results.push(engine::reconciliation::reconcile_asset(
                    &config,
                    &portfolio_state,
                    s,
                ));
            }
        }

        let plan = engine::daily_plan::generate_daily_execution_plan(
            &config,
            &portfolio_state,
            date.clone(),
            &dca_preview,
            &adjusted,
            &kelly,
            &reconciliation_results,
        );

        let settlements =
            storage::dca_store::load_dca_settlements(&state_clone.dca_settlements_path)?;
        let lifecycle = engine::dca_lifecycle::calculate_dca_lifecycle(
            &config,
            &dca_plans,
            &settlements,
            &snapshots,
            &portfolio_state,
            &date,
        );

        Ok::<(models::DailyExecutionPlan, models::DcaLifecycleSummary), anyhow::Error>((
            plan, lifecycle,
        ))
    })
    .await
    .unwrap();

    match result {
        Ok((plan, lifecycle)) => {
            let mut rows = String::new();
            for item in plan.items {
                let badge_class = match item.status.as_str() {
                    "今日应执行" => "badge-red",
                    "暂停执行" | "等待对账" => "badge-gray",
                    "建议观察" | "数据不足" => "badge-orange",
                    _ => "badge-gray",
                };

                rows.push_str(&format!(
                    "<tr>
                        <td>
                            <div style='font-weight: 600;'>{}</div>
                            <div style='font-size: 0.75rem; color: var(--text-muted);'>{}</div>
                        </td>
                        <td>
                            <div style='font-size: 0.8rem; color: var(--text-muted);'>DCA: {:.2}</div>
                            <div style='font-size: 0.8rem; color: var(--text-muted);'>Adj: {:.2}</div>
                        </td>
                        <td>
                            <div class='text-up' style='font-size: 1.1rem; font-weight: 700;'>{:.2}</div>
                        </td>
                        <td>{}</td>
                        <td><span class='badge {}'>{}</span></td>
                        <td><div style='font-size: 0.8rem; color: var(--text-muted); max-width: 200px;'>{}</div></td>
                    </tr>",
                    item.fund_name,
                    item.sector,
                    item.dca_due_amount,
                    item.adjusted_decision_amount,
                    item.recommended_amount,
                    badge_status(&item.reconciliation_status),
                    badge_class,
                    item.status,
                    item.explanation
                ));
                
                if !item.warnings.is_empty() {
                    rows.push_str(&format!(
                        "<tr style='background-color: #FFF7E8;'><td colspan='6'><div style='font-size: 0.75rem; color: var(--warn-color); padding-left: 8px;'>⚠ {}</div></td></tr>",
                        item.warnings.join(" | ")
                    ));
                }
            }

            let mut global_warnings_html = String::new();
            if !plan.warnings.is_empty() {
                global_warnings_html = format!(
                    r#"<div class="message-banner message-error" style="margin-bottom: 20px;">
                        <strong>全局警告:</strong> {}
                    </div>"#,
                    plan.warnings.join(" | ")
                );
            }

            let mut lifecycle_reminder = String::new();
            if lifecycle.count_waiting_confirmation > 0 || lifecycle.count_unapplied > 0 {
                lifecycle_reminder = format!(
                    r#"<div class="message-banner message-success" style="background: #E8F3FF; color: #0052D9; border-color: #B2D3FF;">
                        💡 有 <strong>{}</strong> 笔定投待确认，<strong>{}</strong> 笔确认单待入账。建议先处理以保证数据准确。
                    </div>"#,
                    lifecycle.count_waiting_confirmation, lifecycle.count_unapplied
                );
            }

            let content = format!(
                r#"
                {}
                {}
                
                <div style="display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 16px;">
                    <h1>今日执行计划 ({})</h1>
                    <div style="text-align: right;">
                        <div style="font-size: 0.8rem; color: var(--text-muted);">今日建议买入总额</div>
                        <div style="font-size: 1.6rem; font-weight: 800; color: var(--up-color);">{:.2} <small style="font-size: 0.8rem; font-weight: 400;">CNY</small></div>
                    </div>
                </div>

                <div class="table-container">
                    <table>
                        <thead>
                            <tr>
                                <th>基金名称 / 赛道</th>
                                <th>参考金额 (DCA/Risk)</th>
                                <th>建议执行金额</th>
                                <th>对账状态</th>
                                <th>执行状态</th>
                                <th>说明</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>

                <div class="card" style="background-color: #F7F8FA; border: 1px dashed var(--border-color);">
                    <h3>💡 交易建议</h3>
                    <p style="font-size: 0.9rem; color: var(--text-muted);">
                        1. 优先执行处于 <strong>“今日应执行”</strong> 状态的项。<br>
                        2. <strong>“等待对账”</strong> 的项建议先录入最新的支付宝快照，确认持仓准确后再操作。<br>
                        3. 如果 <strong>“建议执行金额”</strong> 显著低于定投计划，通常是因为全局风险过高或该赛道已过热。
                    </p>
                </div>
                "#,
                global_warnings_html,
                lifecycle_reminder,
                plan.date,
                plan.recommended_total_buy,
                rows
            );

            layout("今日计划", content)
        }
        Err(e) => layout(
            "今日计划",
            format!("<div class='message-banner message-error'>生成执行计划失败: {}</div>", e),
        ),
    }
}
