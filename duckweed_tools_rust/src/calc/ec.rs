//! Rough electrical-conductivity (EC) estimator for a VALAGRO MASTER
//! 15-5-30+TE nutrient solution.

use std::collections::HashMap;

use super::composition::{valagro, FertilizerComposition};
use super::dosing::percent_to_ppm_per_gram_per_litre;

fn scale_factors() -> HashMap<String, f64> {
    let mut m = HashMap::new();
    m.insert("500".to_string(), 500.0);
    m.insert("700".to_string(), 700.0);
    m
}

#[derive(Debug, Clone)]
pub struct ECEstimate {
    pub dose_g_per_l: f64,
    pub total_dissolved_solids_ppm: f64,
    pub scale: String,
    pub estimated_ec_ms_cm: f64,
}

fn total_declared_solids_percent(fert: &FertilizerComposition) -> f64 {
    let label = fert.label_dict();
    let keys = [
        "N_total", "P2O5", "K2O", "MgO", "Fe", "Mn", "Zn", "Cu", "B",
    ];
    keys.iter()
        .map(|k| label.get(*k).copied().unwrap_or(0.0))
        .sum()
}

pub fn estimate_ec(
    dose_g_per_l: f64,
    scale: &str,
    fert: &FertilizerComposition,
) -> Result<ECEstimate, String> {
    if dose_g_per_l < 0.0 {
        return Err("dose_g_per_L must be >= 0".to_string());
    }
    let factors = scale_factors();
    let factor = factors
        .get(scale)
        .ok_or_else(|| format!("scale must be one of {:?}", factors.keys().collect::<Vec<_>>()))?;

    let solids_percent = total_declared_solids_percent(fert);
    let tds_ppm = percent_to_ppm_per_gram_per_litre(solids_percent) * dose_g_per_l;
    let ec = tds_ppm / factor;
    let ec_rounded = (ec * 10000.0).round() / 10000.0;

    Ok(ECEstimate {
        dose_g_per_l,
        total_dissolved_solids_ppm: tds_ppm,
        scale: scale.to_string(),
        estimated_ec_ms_cm: ec_rounded,
    })
}

pub fn estimate_ec_default(dose_g_per_l: f64, scale: &str) -> Result<ECEstimate, String> {
    estimate_ec(dose_g_per_l, scale, &valagro())
}

pub fn dose_for_target_ec(
    target_ec_ms_cm: f64,
    scale: &str,
    fert: &FertilizerComposition,
) -> Result<ECEstimate, String> {
    if target_ec_ms_cm < 0.0 {
        return Err("target_ec_mS_cm must be >= 0".to_string());
    }
    let factors = scale_factors();
    let factor = factors
        .get(scale)
        .ok_or_else(|| format!("scale must be one of {:?}", factors.keys().collect::<Vec<_>>()))?;

    let solids_percent = total_declared_solids_percent(fert);
    let ppm_per_g_per_l = percent_to_ppm_per_gram_per_litre(solids_percent);
    let target_ppm = target_ec_ms_cm * factor;
    let dose_g_per_l = target_ppm / ppm_per_g_per_l;
    let dose_rounded = (dose_g_per_l * 100000.0).round() / 100000.0;

    estimate_ec(dose_rounded, scale, fert)
}

pub fn dose_for_target_ec_default(target_ec_ms_cm: f64, scale: &str) -> Result<ECEstimate, String> {
    dose_for_target_ec(target_ec_ms_cm, scale, &valagro())
}
