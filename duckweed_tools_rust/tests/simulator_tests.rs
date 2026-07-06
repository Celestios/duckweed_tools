use std::collections::HashMap;

use duckweed_server::calc::composition::{
    urea_n_percent, urea, iron_edta
};
use duckweed_server::calc::simulator::{
    lemna_minor_ranges, classify, check_all, DuckweedVessel,
    estimate_days_to_n_exhaustion, simulate_weekly_program,
};
use duckweed_server::calc::stock::calculate_stock_default;
use duckweed_server::data::profiles::ContainerProfile;

fn is_close(a: f64, b: f64, epsilon: f64) -> bool {
    (a - b).abs() < epsilon
}

// ---------------------------------------------------------------------
// urea & iron composition tests
// ---------------------------------------------------------------------

#[test]
fn test_urea_n_percent_matches_known_value() {
    let u_pct = urea_n_percent();
    assert!(is_close(u_pct, 46.6, 0.15));
    assert!(is_close(urea().n_percent, u_pct, 1e-9));
}

#[test]
fn test_chelated_iron_composition() {
    assert!(is_close(iron_edta().fe_percent, 13.0, 1e-9));
    assert!(iron_edta().name.contains("13%"));
}

// ---------------------------------------------------------------------
// duckweed_simulator.py: range classification tests
// ---------------------------------------------------------------------

#[test]
fn test_range_table_values_are_the_verified_ones() {
    let ranges = lemna_minor_ranges();
    assert_eq!(*ranges.get("Mg").unwrap(), (5.0, 97.0, 0.0, 1200.0));
    assert_eq!(*ranges.get("P").unwrap(), (0.4, 11.0, 0.0, 55.0));
    assert_eq!(*ranges.get("K").unwrap(), (39.0, 780.0, 0.0, 2000.0));
    assert_eq!(*ranges.get("NO3_N").unwrap(), (70.0, 700.0, 0.0, 1400.0));
    assert_eq!(*ranges.get("NH4_N").unwrap(), (45.0, 90.0, 9.0, 1350.0));
    assert_eq!(*ranges.get("Fe").unwrap(), (0.1, 11.0, 0.0, 30.0));
}

#[test]
fn test_classify_below_optimal() {
    assert_eq!(classify("Mg", 4.0).unwrap(), "below_optimal");
    assert_eq!(classify("NO3_N", 42.0).unwrap(), "below_optimal");
    assert_eq!(classify("Fe", 0.05).unwrap(), "below_optimal");
}

#[test]
fn test_classify_optimal() {
    assert_eq!(classify("Mg", 70.0).unwrap(), "optimal");
    assert_eq!(classify("P", 5.0).unwrap(), "optimal");
    assert_eq!(classify("Fe", 1.5).unwrap(), "optimal");
}

#[test]
fn test_classify_above_optimal_but_under_max() {
    assert_eq!(classify("K", 900.0).unwrap(), "above_optimal");
    assert_eq!(classify("Fe", 15.0).unwrap(), "above_optimal");
}

#[test]
fn test_classify_exceeds_max() {
    assert_eq!(classify("P", 60.0).unwrap(), "exceeds_documented_max");
    assert_eq!(classify("Fe", 35.0).unwrap(), "exceeds_documented_max");
}

#[test]
fn test_classify_unknown_nutrient_raises() {
    assert!(classify("Unobtainium", 1.0).is_err());
}

#[test]
fn test_check_all_skips_unknown_keys() {
    let mut input = HashMap::new();
    input.insert("P".to_string(), 5.0);
    input.insert("not_tracked".to_string(), 999.0);
    let result = check_all(&input);
    assert!(result.contains_key("P"));
    assert!(!result.contains_key("not_tracked"));
}

// ---------------------------------------------------------------------
// DuckweedVessel tests
// ---------------------------------------------------------------------

#[test]
fn test_vessel_rejects_bad_construction() {
    assert!(DuckweedVessel::new(0.0, 15.5, 23.0).is_err());
    assert!(DuckweedVessel::new(1.0, 0.0, 23.0).is_err());
}

#[test]
fn test_surface_area_calc() {
    let v = DuckweedVessel::new(1.0, 15.5, 23.0).unwrap();
    assert!(is_close(v.surface_area_m2(), (15.5 * 23.0) / 10000.0, 1e-9));
}

#[test]
fn test_add_valagro_matches_hand_calc() {
    let mut v = DuckweedVessel::new(1.0, 15.5, 23.0).unwrap();
    v.add_valagro_default(1.0).unwrap(); // 1 g in 1 L
    let conc = v.concentrations_mg_l();
    assert!(is_close(*conc.get("NO3_N").unwrap(), 84.0, 1e-6));
    assert!(is_close(*conc.get("NH4_N").unwrap(), 36.0, 1e-6));
    assert!(is_close(*conc.get("amide_N").unwrap(), 30.0, 1e-6));
    assert!(is_close(*conc.get("Fe").unwrap(), 0.7, 1e-6));
}

#[test]
fn test_add_urea_only_affects_amide_pool() {
    let mut v = DuckweedVessel::new(1.0, 15.5, 23.0).unwrap();
    v.add_urea_default(0.1).unwrap(); // 0.1 g in 1 L
    let conc = v.concentrations_mg_l();
    assert!(is_close(*conc.get("amide_N").unwrap(), 100.0 * (urea_n_percent() / 100.0), 1e-6));
    assert_eq!(*conc.get("NO3_N").unwrap_or(&0.0), 0.0);
    assert_eq!(*conc.get("NH4_N").unwrap_or(&0.0), 0.0);
}

#[test]
fn test_add_chelated_iron_only_affects_fe_pool() {
    let mut v = DuckweedVessel::new(1.0, 15.5, 23.0).unwrap();
    v.add_chelated_iron_default(0.07).unwrap(); // 70 mg of Fe-EDTA in 1 L
    let conc = v.concentrations_mg_l();
    assert!(is_close(*conc.get("Fe").unwrap(), 9.1, 1e-6));
    assert_eq!(*conc.get("P").unwrap_or(&0.0), 0.0);
}

#[test]
fn test_add_negative_grams_raises() {
    let mut v = DuckweedVessel::new(1.0, 15.5, 23.0).unwrap();
    assert!(v.add_valagro_default(-1.0).is_err());
    assert!(v.add_urea_default(-1.0).is_err());
    assert!(v.add_chelated_iron_default(-1.0).is_err());
}

#[test]
fn test_partial_water_exchange_reduces_pools_proportionally() {
    let mut v = DuckweedVessel::new(1.0, 15.5, 23.0).unwrap();
    v.add_valagro_default(1.0).unwrap();
    let before = v.pools_mg.clone();
    v.partial_water_exchange(0.5).unwrap();
    for (k, val) in &before {
        assert!(is_close(*v.pools_mg.get(k).unwrap(), val * 0.5, 1e-9));
    }
}

#[test]
fn test_partial_water_exchange_rejects_out_of_range() {
    let mut v = DuckweedVessel::new(1.0, 15.5, 23.0).unwrap();
    assert!(v.partial_water_exchange(-0.1).is_err());
    assert!(v.partial_water_exchange(1.1).is_err());
}

#[test]
fn test_total_available_n_excludes_amide() {
    let mut v = DuckweedVessel::new(1.0, 15.5, 23.0).unwrap();
    v.add_valagro_default(1.0).unwrap();
    assert!(is_close(v.total_available_n_mg(), 84.0 + 36.0, 1e-6));
}

// ---------------------------------------------------------------------
// Exhaustion estimate tests
// ---------------------------------------------------------------------

#[test]
fn test_estimate_days_scales_inversely_with_uptake_rate() {
    let mut v = DuckweedVessel::new(1.0, 15.5, 23.0).unwrap();
    v.add_valagro_default(1.0).unwrap();
    let slow = estimate_days_to_n_exhaustion(&v, Some(500.0)).unwrap();
    let fast = estimate_days_to_n_exhaustion(&v, Some(1000.0)).unwrap();
    assert!(is_close(slow, fast * 2.0, 0.02));
}

#[test]
fn test_estimate_days_rejects_bad_rate() {
    let v = DuckweedVessel::new(1.0, 15.5, 23.0).unwrap();
    assert!(estimate_days_to_n_exhaustion(&v, Some(0.0)).is_err());
}

// ---------------------------------------------------------------------
// Weekly simulation tests
// ---------------------------------------------------------------------

#[test]
fn test_simulate_weekly_program_no_exchange_is_monotonic_increasing() {
    let mut v = DuckweedVessel::new(1.0, 15.5, 23.0).unwrap();
    let log = simulate_weekly_program(&mut v, 0.5, 0.0, 4, 0.0, 0.0).unwrap();
    assert_eq!(log.len(), 4);
    let mut prev_k = 0.0;
    for snap in log {
        let k_conc = *snap.concentrations_mg_l.get("K").unwrap();
        assert!(k_conc >= prev_k);
        prev_k = k_conc;
    }
}

#[test]
fn test_simulate_weekly_program_with_exchange_grows_slower() {
    let mut v_no_ex = DuckweedVessel::new(1.0, 15.5, 23.0).unwrap();
    let mut v_ex = DuckweedVessel::new(1.0, 15.5, 23.0).unwrap();
    let log_no_ex = simulate_weekly_program(&mut v_no_ex, 0.5, 0.0, 5, 0.0, 0.0).unwrap();
    let log_ex = simulate_weekly_program(&mut v_ex, 0.5, 0.0, 5, 0.3, 0.0).unwrap();
    assert!(log_ex.last().unwrap().concentrations_mg_l.get("K").unwrap() < log_no_ex.last().unwrap().concentrations_mg_l.get("K").unwrap());
}

#[test]
fn test_simulate_weekly_program_rejects_bad_input() {
    let mut v = DuckweedVessel::new(1.0, 15.5, 23.0).unwrap();
    assert!(simulate_weekly_program(&mut v, 1.0, 0.0, 0, 0.0, 0.0).is_err());
    assert!(simulate_weekly_program(&mut v, -1.0, 0.0, 1, 0.0, 0.0).is_err());
}

#[test]
fn test_real_recipe_flags_phosphorus_overshoot_by_week_two() {
    let mut v = DuckweedVessel::new(1.0, 15.5, 23.0).unwrap();
    let log = simulate_weekly_program(&mut v, 1.4, 0.154, 3, 0.0, 0.0).unwrap();
    assert_eq!(*log[1].statuses.get("P").unwrap(), "above_optimal");
}

#[test]
fn test_simulate_weekly_program_with_iron() {
    let mut v = DuckweedVessel::new(1.0, 15.5, 23.0).unwrap();
    let log = simulate_weekly_program(&mut v, 0.5, 0.1, 3, 0.30, 0.07).unwrap();
    assert_eq!(log.len(), 3);
    let fe_w1 = *log[0].concentrations_mg_l.get("Fe").unwrap();
    assert!(fe_w1 > 8.0);
    assert_eq!(*log[0].statuses.get("Fe").unwrap(), "optimal");
}

// ---------------------------------------------------------------------
// Profile & coverage N-uptake tests
// ---------------------------------------------------------------------

#[test]
fn test_coverage_n_uptake() {
    let v_low = DuckweedVessel::new(1.0, 15.5, 23.0).unwrap().with_coverage(0.2);
    let v_high = DuckweedVessel::new(1.0, 15.5, 23.0).unwrap().with_coverage(0.8);
    let v_mid = DuckweedVessel::new(1.0, 15.5, 23.0).unwrap().with_coverage(0.5);

    assert!(is_close(v_low.get_n_removal_rate(), 2500.0, 1e-9));
    assert!(is_close(v_high.get_n_removal_rate(), 500.0, 1e-9));
    assert!(is_close(v_mid.get_n_removal_rate(), 1500.0, 1e-9));
}

#[test]
fn test_calculate_stock_for_container_schedule() {
    let tray = ContainerProfile {
        name: "Standard Tray".to_string(),
        width_cm: 15.5,
        length_cm: 23.0,
        height_cm: 5.0,
    };

    // Test with Urea and Iron included (lifespan should be 14.0 days)
    let plan = calculate_stock_default(&tray, 7.0, 0.8, true, true, 1.5).unwrap();
    assert!(is_close(plan.stock_lifespan_days, 14.0, 1e-9));
    assert!(is_close(plan.number_of_doses, 2.0, 1e-9));
    assert!(is_close(plan.stock_volume_l, 0.1, 1e-9));
    assert!(is_close(plan.dose_volume_ml, 50.0, 1e-9));

    // Test split dosing: if cycle dose is very high, number of injections increases
    let plan_high = calculate_stock_default(&tray, 7.0, 0.1, true, true, 1.5).unwrap();
    assert!(plan_high.number_of_injections_per_cycle > 1);
    assert!(plan_high.injection_volume_ml < plan_high.dose_volume_ml);
}
