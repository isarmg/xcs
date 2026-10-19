//! Ordinary version files in one renameable directory per stable file identity.
use crate::{disk::*, *};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use xcs_platform::fs_safety::linux::{MountPolicy, OpenAt2Root};

fn invalid(message: &str) -> Error {
    Error::new("CONTAINER_LAYOUT_INVALID", message)
}

fn sequence(name: &str) -> Result<u64> {
    xcs_naming::validate_name(name)?;
    let mut parts = name.splitn(3, '_');
    let number = parts
        .next()
        .and_then(|part| part.strip_prefix('v'))
        .ok_or_else(|| invalid("version storage name has no sequence"))?;
    let stamp = parts
        .next()
        .ok_or_else(|| invalid("version storage name has no timestamp"))?;
    let suffix = parts
        .next()
        .ok_or_else(|| invalid("version storage name has no digest"))?;
    let digest = suffix.split('.').next().unwrap_or("");
    if number.len() < 6
        || !number.bytes().all(|byte| byte.is_ascii_digit())
        || stamp.len() != 16
        || stamp.as_bytes()[8] != b'T'
        || stamp.as_bytes()[15] != b'Z'
        || !stamp
            .bytes()
            .enumerate()
            .all(|(index, byte)| index == 8 || index == 15 || byte.is_ascii_digit())
        || digest.len() != 8
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid(
            "version storage name is not a numbered UTC checkpoint",
        ));
    }
    let value = number
        .parse::<u64>()
        .map_err(|_| invalid("version sequence exceeds the storage format"))?;
    if value == 0 {
        return Err(invalid("version storage sequence starts at one"));
    }
    Ok(value)
}

fn validate_version(version: &FileVersion) -> Result<()> {
    validate_id(&version.file_id, "file")?;
    validate_id(&version.history_tree_id, "history")?;
    validate_id(&version.version_id, "ver")?;
    xcs_naming::validate_name(&version.creation_name)?;
    if version.content_hash.len() != 64
        || !version
            .content_hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid("version content digest is invalid"));
    }
    if let Some(name) = &version.storage_name {
        sequence(name)?;
        let digest = name
            .splitn(3, '_')
            .nth(2)
            .and_then(|value| value.split('.').next())
            .unwrap_or("");
        if digest != &version.content_hash[..8] {
            return Err(invalid(
                "version storage name does not identify its content digest",
            ));
        }
    }
    Ok(())
}

/// Per-file storage sequence; the existing global audit sequence is unchanged.
pub(crate) fn version_identity(
    state: &State,
    file_id: &str,
    path: &str,
    hash: &str,
) -> Result<(String, String)> {
    validate_id(file_id, "file")?;
    xcs_naming::validate_path(path)?;
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid("content digest is invalid"));
    }
    let mut maximum = 0;
    let mut count = 0u64;
    for version in state
        .versions
        .values()
        .filter(|version| version.file_id == file_id)
    {
        count = count
            .checked_add(1)
            .ok_or_else(|| Error::new("RESOURCE_LIMIT", "file version count exhausted"))?;
        if let Some(name) = &version.storage_name {
            maximum = maximum.max(sequence(name)?);
        }
    }
    let number = maximum
        .max(count)
        .checked_add(1)
        .ok_or_else(|| Error::new("RESOURCE_LIMIT", "file version sequence exhausted"))?;
    let now = OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .map_err(|error| invalid(&error.to_string()))?;
    let timestamp = now
        .format(&Rfc3339)
        .map_err(|error| invalid(&error.to_string()))?;
    let format =
        time::format_description::parse_borrowed::<3>("[year][month][day]T[hour][minute][second]Z")
            .map_err(|error| invalid(&error.to_string()))?;
    let stamp = now
        .format(&format)
        .map_err(|error| invalid(&error.to_string()))?;
    let current_name = state
        .file_names
        .get(file_id)
        .map(String::as_str)
        .unwrap_or(path);
    xcs_naming::validate_path(current_name)?;
    let extension = Path::new(current_name)
        .extension()
        .and_then(|name| name.to_str())
        .map(|name| format!(".{name}"))
        .unwrap_or_default();
    let name = format!("v{number:06}_{stamp}_{}{extension}", &hash[..8]);
    xcs_naming::validate_name(&name)?;
    Ok((name, timestamp))
}

fn directory_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            existing_private_dir(path)?;
            Ok(true)
        }
        Ok(_) => Err(invalid(
            "container path must be a private directory without symbolic links",
        )),
    }
}

fn history_root(directory: &Path) -> Result<&Path> {
    directory
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| invalid("version metadata has no history or package root"))
}

/// Return the unique visible container, rejecting ambiguity instead of choosing
/// an arbitrary directory. A missing or empty container remains a missing file.
const NAMESPACE_MARKER: &str = ".xcs-namespace.json";
const OWNER_MARKER: &str = ".xcs-container-owner.json";

fn marker_matches(path: &Path, file_id: &str) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            return Ok(false);
        }
        Ok(_) => {}
    }
    if !file_exists(path)? || read_json::<String>(path)? != file_id {
        return Err(invalid(
            "container location marker differs from its stable file identity",
        ));
    }
    Ok(true)
}

fn named_containers(
    root: &Path,
    file_id: &str,
    depth: usize,
    selected: &mut Option<PathBuf>,
) -> Result<()> {
    if depth > 33 {
        return Err(invalid("container namespace exceeds the path depth limit"));
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if depth == 0 && entry.file_name() == NAMESPACE_MARKER {
            if !marker_matches(&entry.path(), file_id)? {
                return Err(invalid("container namespace marker is missing"));
            }
            continue;
        }
        if !entry.file_type()?.is_dir() || entry.file_type()?.is_symlink() {
            return Err(invalid(
                "container namespace contains a non-directory or symbolic link",
            ));
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| invalid("container directory name is not UTF-8"))?;
        xcs_naming::validate_name(&name)?;
        let path = entry.path();
        existing_private_dir(&path)?;
        if marker_matches(&path.join(OWNER_MARKER), file_id)? {
            if !directory_exists(&path.join("versions"))? {
                return Err(invalid("owned container has no version directory"));
            }
            if selected.replace(path).is_some() {
                return Err(Error::new(
                    "CONTAINER_AMBIGUOUS",
                    "stable file identity has more than one named container",
                ));
            }
        } else {
            named_containers(&path, file_id, depth + 1, selected)?;
        }
    }
    Ok(())
}

fn container(history: &Path, file_id: &str) -> Result<Option<PathBuf>> {
    validate_id(file_id, "file")?;
    if !directory_exists(history)? {
        return Ok(None);
    }
    let root = history.join("file-containers");
    if !directory_exists(&root)? {
        return Ok(None);
    }
    let root = root.join(file_id);
    if !directory_exists(&root)? {
        return Ok(None);
    }
    let mut selected = None;
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() || entry.file_type()?.is_symlink() {
            return Err(invalid(
                "stable file directory contains a non-directory or symbolic link",
            ));
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| invalid("container directory name is not UTF-8"))?;
        xcs_naming::validate_name(&name)?;
        existing_private_dir(&entry.path())?;
        if name == "namespace" && marker_matches(&entry.path().join(NAMESPACE_MARKER), file_id)? {
            // The stable identity is the lookup key; logical aliases never
            // choose the physical location. Empty source parents left by an
            // interrupted move are harmless, but two payload owners fail closed.
            named_containers(&entry.path(), file_id, 0, &mut selected)?;
            continue;
        }
        if selected.replace(entry.path()).is_some() {
            return Err(Error::new(
                "CONTAINER_AMBIGUOUS",
                "stable file identity has more than one named container",
            ));
        }
    }
    Ok(selected)
}

fn file_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
        Ok(metadata)
            if metadata.is_file()
                && !metadata.file_type().is_symlink()
                && metadata.nlink() == 1 =>
        {
            open_regular(path)?;
            Ok(true)
        }
        Ok(_) => Err(invalid("version body must be one ordinary unlinked file")),
    }
}

/// Missing bodies return their expected path so repair can retain its existing
/// MISSING classification. Only malformed or ambiguous layout fails closed.
pub(crate) fn content_path(directory: &Path, version: &FileVersion) -> Result<PathBuf> {
    validate_version(version)?;
    let legacy = directory.join("content").join(&version.creation_name);
    let Some(name) = &version.storage_name else {
        if let Some(named) = container(history_root(directory)?, &version.file_id)? {
            let relocated = named
                .join("legacy")
                .join(&version.version_id)
                .join(&version.creation_name);
            if file_exists(&relocated)?
                || (directory_exists(relocated.parent().expect("legacy version parent"))?
                    && !file_exists(&legacy)?)
            {
                return Ok(relocated);
            }
        }
        return Ok(legacy);
    };
    let history = history_root(directory)?;
    let named = container(history, &version.file_id)?.unwrap_or_else(|| {
        history
            .join("file-containers")
            .join(&version.file_id)
            .join(&version.creation_name)
    });
    let versions = named.join("versions");
    let target = versions.join(name);
    if directory_exists(&versions)? && file_exists(&target)? {
        return Ok(target);
    }
    // Transitional preparation and packages produced by earlier builds can
    // still hold the verified body beside its immutable metadata.
    if file_exists(&legacy)? {
        return Ok(legacy);
    }
    Ok(target)
}

fn held_directory(path: &Path) -> Result<OpenAt2Root> {
    let private = existing_private_dir(path)?;
    OpenAt2Root::open(private.path(), MountPolicy::SameMount)
        .map_err(|error| Error::new("UNSAFE_PATH", error.to_string()))
}

fn verify_body(path: &Path, version: &FileVersion) -> Result<()> {
    if !file_exists(path)? {
        return Err(Error::new("CONTENT_MISSING", "version body is missing"));
    }
    let mut file = open_regular(path)?;
    if file.metadata()?.len() != version.logical_size {
        return Err(Error::new("CONTENT_CORRUPT", "version body length differs"));
    }
    let mut digest = blake3::Hasher::new();
    let mut size = 0u64;
    let mut buffer = [0u8; 65_536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        size = size.checked_add(count as u64).ok_or_else(|| {
            Error::new(
                "CONTENT_CORRUPT",
                "version body grew beyond its declared size",
            )
        })?;
        if size > version.logical_size {
            return Err(Error::new(
                "CONTENT_CORRUPT",
                "version body grew beyond its declared size",
            ));
        }
        digest.update(&buffer[..count]);
    }
    if size != version.logical_size || digest.finalize().to_hex().as_str() != version.content_hash {
        return Err(Error::new(
            "CONTENT_CORRUPT",
            "version body content differs",
        ));
    }
    file.sync_all()?;
    Ok(())
}

/// Move a verified prepared body on its existing filesystem. Never hardlink,
/// reflink or duplicate the payload to represent a version or rename.
pub(crate) fn install_version_content(directory: &Path, version: &FileVersion) -> Result<PathBuf> {
    validate_version(version)?;
    let source = directory.join("content").join(&version.creation_name);
    let Some(name) = &version.storage_name else {
        let resolved = content_path(directory, version)?;
        verify_body(&resolved, version)?;
        return Ok(resolved);
    };
    let history = history_root(directory)?;
    existing_private_dir(history)?;
    let stable = history.join("file-containers").join(&version.file_id);
    private_dir(&stable)?;
    let named = match container(history, &version.file_id)? {
        Some(named) => named,
        None => {
            let named = stable.join(&version.creation_name);
            private_dir(&named)?;
            sync_dir(&stable)?;
            named
        }
    };
    let versions = named.join("versions");
    private_dir(&versions)?;
    let target = versions.join(name);
    if file_exists(&target)? {
        verify_body(&target, version)?;
        if file_exists(&source)? {
            verify_body(&source, version)?;
            let parent = held_directory(source.parent().expect("source parent"))?;
            rustix::fs::unlinkat(
                parent.directory(),
                source.file_name().expect("source name"),
                rustix::fs::AtFlags::empty(),
            )
            .map_err(std::io::Error::from)?;
            parent.directory().sync_all()?;
        }
        // Retrying after an interrupted directory sync must complete durability
        // even when the exclusive move already made the target visible.
        if directory_exists(source.parent().expect("source parent"))? {
            sync_dir(source.parent().expect("source parent"))?;
        }
        sync_dir(&versions)?;
        sync_dir(&named)?;
        sync_dir(&stable)?;
        return Ok(target);
    }
    verify_body(&source, version)?;
    let from = held_directory(source.parent().expect("source parent"))?;
    let to = held_directory(&versions)?;
    rustix::fs::renameat_with(
        from.directory(),
        source.file_name().expect("source name"),
        to.directory(),
        name.as_str(),
        rustix::fs::RenameFlags::NOREPLACE,
    )
    .map_err(std::io::Error::from)?;
    from.directory().sync_all()?;
    to.directory().sync_all()?;
    sync_dir(&named)?;
    Ok(target)
}

/// Initialize missing physical identities from the published namespace. Saved
/// branch aliases never retarget an existing container; actual namespace edits
/// update these fields explicitly, and deletion retains the last owner.
pub(crate) fn capture_names(state: &mut State) {
    let names = state
        .repositories
        .values()
        .filter_map(|repository| state.branches.get(&repository.main_branch_id))
        .filter_map(|branch| state.commits.get(&branch.head_commit_id))
        .flat_map(|commit| &commit.entries)
        .filter(|entry| entry.kind == "file")
        .map(|entry| {
            (
                entry.file_id.clone(),
                entry.name.clone(),
                entry.path.clone(),
            )
        })
        .collect::<Vec<_>>();
    for (file, name, path) in names {
        state.file_names.entry(file.clone()).or_insert(name);
        state.file_locations.entry(file).or_insert(path);
    }
}

fn changed_files(previous: &State, next: &State) -> BTreeSet<String> {
    let mut files = BTreeSet::new();
    for (id, node) in previous
        .file_metadata_nodes
        .iter()
        .chain(&next.file_metadata_nodes)
    {
        if !previous.file_metadata_nodes.contains_key(id)
            || !next.file_metadata_nodes.contains_key(id)
        {
            files.insert(node.file_id.clone());
        }
    }
    for (id, event) in previous
        .file_branch_merges
        .iter()
        .chain(&next.file_branch_merges)
    {
        if !previous.file_branch_merges.contains_key(id)
            || !next.file_branch_merges.contains_key(id)
        {
            files.insert(event.file_id.clone());
        }
    }
    for file in previous
        .file_backup_paths
        .keys()
        .chain(next.file_backup_paths.keys())
    {
        if previous.file_backup_paths.get(file) != next.file_backup_paths.get(file) {
            files.insert(file.clone());
        }
    }
    for file in previous
        .file_backup_policies
        .keys()
        .chain(next.file_backup_policies.keys())
    {
        if previous.file_backup_policies.get(file) != next.file_backup_policies.get(file) {
            files.insert(file.clone());
        }
    }
    for (id, node) in previous.file_path_nodes.iter().chain(&next.file_path_nodes) {
        if !previous.file_path_nodes.contains_key(id) || !next.file_path_nodes.contains_key(id) {
            files.insert(node.file_id.clone());
        }
    }
    for id in previous
        .file_path_notes
        .keys()
        .chain(next.file_path_notes.keys())
    {
        if previous.file_path_notes.get(id) != next.file_path_notes.get(id) {
            if let Some(node) = next
                .file_path_nodes
                .get(id)
                .or_else(|| previous.file_path_nodes.get(id))
            {
                files.insert(node.file_id.clone());
            } else if let Some(node) = next
                .file_metadata_nodes
                .get(id)
                .or_else(|| previous.file_metadata_nodes.get(id))
            {
                files.insert(node.file_id.clone());
            }
        }
    }
    for id in previous
        .file_node_parents
        .keys()
        .chain(next.file_node_parents.keys())
    {
        if previous.file_node_parents.get(id) != next.file_node_parents.get(id) {
            if let Some(version) = next.versions.get(id).or_else(|| previous.versions.get(id)) {
                files.insert(version.file_id.clone());
            }
        }
    }
    for key in previous
        .file_locations
        .keys()
        .chain(next.file_locations.keys())
    {
        if previous.file_locations.get(key) != next.file_locations.get(key) {
            files.insert(key.clone());
        }
    }
    for key in previous.file_names.keys().chain(next.file_names.keys()) {
        if previous.file_names.get(key) != next.file_names.get(key) {
            files.insert(key.clone());
        }
    }
    for key in previous.file_notes.keys().chain(next.file_notes.keys()) {
        if previous.file_notes.get(key) != next.file_notes.get(key) {
            files.insert(key.clone());
        }
    }
    for key in previous
        .file_selections
        .keys()
        .chain(next.file_selections.keys())
    {
        if previous.file_selections.get(key) != next.file_selections.get(key) {
            files.insert(key.clone());
        }
    }
    for key in previous
        .file_contexts
        .keys()
        .chain(next.file_contexts.keys())
    {
        if previous.file_contexts.get(key) != next.file_contexts.get(key) {
            for (state, context) in previous
                .file_contexts
                .get(key)
                .map(|context| (previous, context))
                .into_iter()
                .chain(next.file_contexts.get(key).map(|context| (next, context)))
            {
                if let Some(file) = state
                    .branches
                    .get(&context.branch_id)
                    .and_then(|branch| branch.file_id.as_ref())
                {
                    files.insert(file.clone());
                }
            }
        }
    }
    for key in previous
        .version_notes
        .keys()
        .chain(next.version_notes.keys())
    {
        if previous.version_notes.get(key) != next.version_notes.get(key) {
            if let Some(version) = next
                .versions
                .get(key)
                .or_else(|| previous.versions.get(key))
            {
                files.insert(version.file_id.clone());
            }
        }
    }
    for (id, version) in previous.versions.iter().chain(next.versions.iter()) {
        let changed_placement = match (previous.placements.get(id), next.placements.get(id)) {
            (Some(before), Some(after)) => {
                before.route_slot_ids != after.route_slot_ids
                    || before.binding_generations != after.binding_generations
                    || before.primary_slot_id != after.primary_slot_id
                    || before.target_copies != after.target_copies
            }
            (None, None) => false,
            _ => true,
        };
        if !previous.versions.contains_key(id)
            || !next.versions.contains_key(id)
            || changed_placement
            || previous.content_states.get(id) != next.content_states.get(id)
        {
            files.insert(version.file_id.clone());
        }
    }
    // An inherited branch has no new FileVersion. Its reference metadata must
    // still update the cache, including removal after the first owned write.
    for id in previous.branches.keys().chain(next.branches.keys()) {
        let before = previous.branches.get(id);
        let after = next.branches.get(id);
        let unchanged = match (before, after) {
            (Some(before), Some(after)) => {
                before.pending_file_versions == after.pending_file_versions
                    && before.file_node_id == after.file_node_id
                    && before.name == after.name
                    && before.state == after.state
                    && before.history_tree_id == after.history_tree_id
            }
            (None, None) => true,
            _ => false,
        };
        if !unchanged {
            for branch in before.into_iter().chain(after) {
                files.extend(branch.pending_file_versions.keys().cloned());
            }
        }
    }
    files
}

/// Older releases projected inherited references as empty files. References now
/// exist only in metadata; remove only recognized empty legacy cache files.
/// Validate the whole directory first so an unexpected entry is preserved.
fn cleanup_legacy_pending(named: &Path) -> Result<()> {
    let directory = named.join("pending-branches");
    if !directory_exists(&directory)? {
        return Ok(());
    }
    let held = held_directory(&directory)?;
    let mut removable = Vec::new();
    for entry in fs::read_dir(&directory)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| invalid("pending branch filename is not UTF-8"))?;
        let branch = name
            .strip_suffix(".pending")
            .ok_or_else(|| invalid("pending branch filename has an unsupported form"))?;
        validate_id(branch, "br")?;
        if !file_exists(&entry.path())? || open_regular(&entry.path())?.metadata()?.len() != 0 {
            return Err(invalid("pending branch must be one empty ordinary file"));
        }
        removable.push(name);
    }
    for name in removable {
        rustix::fs::unlinkat(
            held.directory(),
            name.as_str(),
            rustix::fs::AtFlags::empty(),
        )
        .map_err(std::io::Error::from)?;
    }
    held.directory().sync_all()?;
    let parent = held_directory(named)?;
    rustix::fs::unlinkat(
        parent.directory(),
        "pending-branches",
        rustix::fs::AtFlags::REMOVEDIR,
    )
    .map_err(std::io::Error::from)?;
    parent.directory().sync_all()?;
    Ok(())
}

fn synchronize_location(
    current: PathBuf,
    stable: &Path,
    file_id: &str,
    location: Option<&str>,
    name: &str,
) -> Result<PathBuf> {
    let original_parent = current.parent().map(Path::to_path_buf);
    let Some(location) = location else {
        let target = current.parent().expect("container parent").join(name);
        if target != current {
            let parent = held_directory(current.parent().expect("container parent"))?;
            rustix::fs::renameat_with(
                parent.directory(),
                current.file_name().expect("container name"),
                parent.directory(),
                name,
                rustix::fs::RenameFlags::NOREPLACE,
            )
            .map_err(std::io::Error::from)?;
            parent.directory().sync_all()?;
        }
        return Ok(target);
    };
    xcs_naming::validate_path(location)?;
    if location.rsplit('/').next() != Some(name) {
        return Err(invalid(
            "canonical container name differs from its physical location",
        ));
    }
    let namespace = stable.join("namespace");
    let mut current = current;
    // A legacy user file may itself be named `namespace`. Move that one
    // container aside first; discovery and retry still use its stable identity.
    if current == namespace && !marker_matches(&namespace.join(NAMESPACE_MARKER), file_id)? {
        let temporary_name = format!(".xcs-relocate-{}", new_id("req"));
        let parent = held_directory(stable)?;
        rustix::fs::renameat_with(
            parent.directory(),
            "namespace",
            parent.directory(),
            temporary_name.as_str(),
            rustix::fs::RenameFlags::NOREPLACE,
        )
        .map_err(std::io::Error::from)?;
        parent.directory().sync_all()?;
        current = stable.join(temporary_name);
    }
    private_dir(&namespace)?;
    if !marker_matches(&namespace.join(NAMESPACE_MARKER), file_id)? {
        write_json(&namespace.join(NAMESPACE_MARKER), &file_id, false)?;
    }
    let target = namespace.join(location);
    if target != current {
        if target.starts_with(&current) || current.starts_with(&target) {
            let temporary_name = format!(".xcs-relocate-{}", new_id("req"));
            let source = held_directory(current.parent().expect("current container parent"))?;
            let destination = held_directory(stable)?;
            rustix::fs::renameat_with(
                source.directory(),
                current.file_name().expect("current container name"),
                destination.directory(),
                temporary_name.as_str(),
                rustix::fs::RenameFlags::NOREPLACE,
            )
            .map_err(std::io::Error::from)?;
            source.directory().sync_all()?;
            destination.directory().sync_all()?;
            current = stable.join(temporary_name);
        }
        if directory_exists(&target)? {
            // Only structural empty parents can be replaced. Validate all of
            // them before removing any; unexpected bytes or links stay intact.
            let mut empty = Vec::new();
            collect_empty_directories(&target, 0, &mut empty)?;
            for directory in empty {
                let parent = held_directory(directory.parent().expect("empty parent"))?;
                rustix::fs::unlinkat(
                    parent.directory(),
                    directory.file_name().expect("empty name"),
                    rustix::fs::AtFlags::REMOVEDIR,
                )
                .map_err(std::io::Error::from)?;
                parent.directory().sync_all()?;
            }
        }
        let destination_parent = target.parent().expect("canonical location parent");
        private_dir(destination_parent)?;
        let mut directory = Some(destination_parent);
        while let Some(parent) = directory {
            sync_dir(parent)?;
            if parent == stable {
                break;
            }
            directory = parent.parent();
        }
        if !marker_matches(&current.join(OWNER_MARKER), file_id)? {
            write_json(&current.join(OWNER_MARKER), &file_id, false)?;
        }
        sync_dir(&current)?;
        let source = held_directory(current.parent().expect("current container parent"))?;
        let destination = held_directory(destination_parent)?;
        rustix::fs::renameat_with(
            source.directory(),
            current.file_name().expect("current container name"),
            destination.directory(),
            target.file_name().expect("target container name"),
            rustix::fs::RenameFlags::NOREPLACE,
        )
        .map_err(std::io::Error::from)?;
        source.directory().sync_all()?;
        destination.directory().sync_all()?;
    }
    sync_dir(&namespace)?;
    sync_dir(stable)?;
    if let Some(mut parent) = original_parent {
        while parent != namespace && parent.starts_with(&namespace) && directory_exists(&parent)? {
            if fs::read_dir(&parent)?.next().is_some() {
                break;
            }
            let ancestor = parent.parent().expect("namespace ancestor").to_path_buf();
            let held = held_directory(&ancestor)?;
            rustix::fs::unlinkat(
                held.directory(),
                parent.file_name().expect("empty namespace parent"),
                rustix::fs::AtFlags::REMOVEDIR,
            )
            .map_err(std::io::Error::from)?;
            held.directory().sync_all()?;
            parent = ancestor;
        }
    }
    Ok(target)
}

fn migrate_legacy_body(history: &Path, named: &Path, version: &FileVersion) -> Result<()> {
    let source = history
        .join("versions")
        .join(&version.version_id)
        .join("content")
        .join(&version.creation_name);
    let destination = named.join("legacy").join(&version.version_id);
    let target = destination.join(&version.creation_name);
    if !file_exists(&source)? && !file_exists(&target)? {
        return Ok(());
    }
    if file_exists(&target)? {
        verify_body(&target, version)?;
        if file_exists(&source)? {
            verify_body(&source, version)?;
            let source_parent = held_directory(source.parent().expect("legacy content parent"))?;
            rustix::fs::unlinkat(
                source_parent.directory(),
                source.file_name().expect("legacy body name"),
                rustix::fs::AtFlags::empty(),
            )
            .map_err(std::io::Error::from)?;
            source_parent.directory().sync_all()?;
        }
        sync_dir(&destination)?;
        return Ok(());
    }
    verify_body(&source, version)?;
    private_dir(&destination)?;
    sync_dir(destination.parent().expect("legacy parent"))?;
    sync_dir(named)?;
    let source_parent = held_directory(source.parent().expect("legacy content parent"))?;
    let target_parent = held_directory(&destination)?;
    rustix::fs::renameat_with(
        source_parent.directory(),
        source.file_name().expect("legacy body name"),
        target_parent.directory(),
        target.file_name().expect("legacy target name"),
        rustix::fs::RenameFlags::NOREPLACE,
    )
    .map_err(std::io::Error::from)?;
    source_parent.directory().sync_all()?;
    target_parent.directory().sync_all()?;
    Ok(())
}

fn collect_empty_directories(
    root: &Path,
    depth: usize,
    directories: &mut Vec<PathBuf>,
) -> Result<()> {
    if depth > 33 {
        return Err(invalid("container namespace exceeds the path depth limit"));
    }
    existing_private_dir(root)?;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() || entry.file_type()?.is_symlink() {
            return Err(invalid(
                "canonical container target contains unexpected data",
            ));
        }
        collect_empty_directories(&entry.path(), depth + 1, directories)?;
    }
    directories.push(root.to_path_buf());
    Ok(())
}

/// Replayable post-decision projection of names and user notes. The cache is
/// convenient for offline inspection; signed version evidence remains separate.
pub(crate) fn synchronize_names(config: &Config, previous: &State, next: &State) -> Result<()> {
    for file in changed_files(previous, next) {
        validate_id(&file, "file")?;
        let versions = next
            .versions
            .values()
            .filter(|version| version.file_id == file)
            .collect::<Vec<_>>();
        let pending = next
            .branches
            .values()
            .filter_map(|branch| {
                branch
                    .pending_file_versions
                    .get(&file)
                    .map(|source| (branch, source))
            })
            .collect::<Vec<_>>();
        let had_pending = previous
            .branches
            .values()
            .any(|branch| branch.pending_file_versions.contains_key(&file));
        let mut locations = BTreeSet::new();
        for (state, version) in previous
            .versions
            .values()
            .map(|version| (previous, version))
            .chain(next.versions.values().map(|version| (next, version)))
            .filter(|(_, version)| version.file_id == file)
        {
            if version.storage_name.is_none()
                && pending.is_empty()
                && !had_pending
                && !next.file_locations.contains_key(&file)
            {
                continue;
            }
            if let Some(placement) = state.placements.get(&version.version_id) {
                for slot in &placement.route_slot_ids {
                    locations.insert((slot.clone(), version.history_tree_id.clone()));
                }
            }
        }
        for (slot, history_id) in locations {
            let disk = config
                .disks
                .iter()
                .find(|disk| disk.slot_id == slot)
                .ok_or_else(|| {
                    Error::new(
                        "DISK_NOT_FOUND",
                        "container placement references an absent slot",
                    )
                })?;
            let history = disk.root.join("histories").join(&history_id);
            validate_seal(config, disk)?;
            let pending_here = pending
                .iter()
                .copied()
                .filter(|(branch, _)| branch.history_tree_id == history_id)
                .collect::<Vec<_>>();
            let backup_location = next
                .file_backup_paths
                .get(&file)
                .and_then(|paths| paths.get(&slot));
            let physical_location = backup_location.or_else(|| next.file_locations.get(&file));
            let physical_name = backup_location
                .and_then(|path| path.rsplit('/').next())
                .or_else(|| next.file_names.get(&file).map(String::as_str));
            let current = match container(&history, &file)? {
                Some(current) => current,
                None => {
                    if !next.file_locations.contains_key(&file) {
                        continue;
                    }
                    let mut available = None;
                    for version in versions.iter().filter(|version| {
                        version.history_tree_id == history_id && version.storage_name.is_none()
                    }) {
                        let source = history
                            .join("versions")
                            .join(&version.version_id)
                            .join("content")
                            .join(&version.creation_name);
                        if file_exists(&source)? {
                            available = Some(*version);
                            break;
                        }
                    }
                    // Neither metadata-only references nor tombstones create
                    // empty containers. A real legacy body may be relocated
                    // without changing its immutable proof or inode.
                    let Some(version) = available else {
                        continue;
                    };
                    let name = physical_name.unwrap_or(&version.creation_name);
                    xcs_naming::validate_name(name)?;
                    let current = history.join("file-containers").join(&file).join(name);
                    private_dir(&current.join("versions"))?;
                    sync_dir(&current)?;
                    sync_dir(current.parent().expect("stable file parent"))?;
                    current
                }
            };
            let current_name = current
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| invalid("container directory name is not UTF-8"))?;
            let name = physical_name.unwrap_or(current_name);
            xcs_naming::validate_name(name)?;
            for version in versions.iter().filter(|version| {
                version.history_tree_id == history_id && version.storage_name.is_none()
            }) {
                migrate_legacy_body(&history, &current, version)?;
            }
            let stable = history.join("file-containers").join(&file);
            let named = synchronize_location(
                current.clone(),
                &stable,
                &file,
                physical_location.map(String::as_str),
                name,
            )?;
            // Also seal a directory rename that became visible before an
            // earlier attempt returned a durability error.
            sync_dir(&stable)?;
            let mut records = versions.iter().filter(|version| version.history_tree_id == history_id).map(|version| {
                json!({"version_id":version.version_id,"parent_node_ids":crate::business::file_nodes::node_parents(next,&version.version_id),"tags":crate::business::file_nodes::node_tags(next,&version.version_id),"storage_name":version.storage_name,"created_at":version.created_at,"size":version.logical_size.to_string(),"content_hash":version.content_hash,"note":next.version_notes.get(&version.version_id).map(String::as_str).unwrap_or(""),"content_state":next.content_states.get(&version.version_id)})
            }).collect::<Vec<_>>();
            records.sort_by(|a, b| {
                a["storage_name"]
                    .as_str()
                    .cmp(&b["storage_name"].as_str())
                    .then_with(|| a["version_id"].as_str().cmp(&b["version_id"].as_str()))
            });
            cleanup_legacy_pending(&named)?;
            let inherited = pending_here
                .iter()
                .map(|(branch, source)| {
                    json!({"branch_id":branch.branch_id,"name":branch.name,"state":branch.state,"source_node_id":branch.file_node_id.as_ref().unwrap_or(source),"source_version_id":source,"version_id":source,"own_version_id":null,"inherited":true,"materialized":false})
                })
                .collect::<Vec<_>>();
            let contexts = next
                .file_contexts
                .iter()
                .filter(|(_, context)| {
                    next.branches
                        .get(&context.branch_id)
                        .and_then(|branch| branch.file_id.as_ref())
                        == Some(&file)
                })
                .collect::<std::collections::BTreeMap<_, _>>();
            let mut nodes = next
                .file_path_nodes
                .values()
                .filter(|node| node.file_id == file && node.history_tree_id == history_id)
                .collect::<Vec<_>>();
            nodes.sort_by(|a, b| {
                a.sequence
                    .cmp(&b.sequence)
                    .then_with(|| a.node_id.cmp(&b.node_id))
            });
            let path_nodes = nodes.into_iter().map(|node| json!({"node":node,"tags":crate::business::file_nodes::node_tags(next,&node.node_id),"note":next.file_path_notes.get(&node.node_id).map(String::as_str).unwrap_or("")})).collect::<Vec<_>>();
            write_json(
                &named.join("container.json"),
                &json!({"format_version":1,"cache_only":true,"file_id":file,"history_tree_id":history_id,"name":name,"location":physical_location,"primary_location":next.file_locations.get(&file),"backup_policy_id":next.file_backup_policies.get(&file),"metadata_nodes":next.file_metadata_nodes.values().filter(|node|node.file_id==file&&node.history_tree_id==history_id).map(|node|json!({"node":node,"note":next.file_path_notes.get(&node.node_id).map(String::as_str).unwrap_or("")})).collect::<Vec<_>>(),"branch_merges":next.file_branch_merges.values().filter(|event|event.file_id==file&&event.history_tree_id==history_id).collect::<Vec<_>>(),"branch_aliases":next.file_branch_aliases.iter().filter(|(source,_)|next.branches.get(*source).is_some_and(|branch|branch.file_id.as_deref()==Some(file.as_str()))).collect::<BTreeMap<_,_>>(),"note":next.file_notes.get(&file).map(String::as_str).unwrap_or(""),"selected_branch_id":next.file_selections.get(&file),"path_contexts":contexts,"path_nodes":path_nodes,"versions":records,"inherited_branches":inherited}),
                true,
            )?;
            sync_dir(&named)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::BTreeMap,
        os::unix::fs::{PermissionsExt, symlink},
    };

    fn version(state: &State, history: &str, file: &str, name: &str, bytes: &[u8]) -> FileVersion {
        let hash = blake3::hash(bytes).to_hex().to_string();
        let (storage_name, created_at) = version_identity(state, file, name, &hash).unwrap();
        FileVersion {
            format_version: 2,
            history_tree_id: history.into(),
            version_id: new_id("ver"),
            file_id: file.into(),
            creation_primary_slot_id: "P01".into(),
            logical_size: bytes.len() as u64,
            version_sequence: 900,
            content_hash: hash,
            checksum_chunk_bytes: 262_144,
            checksum_manifest_hash: "0".repeat(64),
            creation_name: name.into(),
            storage_name: Some(storage_name),
            created_at: Some(created_at),
            source_version_ids: Vec::new(),
            created_as_full_checkpoint: true,
        }
    }
    fn prepare(history: &Path, version: &FileVersion, bytes: &[u8]) -> PathBuf {
        let directory = history.join("versions").join(&version.version_id);
        private_dir(&directory.join("content")).unwrap();
        write_atomic(
            &directory.join("content").join(&version.creation_name),
            bytes,
            false,
        )
        .unwrap();
        directory
    }

    fn reserve(state: &mut State, version: &FileVersion) -> String {
        let id = new_id("br");
        state.branches.insert(
            id.clone(),
            Branch {
                branch_id: id.clone(),
                history_tree_id: version.history_tree_id.clone(),
                name: "draft".into(),
                head_commit_id: new_id("commit"),
                fork_commit_id: new_id("commit"),
                branch_kind: "file".into(),
                file_id: Some(version.file_id.clone()),
                file_node_id: None,
                pending_file_versions: BTreeMap::from([(
                    version.file_id.clone(),
                    version.version_id.clone(),
                )]),
                generation: 1,
                state: "active".into(),
            },
        );
        id
    }

    fn with_version(version: &FileVersion) -> State {
        let mut state = State::default();
        state
            .versions
            .insert(version.version_id.clone(), version.clone());
        state.placements.insert(
            version.version_id.clone(),
            Placement {
                version_id: version.version_id.clone(),
                primary_slot_id: "P01".into(),
                target_copies: 1,
                route_slot_ids: vec!["P01".into()],
                topology_generation: 1,
                binding_generations: BTreeMap::new(),
            },
        );
        state
            .file_names
            .insert(version.file_id.clone(), version.creation_name.clone());
        state
    }

    #[test]
    fn inherited_references_update_cache_without_creating_files_and_follow_rename() {
        let temporary = tempfile::tempdir().unwrap();
        let config = Config::development([
            temporary.path().join("p"),
            temporary.path().join("b"),
            temporary.path().join("c"),
        ]);
        drop(Engine::initialize(&temporary.path().join("state"), config.clone()).unwrap());
        let history_id = new_id("history");
        let file = new_id("file");
        let item = version(
            &State::default(),
            &history_id,
            &file,
            "original.txt",
            b"body",
        );
        let history = config.disks[0].root.join("histories").join(&history_id);
        let directory = prepare(&history, &item, b"body");
        let body = install_version_content(&directory, &item).unwrap();
        let body_inode = fs::metadata(&body).unwrap().ino();
        let previous = with_version(&item);
        let mut pending = previous.clone();
        let branch = reserve(&mut pending, &item);
        synchronize_names(&config, &previous, &pending).unwrap();
        let named = body.parent().unwrap().parent().unwrap();
        assert!(!named.join("pending-branches").exists());
        synchronize_names(&config, &previous, &pending).unwrap();
        assert!(!named.join("pending-branches").exists());
        let cached: serde_json::Value = read_json(&named.join("container.json")).unwrap();
        assert_eq!(cached["inherited_branches"][0]["branch_id"], branch);
        assert_eq!(
            cached["inherited_branches"][0]["source_version_id"],
            item.version_id
        );
        assert_eq!(
            cached["inherited_branches"][0]["version_id"],
            item.version_id
        );
        assert!(cached["inherited_branches"][0]["own_version_id"].is_null());
        assert_eq!(cached["inherited_branches"][0]["inherited"], true);
        assert_eq!(cached["inherited_branches"][0]["materialized"], false);
        assert!(cached.get("pending_branches").is_none());
        assert!(cached["inherited_branches"][0].get("placeholder").is_none());
        assert_eq!(pending.versions.len(), previous.versions.len());
        assert_eq!(fs::read_dir(body.parent().unwrap()).unwrap().count(), 1);
        assert_eq!(fs::metadata(&body).unwrap().ino(), body_inode);
        assert_eq!(fs::read(&body).unwrap(), b"body");

        // Replay also upgrades a cache left by the older placeholder layout,
        // without treating that empty file as content or preserving its name.
        let legacy = named.join("pending-branches");
        private_dir(&legacy).unwrap();
        write_atomic(&legacy.join(format!("{branch}.pending")), b"", false).unwrap();
        synchronize_names(&config, &previous, &pending).unwrap();
        assert!(!legacy.exists());
        assert_eq!(fs::metadata(&body).unwrap().ino(), body_inode);
        assert_eq!(fs::read(&body).unwrap(), b"body");

        let mut renamed = pending.clone();
        renamed.file_names.insert(file.clone(), "renamed.md".into());
        synchronize_names(&config, &pending, &renamed).unwrap();
        let new_named = named.parent().unwrap().join("renamed.md");
        assert!(!new_named.join("pending-branches").exists());
        let new_body = content_path(&directory, &item).unwrap();
        assert_eq!(fs::metadata(&new_body).unwrap().ino(), body_inode);
        let mut written = renamed.clone();
        written
            .branches
            .get_mut(&branch)
            .unwrap()
            .pending_file_versions
            .clear();
        synchronize_names(&config, &renamed, &written).unwrap();
        assert!(!new_named.join("pending-branches").exists());
        assert_eq!(fs::metadata(&new_body).unwrap().ino(), body_inode);
        let cached: serde_json::Value = read_json(&new_named.join("container.json")).unwrap();
        assert_eq!(cached["inherited_branches"], json!([]));
    }

    #[test]
    fn legacy_inherited_sources_keep_only_the_original_body_without_new_containers() {
        let temporary = tempfile::tempdir().unwrap();
        let config = Config::development([
            temporary.path().join("p"),
            temporary.path().join("b"),
            temporary.path().join("c"),
        ]);
        drop(Engine::initialize(&temporary.path().join("state"), config.clone()).unwrap());
        let history_id = new_id("history");
        let file = new_id("file");
        let mut item = version(
            &State::default(),
            &history_id,
            &file,
            "legacy.txt",
            b"old body",
        );
        item.storage_name = None;
        item.created_at = None;
        let history = config.disks[0].root.join("histories").join(&history_id);
        let directory = prepare(&history, &item, b"old body");
        let body = install_version_content(&directory, &item).unwrap();
        let inode = fs::metadata(&body).unwrap().ino();
        let previous = with_version(&item);
        let mut pending = previous.clone();
        reserve(&mut pending, &item);
        synchronize_names(&config, &previous, &pending).unwrap();
        let containers = history.join("file-containers");
        assert!(!containers.exists());
        assert_eq!(content_path(&directory, &item).unwrap(), body);
        assert_eq!(fs::metadata(&body).unwrap().ino(), inode);
        synchronize_names(&config, &State::default(), &pending).unwrap();
        assert!(!containers.exists());
        assert_eq!(fs::metadata(&body).unwrap().ino(), inode);
        assert_eq!(fs::read(body).unwrap(), b"old body");
    }

    #[test]
    fn physical_move_preserves_one_body_and_old_version_identity_across_replay() {
        let temporary = tempfile::tempdir().unwrap();
        let config = Config::development([
            temporary.path().join("p"),
            temporary.path().join("b"),
            temporary.path().join("c"),
        ]);
        drop(Engine::initialize(&temporary.path().join("state"), config.clone()).unwrap());
        let history_id = new_id("history");
        let file = new_id("file");
        let item = version(
            &State::default(),
            &history_id,
            &file,
            "报告.txt",
            b"retained parent bytes",
        );
        let history = config.disks[0].root.join("histories").join(&history_id);
        let directory = prepare(&history, &item, b"retained parent bytes");
        let original = install_version_content(&directory, &item).unwrap();
        let inode = fs::metadata(&original).unwrap().ino();
        let previous = with_version(&item);
        let mut in_a = previous.clone();
        in_a.file_locations
            .insert(file.clone(), "a/报告.txt".into());
        synchronize_names(&config, &previous, &in_a).unwrap();
        let a = content_path(&directory, &item).unwrap();
        assert!(
            a.to_string_lossy()
                .contains("/namespace/a/报告.txt/versions/")
        );
        let mut in_b = in_a.clone();
        in_b.file_locations
            .insert(file.clone(), "b/报告.txt".into());
        synchronize_names(&config, &in_a, &in_b).unwrap();
        let b = content_path(&directory, &item).unwrap();
        assert!(
            b.to_string_lossy()
                .contains("/namespace/b/报告.txt/versions/")
        );
        assert!(!a.exists());
        assert!(!original.exists());
        assert_eq!(fs::metadata(&b).unwrap().ino(), inode);
        assert_eq!(fs::metadata(&b).unwrap().nlink(), 1);
        assert_eq!(fs::read(&b).unwrap(), b"retained parent bytes");
        assert_eq!(in_b.versions.len(), previous.versions.len());
        // A decided operation may replay against either its preceding state or
        // an empty reconstructed cache; old F1 still resolves by its stable ID.
        synchronize_names(&config, &in_a, &in_b).unwrap();
        synchronize_names(&config, &State::default(), &in_b).unwrap();
        assert_eq!(content_path(&directory, &item).unwrap(), b);
        assert_eq!(fs::metadata(&b).unwrap().ino(), inode);
        let cache: serde_json::Value =
            read_json(&b.parent().unwrap().parent().unwrap().join("container.json")).unwrap();
        assert_eq!(cache["location"], "b/报告.txt");
    }

    #[test]
    fn legacy_proof_remains_unchanged_when_its_body_moves_to_the_new_owner() {
        let temporary = tempfile::tempdir().unwrap();
        let config = Config::development([
            temporary.path().join("p"),
            temporary.path().join("b"),
            temporary.path().join("c"),
        ]);
        drop(Engine::initialize(&temporary.path().join("state"), config.clone()).unwrap());
        let history_id = new_id("history");
        let file = new_id("file");
        let mut item = version(
            &State::default(),
            &history_id,
            &file,
            "legacy.txt",
            b"old proof bytes",
        );
        item.storage_name = None;
        item.created_at = None;
        let immutable = canonical_json(&item).unwrap();
        let history = config.disks[0].root.join("histories").join(&history_id);
        let directory = prepare(&history, &item, b"old proof bytes");
        let original = install_version_content(&directory, &item).unwrap();
        let inode = fs::metadata(&original).unwrap().ino();
        let previous = with_version(&item);
        let mut moved = previous.clone();
        moved
            .file_locations
            .insert(file.clone(), "b/legacy.txt".into());
        synchronize_names(&config, &previous, &moved).unwrap();
        let relocated = content_path(&directory, &item).unwrap();
        assert!(!original.exists());
        assert!(
            relocated
                .to_string_lossy()
                .contains("/namespace/b/legacy.txt/legacy/")
        );
        assert_eq!(fs::metadata(&relocated).unwrap().ino(), inode);
        assert_eq!(fs::metadata(&relocated).unwrap().nlink(), 1);
        assert_eq!(
            canonical_json(&moved.versions[&item.version_id]).unwrap(),
            immutable
        );
        assert_eq!(
            install_version_content(&directory, &item).unwrap(),
            relocated
        );
        synchronize_names(&config, &previous, &moved).unwrap();
        assert_eq!(content_path(&directory, &item).unwrap(), relocated);
    }

    #[test]
    fn decided_cross_directory_move_recovers_both_views_and_one_physical_container() {
        for fault in ["after_finalize", "after_container_names"] {
            let temporary = tempfile::tempdir().unwrap();
            let root = temporary.path().join("state");
            let config = Config::development([
                temporary.path().join("p"),
                temporary.path().join("b"),
                temporary.path().join("c"),
            ]);
            let mut engine = Engine::initialize(&root, config.clone()).unwrap();
            let repo = engine
                .create_repository("移动恢复", "P01", 3, "alice", &new_id("op"))
                .unwrap();
            let workspace = engine
                .create_workspace(&repo.main_branch_id, "alice")
                .unwrap();
            engine
                .stage_bytes(&workspace.workspace_id, "a/f.txt", b"frozen F1")
                .unwrap();
            engine
                .stage_bytes(&workspace.workspace_id, "b/keep.txt", b"untouched")
                .unwrap();
            engine
                .commit_workspace(CommitRequest {
                    workspace_id: workspace.workspace_id,
                    operation_id: new_id("op"),
                    principal: "alice".into(),
                    expected_head: workspace.base_head,
                    fencing_token: workspace.fencing_token,
                    message: "parent files".into(),
                    request_digest: None,
                })
                .unwrap();
            let file = engine
                .list_tree(&repo.main_branch_id)
                .unwrap()
                .into_iter()
                .find(|entry| entry.path == "a/f.txt")
                .unwrap();
            let source = engine
                .initialize_directory_scope(&repo.repository_id, "a", "alice", &new_id("op"))
                .unwrap()
                .result["directory_branch_id"]
                .as_str()
                .unwrap()
                .to_owned();
            let target = engine
                .initialize_directory_scope(&repo.repository_id, "b", "alice", &new_id("op"))
                .unwrap()
                .result["directory_branch_id"]
                .as_str()
                .unwrap()
                .to_owned();
            let retained = engine
                .create_directory_branches(
                    &repo.repository_id,
                    "a",
                    "retained",
                    "alice",
                    &new_id("op"),
                )
                .unwrap()
                .result["directory_branch_id"]
                .as_str()
                .unwrap()
                .to_owned();
            let retained_head = engine.state.directory_branches[&retained]
                .head_tree_id
                .clone();
            let item = engine.state.versions[&file.version_id].clone();
            let inodes = config
                .disks
                .iter()
                .map(|disk| {
                    let directory = disk
                        .root
                        .join("histories")
                        .join(&repo.history_tree_id)
                        .join("versions")
                        .join(&file.version_id);
                    let body = content_path(&directory, &item).unwrap();
                    (body.clone(), fs::metadata(body).unwrap().ino())
                })
                .collect::<Vec<_>>();
            let request = DirectoryNamespaceBatch {
                kind: "move".into(),
                items: vec![DirectoryNamespaceItem {
                    source: DirectoryNamespaceSource {
                        directory_branch_id: source.clone(),
                        expected_tree_id: engine.state.directory_branches[&source]
                            .head_tree_id
                            .clone(),
                        path: "a/f.txt".into(),
                        entry_id: file.entry_id.clone(),
                    },
                    target: Some(DirectoryNamespaceTarget {
                        directory_branch_id: target.clone(),
                        expected_tree_id: engine.state.directory_branches[&target]
                            .head_tree_id
                            .clone(),
                        path: "b/f.txt".into(),
                    }),
                }],
            };
            let operation = new_id("op");
            engine.inject_fault_once(fault);
            assert_eq!(
                engine
                    .directory_namespace_batch(&repo.repository_id, &request, "alice", &operation)
                    .unwrap_err()
                    .code(),
                "IN_DOUBT"
            );
            drop(engine);
            let mut engine = Engine::open(&root).unwrap();
            let receipt = engine
                .directory_namespace_batch(&repo.repository_id, &request, "alice", &operation)
                .unwrap();
            assert_eq!(engine.operation(&operation).unwrap().result, receipt.result);
            assert!(
                !engine.state.directory_branches[&source]
                    .entries
                    .iter()
                    .any(|entry| entry.file_id == file.file_id)
            );
            assert!(
                engine.state.directory_branches[&target]
                    .entries
                    .iter()
                    .any(|entry| entry.file_id == file.file_id
                        && entry.version_id == file.version_id)
            );
            assert_eq!(
                engine.state.directory_branches[&retained].head_tree_id,
                retained_head
            );
            assert_eq!(engine.state.file_locations[&file.file_id], "b/f.txt");
            for (disk, (old_body, inode)) in config.disks.iter().zip(inodes) {
                let directory = disk
                    .root
                    .join("histories")
                    .join(&repo.history_tree_id)
                    .join("versions")
                    .join(&file.version_id);
                let body = content_path(&directory, &item).unwrap();
                assert!(
                    body.to_string_lossy()
                        .contains("/namespace/b/f.txt/versions/")
                );
                assert!(!old_body.exists());
                assert_eq!(fs::metadata(&body).unwrap().ino(), inode);
                assert_eq!(fs::metadata(&body).unwrap().nlink(), 1);
                assert_eq!(fs::read(body).unwrap(), b"frozen F1");
            }
        }
    }

    #[test]
    fn canonical_location_handles_reserved_looking_user_names_and_prefix_moves() {
        let temporary = tempfile::tempdir().unwrap();
        let config = Config::development([
            temporary.path().join("p"),
            temporary.path().join("b"),
            temporary.path().join("c"),
        ]);
        drop(Engine::initialize(&temporary.path().join("state"), config.clone()).unwrap());
        let history_id = new_id("history");
        let file = new_id("file");
        let item = version(
            &State::default(),
            &history_id,
            &file,
            "namespace",
            b"single body",
        );
        let history = config.disks[0].root.join("histories").join(&history_id);
        let directory = prepare(&history, &item, b"single body");
        let original = install_version_content(&directory, &item).unwrap();
        let inode = fs::metadata(&original).unwrap().ino();
        let mut state = with_version(&item);
        for location in [
            "namespace",
            "namespace/sub/namespace",
            "namespace",
            "versions/.xcs-container-owner.json/namespace",
        ] {
            let previous = state.clone();
            state.file_locations.insert(file.clone(), location.into());
            synchronize_names(&config, &previous, &state).unwrap();
            let body = content_path(&directory, &item).unwrap();
            assert_eq!(fs::metadata(&body).unwrap().ino(), inode);
            assert_eq!(fs::read(body).unwrap(), b"single body");
        }
    }

    #[test]
    fn legacy_pending_cleanup_rejects_links_and_nonempty_files_without_removing_them() {
        let temporary = tempfile::tempdir().unwrap();
        fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let named = temporary.path().join("container");
        let pending = named.join("pending-branches");
        private_dir(&pending).unwrap();
        let branch = new_id("br");
        let placeholder = pending.join(format!("{branch}.pending"));
        write_atomic(&placeholder, b"unexpected user bytes", false).unwrap();
        assert_eq!(
            cleanup_legacy_pending(&named).unwrap_err().code(),
            "CONTAINER_LAYOUT_INVALID"
        );
        assert_eq!(fs::read(&placeholder).unwrap(), b"unexpected user bytes");
        fs::remove_file(&placeholder).unwrap();
        let source = temporary.path().join("source");
        write_atomic(&source, b"", false).unwrap();
        symlink(&source, &placeholder).unwrap();
        assert!(cleanup_legacy_pending(&named).is_err());
        assert!(source.is_file());
        assert!(
            fs::symlink_metadata(&placeholder)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn legacy_pending_cleanup_removes_only_empty_cache_files_and_never_recreates_them() {
        let temporary = tempfile::tempdir().unwrap();
        fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let named = temporary.path().join("container");
        let pending = named.join("pending-branches");
        private_dir(&pending).unwrap();
        let stub = pending.join(format!("{}.pending", new_id("br")));
        write_atomic(&stub, b"", false).unwrap();
        cleanup_legacy_pending(&named).unwrap();
        assert!(!pending.exists());
        cleanup_legacy_pending(&named).unwrap();
        assert!(!pending.exists());

        private_dir(&pending).unwrap();
        write_atomic(&stub, b"", false).unwrap();
        let unknown = pending.join("keep.txt");
        write_atomic(&unknown, b"user data", false).unwrap();
        assert!(cleanup_legacy_pending(&named).is_err());
        assert!(stub.is_file());
        assert_eq!(fs::read(&unknown).unwrap(), b"user data");

        fs::remove_file(&unknown).unwrap();
        let linked = temporary.path().join("linked");
        fs::hard_link(&stub, &linked).unwrap();
        assert!(cleanup_legacy_pending(&named).is_err());
        assert!(stub.is_file());
        assert!(linked.is_file());
    }

    #[test]
    fn identity_counts_each_file_independently_and_preserves_legacy_order() {
        let mut state = State::default();
        let file = new_id("file");
        let history = new_id("history");
        let mut old = version(&state, &history, &file, "报告.txt", b"legacy");
        old.storage_name = None;
        old.created_at = None;
        state.versions.insert(old.version_id.clone(), old);
        let next = version(&state, &history, &file, "报告.txt", b"next");
        assert_eq!(sequence(next.storage_name.as_deref().unwrap()).unwrap(), 2);
        assert!(next.storage_name.as_deref().unwrap().ends_with(".txt"));
        assert!(next.created_at.as_deref().unwrap().ends_with('Z'));
        assert_eq!(next.created_at.as_deref().unwrap().len(), 20);
        state.versions.insert(next.version_id.clone(), next);
        state.file_names.insert(file.clone(), "改名.md".into());
        assert!(
            version(&state, &history, &file, "报告.txt", b"new extension")
                .storage_name
                .as_deref()
                .unwrap()
                .ends_with(".md")
        );
        assert_eq!(
            sequence(
                version(&state, &history, &file, "renamed.md", b"third")
                    .storage_name
                    .as_deref()
                    .unwrap()
            )
            .unwrap(),
            3
        );
        assert_eq!(
            sequence(
                version(&state, &history, &new_id("file"), "other.txt", b"other")
                    .storage_name
                    .as_deref()
                    .unwrap()
            )
            .unwrap(),
            1
        );
        assert!(version_identity(&state, &file, "../bad", &"0".repeat(64)).is_err());
    }

    #[test]
    fn installation_moves_one_ordinary_inode_and_missing_paths_remain_classifiable() {
        let temporary = tempfile::tempdir().unwrap();
        let history_id = new_id("history");
        let file = new_id("file");
        let history = temporary.path().join(&history_id);
        let item = version(
            &State::default(),
            &history_id,
            &file,
            "notes.txt",
            b"complete bytes",
        );
        let directory = prepare(&history, &item, b"complete bytes");
        let legacy = directory.join("content/notes.txt");
        assert_eq!(
            content_path(&directory, &item).unwrap(),
            legacy,
            "a prepared body remains readable before its atomic move"
        );
        let before = fs::metadata(&legacy).unwrap();
        let target = install_version_content(&directory, &item).unwrap();
        assert!(!legacy.exists());
        let after = fs::metadata(&target).unwrap();
        assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
        assert_eq!(after.nlink(), 1);
        assert_eq!(fs::read(&target).unwrap(), b"complete bytes");
        assert_eq!(install_version_content(&directory, &item).unwrap(), target);
        assert_eq!(content_path(&directory, &item).unwrap(), target);
        fs::remove_file(&target).unwrap();
        assert_eq!(
            content_path(&directory, &item).unwrap(),
            target,
            "missing content returns the expected path rather than a generic layout error"
        );
        symlink("/does-not-exist", &target).unwrap();
        assert_eq!(
            content_path(&directory, &item).unwrap_err().code(),
            "CONTAINER_LAYOUT_INVALID"
        );
    }

    #[test]
    fn rename_and_note_projection_replays_without_changing_body_identity() {
        let temporary = tempfile::tempdir().unwrap();
        let config = Config::development([
            temporary.path().join("primary"),
            temporary.path().join("backup"),
            temporary.path().join("tertiary"),
        ]);
        let engine = Engine::initialize(&temporary.path().join("state"), config.clone()).unwrap();
        drop(engine);
        let history_id = new_id("history");
        let file = new_id("file");
        let history = config.disks[0].root.join("histories").join(&history_id);
        let mut previous = State::default();
        let item = version(
            &previous,
            &history_id,
            &file,
            "original.txt",
            b"one unique full file",
        );
        let directory = prepare(&history, &item, b"one unique full file");
        let original = install_version_content(&directory, &item).unwrap();
        let inode = fs::metadata(&original).unwrap().ino();
        previous
            .versions
            .insert(item.version_id.clone(), item.clone());
        previous.placements.insert(
            item.version_id.clone(),
            Placement {
                version_id: item.version_id.clone(),
                primary_slot_id: "P01".into(),
                target_copies: 1,
                route_slot_ids: vec!["P01".into()],
                topology_generation: 1,
                binding_generations: BTreeMap::new(),
            },
        );
        previous
            .file_names
            .insert(file.clone(), "original.txt".into());
        let mut next = previous.clone();
        next.file_names.insert(file.clone(), "重命名.md".into());
        next.file_notes.insert(file.clone(), "文件备注".into());
        next.version_notes
            .insert(item.version_id.clone(), "版本备注".into());
        next.file_selections.insert(file.clone(), new_id("br"));
        synchronize_names(&config, &previous, &next).unwrap();
        let renamed = content_path(&directory, &item).unwrap();
        assert!(renamed.to_str().unwrap().contains("/重命名.md/versions/"));
        assert!(!original.exists());
        assert_eq!(fs::metadata(&renamed).unwrap().ino(), inode);
        assert_eq!(
            blake3::hash(&fs::read(&renamed).unwrap()).to_hex().as_str(),
            item.content_hash
        );
        let cache = renamed
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("container.json");
        let cached: serde_json::Value = read_json(&cache).unwrap();
        assert_eq!(cached["name"], "重命名.md");
        assert_eq!(cached["note"], "文件备注");
        assert_eq!(cached["versions"][0]["note"], "版本备注");
        assert_eq!(cached["selected_branch_id"], next.file_selections[&file]);
        synchronize_names(&config, &previous, &next).unwrap();
        assert_eq!(fs::metadata(&renamed).unwrap().ino(), inode);
        let cache_inode = fs::metadata(&cache).unwrap().ino();
        synchronize_names(&config, &next, &next).unwrap();
        assert_eq!(
            fs::metadata(&cache).unwrap().ino(),
            cache_inode,
            "unrelated transactions do not rewrite the cache"
        );
        private_dir(
            &history
                .join("file-containers")
                .join(&file)
                .join("ambiguous.txt"),
        )
        .unwrap();
        assert_eq!(
            content_path(&directory, &item).unwrap_err().code(),
            "CONTAINER_AMBIGUOUS"
        );
    }
}
