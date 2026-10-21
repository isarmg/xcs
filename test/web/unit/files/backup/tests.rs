use super::*;
#[test]
fn multi_primary_routes_use_each_selected_prefix_and_the_shortest_capacity() {
    let configuration = json!({"route_slot_ids":["P01","B01","C01"],"backup_routes":[["P01","B01","C01"],["P02","B02"]],"backup_slot_ids":["B01","C01","B02"],"max_copies":2});
    assert_eq!(max_copies(&configuration), 2);
    assert_eq!(target_slots(&configuration, 2), vec!["B01", "B02"]);
    assert_eq!(target_slots(&configuration, 1), Vec::<String>::new());
    let draft = Draft {
        id: "draft".into(),
        repository: "repo".into(),
        path: "docs".into(),
        configuration,
        generation: "0".into(),
        scope_id: "scope".into(),
        target: 2,
        prefixes: BTreeMap::from([
            ("B01".into(), "first-chain".into()),
            ("B02".into(), "other-chain".into()),
        ]),
        loading: false,
        can_manage: true,
    };
    assert_eq!(
        request(&draft).unwrap()["backup_prefixes"],
        json!({"B01":"first-chain","B02":"other-chain"})
    );
    let mut invalid = draft;
    invalid.target = 3;
    assert!(request(&invalid).is_err());
    assert_eq!(
        target_slots(&json!({"available_route_slot_ids":["P01","B01"]}), 2),
        vec!["B01"]
    );
    assert_eq!(
        max_copies(
            &json!({"route_slot_ids":["P01","B01"],"available_route_slot_ids":["P01","B01","C01"]})
        ),
        3,
        "legacy available route describes capacity rather than the currently configured copy count"
    );
}
#[test]
fn independent_prefixes_are_relative_to_a_stable_scope_and_route() {
    let draft = Draft {
        id: "draft".into(),
        repository: "repo".into(),
        path: "docs".into(),
        configuration: json!({"route_slot_ids":["P01","B01","C01"]}),
        generation: "0".into(),
        scope_id: "stable-directory".into(),
        target: 3,
        prefixes: BTreeMap::from([
            ("B01".into(), "我的备份".into()),
            ("C01".into(), "长期保存".into()),
        ]),
        loading: false,
        can_manage: true,
    };
    let body = request(&draft).unwrap();
    assert_eq!(body["backup_prefixes"]["B01"], "我的备份");
    assert_eq!(body["backup_prefixes"]["C01"], "长期保存");
    assert_eq!(body["expected_scope_id"], "stable-directory");
    assert_eq!(body["expected_generation"], "0");
    assert!(valid_prefix(&"a".repeat(200)));
    assert!(!valid_prefix("../escape"));
    assert!(!valid_prefix("/absolute"));
    let mut invalid = draft;
    invalid.target = 4;
    assert!(request(&invalid).is_err());
}
