//! Product account/session policy built on the local credential primitives.
//! Credentials and token hashes are local protected control data, never public API data.
use crate::ApiError;
use authentication as primitives;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) const SESSION_TTL_SECONDS: u64 = 12 * 60 * 60;
const MAX_ACCOUNTS: usize = 1024;
const MAX_SESSIONS: usize = 4096;
/// Successful writes must remain admissible to the same bounded restart reader.
const MAX_STORE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Account {
    pub username: String,
    pub password_hash: String,
    pub admin: bool,
    pub repositories: BTreeSet<String>,
    pub disabled: bool,
    #[serde(with = "storage_contract::u64_string")]
    pub generation: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Session {
    username: String,
    #[serde(with = "storage_contract::u64_string")]
    generation: u64,
    #[serde(with = "storage_contract::u64_string")]
    expires_at: u64,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Database {
    accounts: BTreeMap<String, Account>,
    sessions: BTreeMap<String, Session>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identity {
    pub principal: String,
    pub admin: bool,
    pub repositories: BTreeSet<String>,
}

impl Identity {
    pub fn can_access(&self, repository: &str) -> bool {
        self.admin || self.repositories.contains(repository)
    }
    pub fn require_admin(&self) -> Result<(), ApiError> {
        if self.admin {
            Ok(())
        } else {
            Err(ApiError::forbidden())
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct LoginResult {
    pub token: String,
    pub csrf_token: String,
    pub principal: String,
    pub admin: bool,
    #[serde(with = "storage_contract::u64_string")]
    pub expires_at: u64,
}

pub type BackupHook = Arc<dyn Fn(serde_json::Value) -> Result<(), ApiError> + Send + Sync>;

pub struct AuthStore {
    path: PathBuf,
    db: Database,
    dummy_hash: String,
    _lock: fs::File,
    backup: Option<BackupHook>,
    blocked: AtomicBool,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl AuthStore {
    fn require_available(&self) -> Result<(), ApiError> {
        if self.blocked.load(Ordering::SeqCst) {
            Err(ApiError::new(
                503,
                "RECOVERY_REQUIRED",
                "authentication publication must be recovered before serving requests",
            ))
        } else {
            Ok(())
        }
    }
    pub fn snapshot(&self) -> Result<serde_json::Value, ApiError> {
        serde_json::to_value(&self.db).map_err(ApiError::invalid)
    }
    pub fn attach_backup(&mut self, backup: BackupHook) -> Result<(), ApiError> {
        backup(self.snapshot()?)?;
        self.backup = Some(backup);
        Ok(())
    }
    pub fn identity_for_principal(&self, username: &str) -> Result<Identity, ApiError> {
        self.require_available()?;
        Self::database_identity(&self.db, username)
    }
    /// Read the engine's authoritative protected snapshot without taking the
    /// mutable authentication-store lock (account publication uses auth -> engine).
    pub(crate) fn identity_from_snapshot(
        snapshot: serde_json::Value,
        username: &str,
    ) -> Result<Identity, ApiError> {
        let database: Database = serde_json::from_value(snapshot).map_err(ApiError::invalid)?;
        Self::database_identity(&database, username)
    }
    fn database_identity(database: &Database, username: &str) -> Result<Identity, ApiError> {
        let account = database
            .accounts
            .get(username)
            .filter(|account| !account.disabled)
            .ok_or_else(ApiError::unauthorized)?;
        Ok(Identity {
            principal: account.username.clone(),
            admin: account.admin,
            repositories: account.repositories.clone(),
        })
    }
    fn acquire_lock(root: &Path) -> Result<fs::File, ApiError> {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(root.join("auth.lock")).map_err(ApiError::io)?;
        file.try_lock().map_err(|_|ApiError::conflict("AUTH_STORE_BUSY","another process owns the credential store; use its authenticated administration API"))?;
        Ok(file)
    }
    /// Create only through the explicit administrative setup command.
    pub fn initialize(root: &Path, username: &str, password: &str) -> Result<Self, ApiError> {
        let path = root.join("auth.json");
        if path.symlink_metadata().is_ok() {
            return Err(ApiError::conflict(
                "AUTH_ALREADY_INITIALIZED",
                "credentials already exist",
            ));
        }
        fs::create_dir_all(root).map_err(ApiError::io)?;
        let auth_lock = Self::acquire_lock(root)?;
        let username =
            primitives::normalize_administrator_username(username).map_err(ApiError::invalid)?;
        let hash = primitives::hash_password(password).map_err(ApiError::invalid)?;
        let account = Account {
            username: username.clone(),
            password_hash: hash.clone(),
            admin: true,
            repositories: BTreeSet::new(),
            disabled: false,
            generation: 1,
        };
        let mut db = Database::default();
        db.accounts.insert(username, account);
        let store = Self {
            path,
            db,
            dummy_hash: hash,
            _lock: auth_lock,
            backup: None,
            blocked: AtomicBool::new(false),
        };
        store.persist(true)?;
        Ok(store)
    }

    pub fn open(root: &Path) -> Result<Self, ApiError> {
        Self::open_with_snapshot(root, None)
    }
    /// Restore only from the engine's already recovered authoritative control state.
    /// Missing local credentials never cause implicit account creation.
    pub fn open_with_snapshot(
        root: &Path,
        authoritative: Option<serde_json::Value>,
    ) -> Result<Self, ApiError> {
        let auth_lock = Self::acquire_lock(root)?;
        let path = root.join("auth.json");
        let local = match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if !metadata.is_file() || metadata.file_type().is_symlink() {
                    return Err(ApiError::invalid("invalid credential store"));
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    if metadata.mode() & 0o077 != 0 || metadata.nlink() != 1 {
                        return Err(ApiError::invalid(
                            "credential store must have mode 0600 and a single link",
                        ));
                    }
                }
                if authoritative.is_some() {
                    // Safety checks still apply, but a recovered authoritative
                    // snapshot replaces local contents without reading them.
                    // A stale or oversized cache cannot veto valid recovery.
                    None
                } else {
                    if metadata.len() > MAX_STORE_BYTES as u64 {
                        return Err(ApiError::invalid("credential store exceeds limit"));
                    }
                    Some(fs::read(&path).map_err(ApiError::io)?)
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(ApiError::io(error)),
        };
        let restore = authoritative.is_some();
        let db: Database = match authoritative {
            Some(snapshot) => serde_json::from_value(snapshot).map_err(ApiError::invalid)?,
            None => storage_contract::decode_strict(&local.ok_or_else(|| {
                ApiError::new(
                    503,
                    "AUTH_RECOVERY_REQUIRED",
                    "credential store is missing and no authoritative backup is available",
                )
            })?)
            .map_err(ApiError::invalid)?,
        };
        if db.accounts.is_empty()
            || db.accounts.len() > MAX_ACCOUNTS
            || db.sessions.len() > MAX_SESSIONS
        {
            return Err(ApiError::invalid("invalid credential store counts"));
        }
        if !db
            .accounts
            .values()
            .any(|account| account.admin && !account.disabled)
        {
            return Err(ApiError::invalid(
                "credential state has no enabled administrator",
            ));
        }
        for (name, account) in &db.accounts {
            primitives::require_canonical_administrator_username(name)
                .map_err(ApiError::invalid)?;
            if &account.username != name {
                return Err(ApiError::invalid("credential account identity mismatch"));
            }
            primitives::require_current_password_hash(&account.password_hash)
                .map_err(ApiError::invalid)?;
        }
        let dummy_hash = db.accounts.values().next().unwrap().password_hash.clone();
        let store = Self {
            path,
            db,
            dummy_hash,
            _lock: auth_lock,
            backup: None,
            blocked: AtomicBool::new(false),
        };
        if restore {
            store.persist(false)?;
        }
        Ok(store)
    }

    fn persist(&self, initial: bool) -> Result<(), ApiError> {
        // Reject an oversized candidate before creating even a temporary file
        // or publishing its protected backup. This is deterministic admission,
        // not an uncertain I/O outcome that should block existing credentials.
        let bytes = serde_json::to_vec(&self.db).map_err(ApiError::invalid)?;
        if bytes.len() > MAX_STORE_BYTES {
            return Err(ApiError::limit(
                "credential store exceeds serialized byte limit",
            ));
        }
        let temporary = self.path.with_extension(format!(
            "tmp-{}",
            primitives::random_token().map_err(ApiError::invalid)?
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary).map_err(ApiError::io)?;
        let result = (|| {
            file.write_all(&bytes).map_err(ApiError::io)?;
            file.sync_all().map_err(ApiError::io)?;
            if let Some(backup) = &self.backup
                && let Err(error) = backup(self.snapshot()?)
            {
                // Admission rejected before any decision or auth publication.
                if error.code != "COMMIT_PREPARATION_BUSY" {
                    self.blocked.store(true, Ordering::SeqCst);
                }
                return Err(error);
            }
            if initial && self.path.symlink_metadata().is_ok() {
                return Err(ApiError::conflict(
                    "AUTH_ALREADY_INITIALIZED",
                    "credentials already exist",
                ));
            }
            if initial {
                fs::hard_link(&temporary, &self.path).map_err(ApiError::io)?;
                fs::remove_file(&temporary).map_err(ApiError::io)?;
            } else {
                fs::rename(&temporary, &self.path).map_err(ApiError::io)?;
            }
            fs::File::open(self.path.parent().unwrap())
                .and_then(|dir| dir.sync_all())
                .map_err(ApiError::io)?;
            Ok(())
        })();
        if result.is_err() {
            if self.backup.is_some()
                && result
                    .as_ref()
                    .err()
                    .is_none_or(|error| error.code != "COMMIT_PREPARATION_BUSY")
            {
                self.blocked.store(true, Ordering::SeqCst);
            }
            let _ = fs::remove_file(temporary);
        }
        result
    }

    pub fn login(&mut self, username: &str, password: &str) -> Result<LoginResult, ApiError> {
        self.require_available()?;
        let canonical = primitives::normalize_administrator_username(username).unwrap_or_default();
        let account = self.db.accounts.get(&canonical);
        let valid = primitives::verify_password(
            password,
            account
                .map(|a| a.password_hash.as_str())
                .unwrap_or(&self.dummy_hash),
        );
        let Some(account) = account.filter(|a| !a.disabled && valid) else {
            return Err(ApiError::unauthorized());
        };
        let token = primitives::random_token().map_err(ApiError::invalid)?;
        let csrf_token = primitives::derive_csrf_token(&token).map_err(ApiError::invalid)?;
        let expires_at = now() + SESSION_TTL_SECONDS;
        let result = LoginResult {
            token: token.clone(),
            csrf_token,
            principal: account.username.clone(),
            admin: account.admin,
            expires_at,
        };
        let session = Session {
            username: account.username.clone(),
            generation: account.generation,
            expires_at,
        };
        let current_time = now();
        let expired = self
            .db
            .sessions
            .iter()
            .filter(|(_, session)| session.expires_at <= current_time)
            .map(|(key, session)| (key.clone(), session.clone()))
            .collect::<Vec<_>>();
        self.db
            .sessions
            .retain(|_, session| session.expires_at > current_time);
        if self.db.sessions.len() >= MAX_SESSIONS {
            self.db.sessions.extend(expired);
            return Err(ApiError::limit("session limit reached"));
        }
        let key = primitives::token_hash_hex(&token);
        self.db.sessions.insert(key.clone(), session);
        if let Err(error) = self.persist(false) {
            self.db.sessions.remove(&key);
            self.db.sessions.extend(expired);
            return Err(error);
        }
        Ok(result)
    }

    pub fn authenticate(&self, token: &str) -> Result<Identity, ApiError> {
        self.require_available()?;
        if !primitives::is_token_shape(token) {
            return Err(ApiError::unauthorized());
        }
        let session = self
            .db
            .sessions
            .get(&primitives::token_hash_hex(token))
            .filter(|session| session.expires_at > now())
            .ok_or_else(ApiError::unauthorized)?;
        let account = self
            .db
            .accounts
            .get(&session.username)
            .filter(|a| !a.disabled && a.generation == session.generation)
            .ok_or_else(ApiError::unauthorized)?;
        Ok(Identity {
            principal: account.username.clone(),
            admin: account.admin,
            repositories: account.repositories.clone(),
        })
    }

    pub fn logout(&mut self, token: &str) -> Result<(), ApiError> {
        self.require_available()?;
        let key = primitives::token_hash_hex(token);
        let previous = self.db.sessions.remove(&key);
        if let Err(error) = self.persist(false) {
            if let Some(session) = previous {
                self.db.sessions.insert(key, session);
            }
            return Err(error);
        }
        Ok(())
    }

    /// Rotate only the authenticated account's password and revoke its sessions.
    pub fn change_password(
        &mut self,
        token: &str,
        current_password: &str,
        new_password: &str,
    ) -> Result<(), ApiError> {
        // Recheck the session under the same lock as the mutation. The caller
        // cannot choose a username, role, repository scope or generation.
        let identity = self.authenticate(token)?;
        let account = self.db.accounts[&identity.principal].clone();
        if !primitives::verify_password(current_password, &account.password_hash) {
            return Err(ApiError::new(
                403,
                "CURRENT_PASSWORD_INVALID",
                "current password is incorrect",
            ));
        }
        self.upsert_account(
            &account.username,
            Some(new_password),
            account.admin,
            account.repositories,
            account.disabled,
        )
    }

    pub fn upsert_account(
        &mut self,
        username: &str,
        password: Option<&str>,
        admin: bool,
        repositories: BTreeSet<String>,
        disabled: bool,
    ) -> Result<(), ApiError> {
        // Callers may already have been admitted before a backup publication
        // became uncertain. Do not let that queued mutation bypass fail-closed
        // authentication state merely because it does not use a session token.
        self.require_available()?;
        let username =
            primitives::normalize_administrator_username(username).map_err(ApiError::invalid)?;
        let previous = self.db.accounts.get(&username).cloned();
        if previous.is_none() && self.db.accounts.len() >= MAX_ACCOUNTS {
            return Err(ApiError::limit("account limit reached"));
        }
        let password_hash = match password {
            Some(password) => primitives::hash_password(password).map_err(ApiError::invalid)?,
            None => previous
                .as_ref()
                .ok_or_else(|| ApiError::invalid("new account requires a password"))?
                .password_hash
                .clone(),
        };
        if previous
            .as_ref()
            .is_some_and(|account| account.admin && !account.disabled)
            && (!admin || disabled)
            && self
                .db
                .accounts
                .values()
                .filter(|a| a.admin && !a.disabled)
                .count()
                <= 1
        {
            return Err(ApiError::conflict(
                "LAST_ADMIN_REQUIRED",
                "cannot disable the last administrator",
            ));
        }
        let account = Account {
            username: username.clone(),
            password_hash,
            admin,
            repositories,
            disabled,
            generation: previous
                .as_ref()
                .map(|account| {
                    account
                        .generation
                        .checked_add(1)
                        .ok_or_else(|| ApiError::limit("account generation has been exhausted"))
                })
                .transpose()?
                .unwrap_or(1),
        };
        self.db.accounts.insert(username.clone(), account);
        if let Err(error) = self.persist(false) {
            match previous {
                Some(account) => {
                    self.db.accounts.insert(username, account);
                }
                None => {
                    self.db.accounts.remove(&username);
                }
            }
            return Err(error);
        }
        Ok(())
    }

    pub fn list_accounts(&self) -> Vec<serde_json::Value> {
        self.db.accounts.values().map(|account| serde_json::json!({"username":account.username,"admin":account.admin,"disabled":account.disabled,"repositories":account.repositories})).collect()
    }
}

#[cfg(test)]
mod budget_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn self_password_rotation_preserves_scope_revokes_sessions_and_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AuthStore::initialize(dir.path(), "admin", "a-long-test-password").unwrap();
        let repositories = BTreeSet::from(["repo-private".to_owned()]);
        store
            .upsert_account(
                "reader",
                Some("reader-old-password"),
                false,
                repositories.clone(),
                false,
            )
            .unwrap();
        let first = store.login("reader", "reader-old-password").unwrap();
        let second = store.login("reader", "reader-old-password").unwrap();
        store
            .change_password(&first.token, "reader-old-password", "reader-new-password")
            .unwrap();
        for token in [&first.token, &second.token] {
            assert_eq!(store.authenticate(token).unwrap_err().code, "UNAUTHORIZED");
        }
        assert!(store.login("reader", "reader-old-password").is_err());
        drop(store);
        let mut store = AuthStore::open(dir.path()).unwrap();
        let session = store.login("reader", "reader-new-password").unwrap();
        let identity = store.authenticate(&session.token).unwrap();
        assert_eq!(identity.principal, "reader");
        assert!(!identity.admin);
        assert_eq!(identity.repositories, repositories);
        assert!(store.login("admin", "a-long-test-password").is_ok());
    }

    #[test]
    fn rejected_password_rotation_keeps_credentials_and_session_intact() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AuthStore::initialize(dir.path(), "admin", "a-long-test-password").unwrap();
        let session = store.login("admin", "a-long-test-password").unwrap();
        let before = store.snapshot().unwrap();
        assert_eq!(
            store
                .change_password(&session.token, "incorrect-password", "a-new-test-password")
                .unwrap_err()
                .code,
            "CURRENT_PASSWORD_INVALID"
        );
        assert!(
            store
                .change_password(&session.token, "a-long-test-password", "short")
                .is_err()
        );
        assert_eq!(store.snapshot().unwrap(), before);
        assert!(store.authenticate(&session.token).is_ok());
        drop(store);
        let mut store = AuthStore::open(dir.path()).unwrap();
        assert!(store.login("admin", "a-long-test-password").is_ok());
    }

    #[test]
    fn persisted_sessions_are_revoked_by_account_generation() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AuthStore::initialize(dir.path(), "admin", "a-long-test-password").unwrap();
        let login = store.login("admin", "a-long-test-password").unwrap();
        drop(store);
        let mut store = AuthStore::open(dir.path()).unwrap();
        assert!(store.authenticate(&login.token).is_ok());
        store
            .upsert_account("admin", None, true, BTreeSet::new(), false)
            .unwrap();
        assert_eq!(
            store.authenticate(&login.token).unwrap_err().code,
            "UNAUTHORIZED"
        );
    }
    #[test]
    fn cannot_disable_last_administrator() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = AuthStore::initialize(dir.path(), "admin", "a-long-test-password").unwrap();
        assert!(
            store
                .upsert_account("admin", None, false, BTreeSet::new(), false)
                .is_err()
        );
    }
    #[test]
    fn missing_local_credentials_restore_only_from_authoritative_snapshot() {
        let root = tempfile::tempdir().unwrap();
        let mut store =
            AuthStore::initialize(root.path(), "admin", "a-long-test-password").unwrap();
        let session = store.login("admin", "a-long-test-password").unwrap();
        let snapshot = store.snapshot().unwrap();
        drop(store);
        fs::remove_file(root.path().join("auth.json")).unwrap();
        assert!(AuthStore::open(root.path()).is_err());
        let restored = AuthStore::open_with_snapshot(root.path(), Some(snapshot)).unwrap();
        assert!(restored.authenticate(&session.token).is_ok());
    }
    #[test]
    fn uncertain_backup_blocks_authentication_until_recovery() {
        let root = tempfile::tempdir().unwrap();
        let mut store =
            AuthStore::initialize(root.path(), "admin", "a-long-test-password").unwrap();
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = calls.clone();
        store
            .attach_backup(Arc::new(move |_| {
                if count.fetch_add(1, Ordering::SeqCst) == 0 {
                    Ok(())
                } else {
                    Err(ApiError::new(
                        503,
                        "COMMIT_IN_DOUBT",
                        "publication interrupted",
                    ))
                }
            }))
            .unwrap();
        assert!(store.login("admin", "a-long-test-password").is_err());
        assert_eq!(
            store.identity_for_principal("admin").unwrap_err().code,
            "RECOVERY_REQUIRED"
        );
        let before = store.snapshot().unwrap();
        assert_eq!(
            store
                .upsert_account(
                    "queued",
                    Some("queued-test-password"),
                    false,
                    BTreeSet::new(),
                    false
                )
                .unwrap_err()
                .code,
            "RECOVERY_REQUIRED"
        );
        assert_eq!(store.snapshot().unwrap(), before);
    }
}
