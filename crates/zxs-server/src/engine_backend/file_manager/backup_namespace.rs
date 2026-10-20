//! Read-only logical backup paths over authorized, retained FULL containers.
//! A backup's user prefix is independent from its live primary namespace.
use super::*;
use zxs_core::{Entry, FileVersion};

fn available_on(engine: &Engine, slot: &str, version: &str) -> bool {
    engine
        .state()
        .content_states
        .get(version)
        .map(String::as_str)
        .unwrap_or("AVAILABLE")
        == "AVAILABLE"
        && engine
            .state()
            .placements
            .get(version)
            .is_some_and(|placement| {
                placement
                    .route_slot_ids
                    .iter()
                    .any(|candidate| candidate == slot)
            })
}
fn conflict() -> ApiError {
    ApiError::new(
        409,
        "BACKUP_PATH_CONFLICT",
        "authorized backup containers have conflicting logical paths; choose distinct backup prefixes",
    )
}
fn retain_node(
    nodes: &mut BTreeMap<String, Value>,
    value: Value,
    maximum: usize,
) -> Result<(), ApiError> {
    let key = zxs_naming::path_key(value["path"].as_str().unwrap_or_default());
    if let Some(existing) = nodes.get(&key) {
        if existing["kind"] == "directory" && value["kind"] == "directory" {
            return Ok(());
        }
        if existing["kind"] == "file"
            && value["kind"] == "file"
            && existing["file_id"] == value["file_id"]
            && existing["repository_id"] == value["repository_id"]
        {
            return Ok(());
        }
        return Err(conflict());
    }
    if nodes.len() >= maximum {
        return Err(ApiError::new(
            429,
            "RESOURCE_LIMIT_EXCEEDED",
            "backup namespace exceeds the bounded entry limit",
        ));
    }
    nodes.insert(key, value);
    Ok(())
}
fn parents(
    nodes: &mut BTreeMap<String, Value>,
    slot: &str,
    path: &str,
    maximum: usize,
) -> Result<(), ApiError> {
    let components = path.split('/').collect::<Vec<_>>();
    for length in 1..components.len() {
        let display = components[..length].join("/");
        let id = format!(
            "backupdir_{}",
            blake3::hash(format!("{slot}/{}", zxs_naming::path_key(&display)).as_bytes()).to_hex()
        );
        let mut entry = directory(components[length - 1], &display, &id);
        entry["read_only"] = true.into();
        entry["backup"] = true.into();
        retain_node(nodes, entry, maximum)?;
    }
    Ok(())
}
// Default namespace for current files without an explicit per-slot backup path.
fn default_backup_path(
    repo: &Repository,
    entry: Option<&Entry>,
    version: &FileVersion,
    engine: &Engine,
) -> String {
    let relative = entry
        .map(|entry| entry.path.as_str())
        .or_else(|| {
            engine
                .state()
                .file_locations
                .get(&version.file_id)
                .map(String::as_str)
        })
        .unwrap_or(&version.creation_name);
    if repo.file_manager_root {
        format!("{}/{relative}", repo.primary_slot_id)
    } else {
        format!("{}/{}/{relative}", repo.primary_slot_id, repo.name)
    }
}
fn file_entry(
    engine: &Engine,
    repo: &Repository,
    current: Option<&Entry>,
    version: &FileVersion,
    display: &str,
) -> Value {
    let matches_current = current.is_some_and(|entry| entry.version_id == version.version_id);
    let mut entry = if let Some(current) = current {
        let mut frozen = current.clone();
        frozen.version_id = version.version_id.clone();
        frozen.size = version.logical_size;
        listed_entry(engine, repo, &frozen, "")
    } else {
        json!({"entry_id":version.file_id,"kind":"file","repository_id":repo.repository_id,"repository_path":null,"container":true,"file_id":version.file_id,"selected_branch_id":null})
    };
    if !matches_current {
        entry["node_id"] = version.version_id.clone().into();
        entry.as_object_mut().expect("file wire object").extend(
            content_wire(engine, Some(&version.version_id), None)
                .as_object()
                .expect("content wire object")
                .clone(),
        );
    }
    entry["path"] = display.into();
    entry["name"] = display.rsplit('/').next().unwrap_or(display).into();
    entry["size"] = version.logical_size.to_string().into();
    entry["version_id"] = version.version_id.clone().into();
    entry["note"] = engine
        .state()
        .file_notes
        .get(&version.file_id)
        .cloned()
        .unwrap_or_default()
        .into();
    entry["read_only"] = true.into();
    entry["backup"] = true.into();
    entry["historical_only"] = current.is_none().into();
    entry
}

pub(super) fn entries(
    engine: &Engine,
    identity: &Identity,
    slot: &str,
    path: &str,
) -> Result<Value, ApiError> {
    let repositories = engine
        .state()
        .repositories
        .values()
        .filter(|repo| authorize(identity, repo, false, false).is_ok())
        .map(|repo| (repo.history_tree_id.as_str(), repo))
        .collect::<BTreeMap<_, _>>();
    // Keep one file container, including retained files absent from the live
    // namespace. A visible canonical alias chooses its body when stored here;
    // otherwise the newest AVAILABLE body on this slot is displayed.
    let mut latest = BTreeMap::<&str, (&Repository, &FileVersion)>::new();
    for version in engine
        .state()
        .versions
        .values()
        .filter(|version| available_on(engine, slot, &version.version_id))
    {
        let Some(repo) = repositories.get(version.history_tree_id.as_str()) else {
            continue;
        };
        match latest.get(version.file_id.as_str()) {
            Some((_, previous))
                if (previous.version_sequence, &previous.version_id)
                    >= (version.version_sequence, &version.version_id) => {}
            _ => {
                latest.insert(&version.file_id, (repo, version));
            }
        }
    }
    let mut canonical = BTreeMap::<String, Entry>::new();
    for repo in repositories.values() {
        for entry in engine
            .list_tree(&repo.main_branch_id)
            .map_err(error)?
            .into_iter()
            .filter(|entry| entry.kind == "file" && latest.contains_key(entry.file_id.as_str()))
        {
            let owner = engine
                .state()
                .file_locations
                .get(&entry.file_id)
                .is_some_and(|owner| zxs_naming::path_key(owner) == entry.lookup_key);
            if owner || !canonical.contains_key(&entry.file_id) {
                canonical.insert(entry.file_id.clone(), entry);
            }
        }
    }
    let maximum = engine.config().max_entries;
    let mut nodes = BTreeMap::new();
    for (file, (repo, newest)) in latest {
        let current = canonical.get(file);
        let version = current
            .filter(|entry| available_on(engine, slot, &entry.version_id))
            .and_then(|entry| engine.state().versions.get(&entry.version_id))
            .unwrap_or(newest);
        let display = engine
            .state()
            .file_backup_paths
            .get(file)
            .and_then(|paths| paths.get(slot))
            .cloned()
            .unwrap_or_else(|| default_backup_path(repo, current, version, engine));
        // State values are validated again at this filesystem-facing boundary.
        zxs_naming::validate_path(&display).map_err(ApiError::invalid)?;
        parents(&mut nodes, slot, &display, maximum)?;
        retain_node(
            &mut nodes,
            file_entry(engine, repo, current, version, &display),
            maximum,
        )?;
    }
    let key = zxs_naming::path_key(path);
    if !path.is_empty()
        && !nodes
            .get(&key)
            .is_some_and(|entry| entry["kind"] == "directory")
    {
        return Err(ApiError::new(
            404,
            "DIRECTORY_NOT_FOUND",
            "backup directory not found",
        ));
    }
    let prefix = if key.is_empty() {
        String::new()
    } else {
        format!("{key}/")
    };
    let file_count = nodes
        .iter()
        .filter(|(key, entry)| key.starts_with(&prefix) && entry["kind"] == "file")
        .count();
    let mut values = nodes
        .into_iter()
        .filter(|(key, _)| key.starts_with(&prefix) && !key[prefix.len()..].contains('/'))
        .map(|(_, entry)| entry)
        .collect::<Vec<_>>();
    values.sort_by(|left, right| {
        let rank = |entry: &Value| if entry["kind"] == "directory" { 0 } else { 1 };
        rank(left)
            .cmp(&rank(right))
            .then_with(|| {
                zxs_naming::lookup_key(left["name"].as_str().unwrap_or_default()).cmp(
                    &zxs_naming::lookup_key(right["name"].as_str().unwrap_or_default()),
                )
            })
            .then_with(|| left["name"].as_str().cmp(&right["name"].as_str()))
    });
    Ok(
        json!({"slot_id":slot,"path":path,"backup":true,"writable":false,"entries":values,"context":null,"directory_file_count":file_count.to_string(),"directory_branches":[],"directory_branches_supported":false,"default_branch_scope":false}),
    )
}
