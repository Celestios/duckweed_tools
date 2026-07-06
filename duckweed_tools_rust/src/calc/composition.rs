//! Core chemical composition data for VALAGRO MASTER 15-5-30+TE,
//! Urea (CH4N2O), and Chelated Iron (Fe EDTA).
//! Exposes elemental conversions (oxide <-> elemental form).

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// 1. Molar-mass based conversion factors (oxide form -> elemental form)
// ---------------------------------------------------------------------------
// Exact IUPAC standard atomic weights (2021): P=30.97376, K=39.0983, Mg=24.305, O=15.999
pub const P2O5_TO_P: f64 = 61.94752 / 141.94252; // ~0.436427
pub const K2O_TO_K: f64 = 78.1966 / 94.1956; // ~0.830152
pub const MGO_TO_MG: f64 = 24.305 / 40.304; // ~0.602992

pub const P_TO_P2O5: f64 = 141.94252 / 61.94752;
pub const K_TO_K2O: f64 = 94.1956 / 78.1966;
pub const MG_TO_MGO: f64 = 40.304 / 24.305;

// ---------------------------------------------------------------------------
// 2. Urea Chemical Composition
// ---------------------------------------------------------------------------
const C_MASS: f64 = 12.011;
const H_MASS: f64 = 1.008;
const N_MASS: f64 = 14.007;
const O_MASS: f64 = 15.999;

pub const UREA_MOLAR_MASS: f64 = C_MASS + 4.0 * H_MASS + 2.0 * N_MASS + O_MASS; // 60.056
const UREA_N_FRACTION: f64 = (2.0 * 14.007) / 60.056; // pre-computed to avoid non-const division

/// Urea N percent, rounded to 3 decimal places (~46.659)
pub fn urea_n_percent() -> f64 {
    let frac = (2.0 * N_MASS) / UREA_MOLAR_MASS;
    (frac * 100.0 * 1000.0).round() / 1000.0
}

#[derive(Debug, Clone)]
pub struct Urea {
    pub name: String,
    pub n_percent: f64,
    pub shelf_life_days: i32,
}

impl Default for Urea {
    fn default() -> Self {
        Self {
            name: "Urea (CH4N2O)".to_string(),
            n_percent: urea_n_percent(),
            shelf_life_days: 14,
        }
    }
}

/// Global Urea constant
pub fn urea() -> Urea {
    Urea::default()
}

// ---------------------------------------------------------------------------
// 3. Chelated Iron Composition (Fe-EDTA)
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct ChelatedIron {
    pub name: String,
    pub fe_percent: f64,
    pub shelf_life_days: i32,
}

impl Default for ChelatedIron {
    fn default() -> Self {
        Self {
            name: "Iron EDTA (13% Fe)".to_string(),
            fe_percent: 13.0,
            shelf_life_days: 90,
        }
    }
}

/// Global Iron EDTA constant
pub fn iron_edta() -> ChelatedIron {
    ChelatedIron::default()
}

// ---------------------------------------------------------------------------
// 4. Valagro Master 15-5-30+TE Composition
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct NitrogenBreakdown {
    pub nitric: f64,
    pub ammoniacal: f64,
    pub ureic: f64,
}

impl Default for NitrogenBreakdown {
    fn default() -> Self {
        Self {
            nitric: 8.4,
            ammoniacal: 3.6,
            ureic: 3.0,
        }
    }
}

impl NitrogenBreakdown {
    pub fn total(&self) -> f64 {
        let sum = self.nitric + self.ammoniacal + self.ureic;
        (sum * 1_000_000.0).round() / 1_000_000.0
    }
}

#[derive(Debug, Clone)]
pub struct TraceElements {
    pub fe: f64,
    pub mn: f64,
    pub zn: f64,
    pub cu: f64,
    pub b: f64,
}

impl Default for TraceElements {
    fn default() -> Self {
        Self {
            fe: 0.07,
            mn: 0.03,
            zn: 0.01,
            cu: 0.05,
            b: 0.02,
        }
    }
}

impl TraceElements {
    pub fn as_map(&self) -> HashMap<String, f64> {
        let mut m = HashMap::new();
        m.insert("Fe".to_string(), self.fe);
        m.insert("Mn".to_string(), self.mn);
        m.insert("Zn".to_string(), self.zn);
        m.insert("Cu".to_string(), self.cu);
        m.insert("B".to_string(), self.b);
        m
    }
}

#[derive(Debug, Clone)]
pub struct FertilizerComposition {
    pub name: String,
    pub total_n: f64,
    pub p2o5: f64,
    pub k2o: f64,
    pub mgo: f64,
    pub nitrogen: NitrogenBreakdown,
    pub trace: TraceElements,
    pub sodium_free: bool,
    pub chloride_free: bool,
    pub fully_water_soluble: bool,
    pub shelf_life_days: i32,
}

impl Default for FertilizerComposition {
    fn default() -> Self {
        Self {
            name: "VALAGRO MASTER 15-5-30+TE".to_string(),
            total_n: 15.0,
            p2o5: 5.0,
            k2o: 30.0,
            mgo: 2.0,
            nitrogen: NitrogenBreakdown::default(),
            trace: TraceElements::default(),
            sodium_free: true,
            chloride_free: true,
            fully_water_soluble: true,
            shelf_life_days: 365,
        }
    }
}

impl FertilizerComposition {
    /// Validate that nitrogen forms sum to declared total
    pub fn validate(&self) -> Result<(), String> {
        let n_sum = self.nitrogen.total();
        if (n_sum - self.total_n).abs() > 0.05 {
            return Err(format!(
                "Nitrogen form breakdown ({}%) does not match declared total N ({}%).",
                n_sum, self.total_n
            ));
        }
        Ok(())
    }

    pub fn elemental_p(&self) -> f64 {
        (self.p2o5 * P2O5_TO_P * 10000.0).round() / 10000.0
    }

    pub fn elemental_k(&self) -> f64 {
        (self.k2o * K2O_TO_K * 10000.0).round() / 10000.0
    }

    pub fn elemental_mg(&self) -> f64 {
        (self.mgo * MGO_TO_MG * 10000.0).round() / 10000.0
    }

    /// Returns a map of all declared label nutrients (percent)
    pub fn label_dict(&self) -> HashMap<String, f64> {
        let mut d = HashMap::new();
        d.insert("N_total".to_string(), self.total_n);
        d.insert("N_nitric".to_string(), self.nitrogen.nitric);
        d.insert("N_ammoniacal".to_string(), self.nitrogen.ammoniacal);
        d.insert("N_ureic".to_string(), self.nitrogen.ureic);
        d.insert("P2O5".to_string(), self.p2o5);
        d.insert("K2O".to_string(), self.k2o);
        d.insert("MgO".to_string(), self.mgo);
        for (k, v) in self.trace.as_map() {
            d.insert(k, v);
        }
        d
    }
}

/// Global Valagro constant
pub fn valagro() -> FertilizerComposition {
    let f = FertilizerComposition::default();
    f.validate().expect("Built-in Valagro composition is invalid");
    f
}

// ---------------------------------------------------------------------------
// Fertilizer source enum for unified dispatch
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub enum FertilizerSource {
    Valagro(FertilizerComposition),
    Urea(Urea),
    Iron(ChelatedIron),
}

impl FertilizerSource {
    pub fn name(&self) -> &str {
        match self {
            FertilizerSource::Valagro(f) => &f.name,
            FertilizerSource::Urea(u) => &u.name,
            FertilizerSource::Iron(i) => &i.name,
        }
    }

    pub fn shelf_life_days(&self) -> i32 {
        match self {
            FertilizerSource::Valagro(f) => f.shelf_life_days,
            FertilizerSource::Urea(u) => u.shelf_life_days,
            FertilizerSource::Iron(i) => i.shelf_life_days,
        }
    }
}

/// Resolve a source string ("valagro", "urea", "iron") to a FertilizerSource
pub fn resolve_source(source: &str) -> FertilizerSource {
    match source {
        "urea" => FertilizerSource::Urea(urea()),
        "iron" => FertilizerSource::Iron(iron_edta()),
        _ => FertilizerSource::Valagro(valagro()),
    }
}
