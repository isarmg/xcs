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

thread_local! { static PARSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
fn counted_time(value: &Value) -> Option<f64> {
    PARSES.with(|c| c.set(c.get() + 1));
    value.as_f64()
}
#[test]
fn chart_series_parse_only_when_history_identity_changes_not_on_resize_or_state_updates() {
    Owner::new().with(|| {
        let state = RwSignal::new(State::default());
        state.update(|s| s.monitor.history = json_codec::json!({"from":0,"to":60000,"samples":[{"collected_at":5000,"cpu":{"usage_percent":10}},{"collected_at":10000,"cpu":{"usage_percent":20}}]}).into());
        PARSES.with(|c| c.set(0));
        let samples = series_projection(state, Metric::Cpu, counted_time);
        assert_eq!(samples.with(|s| s.points.len()), 2);
        assert_eq!(PARSES.with(|c| c.get()), 4);
        for i in 0..20 {
            state.update(|s| {s.notice=i.to_string();s.monitor.history_loading=i%2==0;});
            let p = samples.with(|s| plot(&s.points,s.from,s.to,s.interval,200.+f64::from(i),100.,false));
            assert_eq!(p.paths.len(),1);
        }
        assert_eq!(PARSES.with(|c| c.get()),4);
        state.update(|s| s.monitor.history["samples"][0]["cpu"]["usage_percent"]=30.into());
        assert_eq!(samples.with(|s| s.points[0].1),Some(30.));
        assert_eq!(PARSES.with(|c| c.get()),8);
    });
}
