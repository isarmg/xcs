#[test]
fn generic_standard_f64() {
    let mut rng = test_rng();
    let dist = ComplexDistribution::new(Standard, Standard);
    for _ in 0..100 {
        let c: Complex<f64> = rng.sample(dist);
        assert!(c.re >= 0.0 && c.re < 1.0);
        assert!(c.im >= 0.0 && c.im < 1.0);
    }
}
