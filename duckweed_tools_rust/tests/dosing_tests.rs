use std::collections::HashMap;

use duckweed_server::calc::composition::{
    valagro, NitrogenBreakdown, FertilizerComposition, FertilizerSource,
    P2O5_TO_P, K2O_TO_K, MGO_TO_MG, TraceElements
};
use duckweed_server::calc::dosing::{
    ppm_from_dose, dose_for_target_ppm, percent_to_ppm_per_gram_per_litre
};
use duckweed_server::calc::ec::{estimate_ec_default, dose_for_target_ec_default};
use duckweed_server::calc::stock::{build_stock_default, final_dose_from_stock_default};
use duckweed_server::calc::scheduler::{GrowthStage, simulate_schedule, nutrient_totals_default};

fn is_close(a: f64, b: f64, epsilon: f64) -> bool {
    (a - b).abs() < epsilon
}

// ---------------------------------------------------------------------
// composition.rs tests
// ---------------------------------------------------------------------

#[test]
fn test_label_values_match_spec() {
    let f = valagro();
    assert!(is_close(f.total_n, 15.0, 1e-9));
    assert!(is_close(f.p2o5, 5.0, 1e-9));
    assert!(is_close(f.k2o, 30.0, 1e-9));
    assert!(is_close(f.mgo, 2.0, 1e-9));
    assert!(is_close(f.nitrogen.nitric, 8.4, 1e-9));
    assert!(is_close(f.nitrogen.ammoniacal, 3.6, 1e-9));
    assert!(is_close(f.nitrogen.ureic, 3.0, 1e-9));
    assert!(is_close(f.trace.fe, 0.07, 1e-9));
    assert!(is_close(f.trace.mn, 0.03, 1e-9));
    assert!(is_close(f.trace.zn, 0.01, 1e-9));
    assert!(is_close(f.trace.cu, 0.05, 1e-9));
    assert!(is_close(f.trace.b, 0.02, 1e-9));
}

#[test]
fn test_nitrogen_forms_sum_to_total() {
    let f = valagro();
    assert!(is_close(f.nitrogen.total(), f.total_n, 1e-6));
}

#[test]
fn test_bad_nitrogen_breakdown_raises() {
    let bad_n = NitrogenBreakdown {
        nitric: 8.4,
        ammoniacal: 3.6,
        ureic: 5.0, // sums to 17, not 15
    };
    let bad_f = FertilizerComposition {
        nitrogen: bad_n,
        total_n: 15.0,
        ..Default::default()
    };
    assert!(bad_f.validate().is_err());
}

#[test]
fn test_elemental_conversions_are_correct() {
    let f = valagro();
    assert!(is_close(f.elemental_p(), ((5.0 * P2O5_TO_P * 10000.0) as f64).round() / 10000.0, 1e-6));
    assert!(is_close(f.elemental_k(), ((30.0 * K2O_TO_K * 10000.0) as f64).round() / 10000.0, 1e-6));
    assert!(is_close(f.elemental_mg(), ((2.0 * MGO_TO_MG * 10000.0) as f64).round() / 10000.0, 1e-6));
    
    // known textbook values
    assert!(is_close(f.elemental_p(), 2.182, 0.01));
    assert!(is_close(f.elemental_k(), 24.90, 0.01));
    assert!(is_close(f.elemental_mg(), 1.206, 0.01));
}

// ---------------------------------------------------------------------
// dosing.rs tests
// ---------------------------------------------------------------------

#[test]
fn test_percent_to_ppm_conversion() {
    assert!(is_close(percent_to_ppm_per_gram_per_litre(100.0), 1000.0, 1e-9));
    assert!(is_close(percent_to_ppm_per_gram_per_litre(15.0), 150.0, 1e-9));
}

#[test]
fn test_ppm_from_dose_basic() {
    let src = FertilizerSource::Valagro(valagro());
    let r = ppm_from_dose(1.0, 1.0, &src).unwrap();
    assert!(is_close(*r.ppm.get("N_total").unwrap(), 150.0, 1e-9));
    assert!(is_close(*r.ppm.get("K2O").unwrap(), 300.0, 1e-9));
    assert!(is_close(*r.ppm.get("P2O5").unwrap(), 50.0, 1e-9));
    assert!(is_close(*r.ppm.get("MgO").unwrap(), 20.0, 1e-9));
    assert!(is_close(r.total_grams, 1.0, 1e-9));
}

#[test]
fn test_ppm_from_dose_scales_linearly_with_volume() {
    let src = FertilizerSource::Valagro(valagro());
    let r1 = ppm_from_dose(2.0, 1.0, &src).unwrap();
    let r10 = ppm_from_dose(2.0, 10.0, &src).unwrap();
    assert!(is_close(*r1.ppm.get("N_total").unwrap(), *r10.ppm.get("N_total").unwrap(), 1e-9));
    assert!(is_close(r10.total_grams, r1.total_grams * 10.0, 1e-9));
}

#[test]
fn test_ppm_from_dose_rejects_bad_input() {
    let src = FertilizerSource::Valagro(valagro());
    assert!(ppm_from_dose(-1.0, 1.0, &src).is_err());
    assert!(ppm_from_dose(1.0, 0.0, &src).is_err());
}

#[test]
fn test_dose_for_target_ppm_round_trip() {
    let target = 200.0;
    let src = FertilizerSource::Valagro(valagro());
    let r = dose_for_target_ppm(target, "N_total", 15.0, &src).unwrap();
    assert!(is_close(*r.ppm.get("N_total").unwrap(), target, 1e-6));
    assert!(is_close(r.total_grams, r.dose_g_per_l * 15.0, 1e-9));
}

#[test]
fn test_dose_for_target_ppm_elemental_k() {
    let target = 300.0;
    let src = FertilizerSource::Valagro(valagro());
    let r = dose_for_target_ppm(target, "K_elemental", 5.0, &src).unwrap();
    assert!(is_close(*r.ppm.get("K_elemental").unwrap(), target, 1e-6));
}

#[test]
fn test_dose_for_target_ppm_unknown_nutrient_raises() {
    let src = FertilizerSource::Valagro(valagro());
    assert!(dose_for_target_ppm(100.0, "not_a_real_nutrient", 1.0, &src).is_err());
}

#[test]
fn test_dose_for_target_ppm_zero_percent_nutrient_raises() {
    let bad_f = FertilizerComposition {
        trace: TraceElements {
            fe: 0.07,
            mn: 0.03,
            zn: 0.01,
            cu: 0.05,
            b: 0.0, // 0 percent
        },
        ..Default::default()
    };
    let src = FertilizerSource::Valagro(bad_f);
    assert!(dose_for_target_ppm(1.0, "B", 1.0, &src).is_err());
}

// ---------------------------------------------------------------------
// ec.rs tests
// ---------------------------------------------------------------------

#[test]
fn test_estimate_ec_positive_and_scales_with_dose() {
    let e1 = estimate_ec_default(1.0, "700").unwrap();
    let e2 = estimate_ec_default(2.0, "700").unwrap();
    assert!(e1.estimated_ec_ms_cm > 0.0);
    assert!(is_close(e2.estimated_ec_ms_cm, e1.estimated_ec_ms_cm * 2.0, 1e-3));
}

#[test]
fn test_estimate_ec_scale_500_vs_700() {
    let e500 = estimate_ec_default(1.0, "500").unwrap();
    let e700 = estimate_ec_default(1.0, "700").unwrap();
    assert!(e500.estimated_ec_ms_cm > e700.estimated_ec_ms_cm);
    assert!(is_close(e500.total_dissolved_solids_ppm, e700.total_dissolved_solids_ppm, 1e-9));
}

#[test]
fn test_estimate_ec_invalid_scale_raises() {
    assert!(estimate_ec_default(1.0, "999").is_err());
}

#[test]
fn test_dose_for_target_ec_round_trip() {
    let target_ec = 1.8;
    let result = dose_for_target_ec_default(target_ec, "700").unwrap();
    assert!(is_close(result.estimated_ec_ms_cm, target_ec, 1e-3));
}

// ---------------------------------------------------------------------
// stock.rs tests
// ---------------------------------------------------------------------

#[test]
fn test_stock_solution_round_trip() {
    let final_dose = 1.2;
    let dilution = 100.0;
    let plan = build_stock_default(final_dose, dilution, 1.0).unwrap();
    let check = final_dose_from_stock_default(plan.stock_grams, plan.stock_volume_l, dilution).unwrap();
    assert!(is_close(check.final_dose_g_per_l, final_dose, 1e-3));
}

#[test]
fn test_stock_solution_invalid_inputs_raise() {
    assert!(build_stock_default(0.0, 100.0, 1.0).is_err());
    assert!(build_stock_default(1.0, 0.0, 1.0).is_err());
    assert!(final_dose_from_stock_default(0.0, 1.0, 100.0).is_err());
}

#[test]
fn test_stock_solution_conservation_of_mass() {
    let plan = build_stock_default(1.0, 50.0, 2.0).unwrap();
    let total_stock_grams = plan.stock_grams;
    let total_final_volume_if_all_used = plan.stock_volume_l * (plan.dilution_ratio + 1.0);
    let implied_final_dose = total_stock_grams / total_final_volume_if_all_used;
    assert!(is_close(implied_final_dose, plan.final_dose_g_per_l, 1e-6));
}

// ---------------------------------------------------------------------
// scheduler.rs tests
// ---------------------------------------------------------------------

#[test]
fn test_simulate_schedule_basic_totals() {
    let stages = vec![
        GrowthStage { name: "Veg".to_string(), weeks: 2, feeds_per_week: 2, dose_g_per_l: 1.0, water_per_feed_l: 5.0 },
        GrowthStage { name: "Bloom".to_string(), weeks: 3, feeds_per_week: 3, dose_g_per_l: 2.0, water_per_feed_l: 4.0 },
    ];
    let res = simulate_schedule(&stages).unwrap();
    assert_eq!(res.weeks.len(), 5);
    assert!(is_close(res.total_water_l(), 56.0, 1e-9));
    assert!(is_close(res.total_fertilizer_g(), 92.0, 1e-9));
}

#[test]
fn test_simulate_schedule_cumulative_is_monotonic() {
    let stages = vec![
        GrowthStage { name: "Veg".to_string(), weeks: 2, feeds_per_week: 2, dose_g_per_l: 1.0, water_per_feed_l: 5.0 },
    ];
    let res = simulate_schedule(&stages).unwrap();
    assert!(res.weeks[1].cumulative_water_l > res.weeks[0].cumulative_water_l);
    assert!(res.weeks[1].cumulative_fertilizer_g > res.weeks[0].cumulative_fertilizer_g);
}

#[test]
fn test_simulate_schedule_rejects_bad_stage() {
    assert!(simulate_schedule(&[GrowthStage { name: "Bad".to_string(), weeks: 0, feeds_per_week: 1, dose_g_per_l: 1.0, water_per_feed_l: 1.0 }]).is_err());
    assert!(simulate_schedule(&[GrowthStage { name: "Bad".to_string(), weeks: 1, feeds_per_week: -1, dose_g_per_l: 1.0, water_per_feed_l: 1.0 }]).is_err());
}

#[test]
fn test_nutrient_totals_matches_manual_calc() {
    let stages = vec![
        GrowthStage { name: "Veg".to_string(), weeks: 1, feeds_per_week: 1, dose_g_per_l: 1.0, water_per_feed_l: 10.0 },
    ];
    let totals = nutrient_totals_default(&stages);
    assert!(is_close(*totals.get("N_total").unwrap(), 1.5, 1e-6));
}
