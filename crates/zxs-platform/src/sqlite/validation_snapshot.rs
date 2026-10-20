//! Source-preserving validation snapshots for a separate diagnostics process.
//! Never open an online SQLite source in the process already owning its SQLx
//! connections: closing raw descriptors can affect SQLite's POSIX locks.

use sha2::{Digest, Sha256};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::{
    fs::{self, File, Metadata, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug)]
pub struct SnapshotLimits {
    pub max_total_bytes: u64,
    pub timeout: Duration,
    pub attempts: u8,
}

impl Default for SnapshotLimits {
    fn default() -> Self {
        Self {
            max_total_bytes: 4 * 1024 * 1024 * 1024,
            timeout: Duration::from_secs(10),
            attempts: 3,
        }
    }
}

/// A private coherent main/WAL/journal generation; SHM is inspected and locked
/// but never copied. This is a transient validation artifact, not a backup.
pub struct ValidationSnapshot {
    directory: Arc<tempfile::TempDir>,
    database: PathBuf,
}

impl ValidationSnapshot {
    pub fn capture(path: impl AsRef<Path>) -> Result<Self, SnapshotError> {
        Self::capture_with_limits(path, SnapshotLimits::default())
    }

    pub fn capture_with_limits(
        path: impl AsRef<Path>,
        limits: SnapshotLimits,
    ) -> Result<Self, SnapshotError> {
        Self::capture_at(path, limits, None)
    }

    pub fn capture_in(
        path: impl AsRef<Path>,
        temporary_root: impl AsRef<Path>,
    ) -> Result<Self, SnapshotError> {
        Self::capture_at(
            path,
            SnapshotLimits::default(),
            Some(temporary_root.as_ref()),
        )
    }

    pub fn capture_with_limits_in(
        path: impl AsRef<Path>,
        limits: SnapshotLimits,
        temporary_root: impl AsRef<Path>,
    ) -> Result<Self, SnapshotError> {
        Self::capture_at(path, limits, Some(temporary_root.as_ref()))
    }

    fn capture_at(
        path: impl AsRef<Path>,
        limits: SnapshotLimits,
        temporary_root: Option<&Path>,
    ) -> Result<Self, SnapshotError> {
        if limits.max_total_bytes == 0
            || limits.timeout.is_zero()
            || limits.attempts == 0
            || limits.attempts > 8
        {
            return Err(SnapshotError::InvalidLimits);
        }
        let deadline = Instant::now()
            .checked_add(limits.timeout)
            .ok_or(SnapshotError::InvalidLimits)?;
        let path = path.as_ref();
        if !path.is_absolute() || path.file_name().is_none() {
            return Err(SnapshotError::UnsafeSource);
        }
        for attempt in 0..limits.attempts {
            match capture_once(path, limits.max_total_bytes, deadline, temporary_root) {
                Err(SnapshotError::SourceChanged) if attempt + 1 < limits.attempts => {
                    std::thread::yield_now()
                }
                result => return result,
            }
        }
        Err(SnapshotError::SourceChanged)
    }

    pub fn database_path(&self) -> &Path {
        &self.database
    }

    /// Open the already-captured generation after another read-only adapter
    /// has closed its connection. No source recapture or generation change.
    pub async fn into_pool(self) -> Result<ValidationSnapshotPool, SnapshotError> {
        open_snapshot_pool(self, None).await
    }

    /// Open this exact captured generation with native limits applied to every
    /// newly established connection before any validation statement is prepared.
    pub async fn into_pool_with_connection_limits(
        self,
        limits: crate::sqlite::ConnectionLimits,
    ) -> Result<ValidationSnapshotPool, SnapshotError> {
        limits.validate()?;
        open_snapshot_pool(self, Some(limits)).await
    }
}

pub struct ValidationSnapshotPool {
    pool: SqlitePool,
    _snapshot: ValidationSnapshot,
}

impl ValidationSnapshotPool {
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
    pub async fn close(self) {
        self.pool.close().await;
    }
}

/// Copy under SQLite's native read locks, then open only the private copy. WAL
/// recovery and hot-journal rollback may write to that copy, never the source.
/// SQL access is query-only after connection, including business validation.
pub async fn open_validation_snapshot(
    path: impl AsRef<Path>,
) -> Result<ValidationSnapshotPool, SnapshotError> {
    let path = path.as_ref().to_path_buf();
    let snapshot = tokio::task::spawn_blocking(move || ValidationSnapshot::capture(path))
        .await
        .map_err(|_| SnapshotError::CaptureTaskFailed)??;
    snapshot.into_pool().await
}

pub async fn open_validation_snapshot_with_connection_limits(
    path: impl AsRef<Path>,
    limits: crate::sqlite::ConnectionLimits,
) -> Result<ValidationSnapshotPool, SnapshotError> {
    limits.validate()?;
    let path = path.as_ref().to_path_buf();
    let snapshot = tokio::task::spawn_blocking(move || ValidationSnapshot::capture(path))
        .await
        .map_err(|_| SnapshotError::CaptureTaskFailed)??;
    snapshot.into_pool_with_connection_limits(limits).await
}

async fn open_snapshot_pool(
    snapshot: ValidationSnapshot,
    limits: Option<crate::sqlite::ConnectionLimits>,
) -> Result<ValidationSnapshotPool, SnapshotError> {
    let retained = snapshot.directory.clone();
    let options = SqliteConnectOptions::new()
        .filename(snapshot.database_path())
        .create_if_missing(false)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(2));
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(3))
        .after_connect(move |connection, _| {
            // A cloned pool keeps this closure (and its private directory)
            // alive. Callers cannot delete a snapshot by dropping its wrapper.
            let retained = retained.clone();
            Box::pin(async move {
                if let Some(limits) = limits {
                    crate::sqlite::apply_connection_limits(connection, limits)
                        .await
                        .map_err(|error| sqlx::Error::Io(io::Error::other(error)))?;
                }
                sqlx::query("PRAGMA query_only=ON")
                    .execute(&mut *connection)
                    .await?;
                sqlx::query("PRAGMA trusted_schema=OFF")
                    .execute(&mut *connection)
                    .await?;
                drop(retained);
                Ok(())
            })
        })
        .connect_with(options)
        .await
        .map_err(snapshot_connect_error)?;
    Ok(ValidationSnapshotPool {
        pool,
        _snapshot: snapshot,
    })
}

fn snapshot_connect_error(error: sqlx::Error) -> SnapshotError {
    if let sqlx::Error::Io(source) = error {
        let kind = source.kind();
        if let Some(inner) = source.into_inner() {
            return match inner.downcast::<crate::sqlite::ConnectionLimitError>() {
                Ok(error) => SnapshotError::ConnectionLimits(*error),
                Err(inner) => SnapshotError::Sqlx(sqlx::Error::Io(io::Error::new(kind, inner))),
            };
        }
        return SnapshotError::Sqlx(sqlx::Error::Io(io::Error::from(kind)));
    }
    SnapshotError::Sqlx(error)
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Stamp {
    device: u64,
    inode: u64,
    length: u64,
    modified: (i64, i64),
    changed: (i64, i64),
    mode: u32,
    links: u64,
    owner: u32,
}
impl Stamp {
    fn from(metadata: &Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            length: metadata.len(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
            mode: metadata.mode(),
            links: metadata.nlink(),
            owner: metadata.uid(),
        }
    }
}

struct Source {
    path: PathBuf,
    file: File,
    before: Stamp,
}
impl Source {
    fn open(path: &Path) -> Result<Option<Self>, SnapshotError> {
        let named = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        require_regular(&named)?;
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(
                (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32,
            )
            .open(path)?;
        let opened = file.metadata()?;
        require_regular(&opened)?;
        if Stamp::from(&named) != Stamp::from(&opened) {
            return Err(SnapshotError::SourceChanged);
        }
        Ok(Some(Self {
            path: path.into(),
            file,
            before: Stamp::from(&opened),
        }))
    }
    fn verify(&self) -> Result<(), SnapshotError> {
        let opened = self.file.metadata()?;
        let named = fs::symlink_metadata(&self.path).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                SnapshotError::SourceChanged
            } else {
                error.into()
            }
        })?;
        require_regular(&opened)?;
        require_regular(&named)?;
        if self.before != Stamp::from(&opened) || self.before != Stamp::from(&named) {
            return Err(SnapshotError::SourceChanged);
        }
        Ok(())
    }
}

fn require_regular(metadata: &Metadata) -> Result<(), SnapshotError> {
    if !metadata.is_file() || metadata.nlink() != 1 {
        Err(SnapshotError::UnsafeSource)
    } else {
        Ok(())
    }
}

fn capture_once(
    path: &Path,
    budget: u64,
    deadline: Instant,
    temporary_root: Option<&Path>,
) -> Result<ValidationSnapshot, SnapshotError> {
    check_deadline(deadline)?;
    let parent_path = path.parent().ok_or(SnapshotError::UnsafeSource)?;
    let parent = OpenOptions::new()
        .read(true)
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::DIRECTORY).bits() as i32)
        .open(parent_path)?;
    let parent_before = Stamp::from(&parent.metadata()?);
    let mut main = Source::open(path)?.ok_or(SnapshotError::MissingDatabase)?;
    // Shared OFD locks conflict with SQLite's process locks but remain local
    // to these descriptors. Never fall back to weaker locking if unsupported.
    lock_read(&main.file, 0, 0)?;
    let shm_path = sidecar(path, "-shm");
    let shm = Source::open(&shm_path)?;
    if let Some(shm) = &shm {
        // SQLite os_unix.c: UNIX_SHM_BASE=(22+SQLITE_SHM_NLOCK)*4=120.
        // wal.c slots 0..2 are write, checkpoint and recovery. Reader-mark
        // slots remain available while the bounded copy holds these locks.
        lock_read(&shm.file, 120, 3)?;
    }
    let mut wal = Source::open(&sidecar(path, "-wal"))?;
    let mut journal = Source::open(&sidecar(path, "-journal"))?;
    let total = [Some(&main), wal.as_ref(), journal.as_ref()]
        .into_iter()
        .flatten()
        .try_fold(0u64, |total, source| {
            total
                .checked_add(source.before.length)
                .filter(|value| *value <= budget)
                .ok_or(SnapshotError::BudgetExceeded)
        })?;
    if total == 0 {
        return Err(SnapshotError::InvalidDatabase);
    }
    let mut builder = tempfile::Builder::new();
    builder.prefix(".sqlite-validation-");
    let directory = Arc::new(match temporary_root {
        Some(root) => {
            let directory = crate::fs_safety::PrivateDirectory::open_existing(root)
                .map_err(|_| SnapshotError::UnsafeSource)?;
            directory
                .verify_identity()
                .map_err(|_| SnapshotError::UnsafeSource)?;
            let temporary = builder.tempdir_in(root)?;
            directory
                .verify_identity()
                .map_err(|_| SnapshotError::UnsafeSource)?;
            temporary
        }
        None => builder.tempdir()?,
    });
    let database = directory.path().join("database.sqlite3");
    let mut expected = Vec::new();
    for (source, target) in [
        (Some(&mut main), database.clone()),
        (wal.as_mut(), sidecar(&database, "-wal")),
        (journal.as_mut(), sidecar(&database, "-journal")),
    ] {
        if let Some(source) = source {
            let mut destination = OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(target)?;
            let copied = transfer(
                &mut source.file,
                Some(&mut destination),
                source.before.length,
                deadline,
            )?;
            source.verify()?;
            destination.flush()?;
            expected.push(copied);
        }
    }
    // Re-read and compare complete content while the same native locks are
    // held. Identity, size and timestamps alone are insufficient evidence.
    for (source, expected) in [Some(&mut main), wal.as_mut(), journal.as_mut()]
        .into_iter()
        .flatten()
        .zip(expected)
    {
        source.file.seek(SeekFrom::Start(0))?;
        let actual = transfer(&mut source.file, None, source.before.length, deadline)?;
        source.verify()?;
        if expected != actual {
            return Err(SnapshotError::SourceChanged);
        }
    }
    for (name, present) in [
        (sidecar(path, "-wal"), wal.is_some()),
        (sidecar(path, "-journal"), journal.is_some()),
        (shm_path, shm.is_some()),
    ] {
        match fs::symlink_metadata(name) {
            Ok(_) if present => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound && !present => {}
            _ => return Err(SnapshotError::SourceChanged),
        }
    }
    if let Some(shm) = shm {
        shm.verify()?;
    }
    let named_parent = fs::symlink_metadata(parent_path)?;
    if !named_parent.is_dir()
        || named_parent.dev() != parent_before.device
        || named_parent.ino() != parent_before.inode
    {
        return Err(SnapshotError::SourceChanged);
    }
    Ok(ValidationSnapshot {
        directory,
        database,
    })
}

fn transfer(
    source: &mut File,
    mut destination: Option<&mut File>,
    expected_bytes: u64,
    deadline: Instant,
) -> Result<[u8; 32], SnapshotError> {
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        check_deadline(deadline)?;
        let read = source.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .ok_or(SnapshotError::BudgetExceeded)?;
        if total > expected_bytes {
            return Err(SnapshotError::SourceChanged);
        }
        hash.update(&buffer[..read]);
        if let Some(destination) = &mut destination {
            destination.write_all(&buffer[..read])?;
        }
    }
    if total != expected_bytes {
        return Err(SnapshotError::SourceChanged);
    }
    Ok(hash.finalize().into())
}

fn check_deadline(deadline: Instant) -> Result<(), SnapshotError> {
    if Instant::now() >= deadline {
        Err(SnapshotError::Timeout)
    } else {
        Ok(())
    }
}

#[allow(unsafe_code)]
fn lock_read(file: &File, start: libc::off_t, length: libc::off_t) -> Result<(), SnapshotError> {
    let mut lock = libc::flock {
        l_type: libc::F_RDLCK as _,
        l_whence: libc::SEEK_SET as _,
        l_start: start,
        l_len: length,
        l_pid: 0,
    };
    // SAFETY: the live File owns the fd throughout this call and `lock` is a
    // valid, initialized flock with the required zero PID for an OFD lock.
    let result = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_OFD_SETLK, &mut lock) };
    if result != -1 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if matches!(error.raw_os_error(), Some(libc::EAGAIN | libc::EACCES)) {
        Err(SnapshotError::Busy)
    } else {
        Err(SnapshotError::LockUnavailable(error))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("SQLite validation connection limits failed")]
    ConnectionLimits(#[from] crate::sqlite::ConnectionLimitError),
    #[error("SQLite validation snapshot limits are invalid")]
    InvalidLimits,
    #[error("SQLite validation requires regular, single-linked files and an absolute data path")]
    UnsafeSource,
    #[error("SQLite validation source does not exist")]
    MissingDatabase,
    #[error("SQLite validation source is empty or invalid")]
    InvalidDatabase,
    #[error("SQLite source changed during validation capture; retry after writes stop")]
    SourceChanged,
    #[error("SQLite validation capture exceeds the configured byte budget")]
    BudgetExceeded,
    #[error("SQLite validation capture exceeded its timeout")]
    Timeout,
    #[error("SQLite source has an active writer, checkpoint or recovery; retry later")]
    Busy,
    #[error("SQLite validation source does not support the required native lock: {0}")]
    LockUnavailable(io::Error),
    #[error("SQLite validation capture task failed")]
    CaptureTaskFailed,
    #[error("SQLite validation I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("SQLite private validation failed: {0}")]
    Sqlx(#[from] sqlx::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[tokio::test]
    async fn explicit_private_run_directory_keeps_limits_and_cleans_temporary_files() {
        let root = tempfile::tempdir().unwrap();
        let run = root.path().join("run");
        let held = crate::fs_safety::PrivateDirectory::create(&run).unwrap();
        let source = root.path().join("source.sqlite3");
        let pool = crate::sqlite::create_if_missing(&source, crate::sqlite::PoolOptions::new(1))
            .await
            .unwrap();
        sqlx::raw_sql("CREATE TABLE records(value INTEGER);INSERT INTO records VALUES(7)")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
        let before = generation_bytes(&source);
        assert!(matches!(
            ValidationSnapshot::capture_with_limits_in(
                &source,
                SnapshotLimits {
                    max_total_bytes: 1,
                    ..SnapshotLimits::default()
                },
                &run
            ),
            Err(SnapshotError::BudgetExceeded)
        ));
        let snapshot = ValidationSnapshot::capture_in(&source, &run).unwrap();
        assert!(snapshot.database_path().starts_with(&run));
        let snapshot = snapshot.into_pool().await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT value FROM records")
                .fetch_one(snapshot.pool())
                .await
                .unwrap(),
            7
        );
        snapshot.close().await;
        assert_eq!(before, generation_bytes(&source));
        assert_eq!(fs::read_dir(&run).unwrap().count(), 0);
        fs::rename(&run, root.path().join("old-run")).unwrap();
        crate::fs_safety::PrivateDirectory::create(&run).unwrap();
        assert!(held.verify_identity().is_err());
    }

    #[tokio::test]
    async fn connection_limits_survive_pool_replacement_without_touching_source() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.sqlite3");
        let original =
            crate::sqlite::create_if_missing(&source, crate::sqlite::PoolOptions::new(1))
                .await
                .unwrap();
        sqlx::raw_sql("CREATE TABLE data(value BLOB);INSERT INTO data VALUES(zeroblob(131072))")
            .execute(&original)
            .await
            .unwrap();
        original.close().await;
        let before = generation_bytes(&source);
        assert!(matches!(
            open_validation_snapshot_with_connection_limits(
                &source,
                crate::sqlite::ConnectionLimits::new(1)
            )
            .await,
            Err(SnapshotError::ConnectionLimits(
                crate::sqlite::ConnectionLimitError::InvalidLimits
            ))
        ));
        let snapshot = open_validation_snapshot_with_connection_limits(
            &source,
            crate::sqlite::ConnectionLimits::new(64 * 1024),
        )
        .await
        .unwrap();
        for _ in 0..2 {
            let mut connection = snapshot.pool().acquire().await.unwrap();
            let error = sqlx::query_scalar::<_, Vec<u8>>("SELECT value FROM data")
                .fetch_one(&mut *connection)
                .await
                .unwrap_err();
            assert_eq!(
                error
                    .as_database_error()
                    .and_then(|error| error.code())
                    .as_deref(),
                Some("18")
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT 7")
                    .fetch_one(&mut *connection)
                    .await
                    .unwrap(),
                7
            );
            // Force a new native worker instead of returning this connection.
            connection.close().await.unwrap();
        }
        snapshot.close().await;
        assert_eq!(generation_bytes(&source), before);
    }

    fn generation_bytes(path: &Path) -> Vec<Option<Vec<u8>>> {
        [
            path.to_path_buf(),
            sidecar(path, "-wal"),
            sidecar(path, "-journal"),
            sidecar(path, "-shm"),
        ]
        .into_iter()
        .map(|path| fs::read(path).ok())
        .collect()
    }

    #[tokio::test]
    async fn an_existing_capture_opens_the_same_generation_after_source_disappears() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("database.sqlite3");
        let pool = crate::sqlite::create_if_missing(&source, crate::sqlite::PoolOptions::new(1))
            .await
            .unwrap();
        sqlx::query("CREATE TABLE records (value TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO records VALUES ('captured-generation')")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
        let snapshot = ValidationSnapshot::capture(&source).unwrap();
        fs::rename(&source, directory.path().join("original.sqlite3")).unwrap();
        let pool = snapshot.into_pool().await.unwrap();
        let value: String = sqlx::query_scalar("SELECT value FROM records")
            .fetch_one(pool.pool())
            .await
            .unwrap();
        assert_eq!(value, "captured-generation");
        assert!(!source.exists());
        assert!(
            sqlx::query("DELETE FROM records")
                .execute(pool.pool())
                .await
                .is_err()
        );
        pool.close().await;
    }

    #[tokio::test]
    async fn reads_wal_only_commits_and_preserves_every_source_file() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("database.sqlite3");
        let pool = crate::sqlite::create_if_missing(&source, crate::sqlite::PoolOptions::new(1))
            .await
            .unwrap();
        sqlx::query("CREATE TABLE records (value TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO records VALUES ('wal-only-commit')")
            .execute(&pool)
            .await
            .unwrap();
        assert!(sidecar(&source, "-wal").exists());
        let before = generation_bytes(&source);
        let snapshot = open_validation_snapshot(&source).await.unwrap();
        let value: String = sqlx::query_scalar("SELECT value FROM records")
            .fetch_one(snapshot.pool())
            .await
            .unwrap();
        assert_eq!(value, "wal-only-commit");
        assert!(
            sqlx::query("INSERT INTO records VALUES ('forbidden')")
                .execute(snapshot.pool())
                .await
                .is_err()
        );
        assert_eq!(generation_bytes(&source), before);
        let retained = snapshot.pool().clone();
        drop(snapshot);
        let value: String = sqlx::query_scalar("SELECT value FROM records")
            .fetch_one(&retained)
            .await
            .unwrap();
        assert_eq!(value, "wal-only-commit");
        retained.close().await;
        pool.close().await;
    }

    #[tokio::test]
    async fn active_writer_is_busy_instead_of_an_unlocked_snapshot() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("database.sqlite3");
        let pool = crate::sqlite::create_if_missing(&source, crate::sqlite::PoolOptions::new(1))
            .await
            .unwrap();
        sqlx::query("CREATE TABLE records (value INTEGER)")
            .execute(&pool)
            .await
            .unwrap();
        let mut writer = pool.acquire().await.unwrap();
        sqlx::query("BEGIN IMMEDIATE")
            .execute(&mut *writer)
            .await
            .unwrap();
        sqlx::query("INSERT INTO records VALUES (1)")
            .execute(&mut *writer)
            .await
            .unwrap();
        assert!(matches!(
            ValidationSnapshot::capture(&source),
            Err(SnapshotError::Busy)
        ));
        sqlx::query("ROLLBACK").execute(&mut *writer).await.unwrap();
        drop(writer);
        pool.close().await;
    }

    #[test]
    fn unsafe_missing_oversized_and_invalid_sources_fail_without_source_changes() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        assert!(matches!(
            ValidationSnapshot::capture(&source),
            Err(SnapshotError::MissingDatabase)
        ));
        assert!(matches!(
            ValidationSnapshot::capture("relative.sqlite3"),
            Err(SnapshotError::UnsafeSource)
        ));
        fs::write(&source, b"not a sqlite database").unwrap();
        let limits = SnapshotLimits {
            max_total_bytes: 1,
            ..SnapshotLimits::default()
        };
        assert!(matches!(
            ValidationSnapshot::capture_with_limits(&source, limits),
            Err(SnapshotError::BudgetExceeded)
        ));
        let link = directory.path().join("link");
        symlink(&source, &link).unwrap();
        assert!(matches!(
            ValidationSnapshot::capture(&link),
            Err(SnapshotError::UnsafeSource)
        ));
        fs::hard_link(&source, directory.path().join("alias")).unwrap();
        assert!(matches!(
            ValidationSnapshot::capture(&source),
            Err(SnapshotError::UnsafeSource)
        ));
        assert_eq!(fs::read(source).unwrap(), b"not a sqlite database");
    }
}
