//! JSON file read/write with directory management.
//! Handles the cultivation_log.json database file with CRUD helpers.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::profiles::{
    ContainerData, ContainerProfile, FertilizerData, FertilizerProfile, LightData, LightProfile,
};

/// The top-level database structure matching the Python JSON format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Database {
    #[serde(default)]
    pub container_types: HashMap<String, ContainerData>,
    #[serde(default)]
    pub light_types: HashMap<String, LightData>,
    #[serde(default)]
    pub fertilizer_types: HashMap<String, FertilizerData>,
    #[serde(default)]
    pub containers: HashMap<String, Value>,
    #[serde(default)]
    pub light_sources: HashMap<String, Value>,
    #[serde(default)]
    pub log: Vec<Value>,
}

impl Default for Database {
    fn default() -> Self {
        Self {
            container_types: HashMap::new(),
            light_types: HashMap::new(),
            fertilizer_types: HashMap::new(),
            containers: HashMap::new(),
            light_sources: HashMap::new(),
            log: Vec::new(),
        }
    }
}

/// Shared application state for Axum handlers
#[derive(Debug, Clone)]
pub struct AppState {
    pub data_dir: PathBuf,
    pub db: Arc<Mutex<Database>>,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        let db = load_database(&data_dir);
        Self {
            data_dir,
            db: Arc::new(Mutex::new(db)),
        }
    }

    /// Reload database from disk
    pub fn reload(&self) {
        let db = load_database(&self.data_dir);
        *self.db.lock().unwrap() = db;
    }

    /// Save database to disk
    pub fn save(&self) -> Result<(), String> {
        let db = self.db.lock().unwrap();
        save_database(&self.data_dir, &db)
    }
}

fn db_path(data_dir: &Path) -> PathBuf {
    data_dir.join("cultivation_log.json")
}

pub fn load_database(data_dir: &Path) -> Database {
    let path = db_path(data_dir);
    if !path.exists() {
        return Database::default();
    }
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            eprintln!("Error parsing database: {}", e);
            Database::default()
        }),
        Err(e) => {
            eprintln!("Error reading database: {}", e);
            Database::default()
        }
    }
}

pub fn save_database(data_dir: &Path, db: &Database) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| format!("Error creating data dir: {}", e))?;
    let path = db_path(data_dir);
    // Atomic write: write to temp file then rename
    let tmp_path = path.with_extension("json.tmp");
    let content =
        serde_json::to_string_pretty(db).map_err(|e| format!("Error serializing database: {}", e))?;
    fs::write(&tmp_path, &content).map_err(|e| format!("Error writing database: {}", e))?;
    fs::rename(&tmp_path, &path).map_err(|e| format!("Error renaming temp database: {}", e))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Container Profiles CRUD
// ---------------------------------------------------------------------------

pub fn load_container_profiles(db: &Database) -> HashMap<String, ContainerProfile> {
    db.container_types
        .iter()
        .map(|(name, d)| {
            (
                name.clone(),
                ContainerProfile {
                    name: name.clone(),
                    width_cm: d.width_cm,
                    length_cm: d.length_cm,
                    height_cm: d.height_cm,
                },
            )
        })
        .collect()
}

pub fn save_container_profile(db: &mut Database, p: &ContainerProfile) {
    db.container_types.insert(
        p.name.clone(),
        ContainerData {
            width_cm: p.width_cm,
            length_cm: p.length_cm,
            height_cm: p.height_cm,
        },
    );
}

pub fn delete_container_profile(db: &mut Database, name: &str) -> bool {
    db.container_types.remove(name).is_some()
}

// ---------------------------------------------------------------------------
// Light Profiles CRUD
// ---------------------------------------------------------------------------

pub fn load_light_profiles(db: &Database) -> HashMap<String, LightProfile> {
    db.light_types
        .iter()
        .map(|(name, d)| {
            (
                name.clone(),
                LightProfile {
                    name: name.clone(),
                    wattage_w: d.wattage_w,
                    lumens: d.lumens,
                    kelvin: d.kelvin,
                },
            )
        })
        .collect()
}

pub fn save_light_profile(db: &mut Database, p: &LightProfile) {
    db.light_types.insert(
        p.name.clone(),
        LightData {
            wattage_w: p.wattage_w,
            lumens: p.lumens,
            kelvin: p.kelvin,
        },
    );
}

pub fn delete_light_profile(db: &mut Database, name: &str) -> bool {
    db.light_types.remove(name).is_some()
}

// ---------------------------------------------------------------------------
// Fertilizer Profiles CRUD
// ---------------------------------------------------------------------------

pub fn load_fertilizer_profiles(db: &Database) -> HashMap<String, FertilizerProfile> {
    db.fertilizer_types
        .iter()
        .map(|(name, d)| {
            // Infer default shelf_life_days if not stored
            let default_life = if name.to_lowercase().contains("urea") {
                14
            } else if name.to_lowercase().contains("iron")
                || name.to_lowercase().contains("fe")
            {
                90
            } else {
                365
            };

            (
                name.clone(),
                FertilizerProfile {
                    name: name.clone(),
                    n_total: d.n_total,
                    p2o5: d.p2o5,
                    k2o: d.k2o,
                    mgo: d.mgo,
                    trace_Fe: d.trace_Fe,
                    trace_Mn: d.trace_Mn,
                    trace_Zn: d.trace_Zn,
                    trace_Cu: d.trace_Cu,
                    trace_B: d.trace_B,
                    shelf_life_days: if d.shelf_life_days != 365 {
                        d.shelf_life_days
                    } else {
                        default_life
                    },
                },
            )
        })
        .collect()
}

pub fn save_fertilizer_profile(db: &mut Database, p: &FertilizerProfile) {
    db.fertilizer_types.insert(
        p.name.clone(),
        FertilizerData {
            n_total: p.n_total,
            p2o5: p.p2o5,
            k2o: p.k2o,
            mgo: p.mgo,
            trace_Fe: p.trace_Fe,
            trace_Mn: p.trace_Mn,
            trace_Zn: p.trace_Zn,
            trace_Cu: p.trace_Cu,
            trace_B: p.trace_B,
            shelf_life_days: p.shelf_life_days,
        },
    );
}

pub fn delete_fertilizer_profile(db: &mut Database, name: &str) -> bool {
    db.fertilizer_types.remove(name).is_some()
}
