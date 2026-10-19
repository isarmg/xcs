//! Namespace changes preserve stable identity and path invariants.
use super::*;
use std::collections::BTreeMap;

pub(crate) fn sorted(entries: &mut [Entry]) {
    entries.sort_by(|a, b| a.lookup_key.as_bytes().cmp(b.lookup_key.as_bytes()));
}
pub(crate) fn find<'a>(entries: &'a [Entry], path: &str) -> Option<&'a Entry> {
    let key = xcs_naming::path_key(path);
    entries.iter().find(|e| e.lookup_key == key)
}
pub(crate) fn validate_entries(entries: &mut Vec<Entry>, maximum: usize) -> Result<()> {
    if entries.len() > maximum {
        return Err(Error::new(
            "RESOURCE_LIMIT",
            "directory entry budget exceeded",
        ));
    }

    for e in entries.iter_mut() {
        xcs_naming::validate_path(&e.path)?;
        e.name = e.path.rsplit('/').next().unwrap_or("").into();
        e.lookup_key = xcs_naming::path_key(&e.path);
    }
    // Build once after normalization; parent validation must not scan the
    // complete directory for every entry (up to max_entries per commit).
    let mut kinds = BTreeMap::new();
    for entry in entries.iter() {
        if kinds
            .insert(entry.lookup_key.as_str(), entry.kind.as_str())
            .is_some()
        {
            return Err(Error::new(
                "NAME_COLLISION",
                "two entries have the same normalized lookup key",
            ));
        }
    }
    for entry in entries.iter() {
        if let Some((parent, _)) = entry.lookup_key.rsplit_once('/') {
            if kinds.get(parent).copied() != Some("directory") {
                return Err(Error::new(
                    "PARENT_NOT_DIRECTORY",
                    "file parent is missing or is not a directory",
                ));
            }
        }
    }
    sorted(entries);
    Ok(())
}
pub(crate) fn ensure_parents(entries: &mut Vec<Entry>, path: &str) -> Result<()> {
    let parts = path.split('/').collect::<Vec<_>>();
    let mut p = String::new();
    for c in parts.iter().take(parts.len().saturating_sub(1)) {
        if !p.is_empty() {
            p.push('/');
        }
        p.push_str(c);
        if let Some(existing) = find(entries, &p) {
            if existing.kind != "directory" {
                return Err(Error::new(
                    "PARENT_NOT_DIRECTORY",
                    "parent is not a directory",
                ));
            }
        } else {
            entries.push(Entry {
                entry_id: new_id("entry"),
                path: p.clone(),
                name: (*c).into(),
                lookup_key: xcs_naming::path_key(&p),
                kind: "directory".into(),
                file_id: String::new(),
                version_id: String::new(),
                size: 0,
            });
        }
    }
    Ok(())
}
pub(crate) fn apply_workspace_namespace(
    entries: &mut Vec<Entry>,
    base: &[Entry],
    operations: &[NamespaceOperation],
) -> Result<()> {
    let mut explicit_directories = std::collections::BTreeSet::new();
    for op in operations {
        // Uploaded children may have materialized their missing parents. An
        // explicit mkdir for that new parent belongs to the same workspace.
        let key = xcs_naming::path_key(&op.path);
        if op.kind == "mkdir"
            && find(base, &op.path).is_none()
            && find(entries, &op.path).is_some_and(|e| e.kind == "directory")
            && explicit_directories.insert(key)
        {
            continue;
        }
        apply_namespace(entries, op)?;
        if op.kind == "mkdir" {
            explicit_directories.insert(xcs_naming::path_key(&op.path));
        }
    }
    Ok(())
}
pub(crate) fn validate_workspace_dependencies(workspace: &Workspace) -> Result<()> {
    let overlaps = |left: &str, right: &str| {
        let left = xcs_naming::path_key(left);
        let right = xcs_naming::path_key(right);
        left == right || is_descendant(&left, &right) || is_descendant(&right, &left)
    };
    for operation in &workspace.namespace_operations {
        if operation.kind == "mkdir" {
            continue;
        }
        if workspace.files.values().any(|file| {
            overlaps(&file.path, &operation.path)
                || operation
                    .destination
                    .as_ref()
                    .is_some_and(|path| overlaps(&file.path, path))
        }) {
            return Err(Error::new(
                "WORKSPACE_DEPENDENCY_BUSY",
                "overlapping upload and delete/rename require separate explicit saves; staged bytes are preserved",
            ));
        }
    }
    Ok(())
}

/// Compare already-normalized keys at a complete path-component boundary.
pub(crate) fn is_descendant(key: &str, ancestor: &str) -> bool {
    key.strip_prefix(ancestor)
        .is_some_and(|suffix| suffix.starts_with('/'))
}

pub(crate) fn apply_namespace(entries: &mut Vec<Entry>, op: &NamespaceOperation) -> Result<()> {
    let key = xcs_naming::path_key(&op.path);
    match op.kind.as_str() {
        "mkdir" => {
            if find(entries, &op.path).is_some() {
                return Err(Error::new("NAME_COLLISION", "directory target exists"));
            }
            ensure_parents(entries, &op.path)?;
            entries.push(Entry {
                entry_id: new_id("entry"),
                path: op.path.clone(),
                name: op.path.rsplit('/').next().unwrap_or("").into(),
                lookup_key: key,
                kind: "directory".into(),
                file_id: String::new(),
                version_id: String::new(),
                size: 0,
            });
        }
        "delete" | "delete_tree" => {
            if find(entries, &op.path).is_none() {
                return Err(Error::new("ENTRY_NOT_FOUND", "delete target missing"));
            }
            if op.kind == "delete" && entries.iter().any(|e| is_descendant(&e.lookup_key, &key)) {
                return Err(Error::new(
                    "DIRECTORY_NOT_EMPTY",
                    "directory contains children",
                ));
            }
            entries.retain(|e| {
                e.lookup_key != key
                    && !(op.kind == "delete_tree" && is_descendant(&e.lookup_key, &key))
            });
        }
        "rename" => {
            let target = op
                .destination
                .as_ref()
                .ok_or_else(|| Error::new("INVALID_REQUEST", "rename needs destination"))?;
            let to_key = xcs_naming::path_key(target);
            let original = find(entries, &op.path)
                .cloned()
                .ok_or_else(|| Error::new("ENTRY_NOT_FOUND", "rename source missing"))?;
            if is_descendant(&to_key, &key) {
                return Err(Error::new(
                    "INVALID_MOVE",
                    "cannot move a directory into itself",
                ));
            }
            if key != to_key && find(entries, target).is_some() {
                return Err(Error::new("NAME_COLLISION", "rename destination exists"));
            }
            ensure_parents(entries, target)?;
            // Display spellings can differ in both case and UTF-8 length from
            // the normalized key (Straße / STRASSE). Select by key, then strip
            // display components rather than slicing by a different spelling.
            let depth = original.path.split('/').count();
            for entry in entries.iter_mut() {
                if entry.lookup_key == key {
                    entry.path = target.clone();
                } else if is_descendant(&entry.lookup_key, &key) {
                    let suffix = entry
                        .path
                        .splitn(depth + 1, '/')
                        .nth(depth)
                        .ok_or_else(|| {
                            Error::new("STATE_INVALID", "entry path and lookup depth differ")
                        })?;
                    entry.path = format!("{target}/{suffix}");
                } else {
                    continue;
                }
                entry.lookup_key = xcs_naming::path_key(&entry.path);
            }
        }
        _ => {
            return Err(Error::new(
                "OPERATION_UNSUPPORTED",
                "namespace operation not supported",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, kind: &str) -> Entry {
        Entry {
            entry_id: new_id("entry"),
            path: path.into(),
            name: path.rsplit('/').next().unwrap().into(),
            lookup_key: xcs_naming::path_key(path),
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
}
