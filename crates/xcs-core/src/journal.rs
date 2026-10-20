use crate::{disk::*, *};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
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
#[cfg(test)]
thread_local! {
    static SCAN_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
pub(crate) fn scan(d: &DiskConfig) -> Result<Scan> {
    #[cfg(test)]
    SCAN_COUNT.with(|count| count.set(count.get() + 1));
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
// A process-local optimization, never a recovery authority. Keep only the current
// decision slot per journal; older receipt retries deliberately use a full scan.
// Device/inode plus ctime detect replacement, truncation and same-size rewrites
// (including writers that restore mtime). Restart always verifies from byte zero.
#[derive(Clone, Debug, PartialEq, Eq)]
struct FileStamp {
    dev: u64,
    ino: u64,
    len: u64,
    mtime: (i64, i64),
    ctime: (i64, i64),
}
impl FileStamp {
    fn of(file: &std::fs::File) -> Result<Self> {
        let m = file.metadata()?;
        Ok(Self {
            dev: m.dev(),
            ino: m.ino(),
            len: m.len(),
            mtime: (m.mtime(), m.mtime_nsec()),
            ctime: (m.ctime(), m.ctime_nsec()),
        })
    }
}
struct VerifiedTail {
    stamp: FileStamp,
    records: Vec<(Record, String)>,
}
const MAX_CACHED_JOURNALS: usize = 64;
fn tails() -> &'static Mutex<BTreeMap<PathBuf, VerifiedTail>> {
    static TAILS: OnceLock<Mutex<BTreeMap<PathBuf, VerifiedTail>>> = OnceLock::new();
    TAILS.get_or_init(|| Mutex::new(BTreeMap::new()))
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
    // Serialize cache lookup and append together. A failed operation removes its
    // entry before touching disk, so retries must verify any partial write.
    let mut tails = tails()
        .lock()
        .map_err(|_| Error::new("RECOVERY_REQUIRED", "journal cache lock poisoned"))?;
    let cached = tails.remove(&path);
    let mut file = open_append_regular(&path)?;
    let before = FileStamp::of(&file)?;
    let scanned = match cached {
        Some(tail)
            if tail.stamp == before
                && tail
                    .records
                    .last()
                    .is_some_and(|(r, _)| sequence >= r.sequence) =>
        {
            Scan {
                records: tail.records,
                valid_bytes: before.len,
                torn_tail: false,
            }
        }
        _ => scan(d)?,
    };
    // Reject concurrent replacement/modification while scanning. Engine locks
    // serialize supported writers; this also catches accidental outside writes.
    if FileStamp::of(&file)? != before || FileStamp::of(&open_regular(&path)?)? != before {
        return Err(Error::new(
            "RECOVERY_REQUIRED",
            "journal changed during verification",
        ));
    }
    if let Some((record, hash)) = scanned.records.iter().find(|(r, _)| {
        r.sequence == sequence
            && r.record_type == kind
            && r.candidate_hash == candidate
            && r.object_hash == object
    }) {
        let receipt = Receipt {
            disk_id: d.disk_id.clone(),
            slot_id: d.slot_id.clone(),
            frame_hash: hash.clone(),
            record: record.clone(),
        };
        if !scanned.torn_tail {
            cache_tail(&mut tails, path, before, scanned.records);
        }
        return Ok(receipt);
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
    if scanned
        .records
        .last()
        .is_some_and(|(r, _)| r.sequence > sequence)
    {
        return Err(Error::new(
            "LOG_FORK",
            "cannot append an older decision slot",
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
    if scanned.torn_tail {
        file.set_len(scanned.valid_bytes)?;
        file.sync_all()?;
    }
    file.write_all(&header)?;
    file.write_all(&bytes)?;
    file.write_all(digest.as_bytes())?;
    file.sync_all()?;
    sync_dir(path.parent().expect("log parent"))?;
    let stamp = FileStamp::of(&file)?;
    if stamp.len != scanned.valid_bytes + header.len() as u64 + bytes.len() as u64 + 32
        || FileStamp::of(&open_regular(&path)?)? != stamp
    {
        return Err(Error::new(
            "RECOVERY_REQUIRED",
            "journal changed during append",
        ));
    }
    let mut records = scanned.records;
    records.push((record.clone(), digest.to_hex().to_string()));
    cache_tail(&mut tails, path, stamp, records);
    Ok(Receipt {
        disk_id: d.disk_id.clone(),
        slot_id: d.slot_id.clone(),
        frame_hash: digest.to_hex().to_string(),
        record,
    })
}
fn cache_tail(
    tails: &mut BTreeMap<PathBuf, VerifiedTail>,
    path: PathBuf,
    stamp: FileStamp,
    mut records: Vec<(Record, String)>,
) {
    if let Some((last, _)) = records.last() {
        let sequence = last.sequence;
        records.retain(|(r, _)| r.sequence == sequence);
    }
    // A malformed workload may place arbitrarily many records in one slot.
    // Do not retain an unbounded receipt set in that case.
    let retained_bytes = records
        .iter()
        .map(|(r, hash)| {
            r.record_type.len()
                + r.candidate_hash.len()
                + r.object_hash.len()
                + r.previous_frame_hash.len()
                + hash.len()
        })
        .sum::<usize>();
    if records.len() > 64 || retained_bytes > 64 * 1024 {
        return;
    }
    // Do not retain the allocation of a full historical scan.
    records.shrink_to_fit();
    // Bound cache size across unrelated engines; eviction only costs a rescan.
    if tails.len() >= MAX_CACHED_JOURNALS {
        tails.pop_first();
    }
    tails.insert(path, VerifiedTail { stamp, records });
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
/// Encode an immutable object once for all replicas, retaining the same hash
/// function, reader bounds, atomic publication and existing-byte checks.
pub(crate) fn store_object_replicas_bounded<T: Serialize + ?Sized>(
    members: &[DiskConfig],
    kind: &str,
    value: &T,
    maximum: usize,
) -> Result<String> {
    let maximum = if members.is_empty() {
        maximum
    } else {
        maximum.min(MAX_METADATA)
    };
    let (bytes, hash) = encode_hashed_object(kind, value)?;
    if bytes.len() > maximum {
        return Err(Error::new(
            "RESOURCE_LIMIT",
            format!("serialized metadata exceeds {maximum} byte reader limit"),
        ));
    }
    for disk in members {
        write_atomic(&object_path(disk, kind, &hash), &bytes, false)?;
    }
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
/// Content-defined key boundaries keep an insertion from shifting every later
/// page (about 32 entries per page for uniformly distributed key hashes).
/// Values do not participate: updating an entity only rewrites its page.
/// The hard cap bounds adversarial keys without changing the v2 on-disk format.
/// Readers intentionally accept both these pages and legacy fixed-count pages.
pub(crate) fn state_pages(
    map: &serde_json::Map<String, serde_json::Value>,
) -> impl Iterator<Item = serde_json::Map<String, serde_json::Value>> + '_ {
    let mut entries = map.iter().peekable();
    std::iter::from_fn(move || {
        let mut page = serde_json::Map::new();
        while let Some((key, _)) = entries.peek() {
            let boundary = blake3::hash(key.as_bytes()).as_bytes()[0] & 31 == 0;
            if !page.is_empty() && (page.len() == STATE_PAGE_ENTRIES || boundary) {
                break;
            }
            let (key, value) = entries.next().expect("peeked entry");
            page.insert(key.clone(), value.clone());
        }
        (!page.is_empty()).then_some(page)
    })
}

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
            for page in state_pages(map) {
                let hash = store_object_replicas_bounded(members, "state-page", &page, maximum)?;
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
    let hash = store_object_replicas_bounded(members, "snapshot", &snapshot, maximum)?;
    Ok((hash, snapshot))
}
#[cfg(test)]
thread_local! {
    pub(crate) static SNAPSHOT_LOAD_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
pub(crate) fn load_snapshot(d: &DiskConfig, hash: &str) -> Result<State> {
    #[cfg(test)]
    SNAPSHOT_LOAD_COUNT.with(|count| count.set(count.get() + 1));
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

#[cfg(test)]
mod replication_tests {
    use super::*;
    use std::cell::Cell;

    struct CountedValue {
        value: serde_json::Value,
        serializations: Cell<usize>,
    }
    impl Serialize for CountedValue {
        fn serialize<S: serde::Serializer>(
            &self,
            serializer: S,
        ) -> std::result::Result<S::Ok, S::Error> {
            self.serializations.set(self.serializations.get() + 1);
            self.value.serialize(serializer)
        }
    }

    #[test]
    fn replicated_metadata_matches_original_bytes_and_hash_with_less_serialization() {
        let temp = tempfile::tempdir().unwrap();
        let config = Config::development([
            temp.path().join("primary"),
            temp.path().join("backup"),
            temp.path().join("tertiary"),
        ]);
        let value = CountedValue {
            value: serde_json::json!({"z": "18446744073709551615", "名字": [true, null, "data"], "a": 1}),
            serializations: Cell::new(0),
        };
        // Reproduce the prior snapshot/history-manifest storage algorithm.
        let original_bytes = encode_metadata(&value, MAX_METADATA).unwrap();
        let original_hash = object_hash("state-page", &value).unwrap();
        for disk in &config.disks {
            assert_eq!(
                store_object(disk, "state-page", &value).unwrap(),
                original_hash
            );
        }
        let original_serializations = value.serializations.replace(0);
        let config = Config::development([
            temp.path().join("new-primary"),
            temp.path().join("new-backup"),
            temp.path().join("new-tertiary"),
        ]);
        let hash = store_object_replicas_bounded(&config.disks, "state-page", &value, MAX_METADATA)
            .unwrap();
        assert_eq!(hash, original_hash);
        // Canonical encoding validates then serializes: eight encodings become one.
        assert_eq!(original_serializations, 16);
        assert_eq!(value.serializations.get(), 2);
        for disk in &config.disks {
            assert_eq!(
                read_bytes(&object_path(disk, "state-page", &hash), MAX_METADATA).unwrap(),
                original_bytes
            );
            assert_eq!(
                load_object::<serde_json::Value>(disk, "state-page", &hash).unwrap(),
                value.value
            );
        }
        // Existing immutable bytes are still compared rather than overwritten.
        let path = object_path(&config.disks[0], "state-page", &hash);
        let mut damaged = original_bytes.clone();
        damaged[0] = b'!';
        std::fs::write(&path, &damaged).unwrap();
        assert_eq!(
            store_object_replicas_bounded(&config.disks, "state-page", &value, MAX_METADATA)
                .unwrap_err()
                .code(),
            "OBJECT_COLLISION"
        );
        assert_eq!(std::fs::read(path).unwrap(), damaged);
    }

    #[test]
    fn replicated_metadata_rejects_oversize_before_creating_any_object() {
        let temp = tempfile::tempdir().unwrap();
        let config = Config::development([
            temp.path().join("primary"),
            temp.path().join("backup"),
            temp.path().join("tertiary"),
        ]);
        let value = serde_json::json!({"content": "bounded"});
        let bytes = canonical_json(&value).unwrap();
        let error =
            store_object_replicas_bounded(&config.disks, "snapshot", &value, bytes.len() - 1)
                .unwrap_err();
        assert_eq!(error.code(), "RESOURCE_LIMIT");
        assert!(config.disks.iter().all(|disk| !disk.root.exists()));
    }
}

#[cfg(test)]
mod tail_cache_tests {
    use super::*;
    use std::io::{Seek, SeekFrom};

    fn disk(temp: &tempfile::TempDir) -> DiskConfig {
        Config::development([
            temp.path().join("one"),
            temp.path().join("two"),
            temp.path().join("three"),
        ])
        .disks
        .remove(0)
    }
    fn put(d: &DiskConfig, kind: &str, sequence: u64) -> Receipt {
        append(
            d,
            kind,
            sequence,
            &format!("{sequence:064x}"),
            &"b".repeat(64),
        )
        .unwrap()
    }

    #[test]
    fn latest_receipts_are_idempotent_and_old_retries_rescan() {
        let temp = tempfile::tempdir().unwrap();
        let d = disk(&temp);
        let first = put(&d, "PREPARE", 1);
        let latest = put(&d, "ACCEPT", 1);
        assert_eq!(put(&d, "PREPARE", 1).frame_hash, first.frame_hash);
        assert_eq!(put(&d, "ACCEPT", 1).frame_hash, latest.frame_hash);
        put(&d, "PREPARE", 2);
        let len = std::fs::metadata(log_path(&d)).unwrap().len();
        assert_eq!(put(&d, "PREPARE", 1).frame_hash, first.frame_hash);
        assert_eq!(std::fs::metadata(log_path(&d)).unwrap().len(), len);
        assert_eq!(scan(&d).unwrap().records.len(), 3);
        assert_eq!(
            append(&d, "FINALIZE", 1, &format!("{:064x}", 1), &"b".repeat(64))
                .unwrap_err()
                .code,
            "LOG_FORK"
        );
    }

    #[test]
    fn cached_append_detects_same_length_prefix_corruption() {
        let temp = tempfile::tempdir().unwrap();
        let d = disk(&temp);
        put(&d, "PREPARE", 1);
        put(&d, "PREPARE", 2);
        let path = log_path(&d);
        let old = std::fs::metadata(&path).unwrap();
        let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.seek(SeekFrom::Start(20)).unwrap();
        file.write_all(b"!").unwrap();
        file.set_times(std::fs::FileTimes::new().set_modified(old.modified().unwrap()))
            .unwrap();
        file.sync_all().unwrap();
        assert_eq!(
            append(&d, "ACCEPT", 2, &format!("{:064x}", 2), &"b".repeat(64))
                .unwrap_err()
                .code,
            "LOG_CORRUPT"
        );
    }

    #[test]
    fn cached_append_repairs_torn_external_tail() {
        let temp = tempfile::tempdir().unwrap();
        let d = disk(&temp);
        put(&d, "PREPARE", 1);
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(log_path(&d))
            .unwrap();
        file.write_all(b"XCSLO").unwrap();
        file.sync_all().unwrap();
        put(&d, "ACCEPT", 1);
        let scanned = scan(&d).unwrap();
        assert!(!scanned.torn_tail);
        assert_eq!(scanned.records.len(), 2);
    }

    #[test]
    fn replacement_and_truncation_invalidate_cached_receipts() {
        let temp = tempfile::tempdir().unwrap();
        let d = disk(&temp);
        put(&d, "PREPARE", 1);
        let old = put(&d, "ACCEPT", 1);
        let path = log_path(&d);
        std::fs::rename(&path, path.with_extension("old")).unwrap();
        let fresh = put(&d, "ACCEPT", 1);
        assert_ne!(fresh.frame_hash, old.frame_hash);
        assert_eq!(scan(&d).unwrap().records.len(), 1);
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(0)
            .unwrap();
        put(&d, "PREPARE", 2);
        let scanned = scan(&d).unwrap();
        assert_eq!(scanned.records.len(), 1);
        assert_eq!(scanned.records[0].0.sequence, 2);
    }

    #[test]
    fn cold_append_revalidates_chain_and_cache_retains_only_latest_slot() {
        // Keep this key after ordinary .tmp* fixtures in deterministic eviction
        // order, so unrelated parallel tests cannot evict this measured tail.
        let temp = tempfile::Builder::new()
            .prefix("zzzz-tail-")
            .tempdir()
            .unwrap();
        let d = disk(&temp);
        let before = SCAN_COUNT.with(|count| count.get());
        for sequence in 1..=40 {
            put(&d, "PREPARE", sequence);
            put(&d, "ACCEPT", sequence);
        }
        assert_eq!(
            SCAN_COUNT.with(|count| count.get()) - before,
            1,
            "ordinary appends must scan only the initial cold journal"
        );
        {
            let mut cache = tails().lock().unwrap();
            let tail = cache.remove(&log_path(&d)).unwrap();
            assert_eq!(tail.records.len(), 2);
            assert_eq!(tail.records.capacity(), 2);
            assert!(tail.records.iter().all(|(r, _)| r.sequence == 40));
        }
        put(&d, "FINALIZE", 40);
        assert_eq!(
            SCAN_COUNT.with(|count| count.get()) - before,
            2,
            "evicted/cold state must verify the full journal again"
        );
        assert_eq!(scan(&d).unwrap().records.len(), 81);
    }
}

#[cfg(test)]
mod snapshot_partition_tests {
    use super::*;
    use std::collections::BTreeSet;

    fn partition(
        map: &serde_json::Map<String, serde_json::Value>,
    ) -> Vec<serde_json::Map<String, serde_json::Value>> {
        state_pages(map).collect()
    }

    fn synthetic_map() -> serde_json::Map<String, serde_json::Value> {
        (0u32..4096)
            .map(|n| {
                let key = blake3::hash(&n.to_le_bytes()).to_hex().to_string();
                (key, serde_json::json!({"data": "immutable".repeat(64)}))
            })
            .collect()
    }
    fn hashes(pages: &[serde_json::Map<String, serde_json::Value>]) -> BTreeSet<String> {
        pages
            .iter()
            .map(|page| object_hash("state-page", page).unwrap())
            .collect()
    }

    #[test]
    fn deterministic_random_key_insertion_and_update_reuse_unaffected_pages() {
        let original = synthetic_map();
        let pages = partition(&original);
        assert!(pages.len() > 50);
        assert!(
            pages
                .iter()
                .all(|page| !page.is_empty() && page.len() <= STATE_PAGE_ENTRIES)
        );
        assert_eq!(
            pages
                .iter()
                .flat_map(|p| p.clone())
                .collect::<serde_json::Map<_, _>>(),
            original
        );
        let before = hashes(&pages);
        // Choose a non-boundary new key inside a page that cannot overflow.
        let key = (4096u32..8192)
            .map(|n| blake3::hash(&n.to_le_bytes()).to_hex().to_string())
            .find(|key| {
                blake3::hash(key.as_bytes()).as_bytes()[0] & 31 != 0
                    && pages.iter().any(|page| {
                        page.len() < STATE_PAGE_ENTRIES
                            && page.keys().next().unwrap() < key
                            && key < page.keys().last().unwrap()
                    })
            })
            .unwrap();
        let mut changed = original.clone();
        changed.insert(key.clone(), serde_json::json!("new"));
        let after = hashes(&partition(&changed));
        assert_eq!(before.difference(&after).count(), 1);
        assert_eq!(after.difference(&before).count(), 1);
        // Values cannot influence boundaries, even when their size changes.
        changed.insert(key, serde_json::json!("updated".repeat(1024)));
        let updated = hashes(&partition(&changed));
        assert_eq!(after.difference(&updated).count(), 1);
        assert_eq!(updated.difference(&after).count(), 1);
        // Insertion order has no influence on the content-addressed pages.
        let reverse = changed.into_iter().rev().collect();
        assert_eq!(updated, hashes(&partition(&reverse)));
    }

    #[test]
    fn adversarial_boundary_free_keys_remain_bounded_and_lossless() {
        let map = (0..10000)
            .map(|n| format!("key-{n:05}"))
            .filter(|key| blake3::hash(key.as_bytes()).as_bytes()[0] & 31 != 0)
            .take(STATE_PAGE_ENTRIES * 3 + 1)
            .map(|key| (key, serde_json::json!("value")))
            .collect::<serde_json::Map<_, _>>();
        let pages = partition(&map);
        assert_eq!(
            pages.iter().map(|p| p.len()).collect::<Vec<_>>(),
            vec![128, 128, 128, 1]
        );
        assert_eq!(
            pages
                .into_iter()
                .flatten()
                .collect::<serde_json::Map<_, _>>(),
            map
        );
        assert!(partition(&serde_json::Map::new()).is_empty());
    }

    #[test]
    fn legacy_and_content_defined_snapshots_restore_identical_full_state() {
        let temp = tempfile::tempdir().unwrap();
        let config = Config::development([
            temp.path().join("one"),
            temp.path().join("two"),
            temp.path().join("three"),
        ]);
        let disk = &config.disks[0];
        let state = State {
            sequence: 123,
            business_generation: 19,
            content_states: synthetic_map()
                .into_iter()
                .map(|(key, _)| key)
                .enumerate()
                .map(|(n, key)| (key, if n % 3 == 0 { "PURGED" } else { "AVAILABLE" }.into()))
                .collect(),
            file_notes: [("文件".into(), "preserved note".into())].into(),
            control_auth: Some(serde_json::json!({"nested": {"roles": ["reader"]}})),
            ..Default::default()
        };
        let value = serde_json::to_value(&state).unwrap();
        let mut legacy = Snapshot {
            format_version: 2,
            sequence: state.sequence,
            pages: BTreeMap::new(),
            scalars: serde_json::Map::new(),
        };
        for (field, value) in value.as_object().unwrap() {
            if let Some(map) = value.as_object() {
                let entries = map.iter().collect::<Vec<_>>();
                let pages = entries
                    .chunks(STATE_PAGE_ENTRIES)
                    .map(|chunk| {
                        let page = chunk
                            .iter()
                            .map(|(k, v)| ((*k).clone(), (*v).clone()))
                            .collect::<serde_json::Map<_, _>>();
                        store_object(disk, "state-page", &page).unwrap()
                    })
                    .collect();
                legacy.pages.insert(field.clone(), pages);
            } else {
                legacy.scalars.insert(field.clone(), value.clone());
            }
        }
        let legacy_hash = store_object(disk, "snapshot", &legacy).unwrap();
        let modern_hash = store_snapshot(std::slice::from_ref(disk), &state).unwrap();
        assert_ne!(legacy_hash, modern_hash);
        assert_eq!(load_snapshot(disk, &legacy_hash).unwrap(), state);
        assert_eq!(load_snapshot(disk, &modern_hash).unwrap(), state);
        // Writing a later state must not change either retained snapshot.
        let mut next = state.clone();
        next.sequence += 1;
        next.content_states
            .insert("new-version".into(), "MIGRATED".into());
        let next_hash = store_snapshot(std::slice::from_ref(disk), &next).unwrap();
        assert_eq!(load_snapshot(disk, &next_hash).unwrap(), next);
        assert_eq!(load_snapshot(disk, &legacy_hash).unwrap(), state);
        assert_eq!(load_snapshot(disk, &modern_hash).unwrap(), state);
    }
}
