//! Disk topology, resource admission configuration and OS writer locks.
use crate::{disk::*, *};
use std::{collections::BTreeSet, fs::File, path::Path};

pub(super) fn acquire_locks(
    root: &Path,
    config: &Config,
    allow_missing: bool,
) -> Result<Vec<File>> {
    let mut paths = vec![root.join("writer.lock")];
    for d in &config.disks {
        if d.root.exists() {
            if allow_missing {
                validate_seal(config, d)?;
            }
            paths.push(d.root.join(".writer.lock"));
        }
    }
    let mut locks = Vec::new();
    for path in paths {
        let f = File::from(
            rustix::fs::open(
                &path,
                rustix::fs::OFlags::CREATE
                    | rustix::fs::OFlags::RDWR
                    | rustix::fs::OFlags::NOFOLLOW
                    | rustix::fs::OFlags::NONBLOCK
                    | rustix::fs::OFlags::CLOEXEC,
                rustix::fs::Mode::from_raw_mode(0o600),
            )
            .map_err(std::io::Error::from)?,
        );
        xcs_platform::fs_safety::linux::SingleLinkRequirement::check(&f)
            .map_err(|e| Error::new("UNSAFE_PATH", e.to_string()))?;
        rustix::fs::flock(&f, rustix::fs::FlockOperation::NonBlockingLockExclusive).map_err(
            |_| {
                Error::new(
                    "WRITER_ALREADY_ACTIVE",
                    "another process owns this pool or disk",
                )
            },
        )?;
        locks.push(f);
    }
    Ok(locks)
}
pub(super) fn validate_config(c: &Config) -> Result<()> {
    validate_id(&c.pool_id, "pool")?;
    if c.format_version != 2
        || c.disks.iter().filter(|d| d.metadata_member).count() != 3
        || c.max_file_bytes == 0
        || c.max_workspace_bytes == 0
        || c.max_file_bytes > c.max_workspace_bytes
        || c.max_workspaces == 0
        || c.max_entries == 0
        || !SUPPORTED_CHECKSUM_CHUNK_BYTES.contains(&c.checksum_chunk_bytes)
    {
        return Err(Error::new(
            "CONFIG_INVALID",
            "invalid format, metadata membership, resource limits or checksum size",
        ));
    }
    let chunks = c.max_file_bytes.div_ceil(c.checksum_chunk_bytes as u64);
    if chunks > (MAX_CHECKSUM_MANIFEST as u64 - 32) / 32 {
        return Err(Error::new(
            "CONFIG_INVALID",
            "file limit exceeds the bounded checksum manifest capacity",
        ));
    }
    let mut slots = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut roots = BTreeSet::new();
    let mut successors = BTreeSet::new();
    for d in &c.disks {
        xcs_naming::validate_slot(&d.slot_id)?;
        validate_id(&d.disk_id, "disk")?;
        if !slots.insert(d.slot_id.to_ascii_lowercase())
            || !ids.insert(&d.disk_id)
            || !roots.insert(&d.root)
            || d.level == 0
            || d.binding_generation == 0
        {
            return Err(Error::new(
                "CONFIG_INVALID",
                "duplicate disk, path or slot identity",
            ));
        }
        if d.level == 1 && d.upstream_slot_id.is_some() {
            return Err(Error::new(
                "TOPOLOGY_INVALID",
                "primary cannot have an upstream",
            ));
        }
        if d.level > 1 {
            let up = d
                .upstream_slot_id
                .as_ref()
                .ok_or_else(|| Error::new("TOPOLOGY_INVALID", "backup lacks upstream"))?;
            let source = c
                .disks
                .iter()
                .find(|s| s.slot_id.eq_ignore_ascii_case(up))
                .ok_or_else(|| Error::new("TOPOLOGY_INVALID", "upstream slot missing"))?;
            if source.level + 1 != d.level || !successors.insert(up.to_ascii_lowercase()) {
                return Err(Error::new(
                    "TOPOLOGY_INVALID",
                    "route skips a level or source has multiple successors",
                ));
            }
        }
    }
    for (i, disk) in c.disks.iter().enumerate() {
        if c.disks
            .iter()
            .skip(i + 1)
            .any(|other| disk.root.starts_with(&other.root) || other.root.starts_with(&disk.root))
        {
            return Err(Error::new("CONFIG_INVALID", "disk roots must not overlap"));
        }
    }
    Ok(())
}
