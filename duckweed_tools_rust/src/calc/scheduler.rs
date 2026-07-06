//! Simulates stage-by-stage crop scheduling and bookkeeping of total water/fertilizer used.

use std::collections::HashMap;

use super::composition::{valagro, FertilizerComposition, FertilizerSource};
use super::dosing::ppm_from_dose;

#[derive(Debug, Clone)]
pub struct GrowthStage {
    pub name: String,
    pub weeks: i32,
    pub feeds_per_week: i32,
    pub dose_g_per_l: f64,
    pub water_per_feed_l: f64,
}

#[derive(Debug, Clone)]
pub struct WeekLog {
    pub week_number: i32,
    pub stage_name: String,
    pub feeds_this_week: i32,
    pub water_used_l: f64,
    pub fertilizer_used_g: f64,
    pub cumulative_water_l: f64,
    pub cumulative_fertilizer_g: f64,
}

#[derive(Debug, Clone)]
pub struct ScheduleResult {
    pub weeks: Vec<WeekLog>,
}

impl ScheduleResult {
    pub fn total_water_l(&self) -> f64 {
        self.weeks
            .last()
            .map(|w| w.cumulative_water_l)
            .unwrap_or(0.0)
    }

    pub fn total_fertilizer_g(&self) -> f64 {
        self.weeks
            .last()
            .map(|w| w.cumulative_fertilizer_g)
            .unwrap_or(0.0)
    }
}

pub fn simulate_schedule(stages: &[GrowthStage]) -> Result<ScheduleResult, String> {
    if stages.is_empty() {
        return Err("stages list must not be empty".to_string());
    }

    let mut weeks = Vec::new();
    let mut week_counter = 0;
    let mut cum_water = 0.0_f64;
    let mut cum_fert = 0.0_f64;

    for stage in stages {
        if stage.weeks <= 0 {
            return Err(format!("Stage '{}' must have weeks > 0", stage.name));
        }
        if stage.feeds_per_week < 0 {
            return Err(format!(
                "Stage '{}' feeds_per_week must be >= 0",
                stage.name
            ));
        }
        if stage.dose_g_per_l < 0.0 || stage.water_per_feed_l < 0.0 {
            return Err(format!("Stage '{}' dose/water must be >= 0", stage.name));
        }

        for _ in 0..stage.weeks {
            week_counter += 1;
            let water_this_week = stage.feeds_per_week as f64 * stage.water_per_feed_l;
            let fert_this_week = water_this_week * stage.dose_g_per_l;
            cum_water += water_this_week;
            cum_fert += fert_this_week;
            weeks.push(WeekLog {
                week_number: week_counter,
                stage_name: stage.name.clone(),
                feeds_this_week: stage.feeds_per_week,
                water_used_l: (water_this_week * 10000.0).round() / 10000.0,
                fertilizer_used_g: (fert_this_week * 10000.0).round() / 10000.0,
                cumulative_water_l: (cum_water * 10000.0).round() / 10000.0,
                cumulative_fertilizer_g: (cum_fert * 10000.0).round() / 10000.0,
            });
        }
    }

    Ok(ScheduleResult { weeks })
}

pub fn nutrient_totals(
    stages: &[GrowthStage],
    fert: &FertilizerComposition,
) -> HashMap<String, f64> {
    let mut totals: HashMap<String, f64> = HashMap::new();
    let src = FertilizerSource::Valagro(fert.clone());

    for stage in stages {
        let water_total =
            stage.weeks as f64 * stage.feeds_per_week as f64 * stage.water_per_feed_l;
        if let Ok(result) = ppm_from_dose(stage.dose_g_per_l, 1.0, &src) {
            for (nutrient, ppm) in &result.ppm {
                let grams = ppm * water_total / 1000.0;
                *totals.entry(nutrient.clone()).or_insert(0.0) += grams;
            }
        }
    }

    totals
        .into_iter()
        .map(|(k, v)| (k, (v * 10000.0).round() / 10000.0))
        .collect()
}

pub fn nutrient_totals_default(stages: &[GrowthStage]) -> HashMap<String, f64> {
    nutrient_totals(stages, &valagro())
}
