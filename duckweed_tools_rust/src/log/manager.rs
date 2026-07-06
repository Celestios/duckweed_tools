//! Cultivation log CRUD + markdown export.
//! Port of manage_log.py (web API parts only, no CLI).

use std::fs;
use std::path::Path;

use crate::data::store::Database;

/// Export the cultivation log to a markdown file (Persian).
pub fn export_to_markdown(db: &Database, data_dir: &Path) -> Result<(), String> {
    let log = &db.log;
    if log.is_empty() {
        return Err("داده‌ای برای خروجی وجود ندارد.".to_string());
    }

    let mut md = Vec::new();
    md.push("# پروژه BioMesh: دفترچه گزارش کشت\n".to_string());
    md.push("> [!NOTE]".to_string());
    md.push("> این گزارش پارامترهای محیطی، دوزهای تغذیه‌ای و پاسخ‌های فیزیولوژیکی".to_string());
    md.push("> کلونی‌های Lemna/Wolffia در آزمایش‌های کشت خانگی را ثبت می‌کند. به طور خودکار از پایگاه داده واحد تولید شده است.\n".to_string());
    md.push("## گزارش‌های روزانه کشت\n".to_string());

    for entry in log {
        let day = entry.get("day").and_then(|v| v.as_i64()).unwrap_or(0);
        md.push(format!("### روز {}", day));

        // Environmental header
        let l_type = entry
            .get("light_source")
            .and_then(|v| v.as_str())
            .unwrap_or("نامشخص");
        let dist = entry
            .get("light_distance_cm")
            .and_then(|v| v.as_f64())
            .map(|v| format!("{} سانتیمتر", v))
            .unwrap_or_else(|| "ثبت نشده".to_string());
        let hours_str = match (
            entry.get("photoperiod_start").and_then(|v| v.as_f64()),
            entry.get("photoperiod_end").and_then(|v| v.as_f64()),
        ) {
            (Some(start), Some(end)) => {
                let total = if end >= start { end - start } else { (24.0 - start) + end };
                format!("{}:00 تا {}:00 ({} ساعت)", start, end, total)
            }
            _ => match entry.get("photoperiod_hours").and_then(|v| v.as_f64()) {
                Some(h) => format!("{} ساعت", h),
                None => "ثبت نشده".to_string(),
            },
        };
        md.push(format!(
            "* **منبع نور:** {} | **فاصله:** {} | **دوره نوری:** {}\n",
            l_type, dist, hours_str
        ));

        // Container table
        if let Some(containers) = entry.get("containers").and_then(|v| v.as_object()) {
            if !containers.is_empty() {
                md.push("| ظرف | قالب نوع | عمق آب (سانتیمتر) | حجم محاسبه‌شده (لیتر) | پوشش (%) | TDS (ppm) | وضعیت بیوماس | افزودنی‌ها |".to_string());
                md.push("|:---:|:---:|:----------:|:----------------:|:------------:|:---------:|:--------------|:----------|".to_string());

                for (cid, cdata) in containers {
                    let c_type = cdata
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("سینی استاندارد");
                    let w_dp = cdata
                        .get("water_depth_cm")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1.5);
                    let cov = cdata
                        .get("coverage_percent")
                        .and_then(|v| v.as_f64())
                        .map(|v| format!("{}%", v))
                        .unwrap_or_else(|| "-".to_string());
                    let tds = cdata
                        .get("tds_ppm")
                        .and_then(|v| v.as_i64())
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "-".to_string());
                    let status = cdata
                        .get("biomass_status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("سالم");

                    // Dynamic volume calculation
                    let dims = db
                        .container_types
                        .get(c_type);
                    let width = dims.map(|d| d.width_cm).unwrap_or(15.5);
                    let length = dims.map(|d| d.length_cm).unwrap_or(23.0);
                    let vol_l = (width * length * w_dp) / 1000.0;

                    // Warning highlights (English + Persian keywords)
                    let status_str = {
                        let lower = status.to_lowercase();
                        let warn_words = [
                            "death", "crash", "chlorosis", "yellow", "lethal", "sinking", "stress",
                            "مرگ", "سقوط", "زردشدگی", "زرد", "کشنده", "غرق", "تنش",
                        ];
                        if warn_words.iter().any(|w| lower.contains(w)) {
                            format!("⚠️ {}", status)
                        } else {
                            status.to_string()
                        }
                    };

                    let add_str = cdata
                        .get("additives")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            if arr.is_empty() {
                                "بدون".to_string()
                            } else {
                                arr.iter()
                                    .map(|a| {
                                        let name =
                                            a.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                        let amount =
                                            a.get("amount").and_then(|v| v.as_str()).unwrap_or("");
                                        format!("{} ({})", name, amount)
                                    })
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            }
                        })
                        .unwrap_or_else(|| "بدون".to_string());

                    md.push(format!(
                        "| **{}** | {} | {} | {:.3} | {} | {} | {} | {} |",
                        cid, c_type, w_dp, vol_l, cov, tds, status_str, add_str
                    ));
                }
                md.push(String::new());
            }
        }

        // Transfers
        if let Some(transfers) = entry.get("transfers").and_then(|v| v.as_array()) {
            if !transfers.is_empty() {
                md.push("**انتقال بیوماس:**".to_string());
                for t in transfers {
                    let amount = t.get("amount").and_then(|v| v.as_str()).unwrap_or("");
                    let from = t.get("from").and_then(|v| v.as_str()).unwrap_or("");
                    let to = t.get("to").and_then(|v| v.as_str()).unwrap_or("");
                    md.push(format!(
                        "- {} از **{}** به **{}** انتقال یافت",
                        amount, from, to
                    ));
                }
                md.push(String::new());
            }
        }

        // Operations
        if let Some(ops) = entry.get("operations").and_then(|v| v.as_array()) {
            if !ops.is_empty() {
                md.push("**عملیات انجام‌شده:**".to_string());
                for op in ops {
                    if let Some(s) = op.as_str() {
                        md.push(format!("- {}", s));
                    }
                }
                md.push(String::new());
            }
        }

        // Observations
        if let Some(obs) = entry.get("observations").and_then(|v| v.as_array()) {
            if !obs.is_empty() {
                md.push("**مشاهدات و یادداشت‌ها:**".to_string());
                for o in obs {
                    if let Some(s) = o.as_str() {
                        let lower = s.to_lowercase();
                        let warn_words = [
                            "death", "crash", "chlorosis", "yellow", "lethal", "die",
                            "مرگ", "سقوط", "زردشدگی", "زرد", "کشنده",
                        ];
                        if warn_words.iter().any(|w| lower.contains(w)) {
                            md.push(format!("- ⚠️ {}", s));
                        } else {
                            md.push(format!("- {}", s));
                        }
                    }
                }
                md.push(String::new());
            }
        }

        // Discussions
        if let Some(discs) = entry.get("discussions").and_then(|v| v.as_array()) {
            if !discs.is_empty() {
                md.push("**بحث‌ها و عیب‌یابی:**".to_string());
                for d in discs {
                    if let Some(s) = d.as_str() {
                        md.push(format!("- {}", s));
                    }
                }
                md.push(String::new());
            }
        }

        // Images
        if let Some(images) = entry.get("images").and_then(|v| v.as_array()) {
            if !images.is_empty() {
                md.push("**تصاویر ثبت‌شده:**".to_string());
                for img in images {
                    let filename = img
                        .get("filename")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let desc = img
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    md.push(format!("![{}](images/{})", desc, filename));
                    md.push(format!("*{}*\n", desc));
                }
                md.push(String::new());
            }
        }

        md.push("---".to_string());
    }

    let md_path = data_dir.join("cultivation_log.md");
    fs::create_dir_all(data_dir)
        .map_err(|e| format!("Error creating data dir: {}", e))?;
    fs::write(&md_path, md.join("\n"))
        .map_err(|e| format!("Error writing markdown: {}", e))?;
    Ok(())
}
