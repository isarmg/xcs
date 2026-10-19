use crate::{disk::*, *};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Path, PathBuf},
};
pub(crate) const MAX_FRAME: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub record_type: String,
    #[serde(with = "xcs_contract::u64_string")]
    pub sequence: u64,
    pub candidate_hash: String,
    pub object_hash: String,
    pub previous_frame_hash: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Receipt {
    pub disk_id: String,
    pub slot_id: String,
    pub frame_hash: String,
    pub record: Record,
}
pub(crate) fn receipt_binds(
    receipt: &Receipt,
    kind: &str,
    sequence: u64,
    candidate: &str,
    object: &str,
) -> Result<bool> {
    let r = &receipt.record;
    let payload = canonical_json(r)?;
    if payload.len() > MAX_FRAME {
        return Ok(false);
    }
    let mut header = Vec::with_capacity(16);
    header.extend(b"XCSLOG2\0");
    header.extend(2u16.to_le_bytes());
    header.extend(0u16.to_le_bytes());
    header.extend((payload.len() as u32).to_le_bytes());
    let mut digest = blake3::Hasher::new();
    digest.update(&header);
    digest.update(&payload);
    Ok(r.record_type == kind
        && r.sequence == sequence
        && r.candidate_hash == candidate
        && r.object_hash == object
        && r.previous_frame_hash.len() == 64
        && digest.finalize().to_hex().as_str() == receipt.frame_hash)
}
pub(crate) struct Scan {
    pub records: Vec<(Record, String)>,
    pub valid_bytes: u64,
    pub torn_tail: bool,
}
pub(crate) fn log_path(d: &DiskConfig) -> PathBuf {
    d.root.join("metadata/application-log/journal.bin")
}
pub(crate) fn scan(d: &DiskConfig) -> Result<Scan> {
    let path = log_path(d);
    if !path.exists() {
        return Ok(Scan {
            records: Vec::new(),
            valid_bytes: 0,
            torn_tail: false,
        });
    }
    let mut file = open_regular(&path)?;
    let mut out = Scan {
        records: Vec::new(),
        valid_bytes: 0,
        torn_tail: false,
    };
    let mut previous = "0".repeat(64);
    let mut last_sequence = 0;
    loop {
        let mut header = [0u8; 16];
        let mut got = 0;
        while got < 16 {
            let n = file.read(&mut header[got..])?;
            if n == 0 {
                break;
            }
            got += n;
        }
        if got == 0 {
            break;
        }
        if got < 16 {
            out.torn_tail = true;
            break;
        }
        if &header[..8] != b"XCSLOG2\0"
            || header[8..10] != 2u16.to_le_bytes()
            || header[10..12] != [0, 0]
        {
            return Err(Error::new("LOG_CORRUPT", "invalid log frame header"));
        }
        let len = u32::from_le_bytes(header[12..16].try_into().expect("fixed header")) as usize;
        if len > MAX_FRAME {
            return Err(Error::new("LOG_CORRUPT", "log frame length exceeds limit"));
        }
        let mut payload = vec![0; len];
        if let Err(e) = file.read_exact(&mut payload) {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                out.torn_tail = true;
                break;
            }
            return Err(e.into());
        }
        let mut digest = [0; 32];
        if let Err(e) = file.read_exact(&mut digest) {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                out.torn_tail = true;
                break;
            }
            return Err(e.into());
        }
        let mut h = blake3::Hasher::new();
        h.update(&header);
        h.update(&payload);
        if h.finalize().as_bytes() != &digest {
            return Err(Error::new("LOG_CORRUPT", "log frame checksum differs"));
        }
        let record: Record = decode_strict(&payload)?;
        if !matches!(
            record.record_type.as_str(),
            "PREPARE" | "ACCEPT" | "FINALIZE" | "PUBLICATION" | "REPROTECT"
        ) {
            return Err(Error::new(
                "LOG_FORMAT_UNSUPPORTED",
                "unknown journal record type",
            ));
        }
        if canonical_json(&record)? != payload
            || record.previous_frame_hash != previous
            || record.sequence < last_sequence
        {
            return Err(Error::new(
                "LOG_CORRUPT",
                "log chain or canonical encoding differs",
            ));
        }
        previous = blake3::Hash::from_bytes(digest).to_hex().to_string();
        last_sequence = record.sequence;
        out.records.push((record, previous.clone()));
        out.valid_bytes += 16 + len as u64 + 32;
    }
    Ok(out)
}
pub(crate) fn append(
    d: &DiskConfig,
    kind: &str,
    sequence: u64,
    candidate: &str,
    object: &str,
) -> Result<Receipt> {
    let path = log_path(d);
    private_dir(path.parent().expect("log parent"))?;
    let scanned = scan(d)?;
    if let Some((record, hash)) = scanned.records.iter().find(|(r, _)| {
        r.sequence == sequence
            && r.record_type == kind
            && r.candidate_hash == candidate
            && r.object_hash == object
    }) {
        return Ok(Receipt {
            disk_id: d.disk_id.clone(),
            slot_id: d.slot_id.clone(),
            frame_hash: hash.clone(),
            record: record.clone(),
        });
    }
    if scanned
        .records
        .iter()
        .any(|(r, _)| r.sequence == sequence && r.candidate_hash != candidate)
    {
        return Err(Error::new(
            "LOG_FORK",
            "two candidates occupy one decision slot",
        ));
    }
    let record = Record {
        record_type: kind.into(),
        sequence,
        candidate_hash: candidate.into(),
        object_hash: object.into(),
        previous_frame_hash: scanned
            .records
            .last()
            .map(|(_, h)| h.clone())
            .unwrap_or_else(|| "0".repeat(64)),
    };
    let bytes = canonical_json(&record)?;
    if bytes.len() > MAX_FRAME {
        return Err(Error::new(
            "RESOURCE_LIMIT",
            "log record exceeds frame limit",
        ));
    }
    let mut header = Vec::with_capacity(16);
    header.extend(b"XCSLOG2\0");
    header.extend(2u16.to_le_bytes());
    header.extend(0u16.to_le_bytes());
    header.extend((bytes.len() as u32).to_le_bytes());
    let mut h = blake3::Hasher::new();
    h.update(&header);
    h.update(&bytes);
    let digest = h.finalize();
    let mut file = open_append_regular(&path)?;
    if scanned.torn_tail {
        file.set_len(scanned.valid_bytes)?;
        file.sync_all()?;
    }
    file.write_all(&header)?;
    file.write_all(&bytes)?;
    file.write_all(digest.as_bytes())?;
    file.sync_all()?;
    sync_dir(path.parent().expect("log parent"))?;
    Ok(Receipt {
        disk_id: d.disk_id.clone(),
        slot_id: d.slot_id.clone(),
        frame_hash: digest.to_hex().to_string(),
        record,
    })
}
pub(crate) fn object_path(d: &DiskConfig, kind: &str, hash: &str) -> PathBuf {
    d.root
        .join("metadata/objects")
        .join(kind)
        .join(format!("{hash}.json"))
}
pub(crate) fn store_object<T: Serialize + ?Sized>(
    d: &DiskConfig,
    kind: &str,
    value: &T,
) -> Result<String> {
    let hash = object_hash(kind, value)?;
    write_json(&object_path(d, kind, &hash), value, false)?;
    Ok(hash)
}
pub(crate) fn load_object<T: serde::de::DeserializeOwned + Serialize>(
    d: &DiskConfig,
    kind: &str,
    hash: &str,
) -> Result<T> {
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::new("OBJECT_CORRUPT", "invalid object hash"));
    }
    let value = read_json(&object_path(d, kind, hash))?;
    if object_hash(kind, &value)? != hash {
        return Err(Error::new(
            "OBJECT_CORRUPT",
            "immutable object digest differs",
        ));
    }
    Ok(value)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Snapshot {
    pub format_version: u32,
    #[serde(with = "xcs_contract::u64_string")]
    pub sequence: u64,
    pub pages: BTreeMap<String, Vec<String>>,
    pub scalars: serde_json::Map<String, serde_json::Value>,
}
pub(crate) const STATE_PAGE_ENTRIES: usize = 128;
/// Each page is immutable and independently hashed; the journal holds only roots.
pub(crate) fn store_snapshot(members: &[DiskConfig], state: &State) -> Result<String> {
    store_snapshot_bounded(members, state, MAX_METADATA)
}
pub(crate) fn store_snapshot_bounded(
    members: &[DiskConfig],
    state: &State,
    maximum: usize,
) -> Result<String> {
    store_snapshot_described(members, state, maximum).map(|(hash, _)| hash)
}
pub(crate) fn snapshot_descriptor(state: &State, maximum: usize) -> Result<Snapshot> {
    store_snapshot_described(&[], state, maximum).map(|(_, snapshot)| snapshot)
}
fn store_snapshot_described(
    members: &[DiskConfig],
    state: &State,
    maximum: usize,
) -> Result<(String, Snapshot)> {
    let value = serde_json::to_value(state)?;
    let obj = value
        .as_object()
        .ok_or_else(|| Error::new("STATE_INVALID", "state is not an object"))?;
    let mut pages = BTreeMap::new();
    let mut scalars = serde_json::Map::new();
    for (field, value) in obj {
        if let Some(map) = value.as_object() {
            let mut hashes = Vec::new();
            let entries = map.iter().collect::<Vec<_>>();
            for chunk in entries.chunks(STATE_PAGE_ENTRIES) {
                let page: serde_json::Map<_, _> = chunk
                    .iter()
                    .map(|(k, v)| ((*k).clone(), (*v).clone()))
                    .collect();
                encode_metadata(&page, maximum)?;
                let hash = object_hash("state-page", &page)?;
                for d in members {
                    store_object(d, "state-page", &page)?;
                }
                hashes.push(hash);
            }
            pages.insert(field.clone(), hashes);
        } else {
            scalars.insert(field.clone(), value.clone());
        }
    }
    let snapshot = Snapshot {
        format_version: 2,
        sequence: state.sequence,
        pages,
        scalars,
    };
    encode_metadata(&snapshot, maximum)?;
    let hash = object_hash("snapshot", &snapshot)?;
    for d in members {
        store_object(d, "snapshot", &snapshot)?;
    }
    Ok((hash, snapshot))
}
pub(crate) fn load_snapshot(d: &DiskConfig, hash: &str) -> Result<State> {
    let snapshot: Snapshot = load_object(d, "snapshot", hash)?;
    if snapshot.format_version != 2 {
        return Err(Error::new(
            "FORMAT_UNSUPPORTED",
            "snapshot version unsupported",
        ));
    }
    let mut out = snapshot.scalars;
    for (field, hashes) in snapshot.pages {
        let mut map = serde_json::Map::new();
        for h in hashes {
            let page: serde_json::Map<String, serde_json::Value> =
                load_object(d, "state-page", &h)?;
            for (k, v) in page {
                if map.insert(k, v).is_some() {
                    return Err(Error::new("OBJECT_CORRUPT", "duplicate snapshot entity"));
                }
            }
        }
        out.insert(field, serde_json::Value::Object(map));
    }
    let state: State = serde_json::from_value(serde_json::Value::Object(out))?;
    if state.sequence != snapshot.sequence {
        return Err(Error::new("OBJECT_CORRUPT", "snapshot sequence differs"));
    }
    Ok(state)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct VersionEvidence {
    pub version: FileVersion,
    pub placement: Placement,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Candidate {
    pub format_version: u32,
    pub pool_id: String,
    #[serde(with = "xcs_contract::u64_string")]
    pub sequence: u64,
    pub transaction_id: String,
    pub operation_id: String,
    pub request_digest: String,
    pub previous_snapshot: Option<String>,
    pub snapshot_hash: String,
    pub versions: Vec<VersionEvidence>,
    pub metadata_disk_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub history_manifest_hashes: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Finalize {
    pub candidate_hash: String,
    pub accepts: Vec<Receipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_id: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PayloadReceipt {
    pub version_id: String,
    pub disk_id: String,
    pub slot_id: String,
    pub proof_hash: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CommitProof {
    pub candidate_hash: String,
    pub finalize: Finalize,
    pub version_hash: String,
    pub disk_id: String,
    pub slot_id: String,
    pub route_slot_ids: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Publication {
    pub candidate_hash: String,
    pub finalize: Finalize,
    pub finalize_receipts: Vec<Receipt>,
    pub payload_receipts: Vec<PayloadReceipt>,
    pub result: OperationResult,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery: Option<crate::degraded_recovery::RecoveryProof>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PublicationSeal {
    pub candidate_hash: String,
    pub publication_hash: String,
    pub receipts: Vec<Receipt>,
}
pub(crate) fn publication_seal_path(d: &DiskConfig, candidate: &str) -> PathBuf {
    d.root
        .join("metadata/publication-seals")
        .join(format!("{candidate}.json"))
}
pub(crate) fn verify_publication_seal(
    c: &Candidate,
    p: &Publication,
    seal: &PublicationSeal,
) -> Result<()> {
    let candidate = object_hash("candidate", c)?;
    let publication = object_hash("publication", p)?;
    let finalize = object_hash("finalize", &p.finalize)?;
    let mut ids = c.metadata_disk_ids.clone();
    ids.sort();
    ids.dedup();
    let mut actual = seal
        .receipts
        .iter()
        .map(|r| r.disk_id.clone())
        .collect::<Vec<_>>();
    actual.sort();
    actual.dedup();
    let member_ids = |receipts: &[Receipt]| {
        let mut out = receipts
            .iter()
            .map(|r| r.disk_id.clone())
            .collect::<Vec<_>>();
        out.sort();
        out.dedup();
        out
    };
    if ids.len() != 3
        || c.metadata_disk_ids.len() != 3
        || actual != ids
        || (p.recovery.is_none()
            && (member_ids(&p.finalize.accepts) != ids || p.finalize.accepts.len() != 3))
        || member_ids(&p.finalize_receipts) != ids
        || p.finalize_receipts.len() != 3
        || seal.receipts.len() != 3
        || seal.candidate_hash != candidate
        || seal.publication_hash != publication
        || p.candidate_hash != candidate
        || p.finalize.candidate_hash != candidate
        || p.result.operation_id != c.operation_id
        || p.result.request_digest != c.request_digest
        || p.result.sequence != c.sequence
        || (p.recovery.is_none()
            && (p.result.commit_state != "PUBLISHED" || p.finalize.recovery_id.is_some()))
    {
        return Err(Error::new(
            "PUBLICATION_INVALID",
            "three synchronized publication receipts are required",
        ));
    }
    if let Some(recovery) = &p.recovery {
        recovery.verify(c, p)?;
    }
    let mut slots = std::collections::BTreeSet::new();
    for r in &seal.receipts {
        let f = p
            .finalize_receipts
            .iter()
            .find(|f| f.disk_id == r.disk_id)
            .expect("member sets checked");
        let predecessor = if let Some(recovery) = &p.recovery {
            let fence = recovery
                .reprotect
                .iter()
                .find(|fence| fence.disk_id == r.disk_id)
                .expect("recovery member sets checked");
            if fence.slot_id != r.slot_id {
                return Err(Error::new(
                    "PUBLICATION_INVALID",
                    "recovery fence slot differs",
                ));
            }
            fence.frame_hash.as_str()
        } else {
            let accept = p
                .finalize
                .accepts
                .iter()
                .find(|a| a.disk_id == r.disk_id)
                .expect("member sets checked");
            if accept.slot_id != r.slot_id
                || !receipt_binds(accept, "ACCEPT", c.sequence, &candidate, &candidate)?
            {
                return Err(Error::new("PUBLICATION_INVALID", "ACCEPT receipt differs"));
            }
            accept.frame_hash.as_str()
        };
        if !slots.insert(r.slot_id.clone())
            || r.slot_id != f.slot_id
            || f.record.previous_frame_hash != predecessor
            || r.record.previous_frame_hash != f.frame_hash
            || !receipt_binds(f, "FINALIZE", c.sequence, &candidate, &finalize)?
        {
            return Err(Error::new(
                "PUBLICATION_INVALID",
                "member synchronized chain is invalid",
            ));
        }
        if !receipt_binds(r, "PUBLICATION", c.sequence, &candidate, &publication)? {
            return Err(Error::new(
                "PUBLICATION_INVALID",
                "publication frame receipt is invalid",
            ));
        }
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct LocalPublication {
    pub candidate: Candidate,
    pub publication: Publication,
    pub commit_proof: CommitProof,
    pub publication_receipts: Vec<Receipt>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContentTombstone {
    pub version_id: String,
    pub state: String,
    pub candidate: Candidate,
    pub publication: Publication,
    pub publication_seal: PublicationSeal,
    pub snapshot: Snapshot,
    pub content_state_page: serde_json::Map<String, serde_json::Value>,
}
pub(crate) fn verify_tombstone(directory: &Path, version: &FileVersion) -> Result<()> {
    let t: ContentTombstone = read_json(&directory.join("purged.json"))?;
    verify_publication_seal(&t.candidate, &t.publication, &t.publication_seal)?;
    let root = directory
        .ancestors()
        .nth(4)
        .ok_or_else(|| Error::new("PUBLICATION_INVALID", "invalid version location"))?;
    let seal: DiskSeal = read_json(&root.join("disk.json"))?;
    if t.version_id != version.version_id
        || !["PURGED", "MIGRATED"].contains(&t.state.as_str())
        || t.candidate.pool_id != seal.pool_id
        || t.publication.candidate_hash != object_hash("candidate", &t.candidate)?
        || object_hash("snapshot", &t.snapshot)? != t.candidate.snapshot_hash
        || t.snapshot.sequence != t.candidate.sequence
        || t.content_state_page
            .get(&t.version_id)
            .and_then(|v| v.as_str())
            != Some(t.state.as_str())
        || !t.snapshot.pages.get("content_states").is_some_and(|pages| {
            object_hash("state-page", &t.content_state_page).is_ok_and(|hash| pages.contains(&hash))
        })
    {
        return Err(Error::new(
            "TOMBSTONE_INVALID",
            "content tombstone is not covered by a published state",
        ));
    }
    Ok(())
}

/// A later protection transaction reuses immutable FULL identities. Preserve
/// the first locally published proof and retain each later receipt by candidate.
pub(crate) fn local_proof_directory(directory: &Path, candidate: &str) -> Result<PathBuf> {
    let original = directory.join("commit-proof.json");
    if original.try_exists()? {
        let proof: CommitProof = read_json(&original)?;
        if proof.candidate_hash != candidate {
            return Ok(directory.join("protection-proofs").join(candidate));
        }
    }
    Ok(directory.to_owned())
}

pub(crate) fn verify_local_publication(directory: &Path, version: &FileVersion) -> Result<()> {
    if directory.file_name().and_then(|s| s.to_str()) != Some(version.version_id.as_str())
        || directory
            .ancestors()
            .nth(1)
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            != Some("versions")
        || directory
            .ancestors()
            .nth(2)
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            != Some(version.history_tree_id.as_str())
        || directory
            .ancestors()
            .nth(3)
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            != Some("histories")
    {
        return Err(Error::new(
            "PUBLICATION_INVALID",
            "version directory identity differs from its descriptor",
        ));
    }
    let local: LocalPublication = read_json(&directory.join("publication-proof.json"))?;
    let proof: CommitProof = read_json(&directory.join("commit-proof.json"))?;
    let candidate_hash = object_hash("candidate", &local.candidate)?;
    let c = &local.candidate;
    let p = &local.publication;
    verify_publication_seal(
        c,
        p,
        &PublicationSeal {
            candidate_hash: candidate_hash.clone(),
            publication_hash: object_hash("publication", p)?,
            receipts: local.publication_receipts.clone(),
        },
    )?;
    let root = directory
        .ancestors()
        .nth(4)
        .ok_or_else(|| Error::new("PUBLICATION_INVALID", "version is outside disk layout"))?;
    let seal: DiskSeal = read_json(&root.join("disk.json"))?;
    let finalize_hash = object_hash("finalize", &p.finalize)?;
    if c.format_version != 2
        || p.candidate_hash != candidate_hash
        || proof.candidate_hash != candidate_hash
        || object_hash("commit-proof", &proof)? != object_hash("commit-proof", &local.commit_proof)?
        || proof.version_hash != object_hash("version", version)?
        || seal.pool_id != c.pool_id
        || seal.disk_id != proof.disk_id
        || seal.slot_id != proof.slot_id
        || object_hash("finalize", &proof.finalize)? != finalize_hash
    {
        return Err(Error::new(
            "PUBLICATION_INVALID",
            "local proof does not bind the version",
        ));
    }
    let evidence = c
        .versions
        .iter()
        .find(|e| e.version.version_id == version.version_id)
        .ok_or_else(|| Error::new("PUBLICATION_INVALID", "version is absent from candidate"))?;
    if object_hash("version", &evidence.version)? != object_hash("version", version)?
        || evidence.placement.route_slot_ids != proof.route_slot_ids
    {
        return Err(Error::new(
            "PUBLICATION_INVALID",
            "candidate placement mismatch",
        ));
    }
    let receipt = p
        .payload_receipts
        .iter()
        .find(|r| {
            r.version_id == version.version_id
                && r.disk_id == proof.disk_id
                && r.slot_id == proof.slot_id
        })
        .ok_or_else(|| Error::new("PUBLICATION_INVALID", "local payload receipt absent"))?;
    if receipt.proof_hash != object_hash("commit-proof", &proof)? {
        return Err(Error::new(
            "PUBLICATION_INVALID",
            "local receipt digest differs",
        ));
    }
    for slot in &evidence.placement.route_slot_ids {
        if !p
            .payload_receipts
            .iter()
            .any(|r| r.version_id == version.version_id && &r.slot_id == slot)
        {
            return Err(Error::new(
                "PUBLICATION_INVALID",
                "a required route receipt is absent",
            ));
        }
    }
    if directory.join("purged.json").exists() {
        return Err(Error::new(
            "CONTENT_PURGED",
            "this version has a local content tombstone",
        ));
    }
    Ok(())
}
