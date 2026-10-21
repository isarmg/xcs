use super::*;

fn entry(path: &str, kind: &str) -> Entry {
    Entry {
        entry_id: new_id("entry"),
        path: path.into(),
        name: path.rsplit('/').next().unwrap().into(),
        lookup_key: name_policy::path_key(path),
        kind: kind.into(),
        file_id: if kind == "file" {
            new_id("file")
        } else {
            String::new()
        },
        version_id: if kind == "file" {
            new_id("ver")
        } else {
            String::new()
        },
        size: 0,
    }
}

#[test]
fn relocating_entry_updates_path_fields_without_changing_identity_or_body() {
    let mut file = entry("Old/Document.txt", "file");
    file.size = 123;
    let original = file.clone();
    for path in ["Straße/文档.txt", "STRASSE/文档.txt", "STRASSE/文档.txt"] {
        relocate_entry(&mut file, path.into()).unwrap();
        assert_eq!(file.path, path);
        assert_eq!(file.name, "文档.txt");
        assert_eq!(file.lookup_key, name_policy::path_key(path));
        assert_eq!(file.entry_id, original.entry_id);
        assert_eq!(file.file_id, original.file_id);
        assert_eq!(file.version_id, original.version_id);
        assert_eq!(file.kind, original.kind);
        assert_eq!(file.size, original.size);
    }
    let before = json_codec::to_value(&file).unwrap();
    assert!(relocate_entry(&mut file, "../escape".into()).is_err());
    assert_eq!(json_codec::to_value(&file).unwrap(), before);
}

#[test]
fn rebasing_directory_preserves_identity_and_uses_component_depth() {
    let original = entry("STRASSE/文档.txt", "file");
    let rebased =
        super::super::directory::rebase_entries(std::slice::from_ref(&original), "Straße", "Moved")
            .unwrap();
    assert_eq!(rebased[0].path, "Moved/文档.txt");
    assert_eq!(rebased[0].name, "文档.txt");
    assert_eq!(
        rebased[0].lookup_key,
        name_policy::path_key("Moved/文档.txt")
    );
    assert_eq!(rebased[0].entry_id, original.entry_id);
    assert_eq!(rebased[0].file_id, original.file_id);
    assert_eq!(rebased[0].version_id, original.version_id);
}

#[test]
fn directory_rename_follows_normalized_ancestry_without_slicing_utf8_by_key_length() {
    for source in ["A", "Straße", "Σ"] {
        let alias = match source {
            "A" => "a",
            "Straße" => "STRASSE",
            _ => "ς",
        };
        let mut entries = vec![
            entry(source, "directory"),
            entry(&format!("{alias}/文档"), "directory"),
            entry(&format!("{alias}/文档/报告.txt"), "file"),
            entry(&format!("{source}-other"), "directory"),
        ];
        validate_entries(&mut entries, 10).unwrap();
        let file_id = entries
            .iter()
            .find(|e| e.kind == "file")
            .unwrap()
            .file_id
            .clone();
        apply_namespace(
            &mut entries,
            &NamespaceOperation {
                kind: "rename".into(),
                path: alias.into(),
                destination: Some("Moved".into()),
            },
        )
        .unwrap();
        validate_entries(&mut entries, 10).unwrap();
        assert_eq!(
            find(&entries, "Moved/文档/报告.txt").unwrap().file_id,
            file_id
        );
        assert!(find(&entries, &format!("{source}-other")).is_some());
    }
}
