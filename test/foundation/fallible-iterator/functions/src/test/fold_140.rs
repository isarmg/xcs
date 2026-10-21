#[test]
fn fold() {
    fn add_smol(a: u32, b: u32) -> Result<u32, u32> {
        if b <= 2 {
            Ok(a + b)
        } else {
            Err(b)
        }
    }

    let it = convert(vec![0, 1, 3, 2].into_iter().map(Ok::<u32, u32>));
    assert_eq!(it.fold(0, add_smol), Err(3));

    let it = convert(vec![0, 1, 2, 1].into_iter().map(Ok::<u32, u32>));
    assert_eq!(it.fold(0, add_smol), Ok(4));
}
