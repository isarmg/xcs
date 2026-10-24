use super::*;
use data_serialization::{Deserialize, Serialize};
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serialization(deny_unknown_fields)]
struct Integers {
    #[serialization(with = "crate::u64_string")]
    value: u64,
    #[serialization(default, with = "crate::optional_u64_string")]
    optional: Option<u64>,
    #[serialization(with = "crate::u64_string_map")]
    map: std::collections::BTreeMap<String, u64>,
}
#[test]
fn full_width_wire_integers_roundtrip_without_float_or_precision_loss() {
    let value = Integers {
        value: u64::MAX,
        optional: Some(u64::MAX),
        map: std::collections::BTreeMap::from([("P01".into(), u64::MAX)]),
    };
    let json = json_codec::to_value(&value).unwrap();
    assert_eq!(json["value"], u64::MAX.to_string());
    assert_eq!(json["optional"], u64::MAX.to_string());
    assert_eq!(json["map"]["P01"], u64::MAX.to_string());
    assert_eq!(
        decode_strict::<Integers>(&json_codec::to_vec(&json).unwrap()).unwrap(),
        value
    );
    for invalid in [
        "",
        "01",
        "+1",
        "-1",
        "1 ",
        "1\n",
        "１８",
        "18446744073709551616",
    ] {
        assert!(parse_u64_decimal(invalid).is_err(), "accepted {invalid:?}");
    }
    for invalid in [
        r#"{"value":1,"map":{}}"#,
        r#"{"value":"1","optional":1,"map":{}}"#,
        r#"{"value":"1","map":{"P01":1}}"#,
        r#"{"value":"1","map":{},"unknown":true}"#,
    ] {
        assert!(decode_strict::<Integers>(invalid.as_bytes()).is_err());
    }
}

#[test]
fn persisted_workspace_requires_explicit_purpose_without_changing_current_bytes() {
    for purpose in ["normal", "historical_branch"] {
        let bytes = format!(
            r#"{{"base_head":"commit_fixture","branch_id":"br_fixture","created_at":"1","fencing_token":"1","files":{{}},"namespace_operations":[],"principal":"admin","purpose":"{purpose}","state":"open","workspace_id":"ws_fixture"}}"#
        );
        let workspace: Workspace = decode_strict(bytes.as_bytes()).unwrap();
        assert_eq!(workspace.purpose, purpose);
        assert_eq!(canonical_json(&workspace).unwrap(), bytes.as_bytes());
        // Empty selection maps remain legitimately omitted in the current format.
        assert!(workspace.selected_file_contexts.is_empty());
        assert!(workspace.selected_directory_branches.is_empty());
        let mut missing = json_codec::to_value(&workspace).unwrap();
        assert!(missing.as_object_mut().unwrap().remove("purpose").is_some());
        assert!(json_codec::from_value::<Workspace>(missing).is_err());
    }
}

#[test]
fn persisted_state_requires_nonsparse_fields_without_changing_current_bytes() {
    for generation in [0, u64::MAX] {
        let bytes = format!(
            r#"{{"branches":{{}},"business_generation":"{generation}","commits":{{}},"content_states":{{}},"control_auth":null,"events":[],"histories":{{}},"history_boundaries":{{}},"merge_links":{{}},"operation_principals":{{}},"operations":{{}},"placement_rules":{{}},"placements":{{}},"plans":{{}},"protection_policies":{{}},"repositories":{{}},"sequence":"0","topology_generation":"0","versions":{{}}}}"#
        );
        let state = State {
            business_generation: generation,
            ..Default::default()
        };
        assert_eq!(canonical_json(&state).unwrap(), bytes.as_bytes());
        assert_eq!(decode_strict::<State>(bytes.as_bytes()).unwrap(), state);
        for field in [
            "business_generation",
            "history_boundaries",
            "placement_rules",
            "protection_policies",
            "operation_principals",
        ] {
            let mut missing = json_codec::to_value(&state).unwrap();
            assert!(missing.as_object_mut().unwrap().remove(field).is_some());
            assert!(
                json_codec::from_value::<State>(missing).is_err(),
                "accepted missing {field} at generation {generation}"
            );
        }
    }
}
