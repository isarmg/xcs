use super::*;
#[test]
fn obsolete_body_version_graph_fields_are_not_interpreted_as_nodes() {
    let records = vec![
        json_codec::json!({"file_id":"file","version_id":"old","created_at":"1","source_version_ids":[]}),
        json_codec::json!({"file_id":"file","node_id":"new","version_id":"body","created_at":2,"source_version_ids":["old"]}),
    ];
    let parsed = metadata(&records, "file");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].id, "new");
    assert!(parsed[0].parents.is_empty());
    assert!(parsed[0].created.is_empty());
}

#[test]
fn rename_nodes_reuse_body_versions_but_keep_their_own_ancestry() {
    let records = vec![
        json_codec::json!({"file_id":"file","node_id":"ver-one","version_id":"ver-one","parent_node_ids":[],"kind":"content","created_at":"1"}),
        json_codec::json!({"file_id":"file","node_id":"fnode-rename","version_id":"ver-one","parent_node_ids":["ver-one"],"kind":"rename","created_at":"2"}),
        json_codec::json!({"file_id":"file","node_id":"ver-two","version_id":"ver-two","parent_node_ids":["fnode-rename"],"source_version_ids":["ver-one"],"kind":"content","created_at":"3"}),
    ];
    let tree = layout(&metadata(&records, "file"));
    assert_eq!(
        tree.levels,
        vec![
            vec!["ver-two".to_owned()],
            vec!["fnode-rename".to_owned()],
            vec!["ver-one".to_owned()]
        ]
    );
    assert_eq!(
        tree.edges,
        vec![
            ("fnode-rename".into(), "ver-two".into()),
            ("ver-one".into(), "fnode-rename".into())
        ]
    );
    let model = Model {
        file_history:
            json_codec::json!({"node_id":"fnode-rename","version_id":"ver-one","inherited":true})
                .into(),
        ..Default::default()
    };
    assert!(inherited_current(&model, "fnode-rename"));
    assert!(!inherited_current(&model, "ver-one"));
}
fn v(id: &str, created: u64, parents: &[&str]) -> Version {
    Version {
        id: id.into(),
        created: created.to_string(),
        parents: parents.iter().map(|p| (*p).into()).collect(),
    }
}
fn ids(tree: &Layout) -> BTreeSet<String> {
    tree.levels.iter().flatten().cloned().collect()
}

#[test]
fn a_real_fork_has_two_siblings_and_a_merge_keeps_both_parent_edges() {
    let tree = layout(&[
        v("root", 1, &[]),
        v("left", 2, &["root"]),
        v("right", 3, &["root"]),
        v("merge", 4, &["left", "right"]),
    ]);
    assert_eq!(
        tree.levels,
        vec![vec!["merge"], vec!["right", "left"], vec!["root"]]
    );
    assert_eq!(tree.edges.len(), 4);
    assert!(tree.edges.contains(&("left".into(), "merge".into())));
    assert!(tree.edges.contains(&("right".into(), "merge".into())));
}
#[test]
fn timestamps_without_parent_evidence_never_invent_a_chain() {
    let tree = layout(&[
        v("new", 30, &[]),
        v("old", 1, &[]),
        v("missing", 2, &["invisible"]),
    ]);
    assert_eq!(tree.levels, vec![vec!["new", "missing", "old"]]);
    assert!(tree.edges.is_empty());
}
#[test]
fn topology_wins_over_misordered_timestamps_and_input_order_is_irrelevant() {
    let records = vec![
        v("late-parent", 99, &[]),
        v("early-child", 1, &["late-parent"]),
        v("sibling", 2, &["late-parent"]),
    ];
    let tree = layout(&records);
    let mut reversed = records;
    reversed.reverse();
    assert_eq!(layout(&reversed), tree);
    assert_eq!(
        tree.levels,
        vec![vec!["sibling", "early-child"], vec!["late-parent"]]
    );
}
#[test]
fn cycles_and_duplicates_are_safe_without_losing_unrelated_edges_or_nodes() {
    let tree = layout(&[
        v("a", 1, &["b"]),
        v("b", 2, &["a"]),
        v("c", 3, &["a", "a", "c"]),
        v("c", 3, &["a", "a", "c"]),
        v("d", 4, &[]),
    ]);
    assert_eq!(
        ids(&tree),
        BTreeSet::from(["a".into(), "b".into(), "c".into(), "d".into()])
    );
    assert_eq!(tree.edges, vec![("a".into(), "c".into())]);
    assert_eq!(tree.levels, vec![vec!["d", "c", "b"], vec!["a"]]);
}
#[test]
fn old_ancestors_group_by_their_newer_children_before_timestamps_and_ids() {
    let records = vec![
        v("root", 1, &[]),
        v("left", 2, &["root"]),
        v("right", 3, &["root"]),
        v("z-left-one", 30, &["left"]),
        v("z-left-two", 50, &["left"]),
        v("a-right-one", 4, &["right"]),
        v("a-right-two", 20, &["right"]),
        v("merge", 60, &["z-left-two", "a-right-one"]),
    ];
    let tree = layout(&records);
    assert_eq!(tree.levels[2], vec!["left", "right"]);
    assert_eq!(tree.edges.len(), 8);
    assert_eq!(ids(&tree).len(), records.len());
    let mut reversed = records;
    reversed.reverse();
    assert_eq!(layout(&reversed), tree);
}
#[test]
fn mixed_node_timestamp_formats_keep_a_total_stable_order_and_unknown_dates_are_last() {
    let versions = [
        v("two", 2, &[]),
        v("ten", 10, &[]),
        Version {
            id: "legacy".into(),
            created: "2020-01-01T00:00:00Z".into(),
            parents: vec![],
        },
        Version {
            id: "unknown".into(),
            created: String::new(),
            parents: vec![],
        },
    ];
    for a in &versions {
        for b in &versions {
            for c in &versions {
                if newer_first(a, b).is_le() && newer_first(b, c).is_le() {
                    assert!(newer_first(a, c).is_le());
                }
            }
        }
    }
    let tree = layout(&versions);
    assert_eq!(tree.levels, vec![vec!["ten", "two", "legacy", "unknown"]]);
}
#[test]
fn every_branch_tip_is_above_its_ancestors_even_when_branches_have_unequal_depths() {
    let tree = layout(&[
        v("root", 1, &[]),
        v("long-mid", 2, &["root"]),
        v("long-tip", 3, &["long-mid"]),
        v("short-tip", 100, &["root"]),
    ]);
    assert_eq!(
        tree.levels,
        vec![
            vec!["short-tip", "long-tip"],
            vec!["long-mid"],
            vec!["root"]
        ]
    );
    for (parent, child) in &tree.edges {
        let depth = |id: &String| {
            tree.levels
                .iter()
                .position(|level| level.contains(id))
                .unwrap()
        };
        assert!(
            depth(parent) > depth(child),
            "every real ancestry edge points upwards"
        );
    }
    assert!(tree.edges.contains(&("root".into(), "short-tip".into())));
}
#[test]
fn upward_cross_layer_routes_avoid_nodes_and_enter_below_the_tallest_target_sibling() {
    let child = NodeBounds {
        left: 100.,
        right: 394.,
        top: 0.,
        bottom: 110.,
        depth: 0,
    };
    let tall_sibling = NodeBounds {
        left: 430.,
        right: 724.,
        top: 0.,
        bottom: 182.,
        depth: 0,
    };
    let middle = NodeBounds {
        left: 100.,
        right: 394.,
        top: 200.,
        bottom: 310.,
        depth: 1,
    };
    let parent = NodeBounds {
        left: 100.,
        right: 394.,
        top: 328.,
        bottom: 438.,
        depth: 2,
    };
    let boxes = [child, tall_sibling, middle, parent];
    let route = long_route(parent, child, &boxes);
    assert_eq!(middle.top - tall_sibling.bottom, 18.);
    assert_eq!(parent.top - middle.bottom, 18.);
    assert_eq!(route.first(), Some(&(parent.center(), parent.top)));
    assert_eq!(route[1].1, parent.top - 6.);
    assert_eq!(route[3].1, tall_sibling.bottom + 6.);
    assert!(route.windows(2).all(|segment| segment[1].1 <= segment[0].1));
    assert_eq!(route.last(), Some(&(child.center(), child.bottom + 3.)));
    assert!(route[2].0 < child.left);
    for segment in route.windows(2) {
        let [(x1, y1), (x2, y2)] = [segment[0], segment[1]];
        assert!(x1 == x2 || y1 == y2);
        for node in boxes {
            let enters = if x1 == x2 {
                x1 > node.left
                    && x1 < node.right
                    && y1.min(y2) < node.bottom
                    && y1.max(y2) > node.top
            } else {
                y1 > node.top
                    && y1 < node.bottom
                    && x1.min(x2) < node.right
                    && x1.max(x2) > node.left
            };
            assert!(
                !enters,
                "an upward route must never enter a node's interior: {segment:?} / {node:?}"
            );
        }
    }
    let across = edge_path(parent, child, &boxes).unwrap();
    assert!(across.contains('H') && across.contains('V') && across.contains('A'));
    assert!(!across.contains(['C', 'L', 'Q']));
    let adjacent = edge_path(middle, child, &boxes).unwrap();
    assert_eq!(adjacent, "M247.000,200.000 V113.000");
    assert!(edge_path(child, parent, &boxes).is_none());
}
#[test]
fn an_eighteen_pixel_layer_gap_places_rounded_right_angles_between_nodes() {
    let child = NodeBounds {
        left: 18.,
        right: 312.,
        top: 0.,
        bottom: 110.,
        depth: 0,
    };
    let sibling = NodeBounds {
        left: 330.,
        right: 624.,
        top: 0.,
        bottom: 182.,
        depth: 0,
    };
    let parent = NodeBounds {
        left: 330.,
        right: 624.,
        top: 200.,
        bottom: 310.,
        depth: 1,
    };
    let boxes = [child, sibling, parent];
    assert_eq!(parent.top - sibling.bottom, 18.);
    let (exit, entry) = gap_heights(parent, child, &boxes);
    assert_eq!((exit, entry), (194., 188.));
    assert!(parent.top > exit && exit > entry && entry > child.bottom + 3.);
    let path = edge_path(parent, child, &boxes).unwrap();
    assert_eq!(
        path,
        "M477.000,200.000 V195.000 A4.000,4.000 0 0 0 473.000,191.000 H169.000 A4.000,4.000 0 0 1 165.000,187.000 V113.000"
    );
    assert!(191. - EDGE_CORNER_RADIUS > sibling.bottom);
    assert!(191. + EDGE_CORNER_RADIUS < parent.top);
}
#[test]
fn orthogonal_turns_have_circular_arcs_and_short_runs_limit_both_corner_radii() {
    assert_eq!(
        rounded_route(&[(10., 40.), (10., 20.), (50., 20.), (50., 0.)], 4.).unwrap(),
        "M10.000,40.000 V24.000 A4.000,4.000 0 0 1 14.000,20.000 H46.000 A4.000,4.000 0 0 0 50.000,16.000 V0.000"
    );
    assert_eq!(
        rounded_route(&[(0., 10.), (0., 5.), (2., 5.), (2., 0.)], 4.).unwrap(),
        "M0.000,10.000 V6.000 A1.000,1.000 0 0 1 1.000,5.000 A1.000,1.000 0 0 0 2.000,4.000 V0.000"
    );
    assert_eq!(
        rounded_route(&[(3., 10.), (3., 10.), (3., 5.), (3., 0.)], 4.).unwrap(),
        "M3.000,10.000 V0.000"
    );
    for points in [
        &[(0., 0.)][..],
        &[(0., 0.), (1., 1.)][..],
        &[(0., 0.), (0., 1.), (0., 0.)][..],
        &[(0., 0.), (f64::NAN, 1.)][..],
    ] {
        assert!(rounded_route(points, 4.).is_none());
    }
    assert!(rounded_route(&[(0., 0.), (0., 1.)], f64::INFINITY).is_none());
}
#[test]
fn metadata_ignores_notes_current_markers_and_foreign_file_records() {
    let mut records = vec![
        json_codec::json!({"file_id":"selected","node_id":"a","version_id":"a","created_at":"1","parent_node_ids":[],"note":"initial"}),
        json_codec::json!({"file_id":"other","node_id":"foreign","version_id":"foreign","created_at":"0"}),
        json_codec::json!({"file_id":"selected","node_id":"b","version_id":"b","created_at":"2","parent_node_ids":["foreign","a","a",null]}),
    ];
    let before = metadata(&records, "selected");
    records[0]["note"] = "typing stays mounted".into();
    records[0]["current"] = true.into();
    assert_eq!(metadata(&records, "selected"), before);
    let tree = layout(&before);
    assert_eq!(tree.edges, vec![("a".into(), "b".into())]);
    assert_eq!(ids(&tree), BTreeSet::from(["a".into(), "b".into()]));
}
#[test]
fn placeholder_records_never_create_tree_nodes_or_source_aliases() {
    let records = vec![
        json_codec::json!({"file_id":"selected","node_id":"source","version_id":"source","created_at":"1"}),
        json_codec::json!({"file_id":"selected","branch_id":"pending","materialized":false,"content_state":"PENDING","version_id":null,"source_version_id":"source"}),
        json_codec::json!({"file_id":"selected","node_id":"not-real","version_id":"not-real","materialized":false,"parent_node_ids":["source"]}),
        json_codec::json!({"file_id":"selected","node_id":"first-write","version_id":"first-write","materialized":true,"created_at":"2","size":"0","parent_node_ids":["source"]}),
    ];
    let tree = layout(&metadata(&records, "selected"));
    assert_eq!(
        ids(&tree),
        BTreeSet::from(["source".into(), "first-write".into()])
    );
    assert_eq!(tree.edges, vec![("source".into(), "first-write".into())]);
}
#[test]
fn inherited_current_markers_do_not_change_real_version_identity_or_ancestry() {
    let versions = vec![
        json_codec::json!({"file_id":"file","node_id":"parent","version_id":"parent","created_at":"1","current":true}),
    ];
    let mut model = Model {
        file_history: json_codec::json!({"node_id":"parent","version_id":"parent","materialized":false,"inherited":true,"own_version_id":null,"source_version_id":"parent","nodes":versions}).into(),
        ..Default::default()
    };
    let before = metadata(history_nodes_ref(&model.file_history), "file");
    assert!(inherited_current(&model, "parent"));
    assert!(!inherited_current(&model, "other"));
    model.file_history["materialized"] = true.into();
    model.file_history["inherited"] = false.into();
    model.file_history["own_version_id"] = "parent".into();
    assert!(!inherited_current(&model, "parent"));
    assert_eq!(
        metadata(history_nodes_ref(&model.file_history), "file"),
        before
    );
    assert_eq!(layout(&before).levels, vec![vec!["parent"]]);
}

#[test]
fn history_identity_indexes_payloads_once_and_preserves_node_keys_on_note_refresh() {
    Owner::new().with(|| {
        let count = 128;
        let nodes = (0..count)
            .map(|i| json_codec::json!({"node_id":format!("node-{i}"),"note":"original"}))
            .collect::<Vec<_>>();
        // Previous per-node full-list projection copied n records for each of n nodes.
        let baseline_payload_copies: usize = (0..count).map(|_| nodes.clone().len()).sum();
        assert_eq!(baseline_payload_copies, count * count);
        let state = RwSignal::new(State::default());
        state.update(|s| s.files.file_history = json_codec::json!({"nodes":nodes}).into());
        let history = Memo::new(move |_| state.with(|s| s.files.file_history.clone()));
        INDEX_COPIES.with(|c| c.set(0));
        let records = index_projection(history);
        assert_eq!(records.with(BTreeMap::len), count);
        assert_eq!(INDEX_COPIES.with(|c| c.get()), count);
        let keys = records.with(|r| r.keys().cloned().collect::<Vec<_>>());
        for i in 0..20 {
            state.update(|s| {
                s.notice = i.to_string();
                s.files.checked_nodes.insert("node-1".into());
            });
            assert_eq!(records.with(BTreeMap::len), count);
        }
        assert_eq!(INDEX_COPIES.with(|c| c.get()), count);
        state.update(|s| s.files.file_history["nodes"][0]["note"] = "updated".into());
        assert_eq!(records.with(|r| r["node-0"]["note"].clone()), "updated");
        assert_eq!(
            records.with(|r| r.keys().cloned().collect::<Vec<_>>()),
            keys
        );
        assert_eq!(INDEX_COPIES.with(|c| c.get()), 2 * count);
    });
}
