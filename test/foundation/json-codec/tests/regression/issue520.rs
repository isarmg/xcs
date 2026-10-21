#![allow(clippy::float_cmp)]

use serialization_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serialization(tag = "type", content = "data")]
enum E {
    Float(f32),
}

#[test]
fn test() {
    let e = E::Float(159.1);
    let v = json_codec::to_value(e).unwrap();
    let e = json_codec::from_value::<E>(v).unwrap();

    match e {
        E::Float(f) => assert_eq!(f, 159.1),
    }
}
