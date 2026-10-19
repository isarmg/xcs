use super::{graph, model::*, scenarios};
use std::collections::BTreeSet;
fn action(s: &mut Workspace, a: Action) {
    s.apply(a).unwrap();
}
fn create(s: &mut Workspace, n: &str) {
    action(
        s,
        Action::CreateFileBranch {
            path: "A/报告.txt".into(),
            name: n.into(),
        },
    );
}
fn switch(s: &mut Workspace, n: &str) {
    action(
        s,
        Action::SwitchFileBranch {
            path: "A/报告.txt".into(),
            name: n.into(),
        },
    );
}
fn write(s: &mut Workspace, body: &str) {
    action(
        s,
        Action::Write {
            path: "A/报告.txt".into(),
            content: body.into(),
        },
    );
}
#[test]
fn registration_and_switch_reuse_parent_until_first_actual_write_including_empty() {
    let mut s = Workspace::seed();
    let (_, _, parent) = s.file_at("A/报告.txt").unwrap();
    let bodies = s.bodies.len();
    let nodes = s.files.len();
    create(&mut s, "审阅");
    assert_eq!(s.bodies.len(), bodies);
    assert_eq!(s.files.len(), nodes);
    switch(&mut s, "审阅");
    assert_eq!(s.file_at("A/报告.txt").unwrap().2, parent);
    assert_eq!(s.content(parent), "预算 100");
    write(&mut s, "");
    assert_eq!(s.bodies.len(), bodies + 1);
    let new = s.file_at("A/报告.txt").unwrap().2;
    assert_eq!(s.content(new), "");
    assert_eq!(s.files[&new].parents, vec![parent]);
    switch(&mut s, "main");
    assert_eq!(s.file_at("A/报告.txt").unwrap().2, parent);
}
#[test]
fn directory_fork_creates_public_file_branches_only_as_each_file_is_written() {
    let mut s = Workspace::seed();
    let n = s.file_branches.len();
    let originals = s.directories.clone();
    action(
        &mut s,
        Action::CreateDirectoryBranch {
            path: "".into(),
            name: "review".into(),
        },
    );
    action(
        &mut s,
        Action::SwitchDirectoryBranch {
            path: "".into(),
            name: "review".into(),
        },
    );
    assert_eq!(s.file_branches.len(), n);
    write(&mut s, "预算 140");
    assert_eq!(s.file_branches.len(), n + 1);
    let report = s.file_at("A/报告.txt").unwrap().2;
    let readme = s.file_at("A/说明.txt").unwrap().2;
    assert_eq!(s.content(readme), "项目说明");
    action(
        &mut s,
        Action::Write {
            path: "A/说明.txt".into(),
            content: "补充说明".into(),
        },
    );
    assert_eq!(s.file_branches.len(), n + 2);
    assert_eq!(s.file_at("A/报告.txt").unwrap().2, report);
    assert_eq!(s.root_branches(), vec!["main", "review"]);
    for (id, node) in originals {
        assert_eq!(s.directories[&id], node);
    }
    action(
        &mut s,
        Action::SwitchDirectoryBranch {
            path: "".into(),
            name: "main".into(),
        },
    );
    assert_eq!(s.content(s.file_at("A/报告.txt").unwrap().2), "预算 100");
}
#[test]
fn folder_rename_adds_path_nodes_for_every_file_without_copying_bodies() {
    let mut s = Workspace::seed();
    let old = s.clone();
    action(
        &mut s,
        Action::CreateDirectoryBranch {
            path: "".into(),
            name: "archive".into(),
        },
    );
    action(
        &mut s,
        Action::Rename {
            path: "A".into(),
            name: "项目".into(),
        },
    );
    assert_eq!(s.bodies, old.bodies);
    assert_eq!(s.files.len(), old.files.len() + 2);
    let new = s.file_at("项目/报告.txt").unwrap().2;
    assert_eq!(s.files[&new].kind, Kind::Path);
    action(
        &mut s,
        Action::SwitchDirectoryBranch {
            path: "".into(),
            name: "archive".into(),
        },
    );
    assert!(s.entry("A/报告.txt").is_ok());
    assert!(s.entry("项目").is_err());
}
#[test]
fn move_transfers_identity_copy_gets_independent_body_and_delete_keeps_history() {
    let mut s = Workspace::seed();
    let file = s.file_at("A/报告.txt").unwrap().0;
    let count = s.bodies.len();
    action(
        &mut s,
        Action::Move {
            path: "A/报告.txt".into(),
            target: "B".into(),
        },
    );
    assert!(s.entry("A/报告.txt").is_err());
    assert_eq!(s.file_at("B/报告.txt").unwrap().0, file);
    assert_eq!(s.bodies.len(), count);
    action(
        &mut s,
        Action::Copy {
            path: "B/报告.txt".into(),
            target: "A".into(),
        },
    );
    assert_ne!(s.file_at("A/报告.txt").unwrap().0, file);
    assert_eq!(s.bodies.len(), count + 1);
    let before = s.files.len();
    action(
        &mut s,
        Action::Delete {
            path: "A/报告.txt".into(),
        },
    );
    assert_eq!(s.files.len(), before);
    assert!(s.entry("B/报告.txt").is_ok());
}
#[test]
fn failed_actions_are_atomic_and_validate_utf8_bytes() {
    let mut s = Workspace::seed();
    let before = s.clone();
    assert!(
        s.apply(Action::Move {
            path: "A".into(),
            target: "A".into()
        })
        .is_err()
    );
    assert_eq!(s, before);
    assert!(
        s.apply(Action::CreateFileBranch {
            path: "A/报告.txt".into(),
            name: "中".repeat(43)
        })
        .is_err()
    );
    assert_eq!(s, before);
    let node = s.file_at("A/报告.txt").unwrap().2;
    assert!(
        s.apply(Action::Tags {
            path: "A/报告.txt".into(),
            source: node,
            tags: BTreeSet::from(["中".repeat(43)])
        })
        .is_err()
    );
    assert_eq!(s, before);
}
fn merge_ready() -> (Workspace, Id, Id) {
    let mut s = Workspace::seed();
    create(&mut s, "审阅");
    write(&mut s, "预算 120");
    let a = s.file_at("A/报告.txt").unwrap().2;
    switch(&mut s, "审阅");
    write(&mut s, "预算 110");
    let b = s.file_at("A/报告.txt").unwrap().2;
    (s, a, b)
}
#[test]
fn merge_reuses_input_head_preserves_alias_history_and_updates_directory_refs() {
    let (mut s, a, b) = merge_ready();
    let before = s.files.len();
    let bodies = s.bodies.len();
    let old = s.directories.clone();
    action(
        &mut s,
        Action::Merge {
            path: "A/报告.txt".into(),
            branches: vec!["main".into(), "审阅".into()],
            chosen: b,
            owner: "main".into(),
        },
    );
    assert_eq!(s.files.len(), before);
    assert_eq!(s.bodies.len(), bodies);
    assert!(s.files[&b].parents.contains(&a));
    assert_eq!(s.file_at("A/报告.txt").unwrap().1, "main");
    let (file, _, _) = s.file_at("A/报告.txt").unwrap();
    assert_eq!(
        s.file_branches[&(file, "审阅".into())].alias.as_deref(),
        Some("main")
    );
    switch(&mut s, "审阅");
    assert!(
        s.apply(Action::Write {
            path: "A/报告.txt".into(),
            content: "不允许".into()
        })
        .is_err()
    );
    for (id, node) in old {
        assert_eq!(s.directories[&id], node);
    }
}
#[test]
fn arbitrary_history_merge_reuses_body_and_records_all_parents() {
    let (mut s, a, b) = merge_ready();
    let source = s.files[&a].parents[0];
    let body = s.files[&source].body;
    let before = s.bodies.len();
    action(
        &mut s,
        Action::Merge {
            path: "A/报告.txt".into(),
            branches: vec!["main".into(), "审阅".into()],
            chosen: source,
            owner: "审阅".into(),
        },
    );
    let new = s.merges[0].result;
    assert_eq!(s.files[&new].body, body);
    assert_eq!(s.bodies.len(), before);
    assert_eq!(
        s.files[&new]
            .parents
            .iter()
            .copied()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([a, b, source])
    );
}
#[test]
fn ancestor_head_reuse_skips_cycle_and_preserves_merge_provenance() {
    let mut s = Workspace::seed();
    create(&mut s, "旧版");
    let old = s.file_at("A/报告.txt").unwrap().2;
    write(&mut s, "新版");
    let new = s.file_at("A/报告.txt").unwrap().2;
    action(
        &mut s,
        Action::Merge {
            path: "A/报告.txt".into(),
            branches: vec!["main".into(), "旧版".into()],
            chosen: old,
            owner: "main".into(),
        },
    );
    assert!(!s.files[&old].parents.contains(&new));
    assert!(s.merges[0].heads.contains(&new));
    s.validate().unwrap();
}
#[test]
fn tags_inherit_and_editing_history_does_not_rewind_newer_content() {
    let mut s = Workspace::seed();
    let source = s.file_at("A/报告.txt").unwrap().2;
    action(
        &mut s,
        Action::Tags {
            path: "A/报告.txt".into(),
            source,
            tags: BTreeSet::from(["已审阅".into()]),
        },
    );
    write(&mut s, "预算 130");
    let current = s.file_at("A/报告.txt").unwrap().2;
    assert!(s.files[&current].tags.contains("已审阅"));
    let bodies = s.bodies.len();
    action(
        &mut s,
        Action::Tags {
            path: "A/报告.txt".into(),
            source,
            tags: BTreeSet::from(["归档".into()]),
        },
    );
    assert_eq!(s.file_at("A/报告.txt").unwrap().2, current);
    assert_eq!(s.content(current), "预算 130");
    assert_eq!(s.bodies.len(), bodies);
    assert!(s.files[&source].tags.contains("草稿"));
}
#[test]
fn every_guided_step_uses_valid_shared_commands_and_arrows_always_point_up() {
    for (index, scene) in scenarios::all().iter().enumerate() {
        for cursor in 0..=scene.steps.len() {
            let (s, _, node, path) = scenarios::replay(index, cursor)
                .unwrap_or_else(|e| panic!("{} step {cursor}: {e}", scene.id));
            s.validate().unwrap();
            if let Ok((file, _, _)) = s.file_at(&path) {
                let graph = graph::file(&s, file, node, None);
                for (parent, child) in &graph.edges {
                    assert!(
                        graph.positions[parent].1 > graph.positions[child].1 + 108,
                        "{} step {cursor}",
                        scene.id
                    );
                }
            }
            let ctx = s.context("").unwrap();
            let graph = graph::directory(&s, 0, ctx.node, None);
            for (parent, child) in &graph.edges {
                assert!(graph.positions[parent].1 > graph.positions[child].1 + 108);
            }
        }
    }
}
#[test]
fn graph_escapes_user_content() {
    let mut s = Workspace::seed();
    write(&mut s, "<img src=x onerror=alert(1)>");
    let (file, _, node) = s.file_at("A/报告.txt").unwrap();
    let html = graph::file(&s, file, node, None).html;
    assert!(!html.contains("<img"));
    assert!(html.contains("&lt;img"));
}

#[test]
fn alias_reads_owner_head_while_retaining_its_original_history() {
    let (mut s, a, b) = merge_ready();
    let file = s.file_at("A/报告.txt").unwrap().0;
    action(
        &mut s,
        Action::Merge {
            path: "A/报告.txt".into(),
            branches: vec!["main".into(), "审阅".into()],
            chosen: a,
            owner: "main".into(),
        },
    );
    assert_eq!(s.file_branches[&(file, "审阅".into())].head, b);
    switch(&mut s, "审阅");
    assert_eq!(s.file_at("A/报告.txt").unwrap().2, a);
    switch(&mut s, "main");
    write(&mut s, "合并后再写");
    let current = s.file_at("A/报告.txt").unwrap().2;
    switch(&mut s, "审阅");
    assert_eq!(s.file_at("A/报告.txt").unwrap().2, current);
}

#[test]
fn merge_updates_inherited_input_references_and_leaves_unrelated_branches_frozen() {
    let (mut s, _, b) = merge_ready();
    create(&mut s, "独立");
    switch(&mut s, "独立");
    action(
        &mut s,
        Action::CreateDirectoryBranch {
            path: "".into(),
            name: "keep".into(),
        },
    );
    let keep = s.directory_branches[&(0, "keep".into())].head;
    switch(&mut s, "审阅");
    action(
        &mut s,
        Action::CreateDirectoryBranch {
            path: "".into(),
            name: "dependent".into(),
        },
    );
    let dependent = s.directory_branches[&(0, "dependent".into())].head;
    action(
        &mut s,
        Action::Merge {
            path: "A/报告.txt".into(),
            branches: vec!["main".into(), "审阅".into()],
            chosen: b,
            owner: "main".into(),
        },
    );
    assert_eq!(s.directory_branches[&(0, "keep".into())].head, keep);
    assert_ne!(
        s.directory_branches[&(0, "dependent".into())].head,
        dependent
    );
    action(
        &mut s,
        Action::SwitchDirectoryBranch {
            path: "".into(),
            name: "keep".into(),
        },
    );
    assert_eq!(s.file_at("A/报告.txt").unwrap().1, "独立");
}

#[test]
fn note_edits_enforce_byte_limit_without_creating_a_node() {
    let mut s = Workspace::seed();
    let source = s.file_at("A/报告.txt").unwrap().2;
    let before = s.clone();
    assert!(
        s.apply(Action::Note {
            path: "A/报告.txt".into(),
            source,
            text: "中".repeat(43)
        })
        .is_err()
    );
    assert_eq!(s, before);
    action(
        &mut s,
        Action::Note {
            path: "A/报告.txt".into(),
            source,
            text: "确认".into(),
        },
    );
    assert_eq!(s.files.len(), before.files.len());
    assert_eq!(s.bodies, before.bodies);
    action(
        &mut s,
        Action::Note {
            path: "A/报告.txt".into(),
            source,
            text: String::new(),
        },
    );
    assert!(s.files[&source].note.is_empty());
}
