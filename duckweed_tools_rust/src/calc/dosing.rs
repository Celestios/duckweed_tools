//! Generic and modular dosing calculator supporting different chemical inputs
//! (VALAGRO MASTER 15-5-30+TE, Urea, Chelated Iron).

use std::collections::HashMap;

use super::composition::{FertilizerComposition, FertilizerSource};

pub fn percent_to_ppm_per_gram_per_litre(percent: f64) -> f64 {
    percent * 10.0
}

#[derive(Debug, Clone)]
pub struct DoseResult {
    pub dose_g_per_l: f64,
    pub water_volume_l: f64,
    pub total_grams: f64,
    pub ppm: HashMap<String, f64>,
    pub source_name: String,
}

/// Forward calculation: given a dose (g/L) and volume, return the ppm of
/// nutrients delivered by the specified source.
pub fn ppm_from_dose(
    dose_g_per_l: f64,
    water_volume_l: f64,
    source: &FertilizerSource,
) -> Result<DoseResult, String> {
    if dose_g_per_l < 0.0 {
        return Err("dose_g_per_L must be >= 0".to_string());
    }
    if water_volume_l <= 0.0 {
        return Err("water_volume_L must be > 0".to_string());
    }

    let ppm = match source {
        FertilizerSource::Urea(u) => {
            let mut m = HashMap::new();
            let n_ppm = (percent_to_ppm_per_gram_per_litre(u.n_percent) * dose_g_per_l * 10000.0)
                .round()
                / 10000.0;
            m.insert("N_total".to_string(), n_ppm);
            m.insert("amide_N".to_string(), n_ppm);
            m
        }
        FertilizerSource::Iron(i) => {
            let mut m = HashMap::new();
            let fe_ppm = (percent_to_ppm_per_gram_per_litre(i.fe_percent) * dose_g_per_l
                * 10000.0)
                .round()
                / 10000.0;
            m.insert("Fe".to_string(), fe_ppm);
            m
        }
        FertilizerSource::Valagro(f) => {
            let label = f.label_dict();
            let mut m: HashMap<String, f64> = label
                .iter()
                .map(|(k, v)| {
                    let val =
                        (percent_to_ppm_per_gram_per_litre(*v) * dose_g_per_l * 10000.0).round()
                            / 10000.0;
                    (k.clone(), val)
                })
                .collect();
            m.insert(
                "P_elemental".to_string(),
                (percent_to_ppm_per_gram_per_litre(f.elemental_p()) * dose_g_per_l * 10000.0)
                    .round()
                    / 10000.0,
            );
            m.insert(
                "K_elemental".to_string(),
                (percent_to_ppm_per_gram_per_litre(f.elemental_k()) * dose_g_per_l * 10000.0)
                    .round()
                    / 10000.0,
            );
            m.insert(
                "Mg_elemental".to_string(),
                (percent_to_ppm_per_gram_per_litre(f.elemental_mg()) * dose_g_per_l * 10000.0)
                    .round()
                    / 10000.0,
            );
            m
        }
    };

    let total_grams = dose_g_per_l * water_volume_l;
    Ok(DoseResult {
        dose_g_per_l,
        water_volume_l,
        total_grams,
        ppm,
        source_name: source.name().to_string(),
    })
}

/// Forward calculation using default Valagro source
pub fn ppm_from_dose_valagro(
    dose_g_per_l: f64,
    water_volume_l: f64,
    fert: &FertilizerComposition,
) -> Result<DoseResult, String> {
    let src = FertilizerSource::Valagro(fert.clone());
    ppm_from_dose(dose_g_per_l, water_volume_l, &src)
}

/// Lookup table: nutrient name -> percent extractor for Valagro
fn valagro_nutrient_percent(nutrient: &str, f: &FertilizerComposition) -> Option<f64> {
    match nutrient {
        "N_total" => Some(f.total_n),
        "P2O5" => Some(f.p2o5),
        "P_elemental" => Some(f.elemental_p()),
        "K2O" => Some(f.k2o),
        "K_elemental" => Some(f.elemental_k()),
        "MgO" => Some(f.mgo),
        "Mg_elemental" => Some(f.elemental_mg()),
        "Fe" => Some(f.trace.fe),
        "Mn" => Some(f.trace.mn),
        "Zn" => Some(f.trace.zn),
        "Cu" => Some(f.trace.cu),
        "B" => Some(f.trace.b),
        _ => None,
    }
}

const VALAGRO_NUTRIENTS: &[&str] = &[
    "B",
    "Cu",
    "Fe",
    "K2O",
    "K_elemental",
    "Mg_elemental",
    "MgO",
    "Mn",
    "N_total",
    "P2O5",
    "P_elemental",
    "Zn",
];

/// Reverse calculation: how many g/L are needed to reach `target_ppm` of `nutrient`?
pub fn dose_for_target_ppm(
    target_ppm: f64,
    nutrient: &str,
    water_volume_l: f64,
    source: &FertilizerSource,
) -> Result<DoseResult, String> {
    if target_ppm < 0.0 {
        return Err("target_ppm must be >= 0".to_string());
    }

    let percent = match source {
        FertilizerSource::Urea(u) => {
            if nutrient == "N_total" || nutrient == "amide_N" || nutrient == "N_ureic" {
                u.n_percent
            } else {
                return Err(format!("Urea does not supply '{}'.", nutrient));
            }
        }
        FertilizerSource::Iron(i) => {
            if nutrient == "Fe" || nutrient == "Fe_elemental" {
                i.fe_percent
            } else {
                return Err(format!("Chelated Iron does not supply '{}'.", nutrient));
            }
        }
        FertilizerSource::Valagro(f) => match valagro_nutrient_percent(nutrient, f) {
            Some(p) => p,
            None => {
                return Err(format!(
                    "Unknown nutrient '{}'. Choose from: {:?}",
                    nutrient, VALAGRO_NUTRIENTS
                ));
            }
        },
    };

    if percent == 0.0 {
        return Err(format!(
            "'{}' is 0% in this source; cannot solve for a dose.",
            nutrient
        ));
    }

    let ppm_per_g_per_l = percent_to_ppm_per_gram_per_litre(percent);
    let dose_g_per_l = target_ppm / ppm_per_g_per_l;
    ppm_from_dose(dose_g_per_l, water_volume_l, source)
}
