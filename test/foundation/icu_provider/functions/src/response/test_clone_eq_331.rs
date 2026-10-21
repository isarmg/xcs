#[test]
fn test_clone_eq() {
    use crate::hello_world::*;
    let p1 = DataPayload::<HelloWorldV1>::from_static_str("Demo");
    let p2 = p1.clone();
    assert_eq!(p1, p2);

    let p1 = DataPayloadOr::<HelloWorldV1, usize>::from_payload(p1);
    let p2 = p1.clone();
    assert_eq!(p1, p2);

    let p3 = DataPayloadOr::<HelloWorldV1, usize>::from_other(555);
    let p4 = p3.clone();
    assert_eq!(p3, p4);

    let p5 = DataPayloadOr::<HelloWorldV1, usize>::from_other(666);
    assert_ne!(p3, p5);
    assert_ne!(p4, p5);

    assert_ne!(p1, p3);
    assert_ne!(p1, p4);
    assert_ne!(p1, p5);
    assert_ne!(p2, p3);
    assert_ne!(p2, p4);
    assert_ne!(p2, p5);
}
