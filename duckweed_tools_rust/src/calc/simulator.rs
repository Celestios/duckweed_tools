//! Vessel water quality modeling and biology simulations, tracking N, P, K, Mg,
//! and trace Iron pools against academic parameters.

use std::collections::HashMap;

use super::composition::{
    valagro, urea, iron_edta,
    FertilizerComposition, Urea, ChelatedIron,
};

// ---------------------------------------------------------------------
// Verified Lemna minor ranges (mg/L unless noted)
// Tuple layout: (optimal_low, optimal_high, maximal_low, maximal_high)
// ---------------------------------------------------------------------
pub type NutrientRange = (f64, f64, f64, f64);

pub fn lemna_minor_ranges() -> HashMap<String, NutrientRange> {
    let mut m = HashMap::new();
    m.insert("EC_mS_cm".to_string(), (0.6, 1.4, 0.0, 10.9));
    m.insert("NO3_N".to_string(), (70.0, 700.0, 0.0, 1400.0));
    m.insert("NH4_N".to_string(), (45.0, 90.0, 9.0, 1350.0));
    m.insert("P".to_string(), (0.4, 11.0, 0.0, 55.0));
    m.insert("K".to_string(), (39.0, 780.0, 0.0, 2000.0));
    m.insert("Mg".to_string(), (5.0, 97.0, 0.0, 1200.0));
    m.insert("Fe".to_string(), (0.1, 11.0, 0.0, 30.0));
    m
}

/// Literature uptake-rate range (mg N / m^2 / day), Walsh et al. 2021
pub const LITERATURE_N_UPTAKE_RANGE: (f64, f64) = (500.0, 2500.0);
pub const DEFAULT_N_UPTAKE_MG_M2_DAY: f64 = 500.0;

pub fn classify(nutrient: &str, value_mg_l: f64) -> Result<String, String> {
    let ranges = lemna_minor_ranges();
    let (opt_lo, opt_hi, _max_lo, max_hi) = ranges
        .get(nutrient)
        .ok_or_else(|| {
            format!(
                "No range data for '{}'. Available: {:?}",
                nutrient,
                {
                    let mut keys: Vec<_> = ranges.keys().cloned().collect();
                    keys.sort();
                    keys
                }
            )
        })?;

    if value_mg_l > *max_hi {
        Ok("exceeds_documented_max".to_string())
    } else if value_mg_l < *opt_lo {
        Ok("below_optimal".to_string())
    } else if value_mg_l <= *opt_hi {
        Ok("optimal".to_string())
    } else {
        Ok("above_optimal".to_string())
    }
}

pub fn check_all(concentrations: &HashMap<String, f64>) -> HashMap<String, String> {
    let ranges = lemna_minor_ranges();
    concentrations
        .iter()
        .filter(|(k, _)| ranges.contains_key(k.as_str()))
        .filter_map(|(k, v)| classify(k, *v).ok().map(|status| (k.clone(), status)))
        .collect()
}

// ---------------------------------------------------------------------
// DuckweedVessel
// ---------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct DuckweedVessel {
    pub volume_l: f64,
    pub width_cm: f64,
    pub length_cm: f64,
    pub pools_mg: HashMap<String, f64>,
    pub water_depth_cm: f64,
    pub coverage_fraction: f64,
}

impl DuckweedVessel {
    pub fn new(
        volume_l: f64,
        width_cm: f64,
        length_cm: f64,
    ) -> Result<Self, String> {
        if width_cm <= 0.0 || length_cm <= 0.0 {
            return Err("width_cm/length_cm must be > 0".to_string());
        }
        if volume_l <= 0.0 {
            return Err("volume_L must be > 0".to_string());
        }
        let water_depth_cm = (volume_l * 1000.0) / (width_cm * length_cm);
        Ok(Self {
            volume_l,
            width_cm,
            length_cm,
            pools_mg: HashMap::new(),
            water_depth_cm,
            coverage_fraction: 0.8,
        })
    }

    pub fn with_coverage(mut self, coverage_fraction: f64) -> Self {
        self.coverage_fraction = coverage_fraction;
        self
    }

    pub fn surface_area_m2(&self) -> f64 {
        (self.width_cm * self.length_cm) / 10000.0
    }

    /// Calculate dynamic N removal rate (mg N / m^2 / day) based on surface coverage.
    /// Interpolates linearly between 2500 (at 20% coverage) and 500 (at 80% coverage).
    pub fn get_n_removal_rate(&self) -> f64 {
        let cov = self.coverage_fraction.clamp(0.0, 1.0);
        if cov <= 0.2 {
            return 2500.0;
        }
        if cov >= 0.8 {
            return 500.0;
        }
        let fraction = (cov - 0.2) / 0.6;
        2500.0 - fraction * (2500.0 - 500.0)
    }

    pub fn concentrations_mg_l(&self) -> HashMap<String, f64> {
        self.pools_mg
            .iter()
            .map(|(k, v)| (k.clone(), (v / self.volume_l * 10000.0).round() / 10000.0))
            .collect()
    }

    /// Dissolve `grams` of Valagro into the vessel.
    pub fn add_valagro(
        &mut self,
        grams: f64,
        fert: &FertilizerComposition,
    ) -> Result<(), String> {
        if grams < 0.0 {
            return Err("grams must be >= 0".to_string());
        }
        let mg_total = grams * 1000.0;
        *self.pools_mg.entry("NO3_N".to_string()).or_insert(0.0) +=
            mg_total * (fert.nitrogen.nitric / 100.0);
        *self.pools_mg.entry("NH4_N".to_string()).or_insert(0.0) +=
            mg_total * (fert.nitrogen.ammoniacal / 100.0);
        *self.pools_mg.entry("amide_N".to_string()).or_insert(0.0) +=
            mg_total * (fert.nitrogen.ureic / 100.0);
        *self.pools_mg.entry("P".to_string()).or_insert(0.0) +=
            mg_total * (fert.elemental_p() / 100.0);
        *self.pools_mg.entry("K".to_string()).or_insert(0.0) +=
            mg_total * (fert.elemental_k() / 100.0);
        *self.pools_mg.entry("Mg".to_string()).or_insert(0.0) +=
            mg_total * (fert.elemental_mg() / 100.0);
        *self.pools_mg.entry("Fe".to_string()).or_insert(0.0) +=
            mg_total * (fert.trace.fe / 100.0);
        Ok(())
    }

    pub fn add_valagro_default(&mut self, grams: f64) -> Result<(), String> {
        self.add_valagro(grams, &valagro())
    }

    /// Dissolve `grams` of urea into the vessel.
    pub fn add_urea(&mut self, grams: f64, urea_src: &Urea) -> Result<(), String> {
        if grams < 0.0 {
            return Err("grams must be >= 0".to_string());
        }
        *self.pools_mg.entry("amide_N".to_string()).or_insert(0.0) +=
            grams * 1000.0 * (urea_src.n_percent / 100.0);
        Ok(())
    }

    pub fn add_urea_default(&mut self, grams: f64) -> Result<(), String> {
        self.add_urea(grams, &urea())
    }

    /// Dissolve `grams` of chelated iron into the vessel.
    pub fn add_chelated_iron(
        &mut self,
        grams: f64,
        iron_profile: &ChelatedIron,
    ) -> Result<(), String> {
        if grams < 0.0 {
            return Err("grams must be >= 0".to_string());
        }
        *self.pools_mg.entry("Fe".to_string()).or_insert(0.0) +=
            grams * 1000.0 * (iron_profile.fe_percent / 100.0);
        Ok(())
    }

    pub fn add_chelated_iron_default(&mut self, grams: f64) -> Result<(), String> {
        self.add_chelated_iron(grams, &iron_edta())
    }

    /// Replace `fraction_removed` of the vessel's solution with fresh water.
    pub fn partial_water_exchange(&mut self, fraction_removed: f64) -> Result<(), String> {
        if !(0.0..=1.0).contains(&fraction_removed) {
            return Err("fraction_removed must be between 0 and 1".to_string());
        }
        for v in self.pools_mg.values_mut() {
            *v *= 1.0 - fraction_removed;
        }
        Ok(())
    }

    pub fn total_available_n_mg(&self) -> f64 {
        self.pools_mg.get("NO3_N").copied().unwrap_or(0.0)
            + self.pools_mg.get("NH4_N").copied().unwrap_or(0.0)
    }
}

pub fn estimate_days_to_n_exhaustion(
    vessel: &DuckweedVessel,
    uptake_rate_mg_m2_day: Option<f64>,
) -> Result<f64, String> {
    let rate = uptake_rate_mg_m2_day.unwrap_or_else(|| vessel.get_n_removal_rate());
    if rate <= 0.0 {
        return Err("uptake_rate_mg_m2_day must be > 0".to_string());
    }
    let daily_removal_mg = vessel.surface_area_m2() * rate;
    let days = vessel.total_available_n_mg() / daily_removal_mg;
    Ok((days * 100.0).round() / 100.0)
}

// ---------------------------------------------------------------------
// Weekly simulation
// ---------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct WeekSnapshot {
    pub week: i32,
    pub concentrations_mg_l: HashMap<String, f64>,
    pub statuses: HashMap<String, String>,
}

pub fn simulate_weekly_program(
    vessel: &mut DuckweedVessel,
    valagro_g_per_week: f64,
    urea_g_per_week: f64,
    weeks: i32,
    exchange_fraction: f64,
    iron_g_per_week: f64,
) -> Result<Vec<WeekSnapshot>, String> {
    if weeks <= 0 {
        return Err("weeks must be > 0".to_string());
    }
    if valagro_g_per_week < 0.0 || urea_g_per_week < 0.0 || iron_g_per_week < 0.0 {
        return Err("doses must be >= 0".to_string());
    }

    // Stoichiometric tissue ratios relative to Nitrogen (N = 1.0)
    let p_n_uptake_ratio = 0.133;
    let k_n_uptake_ratio = 0.667;
    let mg_n_uptake_ratio = 0.089;
    let fe_n_uptake_ratio = 0.009;

    let fert = valagro();
    let urea_src = urea();
    let iron_src = iron_edta();
    let mut log = Vec::new();

    for week in 1..=weeks {
        if week > 1 && exchange_fraction > 0.0 {
            vessel.partial_water_exchange(exchange_fraction)?;
        }
        vessel.add_valagro(valagro_g_per_week, &fert)?;
        if urea_g_per_week > 0.0 {
            vessel.add_urea(urea_g_per_week, &urea_src)?;
        }
        if iron_g_per_week > 0.0 {
            vessel.add_chelated_iron(iron_g_per_week, &iron_src)?;
        }

        // Simulate hydrolysis of Urea (amide_N -> NH4_N)
        let amide = vessel.pools_mg.get("amide_N").copied().unwrap_or(0.0);
        if amide > 0.0 {
            *vessel.pools_mg.entry("NH4_N".to_string()).or_insert(0.0) += amide;
            vessel.pools_mg.insert("amide_N".to_string(), 0.0);
        }

        // Simulate weekly nutrient uptake subtraction
        let n_rate = vessel.get_n_removal_rate();
        let weekly_removal_mg = vessel.surface_area_m2() * n_rate * 7.0;

        let no3 = vessel.pools_mg.get("NO3_N").copied().unwrap_or(0.0);
        let nh4 = vessel.pools_mg.get("NH4_N").copied().unwrap_or(0.0);
        let total_avail_n = no3 + nh4;

        if total_avail_n > 0.0 {
            let actual_n_taken = weekly_removal_mg.min(total_avail_n);
            let no3_frac = no3 / total_avail_n;
            let nh4_frac = nh4 / total_avail_n;

            vessel.pools_mg.insert(
                "NO3_N".to_string(),
                (no3 - actual_n_taken * no3_frac).max(0.0),
            );
            vessel.pools_mg.insert(
                "NH4_N".to_string(),
                (nh4 - actual_n_taken * nh4_frac).max(0.0),
            );

            let p = vessel.pools_mg.get("P").copied().unwrap_or(0.0);
            vessel
                .pools_mg
                .insert("P".to_string(), (p - actual_n_taken * p_n_uptake_ratio).max(0.0));

            let k = vessel.pools_mg.get("K").copied().unwrap_or(0.0);
            vessel
                .pools_mg
                .insert("K".to_string(), (k - actual_n_taken * k_n_uptake_ratio).max(0.0));

            let mg = vessel.pools_mg.get("Mg").copied().unwrap_or(0.0);
            vessel.pools_mg.insert(
                "Mg".to_string(),
                (mg - actual_n_taken * mg_n_uptake_ratio).max(0.0),
            );

            let fe = vessel.pools_mg.get("Fe").copied().unwrap_or(0.0);
            vessel.pools_mg.insert(
                "Fe".to_string(),
                (fe - actual_n_taken * fe_n_uptake_ratio).max(0.0),
            );
        }

        let conc = vessel.concentrations_mg_l();
        let statuses = check_all(&conc);
        log.push(WeekSnapshot {
            week,
            concentrations_mg_l: conc,
            statuses,
        });
    }

    Ok(log)
}
