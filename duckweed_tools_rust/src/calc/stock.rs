//! Home-scale "make a concentrated stock, then dilute it" calculator.

use std::collections::HashMap;

use super::composition::{
    valagro, urea, iron_edta,
    FertilizerComposition, FertilizerSource,
};
use super::dosing::{ppm_from_dose, DoseResult};
use super::simulator::lemna_minor_ranges;

#[derive(Debug, Clone)]
pub struct StockPlan {
    pub stock_volume_l: f64,
    pub stock_grams: f64,
    pub stock_dose_g_per_l: f64,
    pub dilution_ratio: f64,
    pub final_dose_g_per_l: f64,
    pub final_ppm: DoseResult,
}

pub fn build_stock_for_target_final_dose(
    final_dose_g_per_l: f64,
    dilution_ratio: f64,
    stock_volume_l: f64,
    fert: &FertilizerComposition,
) -> Result<StockPlan, String> {
    if final_dose_g_per_l <= 0.0 {
        return Err("final_dose_g_per_L must be > 0".to_string());
    }
    if dilution_ratio <= 0.0 {
        return Err("dilution_ratio must be > 0".to_string());
    }
    if stock_volume_l <= 0.0 {
        return Err("stock_volume_L must be > 0".to_string());
    }

    let stock_dose_g_per_l = final_dose_g_per_l * (dilution_ratio + 1.0);
    let stock_grams = stock_dose_g_per_l * stock_volume_l;
    let src = FertilizerSource::Valagro(fert.clone());
    let final_ppm = ppm_from_dose(final_dose_g_per_l, 1.0, &src)?;

    Ok(StockPlan {
        stock_volume_l,
        stock_grams: (stock_grams * 10000.0).round() / 10000.0,
        stock_dose_g_per_l: (stock_dose_g_per_l * 10000.0).round() / 10000.0,
        dilution_ratio,
        final_dose_g_per_l,
        final_ppm,
    })
}

pub fn build_stock_default(
    final_dose_g_per_l: f64,
    dilution_ratio: f64,
    stock_volume_l: f64,
) -> Result<StockPlan, String> {
    build_stock_for_target_final_dose(final_dose_g_per_l, dilution_ratio, stock_volume_l, &valagro())
}

pub fn final_dose_from_stock(
    stock_grams: f64,
    stock_volume_l: f64,
    dilution_ratio: f64,
    fert: &FertilizerComposition,
) -> Result<StockPlan, String> {
    if stock_grams <= 0.0 {
        return Err("stock_grams must be > 0".to_string());
    }
    if stock_volume_l <= 0.0 {
        return Err("stock_volume_L must be > 0".to_string());
    }
    if dilution_ratio <= 0.0 {
        return Err("dilution_ratio must be > 0".to_string());
    }

    let stock_dose_g_per_l = stock_grams / stock_volume_l;
    let final_dose_g_per_l = stock_dose_g_per_l / (dilution_ratio + 1.0);
    let src = FertilizerSource::Valagro(fert.clone());
    let final_ppm = ppm_from_dose(final_dose_g_per_l, 1.0, &src)?;

    Ok(StockPlan {
        stock_volume_l,
        stock_grams: (stock_grams * 10000.0).round() / 10000.0,
        stock_dose_g_per_l: (stock_dose_g_per_l * 10000.0).round() / 10000.0,
        dilution_ratio,
        final_dose_g_per_l: (final_dose_g_per_l * 10000.0).round() / 10000.0,
        final_ppm,
    })
}

pub fn final_dose_from_stock_default(
    stock_grams: f64,
    stock_volume_l: f64,
    dilution_ratio: f64,
) -> Result<StockPlan, String> {
    final_dose_from_stock(stock_grams, stock_volume_l, dilution_ratio, &valagro())
}

// ---------------------------------------------------------------------------
// Dynamic Container-Driven Stock Solution Recipe Builder
// ---------------------------------------------------------------------------
use crate::data::profiles::ContainerProfile;

#[derive(Debug, Clone)]
pub struct ContainerStockPlan {
    pub container_name: String,
    pub surface_area_m2: f64,
    pub water_depth_cm: f64,
    pub vessel_volume_l: f64,

    // Intervals
    pub stock_lifespan_days: f64,
    pub dosing_cycle_days: f64,
    pub injection_interval_days: f64,

    // Volumes
    pub stock_volume_l: f64,
    pub dose_volume_ml: f64,
    pub injection_volume_ml: f64,
    pub number_of_doses: f64,
    pub number_of_injections_per_cycle: i32,

    // Quantities per cycle
    pub valagro_g_per_cycle: f64,
    pub urea_g_per_cycle: f64,
    pub iron_g_per_cycle: f64,

    // Quantities in stock bottle
    pub valagro_g_in_stock: f64,
    pub urea_g_in_stock: f64,
    pub iron_g_in_stock: f64,

    // PPM results
    pub cumulative_ppm: HashMap<String, f64>,
    pub injection_ppm: HashMap<String, f64>,

    // Warnings
    pub warnings: Vec<String>,
}

pub fn calculate_stock_for_container_schedule(
    container: &ContainerProfile,
    dosing_interval_days: f64,
    coverage_fraction: f64,
    include_urea: bool,
    include_iron: bool,
    fert: &FertilizerComposition,
    water_depth_cm: f64,
) -> Result<ContainerStockPlan, String> {
    let urea_src = urea();
    let iron_src = iron_edta();

    // 1. Determine shelf life of stock solution
    let mut active_lives = vec![fert.shelf_life_days as f64];
    if include_urea {
        active_lives.push(urea_src.shelf_life_days as f64);
    }
    if include_iron {
        active_lives.push(iron_src.shelf_life_days as f64);
    }
    let stock_lifespan_days = active_lives
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min);

    // 2. Number of doses in stock
    if dosing_interval_days <= 0.0 {
        return Err("dosing_interval_days must be > 0".to_string());
    }
    let num_doses = stock_lifespan_days / dosing_interval_days;

    // 3. Nitrogen requirement
    let s_area = container.surface_area_m2();
    let cov = coverage_fraction.clamp(0.0, 1.0);
    let n_rate = if cov <= 0.2 {
        2500.0
    } else if cov >= 0.8 {
        500.0
    } else {
        let fraction = (cov - 0.2) / 0.6;
        2500.0 - fraction * (2500.0 - 500.0)
    };

    let total_n_required = n_rate * s_area * dosing_interval_days;

    // 4. Balancing N using Valagro and Urea at 5:1 ratio
    let (valagro_g, urea_g) = if include_urea {
        let urea_n_fraction = urea_src.n_percent / 100.0;
        let valagro_n_fraction = fert.total_n / 100.0;
        let factor = 1000.0 * (urea_n_fraction + 5.0 * valagro_n_fraction);
        let ug = total_n_required / factor;
        (5.0 * ug, ug)
    } else {
        let valagro_n_fraction = fert.total_n / 100.0;
        let vg = total_n_required / (1000.0 * valagro_n_fraction);
        (vg, 0.0)
    };

    let iron_g = if include_iron {
        0.05 * valagro_g
    } else {
        0.0
    };

    // 5. Scale to stock
    let valagro_stock = valagro_g * num_doses;
    let urea_stock = urea_g * num_doses;
    let iron_stock = iron_g * num_doses;

    // 6. Solve stock bottle volume (max 100 g/L Valagro for solubility)
    let v_min = valagro_stock / 100.0;
    let standard_volumes = [0.1, 0.25, 0.5, 1.0, 2.0];
    let stock_volume_l = standard_volumes
        .iter()
        .find(|&&vol| vol >= v_min)
        .copied()
        .unwrap_or_else(|| (v_min * 2.0).ceil() / 2.0);

    // 7. Single dose volume
    let dose_volume_ml = (stock_volume_l * 1000.0) / num_doses;

    // 8. Container liquid volume
    let vessel_vol = s_area * water_depth_cm * 10.0; // m2 * cm * 10 = Liters
    if vessel_vol <= 0.0 {
        return Err(
            "Vessel water volume is 0 or negative. Verify container dimensions.".to_string(),
        );
    }

    // 9. Concentrations for the whole cycle
    let c_valagro = valagro_g / vessel_vol;
    let c_urea = urea_g / vessel_vol;
    let c_iron = iron_g / vessel_vol;

    let mut cumulative_ppm = HashMap::new();
    cumulative_ppm.insert(
        "NO3_N".to_string(),
        (c_valagro * fert.nitrogen.nitric * 10.0 * 1000.0).round() / 1000.0,
    );
    cumulative_ppm.insert(
        "NH4_N".to_string(),
        (c_valagro * fert.nitrogen.ammoniacal * 10.0 * 1000.0).round() / 1000.0,
    );
    cumulative_ppm.insert(
        "amide_N".to_string(),
        ((c_valagro * fert.nitrogen.ureic * 10.0
            + c_urea * urea_src.n_percent * 10.0)
            * 1000.0)
            .round()
            / 1000.0,
    );
    cumulative_ppm.insert(
        "P".to_string(),
        (c_valagro * fert.elemental_p() * 10.0 * 1000.0).round() / 1000.0,
    );
    cumulative_ppm.insert(
        "K".to_string(),
        (c_valagro * fert.elemental_k() * 10.0 * 1000.0).round() / 1000.0,
    );
    cumulative_ppm.insert(
        "Mg".to_string(),
        (c_valagro * fert.elemental_mg() * 10.0 * 1000.0).round() / 1000.0,
    );
    cumulative_ppm.insert(
        "Fe".to_string(),
        ((c_valagro * fert.trace.fe * 10.0 + c_iron * iron_src.fe_percent * 10.0) * 1000.0)
            .round()
            / 1000.0,
    );

    let nh4 = *cumulative_ppm.get("NH4_N").unwrap();
    let amide = *cumulative_ppm.get("amide_N").unwrap();
    cumulative_ppm.insert(
        "potential_NH4_N".to_string(),
        ((nh4 + amide) * 1000.0).round() / 1000.0,
    );

    // Estimate cumulative EC (700 scale)
    let solids_percent = fert.total_n
        + fert.p2o5
        + fert.k2o
        + fert.mgo
        + fert.trace.fe
        + fert.trace.mn
        + fert.trace.zn
        + fert.trace.cu
        + fert.trace.b;
    cumulative_ppm.insert(
        "EC_mS_cm".to_string(),
        ((solids_percent * 10.0 * c_valagro) / 700.0 * 1000.0).round() / 1000.0,
    );

    // 10. Check against optimal high limits for split-dosing
    let ranges = lemna_minor_ranges();
    let mut optimal_high_limits = HashMap::new();
    if let Some(r) = ranges.get("NO3_N") {
        optimal_high_limits.insert("NO3_N", r.1);
    }
    if let Some(r) = ranges.get("NH4_N") {
        optimal_high_limits.insert("potential_NH4_N", r.1);
    }
    if let Some(r) = ranges.get("P") {
        optimal_high_limits.insert("P", r.1);
    }
    if let Some(r) = ranges.get("K") {
        optimal_high_limits.insert("K", r.1);
    }
    if let Some(r) = ranges.get("Mg") {
        optimal_high_limits.insert("Mg", r.1);
    }
    if let Some(r) = ranges.get("Fe") {
        optimal_high_limits.insert("Fe", r.1);
    }
    if let Some(r) = ranges.get("EC_mS_cm") {
        optimal_high_limits.insert("EC_mS_cm", r.1);
    }

    let mut n_injections = 1_i32;
    for (key, limit) in &optimal_high_limits {
        if let Some(val) = cumulative_ppm.get(*key) {
            if *val > *limit {
                let needed = (*val / *limit).ceil() as i32;
                if needed > n_injections {
                    n_injections = needed;
                }
            }
        }
    }

    // 11. Calculate injection properties
    let injection_interval_days =
        ((dosing_interval_days / n_injections as f64) * 100.0).round() / 100.0;
    let injection_volume_ml = ((dose_volume_ml / n_injections as f64) * 100.0).round() / 100.0;

    let injection_ppm: HashMap<String, f64> = cumulative_ppm
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                (v / n_injections as f64 * 1000.0).round() / 1000.0,
            )
        })
        .collect();

    // Generate safety alerts
    let mut warnings = Vec::new();
    for (key, range) in &ranges {
        let max_hi = range.3;
        let mapped_key = if key == "NH4_N" {
            "potential_NH4_N"
        } else {
            key.as_str()
        };
        if let Some(val) = injection_ppm.get(mapped_key) {
            if *val > max_hi {
                warnings.push(format!(
                    "حتی با تزریق تقسیمی، غلظت تک تزریق {} ({:.1} ppm) از حداکثر مطلق ({:.1} ppm) فراتر است!",
                    mapped_key, val, max_hi
                ));
            }
        }
    }

    if n_injections > 1 {
        let pot_nh4 = cumulative_ppm.get("potential_NH4_N").copied().unwrap_or(0.0);
        let p_val = cumulative_ppm.get("P").copied().unwrap_or(0.0);
        warnings.push(format!(
            "نیاز به تزریق تقسیمی: افزودن کل دوز در یک مرحله باعث افزایش آمونیوم بالقوه ({:.1} ppm) یا فسفر ({:.1} ppm) فراتر از مقادیر بهینه بالا (به ترتیب ۹۰٫۰ و ۱۱٫۰ ppm) می‌شود.",
            pot_nh4, p_val
        ));
    }

    Ok(ContainerStockPlan {
        container_name: container.name.clone(),
        surface_area_m2: s_area,
        water_depth_cm,
        vessel_volume_l: vessel_vol,
        stock_lifespan_days,
        dosing_cycle_days: dosing_interval_days,
        injection_interval_days,
        stock_volume_l,
        dose_volume_ml: (dose_volume_ml * 100.0).round() / 100.0,
        injection_volume_ml,
        number_of_doses: (num_doses * 10.0).round() / 10.0,
        number_of_injections_per_cycle: n_injections,
        valagro_g_per_cycle: (valagro_g * 100000.0).round() / 100000.0,
        urea_g_per_cycle: (urea_g * 100000.0).round() / 100000.0,
        iron_g_per_cycle: (iron_g * 100000.0).round() / 100000.0,
        valagro_g_in_stock: (valagro_stock * 1000.0).round() / 1000.0,
        urea_g_in_stock: (urea_stock * 1000.0).round() / 1000.0,
        iron_g_in_stock: (iron_stock * 1000.0).round() / 1000.0,
        cumulative_ppm,
        injection_ppm,
        warnings,
    })
}

pub fn calculate_stock_default(
    container: &ContainerProfile,
    dosing_interval_days: f64,
    coverage_fraction: f64,
    include_urea: bool,
    include_iron: bool,
    water_depth_cm: f64,
) -> Result<ContainerStockPlan, String> {
    calculate_stock_for_container_schedule(
        container,
        dosing_interval_days,
        coverage_fraction,
        include_urea,
        include_iron,
        &valagro(),
        water_depth_cm,
    )
}
