use super::*;
#[test]
fn compact_plots_preserve_graph_height_and_the_measurement_scale() {
    let p = plot(
        &[(5., Some(20.)), (10., Some(40.))],
        0.,
        60.,
        5.,
        150.,
        36.,
        false,
    );
    assert!(p.bottom - p.top >= 22.0);
    assert_eq!(p.maximum, 100.0);
}
#[test]
fn missing_values_and_collection_gaps_do_not_turn_into_zero_or_continuous_lines() {
    let p = plot(
        &[
            (0., Some(10.)),
            (5., Some(20.)),
            (10., None),
            (15., Some(30.)),
            (50., Some(40.)),
            (55., Some(50.)),
        ],
        0.,
        60.,
        5.,
        300.,
        120.,
        false,
    );
    assert_eq!(p.paths.len(), 2);
    assert_eq!(p.dots.len(), 1);
    assert_eq!(p.maximum, 100.);
    assert!(!p.paths.iter().any(|path| path.contains("NaN")));
}
#[test]
fn power_scales_to_real_measurements_and_single_zero_is_still_a_real_point() {
    let p = plot(
        &[(5., Some(20.)), (10., Some(40.))],
        0.,
        60.,
        5.,
        180.,
        70.,
        true,
    );
    assert_eq!(p.maximum, 44.);
    assert_eq!(p.paths.len(), 1);
    let p = plot(&[(5., Some(0.))], 0., 60., 5., 180., 70., true);
    assert_eq!(p.dots.len(), 1);
    assert!((p.dots[0].1 - p.bottom).abs() < 0.0001);
    let p = plot(&[(5., None)], 0., 60., 5., 180., 70., true);
    assert!(p.paths.is_empty() && p.dots.is_empty());
}
