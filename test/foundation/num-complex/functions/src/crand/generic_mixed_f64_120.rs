#[test]
fn generic_mixed_f64() {
    use rand::distributions::Uniform;

    let mut rng = test_rng();
    let re = Uniform::new(-100.0, 0.0);
    let dist = ComplexDistribution::new(re, Standard);
    for _ in 0..100 {
        // no type annotation required, since `Uniform` only produces one type.
        let c = rng.sample(dist);
        assert!(c.re >= -100.0 && c.re < 0.0);
        assert!(c.im >= 0.0 && c.im < 1.0);
    }
}
