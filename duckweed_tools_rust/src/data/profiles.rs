//! Data profile structs: ContainerProfile, LightProfile, FertilizerProfile.
//! These match the JSON schema from the Python project exactly.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerProfile {
    pub name: String,
    pub width_cm: f64,
    pub length_cm: f64,
    pub height_cm: f64,
}

impl ContainerProfile {
    pub fn surface_area_m2(&self) -> f64 {
        (self.width_cm * self.length_cm) / 10000.0
    }
}

/// JSON-level container data (without name, since name is the map key)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerData {
    pub width_cm: f64,
    pub length_cm: f64,
    pub height_cm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightProfile {
    pub name: String,
    #[serde(rename = "wattage_W")]
    pub wattage_w: f64,
    pub lumens: f64,
    pub kelvin: f64,
}

/// JSON-level light data (without name)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightData {
    #[serde(rename = "wattage_W")]
    pub wattage_w: f64,
    pub lumens: f64,
    pub kelvin: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FertilizerProfile {
    pub name: String,
    #[serde(rename = "N_total")]
    pub n_total: f64,
    #[serde(rename = "P2O5")]
    pub p2o5: f64,
    #[serde(rename = "K2O")]
    pub k2o: f64,
    #[serde(rename = "MgO")]
    pub mgo: f64,
    pub trace_Fe: f64,
    #[serde(default)]
    pub trace_Mn: f64,
    #[serde(default)]
    pub trace_Zn: f64,
    #[serde(default)]
    pub trace_Cu: f64,
    #[serde(default)]
    pub trace_B: f64,
    #[serde(default = "default_shelf_life")]
    pub shelf_life_days: i32,
}

fn default_shelf_life() -> i32 {
    365
}

/// JSON-level fertilizer data (without name)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FertilizerData {
    #[serde(rename = "N_total")]
    pub n_total: f64,
    #[serde(rename = "P2O5")]
    pub p2o5: f64,
    #[serde(rename = "K2O")]
    pub k2o: f64,
    #[serde(rename = "MgO")]
    pub mgo: f64,
    pub trace_Fe: f64,
    #[serde(default)]
    pub trace_Mn: f64,
    #[serde(default)]
    pub trace_Zn: f64,
    #[serde(default)]
    pub trace_Cu: f64,
    #[serde(default)]
    pub trace_B: f64,
    #[serde(default = "default_shelf_life")]
    pub shelf_life_days: i32,
}
