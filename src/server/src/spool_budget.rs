//! Recoverable admission for the HTTP spool, independent of engine/SMB budgets.
//!
//! Reservations precede body consumption. Their owner stays alive with every
//! blocking writer; dropping one settles to retained physical file length, not
//! upload success. Unknown files and uncertain I/O never earn free capacity.
use crate::{ApiError, lock, uploads::UploadRecord};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

/// Covers owner, current/retained progress and one atomic metadata replacement.
/// This also bounds zero-byte uploads. It is a conservative logical charge,
/// not a claim about filesystem allocation, compression or sparse extents.
pub(crate) const METADATA_ALLOWANCE: u64 = 64 * 1024;
const MAX_METADATA: u64 = 16 * 1024;

#[derive(Clone, Debug)]
pub struct SpoolLimits {
    pub bytes_per_account: u64,
    pub bytes_total: u64,
    /// Free filesystem bytes kept outside all admitted upload reservations.
    pub recovery_reserve_bytes: u64,
}
impl Default for SpoolLimits {
    fn default() -> Self {
        Self {
            bytes_per_account: 100 * 1024 * 1024 * 1024,
            bytes_total: 500 * 1024 * 1024 * 1024,
            recovery_reserve_bytes: 1024 * 1024 * 1024,
        }
    }
}
impl SpoolLimits {
    pub(crate) fn validate(&self) -> Result<(), ApiError> {
        if self.bytes_per_account < METADATA_ALLOWANCE
            || self.bytes_total < self.bytes_per_account
            || self.recovery_reserve_bytes == 0
        {
            return Err(ApiError::invalid(
                "HTTP spool limits require account >= 65536, total >= account, and positive recovery reserve",
            ));
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Owner {
    data_file: String,
    principal: String,
}
struct Charge {
    owner: Option<String>,
    bytes: u64,
    overhead: u64,
    /// Keep the whole reservation in the free-space calculation while active.
    /// Written bytes may therefore be counted twice there, conservatively.
    reserved: u64,
}
#[derive(Default)]
struct Ledger {
    files: BTreeMap<String, Charge>,
    uncertain: bool,
    total: u64,
    unknown: u64,
    accounts: BTreeMap<String, u64>,
    reserved: u64,
}
impl Ledger {
    fn rebuild_totals(&mut self) -> Result<(), ApiError> {
        self.total = 0;
        self.unknown = 0;
        self.reserved = 0;
        self.accounts.clear();
        for entry in self.files.values() {
            self.total = self
                .total
                .checked_add(entry.bytes)
                .ok_or_else(|| invalid_state("spool accounting overflow"))?;
            if let Some(owner) = &entry.owner {
                let count = self.accounts.entry(owner.clone()).or_default();
                *count = count
                    .checked_add(entry.bytes)
                    .ok_or_else(|| invalid_state("spool account overflow"))?;
            } else {
                self.unknown = self
                    .unknown
                    .checked_add(entry.bytes)
                    .ok_or_else(|| invalid_state("spool accounting overflow"))?;
            }
            self.reserved = self
                .reserved
                .checked_add(entry.reserved)
                .ok_or_else(|| invalid_state("spool reservation overflow"))?;
        }
        Ok(())
    }
    /// All live accounting changes use this one transition. Ownership never
    /// changes through admission; recovery is the only attribution boundary.
    fn replace(&mut self, name: &str, next: Option<Charge>) -> Result<(), ApiError> {
        let old = self.files.get(name);
        let owner = next
            .as_ref()
            .and_then(|entry| entry.owner.as_ref())
            .or_else(|| old.and_then(|entry| entry.owner.as_ref()))
            .cloned();
        if old
            .zip(next.as_ref())
            .is_some_and(|(old, next)| old.owner != next.owner)
        {
            return Err(invalid_state(
                "spool ownership cannot change during admission",
            ));
        }
        let old_bytes = old.map_or(0, |entry| entry.bytes);
        let new_bytes = next.as_ref().map_or(0, |entry| entry.bytes);
        let old_reserved = old.map_or(0, |entry| entry.reserved);
        let new_reserved = next.as_ref().map_or(0, |entry| entry.reserved);
        let update = |value: u64, before: u64, after: u64| {
            value
                .checked_sub(before)
                .and_then(|value| value.checked_add(after))
                .ok_or_else(|| invalid_state("spool accounting overflow"))
        };
        let total = update(self.total, old_bytes, new_bytes)?;
        let reserved = update(self.reserved, old_reserved, new_reserved)?;
        let account = update(
            owner.as_ref().map_or(self.unknown, |owner| {
                self.accounts.get(owner).copied().unwrap_or(0)
            }),
            old_bytes,
            new_bytes,
        )?;
        self.total = total;
        self.reserved = reserved;
        if let Some(owner) = owner {
            if account == 0 {
                self.accounts.remove(&owner);
            } else {
                self.accounts.insert(owner, account);
            }
        } else {
            self.unknown = account;
        }
        match next {
            Some(entry) => {
                self.files.insert(name.to_owned(), entry);
            }
            None => {
                self.files.remove(name);
            }
        }
        Ok(())
    }
}
pub(crate) struct SpoolBudget {
    root: PathBuf,
    limits: SpoolLimits,
    ledger: Mutex<Ledger>,
    _directory_lock: fs::File,
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && !name.contains(['/', '\\'])
}
fn metadata<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .ok()?
        .take(MAX_METADATA + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_METADATA {
        return None;
    }
    storage_contract::decode_strict(&bytes).ok()
}
fn charge_bytes(bytes: u64) -> Result<u64, ApiError> {
    bytes
        .checked_add(METADATA_ALLOWANCE)
        .ok_or_else(|| ApiError::new(500, "UPLOAD_STATE_INVALID", "spool accounting overflow"))
}
fn invalid_state(message: &str) -> ApiError {
    ApiError::new(500, "UPLOAD_STATE_INVALID", message)
}
fn set_owner(
    owners: &mut BTreeMap<String, String>,
    name: String,
    owner: String,
) -> Result<(), ApiError> {
    if !valid_name(&name) || owner.is_empty() || owner.len() > 1024 {
        return Err(invalid_state("invalid spool ownership metadata"));
    }
    if owners.get(&name).is_some_and(|previous| previous != &owner) {
        return Err(invalid_state("conflicting spool ownership metadata"));
    }
    owners.insert(name, owner);
    Ok(())
}

impl SpoolBudget {
    pub(crate) fn recover(root: &Path, limits: SpoolLimits) -> Result<Arc<Self>, ApiError> {
        limits.validate()?;
        let mut options = OpenOptions::new();
        options.create(true).truncate(false).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options
                .mode(0o600)
                .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32);
        }
        let directory_lock = options
            .open(root.join(".budget.lock"))
            .map_err(ApiError::io)?;
        rustix::fs::flock(
            &directory_lock,
            rustix::fs::FlockOperation::NonBlockingLockExclusive,
        )
        .map_err(|_| invalid_state("HTTP spool is already owned by another server"))?;
        let mut files = BTreeMap::new();
        let mut owners = BTreeMap::new();
        let mut covered_metadata = BTreeMap::new();
        // Inventory, never cleanup. Corrupt/unrecognized regular files remain
        // charged as unknown rather than being silently discarded or ignored.
        for entry in fs::read_dir(root).map_err(ApiError::io)? {
            let entry = entry.map_err(ApiError::io)?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| invalid_state("spool filenames must be valid UTF-8"))?;
            let info = fs::symlink_metadata(entry.path()).map_err(ApiError::io)?;
            if !info.is_file() {
                return Err(invalid_state("spool contains a non-regular file"));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if info.nlink() != 1 {
                    return Err(invalid_state("spool contains an aliased data file"));
                }
            }
            if name == ".budget.lock" {
                continue;
            }
            files.insert(name.clone(), info.len());
            if name.ends_with(".owner.json") {
                if let Some(owner) = metadata::<Owner>(&entry.path()) {
                    if name != format!("{}.owner.json", owner.data_file) {
                        return Err(invalid_state("spool ownership filename mismatch"));
                    }
                    covered_metadata.insert(name, owner.data_file.clone());
                    set_owner(&mut owners, owner.data_file, owner.principal)?;
                }
            } else if name.contains(".json")
                && let Some(record) = metadata::<UploadRecord>(&entry.path())
            {
                covered_metadata.insert(name, record.data_file.clone());
                set_owner(&mut owners, record.data_file, record.principal)?;
            }
        }
        let mut ledger = Ledger::default();
        // A durable owner record with a missing data file still costs metadata.
        for (name, principal) in owners {
            let bytes = files.get(&name).copied().unwrap_or(0);
            ledger.files.insert(
                name,
                Charge {
                    owner: Some(principal),
                    bytes: charge_bytes(bytes)?,
                    overhead: METADATA_ALLOWANCE,
                    reserved: 0,
                },
            );
        }
        let mut metadata_sizes = BTreeMap::<String, u64>::new();
        for (name, bytes) in files {
            if ledger.files.contains_key(&name) {
                continue;
            }
            if let Some(data) = covered_metadata.get(&name) {
                let size = metadata_sizes.entry(data.clone()).or_default();
                *size = size
                    .checked_add(bytes)
                    .ok_or_else(|| invalid_state("spool accounting overflow"))?;
            } else {
                ledger.files.insert(
                    name,
                    Charge {
                        owner: None,
                        bytes: charge_bytes(bytes)?,
                        overhead: METADATA_ALLOWANCE,
                        reserved: 0,
                    },
                );
            }
        }
        // A damaged/legacy directory may contain more metadata than the normal
        // per-file allowance. Count the excess rather than assuming a bound.
        for (name, bytes) in metadata_sizes {
            if bytes > METADATA_ALLOWANCE {
                let entry = ledger
                    .files
                    .get_mut(&name)
                    .ok_or_else(|| invalid_state("missing spool owner"))?;
                entry.overhead = bytes;
                entry.bytes = entry
                    .bytes
                    .checked_add(bytes - METADATA_ALLOWANCE)
                    .ok_or_else(|| invalid_state("spool accounting overflow"))?;
            }
        }
        ledger.rebuild_totals()?;
        Ok(Arc::new(Self {
            root: root.to_owned(),
            limits,
            ledger: Mutex::new(ledger),
            _directory_lock: directory_lock,
        }))
    }

    pub(crate) fn reserve(
        self: &Arc<Self>,
        principal: &str,
        name: &str,
        length: u64,
    ) -> Result<Reservation, ApiError> {
        let available = rustix::fs::statvfs(&self.root)
            .map_err(std::io::Error::from)
            .map_err(ApiError::io)?;
        self.reserve_with_available(
            principal,
            name,
            length,
            available.f_bavail.saturating_mul(available.f_frsize),
        )
    }

    fn reserve_with_available(
        self: &Arc<Self>,
        principal: &str,
        name: &str,
        length: u64,
        available: u64,
    ) -> Result<Reservation, ApiError> {
        if !valid_name(name) || principal.is_empty() || principal.len() > 1024 {
            return Err(ApiError::invalid("invalid spool reservation identity"));
        }
        let mut ledger = lock(&self.ledger)?;
        if ledger.uncertain {
            return Err(invalid_state(
                "spool accounting requires restart and inventory",
            ));
        }
        let previous = ledger.files.get(name);
        if previous
            .is_some_and(|entry| entry.reserved != 0 || entry.owner.as_deref() != Some(principal))
        {
            return Err(invalid_state(
                "spool file is already reserved or has another owner",
            ));
        }
        let overhead = previous.map_or(METADATA_ALLOWANCE, |entry| entry.overhead);
        let requested = length
            .checked_add(overhead)
            .ok_or_else(|| invalid_state("spool accounting overflow"))?;
        let baseline = previous.map_or(0, |entry| entry.bytes);
        let bytes = requested.max(baseline);
        let growth = bytes - baseline;
        let total = ledger.total;
        let account = ledger
            .accounts
            .get(principal)
            .copied()
            .unwrap_or(0)
            .checked_add(ledger.unknown)
            .ok_or_else(|| invalid_state("spool accounting overflow"))?;
        if growth > self.limits.bytes_total.saturating_sub(total)
            || growth > self.limits.bytes_per_account.saturating_sub(account)
            || (growth > 0
                && (total > self.limits.bytes_total || account > self.limits.bytes_per_account))
        {
            return Err(ApiError::limit(
                "HTTP spool retained data and reservations exceed the account or total budget",
            ));
        }
        let reserved = ledger.reserved;
        if available
            < self
                .limits
                .recovery_reserve_bytes
                .saturating_add(reserved)
                .saturating_add(growth)
        {
            return Err(ApiError::limit(
                "HTTP spool must preserve filesystem recovery headroom",
            ));
        }
        ledger.replace(
            name,
            Some(Charge {
                owner: Some(principal.to_owned()),
                bytes,
                overhead,
                reserved: growth.max(1),
            }),
        )?;
        Ok(Reservation {
            budget: self.clone(),
            name: name.to_owned(),
            principal: principal.to_owned(),
            max_length: length,
        })
    }
}

pub(crate) struct Reservation {
    budget: Arc<SpoolBudget>,
    name: String,
    principal: String,
    max_length: u64,
}
impl Reservation {
    pub(crate) fn block_admission(&self) {
        if let Ok(mut ledger) = self.budget.ledger.lock() {
            ledger.uncertain = true;
        }
    }
    pub(crate) fn check_headroom(
        &self,
        current_length: u64,
        next_write: u64,
    ) -> Result<(), ApiError> {
        if current_length
            .checked_add(next_write)
            .is_none_or(|length| length > self.max_length)
        {
            return Err(invalid_state("write exceeds its spool reservation"));
        }
        let available = rustix::fs::statvfs(&self.budget.root)
            .map_err(std::io::Error::from)
            .map_err(ApiError::io)?;
        if available.f_bavail.saturating_mul(available.f_frsize)
            < self
                .budget
                .limits
                .recovery_reserve_bytes
                .saturating_add(next_write)
        {
            return Err(ApiError::limit(
                "HTTP spool must preserve filesystem recovery headroom",
            ));
        }
        Ok(())
    }
    /// Establish attribution before creating/writing a new payload. On any
    /// uncertain outcome both files are retained; recovery inventories them.
    pub(crate) fn persist_owner(&self) -> Result<(), ApiError> {
        let path = self.budget.root.join(format!("{}.owner.json", self.name));
        if path.exists() {
            let owner = metadata::<Owner>(&path)
                .ok_or_else(|| invalid_state("invalid spool owner record"))?;
            if owner.data_file != self.name || owner.principal != self.principal {
                return Err(invalid_state("spool owner record mismatch"));
            }
            return Ok(());
        }
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path).map_err(ApiError::io)?;
        let bytes = serde_json::to_vec(&Owner {
            data_file: self.name.clone(),
            principal: self.principal.clone(),
        })
        .map_err(ApiError::invalid)?;
        file.write_all(&bytes).map_err(ApiError::io)?;
        file.sync_all().map_err(ApiError::io)?;
        fs::File::open(&self.budget.root)
            .and_then(|file| file.sync_all())
            .map_err(ApiError::io)
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        let Ok(mut ledger) = self.budget.ledger.lock() else {
            return;
        };
        let Some(previous) = ledger.files.get(&self.name) else {
            ledger.uncertain = true;
            return;
        };
        let overhead = previous.overhead;
        let maximum = previous.bytes;
        // Last writer/metadata task has finished. Unknown durability and upload
        // success are irrelevant: charge every retained physical byte.
        let retained = match fs::symlink_metadata(self.budget.root.join(&self.name)) {
            Ok(info) if info.is_file() => info.len().checked_add(overhead),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                match fs::symlink_metadata(
                    self.budget.root.join(format!("{}.owner.json", self.name)),
                ) {
                    Ok(_) => Some(overhead),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some(0),
                    Err(_) => None,
                }
            }
            _ => None,
        };
        let Some(bytes) = retained.filter(|bytes| *bytes <= maximum) else {
            ledger.uncertain = true;
            return;
        };
        let next = (bytes > 0).then(|| Charge {
            owner: Some(self.principal.clone()),
            bytes,
            overhead,
            reserved: 0,
        });
        if ledger.replace(&self.name, next).is_err() {
            ledger.uncertain = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uploads::{self, StagedFile};

    fn limits(account: u64, total: u64) -> SpoolLimits {
        SpoolLimits {
            bytes_per_account: account,
            bytes_total: total,
            recovery_reserve_bytes: 1,
        }
    }
    fn retained(
        budget: &Arc<SpoolBudget>,
        principal: &str,
        name: &str,
        declared: u64,
        bytes: &[u8],
    ) {
        let reservation = budget
            .reserve_with_available(principal, name, declared, u64::MAX)
            .unwrap();
        let staged = StagedFile::open_budgeted(budget.root.join(name), false, reservation).unwrap();
        staged.write(bytes).unwrap();
        staged.sync().unwrap();
    }
    fn total(budget: &SpoolBudget) -> u64 {
        budget
            .ledger
            .lock()
            .unwrap()
            .files
            .values()
            .map(|entry| entry.bytes)
            .sum()
    }
    fn record(name: &str, principal: &str) -> UploadRecord {
        UploadRecord {
            workspace_id: "ws".into(),
            item_id: "item".into(),
            principal: principal.into(),
            path: "file".into(),
            generation: 1,
            logical_size: 20,
            content_hash: Some("0".repeat(64)),
            persisted_bytes: 3,
            persisted_hash: blake3::hash(b"abc").to_hex().to_string(),
            state: "complete".into(),
            data_file: name.into(),
        }
    }
    #[test]
    fn reservation_settles_to_retained_bytes_not_success_or_declared_length() {
        let root = tempfile::tempdir().unwrap();
        let budget = SpoolBudget::recover(
            root.path(),
            limits(2 * METADATA_ALLOWANCE + 13, 2 * METADATA_ALLOWANCE + 13),
        )
        .unwrap();
        retained(&budget, "alice", "a.data", 10, b"abc");
        assert_eq!(total(&budget), METADATA_ALLOWANCE + 3);
        let second = budget
            .reserve_with_available("alice", "b.data", 10, u64::MAX)
            .unwrap();
        assert_eq!(total(&budget), 2 * METADATA_ALLOWANCE + 13);
        assert!(
            budget
                .reserve_with_available("bob", "c.data", 0, u64::MAX)
                .is_err()
        );
        drop(second);
        assert_eq!(total(&budget), METADATA_ALLOWANCE + 3);
        // Same physical file has one charge, even at its full quota.
        let retry = budget
            .reserve_with_available("alice", "a.data", 10, u64::MAX)
            .unwrap();
        assert_eq!(total(&budget), METADATA_ALLOWANCE + 10);
        drop(retry);
        assert_eq!(total(&budget), METADATA_ALLOWANCE + 3);
    }
    #[test]
    fn concurrent_reservations_cannot_oversubscribe_account_or_pool() {
        let root = tempfile::tempdir().unwrap();
        let charge = METADATA_ALLOWANCE + 1;
        let budget = SpoolBudget::recover(root.path(), limits(2 * charge, 3 * charge)).unwrap();
        let barrier = Arc::new(std::sync::Barrier::new(9));
        let mut threads = Vec::new();
        for index in 0..8 {
            let budget = budget.clone();
            let barrier = barrier.clone();
            threads.push(std::thread::spawn(move || {
                let reservation =
                    budget.reserve_with_available("alice", &format!("{index}.data"), 1, u64::MAX);
                let accepted = reservation.is_ok();
                barrier.wait();
                barrier.wait();
                accepted
            }));
        }
        barrier.wait();
        assert_eq!(total(&budget), 2 * charge);
        let bob = budget
            .reserve_with_available("bob", "bob.data", 1, u64::MAX)
            .unwrap();
        assert!(
            budget
                .reserve_with_available("charlie", "c.data", 1, u64::MAX)
                .is_err()
        );
        barrier.wait();
        assert_eq!(
            threads
                .into_iter()
                .map(|thread| thread.join().unwrap())
                .filter(|accepted| *accepted)
                .count(),
            2
        );
        drop(bob);
        assert_eq!(total(&budget), 0);
    }
    #[test]
    fn recovery_counts_retained_generations_failed_unknown_and_orphan_bytes_once() {
        let root = tempfile::tempdir().unwrap();
        let config = limits(8 * METADATA_ALLOWANCE, 8 * METADATA_ALLOWANCE);
        let budget = SpoolBudget::recover(root.path(), config.clone()).unwrap();
        retained(&budget, "alice", "old.data", 20, b"abc");
        let old = record("old.data", "alice");
        uploads::persist(root.path(), &old).unwrap();
        uploads::retain_generation(root.path(), &old).unwrap();
        uploads::retain_generation(root.path(), &old).unwrap();
        retained(&budget, "alice", "new.data", 20, b"failure");
        let mut new = record("new.data", "alice");
        new.generation = 2;
        new.state = "checksum_mismatch".into();
        uploads::persist(root.path(), &new).unwrap();
        drop(budget);
        fs::write(root.path().join("unknown.data"), b"unknown").unwrap();
        fs::write(root.path().join("broken.json"), b"{broken").unwrap();
        let budget = SpoolBudget::recover(root.path(), config).unwrap();
        assert_eq!(total(&budget), 4 * METADATA_ALLOWANCE + 3 + 7 + 7 + 7);
        assert_eq!(budget.ledger.lock().unwrap().files.len(), 4);
        assert_eq!(fs::read(root.path().join("old.data")).unwrap(), b"abc");
        assert_eq!(
            fs::read(root.path().join("unknown.data")).unwrap(),
            b"unknown"
        );
    }
    #[test]
    fn unknown_owner_is_not_an_account_quota_escape_and_overbudget_startup_stays_readable() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("unattributed.data"), b"unknown").unwrap();
        let budget = SpoolBudget::recover(
            root.path(),
            limits(METADATA_ALLOWANCE, 4 * METADATA_ALLOWANCE),
        )
        .unwrap();
        assert!(
            budget
                .reserve_with_available("alice", "new.data", 0, u64::MAX)
                .is_err()
        );
        assert!(
            budget
                .reserve_with_available("bob", "new.data", 0, u64::MAX)
                .is_err()
        );
        assert_eq!(
            fs::read(root.path().join("unattributed.data")).unwrap(),
            b"unknown"
        );
    }
    #[test]
    fn headroom_accounts_for_concurrent_reservations_without_filling_a_disk() {
        let root = tempfile::tempdir().unwrap();
        let budget = SpoolBudget::recover(
            root.path(),
            limits(4 * METADATA_ALLOWANCE, 4 * METADATA_ALLOWANCE),
        )
        .unwrap();
        let charge = METADATA_ALLOWANCE + 5;
        assert!(
            budget
                .reserve_with_available("alice", "one.data", 5, charge)
                .is_err()
        );
        let first = budget
            .reserve_with_available("alice", "one.data", 5, charge + 1)
            .unwrap();
        assert!(
            budget
                .reserve_with_available("bob", "two.data", 5, 2 * charge)
                .is_err()
        );
        assert!(
            budget
                .reserve_with_available("bob", "two.data", 5, 2 * charge + 1)
                .is_ok()
        );
        drop(first);
    }
    #[test]
    fn directory_has_one_inventory_owner_and_aliases_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        let config = limits(4 * METADATA_ALLOWANCE, 4 * METADATA_ALLOWANCE);
        let budget = SpoolBudget::recover(root.path(), config.clone()).unwrap();
        assert!(SpoolBudget::recover(root.path(), config.clone()).is_err());
        drop(budget);
        fs::write(root.path().join("one.data"), b"abc").unwrap();
        fs::hard_link(root.path().join("one.data"), root.path().join("alias.data")).unwrap();
        assert!(SpoolBudget::recover(root.path(), config).is_err());
    }
    #[test]
    fn sidecar_only_crash_remains_charged_and_rebuilds_without_a_payload() {
        let root = tempfile::tempdir().unwrap();
        let config = limits(2 * METADATA_ALLOWANCE, 2 * METADATA_ALLOWANCE);
        let budget = SpoolBudget::recover(root.path(), config.clone()).unwrap();
        let reservation = budget
            .reserve_with_available("alice", "missing.data", 10, u64::MAX)
            .unwrap();
        reservation.persist_owner().unwrap();
        drop(reservation);
        assert_eq!(total(&budget), METADATA_ALLOWANCE);
        drop(budget);
        let budget = SpoolBudget::recover(root.path(), config).unwrap();
        assert_eq!(total(&budget), METADATA_ALLOWANCE);
    }
}
