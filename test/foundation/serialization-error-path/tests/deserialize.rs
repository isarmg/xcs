#![allow(clippy::unreadable_literal, dead_code)]

use data_serialization::Deserialize;
use serialization_derive::Deserialize;
use std::collections::BTreeMap as Map;
use std::fmt::Debug;

fn test<'de, T>(json: &'de str, expected: &str)
where
    T: Deserialize<'de> + Debug,
{
    let de = &mut json_codec::Deserializer::from_str(json);
    let result: Result<T, _> = serialization_error_path::deserialize(de);
    let path = result.unwrap_err().path().to_string();
    assert_eq!(path, expected);
}

#[test]
fn test_struct() {
    #[derive(Deserialize, Debug)]
    struct Package {
        name: String,
        dependencies: Map<String, Dependency>,
    }

    #[derive(Deserialize, Debug)]
    struct Dependency {
        version: String,
    }

    let j = r#"{
        "name": "demo",
        "dependencies": {
            "serde": {
                "version": 1
            }
        }
    }"#;

    test::<Package>(j, "dependencies.serde.version");
}

#[test]
fn test_vec() {
    #[derive(Deserialize, Debug)]
    struct Package {
        dependencies: Vec<Dependency>,
    }

    #[derive(Deserialize, Debug)]
    struct Dependency {
        name: String,
        version: String,
    }

    let j = r#"{
        "dependencies": [
            {
                "name": "serde",
                "version": "1.0"
            },
            {
                "name": "serde_json",
                "version": 1
            }
        }
    }"#;

    test::<Package>(j, "dependencies[1].version");
}

#[test]
fn test_option() {
    #[derive(Deserialize, Debug)]
    struct Package {
        dependency: Option<Dependency>,
    }

    #[derive(Deserialize, Debug)]
    struct Dependency {
        version: String,
    }

    let j = r#"{
        "dependency": {
            "version": 1
        }
    }"#;

    test::<Package>(j, "dependency.version");
}

#[test]
fn test_struct_variant() {
    #[derive(Deserialize, Debug)]
    struct Package {
        dependency: Dependency,
    }

    #[derive(Deserialize, Debug)]
    enum Dependency {
        Struct { version: String },
    }

    let j = r#"{
        "dependency": {
            "Struct": {
                "version": 1
            }
        }
    }"#;

    test::<Package>(j, "dependency.Struct.version");
}

#[test]
fn test_tuple_variant() {
    #[derive(Deserialize, Debug)]
    struct Package {
        dependency: Dependency,
    }

    #[derive(Deserialize, Debug)]
    enum Dependency {
        Tuple(String, String),
    }

    let j = r#"{
        "dependency": {
            "Tuple": ["serde", 1]
        }
    }"#;

    test::<Package>(j, "dependency.Tuple[1]");
}

#[test]
fn test_unknown_field() {
    #[derive(Deserialize, Debug)]
    struct Package {
        dependency: Dependency,
    }

    #[derive(Deserialize, Debug)]
    #[serialization(deny_unknown_fields)]
    struct Dependency {
        version: String,
    }

    let j = r#"{
        "dependency": {
            "version": "1.0",
            "name": "serde"
        }
    }"#;

    test::<Package>(j, "dependency.name");
}

#[test]
fn test_invalid_length() {
    #[derive(Deserialize, Debug)]
    struct Package {
        dependency: Dependency,
    }

    #[derive(Deserialize, Debug)]
    struct Dependency(String, String);

    let j = r#"{
        "dependency": ["serde"]
    }"#;

    test::<Package>(j, "dependency");
}

#[test]
fn test_syntax_error() {
    #[derive(Deserialize, Debug)]
    struct Package {
        dependency: Dependency,
    }

    #[derive(Deserialize, Debug)]
    struct Dependency {
        version: String,
    }

    let j = r#"{
        "dependency": {
            "error": *
    }"#;

    test::<Package>(j, "dependency.error");
}

#[test]
fn test_u128() {
    #[derive(Deserialize, Debug)]
    struct Container {
        n: u128,
    }

    let j = r#"{
        "n": 130033514578017493995102500318550798591
    }"#;

    let de = &mut json_codec::Deserializer::from_str(j);
    let container: Container = serialization_error_path::deserialize(de).expect("failed to deserialize");

    assert_eq!(container.n, 130033514578017493995102500318550798591u128);
}

#[test]
fn test_map_nonstring_key() {
    #[derive(Deserialize, Debug)]
    struct Dependency {
        version: String,
    }

    let j = r#"{
        "100": {
            "version": false
        }
    }"#;

    test::<Map<i32, Dependency>>(j, "100.version");
}
