pub fn fmt_f64_opt(val: Option<f64>, precision: usize) -> String {
    match val {
        Some(v) => format!("{:.1$}", v, precision),
        None => "-".to_string(),
    }
}

pub fn fmt_amount(val: f64) -> String {
    format!("{:.2}", val)
}

pub fn fmt_nav(val: f64) -> String {
    format!("{:.4}", val)
}

pub fn fmt_pct(val: f64) -> String {
    format!("{:.2}%", val * 100.0)
}

pub fn safe_div(num: f64, den: f64) -> String {
    if den.abs() < 0.000001 {
        "N/A".to_string()
    } else {
        format!("{:.2}%", (num / den) * 100.0)
    }
}

pub fn color_class(val: f64) -> &'static str {
    if val > 0.1 {
        "text-up"
    } else if val < -0.1 {
        "text-down"
    } else {
        ""
    }
}

pub fn badge_regime(label: &str) -> String {
    let (cls, txt) = match label {
        "极热" | "过热" => ("badge-red", label),
        "偏热" => ("badge-orange", label),
        "中性" => ("badge-blue", label),
        "偏冷" => ("badge-outline", label),
        "极冷" | "过冷" => ("badge-green", label),
        _ => ("badge-gray", label),
    };
    format!("<span class='badge {}'>{}</span>", cls, txt)
}

pub fn badge_risk(label: &str) -> String {
    let cls = if label.contains("低") {
        "badge-green"
    } else if label.contains("高") {
        "badge-red"
    } else {
        "badge-blue"
    };
    format!("<span class='badge {}'>{}</span>", cls, label)
}

pub fn badge_status(status: &str) -> String {
    let cls = match status {
        "正常" | "已确认" | "成功" => "badge-green",
        "待处理" | "部分" | "警告" => "badge-orange",
        "失败" | "异常" | "不一致" => "badge-red",
        _ => "badge-gray",
    };
    format!("<span class='badge {}'>{}</span>", cls, status)
}
