#[test]
fn generic_uniform_i32() {
    use rand::distributions::Uniform;

    let mut rng = test_rng();
    let re = Uniform::new(-100, 0);
    let im = Uniform::new(0, 100);
    let dist = ComplexDistribution::new(re, im);
    for _ in 0..100 {
        // no type annotation required, since `Uniform` only produces one type.
        let c = rng.sample(dist);
        assert!(c.re >= -100 && c.re < 0);
        assert!(c.im >= 0 && c.im < 100);
    }
}
