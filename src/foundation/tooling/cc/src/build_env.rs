//! The environment a [`Build`](crate::Build) reads its configuration from and
//! runs its tools in.

use std::{
    cmp::Ordering,
    collections::hash_map::DefaultHasher,
    env,
    ffi::OsStr,
    fmt,
    hash::{Hash, Hasher},
    process::Command,
    sync::Arc,
};

use crate::utilities::OnceLock;

/// Environment variables, in the order they are applied.
pub(crate) type EnvVars = [(Arc<OsStr>, Arc<OsStr>)];

/// The environment of a [`Build`](crate::Build): a snapshot of the process
/// environment, taken on the first read or given with
/// `Build::set_envs_snapshot`, and used for cc's own lookups, then the entries
/// set with [`Build::env`](crate::Build::env), which win over it in child
/// processes.
#[derive(Clone, Debug, Default)]
pub(crate) struct BuildEnv {
    /// Private, so that nothing can read the environment without taking the
    /// snapshot first.
    inherited: OnceLock<EnvSnapshot>,
    pub(crate) explicit: Vec<(Arc<OsStr>, Arc<OsStr>)>,
}

impl BuildEnv {
    /// An environment whose snapshot is already taken.
    pub(crate) fn with_inherited(
        inherited: EnvSnapshot,
        explicit: Vec<(Arc<OsStr>, Arc<OsStr>)>,
    ) -> Self {
        Self {
            inherited: inherited.into(),
            explicit,
        }
    }

    /// The process environment as it was when this was first called, unless
    /// [`BuildEnv::set_inherited`] replaced it.
    pub(crate) fn inherited(&self) -> &EnvSnapshot {
        self.inherited.get_or_init(EnvSnapshot::capture)
    }

    /// Use `inherited` instead of the process environment, also if the
    /// snapshot was already taken.
    pub(crate) fn set_inherited(&mut self, inherited: EnvSnapshot) {
        self.inherited = inherited.into();
    }

    /// Make `cmd` run in this environment. Like [`EnvSnapshot::apply`], this
    /// clears anything set on `cmd`'s environment before.
    pub(crate) fn apply(&self, cmd: &mut Command) {
        self.inherited().apply(cmd);
        cmd.envs(pairs(&self.explicit));
    }
}

/// A copy of the process environment, or the variables given to
/// `Build::set_envs_snapshot`. Clones share the copy.
#[derive(Clone)]
pub(crate) struct EnvSnapshot {
    vars: Arc<EnvVars>,
    /// Hash of `vars`, worked out once.
    hash: u64,
}

impl EnvSnapshot {
    fn new(vars: Arc<EnvVars>) -> Self {
        // A fixed hasher, so that equal snapshots get equal hashes for the
        // whole process.
        let mut hasher = DefaultHasher::new();
        vars.hash(&mut hasher);
        Self {
            hash: hasher.finish(),
            vars,
        }
    }

    /// Copy the process environment as it is now. A variable it holds twice,
    /// such as `Path` and `PATH` on Windows, takes its last value, as in the
    /// tools cc runs.
    pub(crate) fn capture() -> Self {
        Self::from_vars(&mut env::vars_os().map(|(key, value)| (key.into(), value.into())))
    }

    /// A snapshot holding `vars`. A variable given more than once takes its
    /// last value, as with [`Command::envs`].
    pub(crate) fn from_vars(vars: &mut dyn Iterator<Item = (Arc<OsStr>, Arc<OsStr>)>) -> Self {
        let mut deduped: Vec<(Arc<OsStr>, Arc<OsStr>)> = Vec::with_capacity(vars.size_hint().0);
        for (key, value) in vars {
            match deduped.iter_mut().find(|(k, _)| is_same_key(k, &key)) {
                Some(entry) => entry.1 = value,
                None => deduped.push((key, value)),
            }
        }
        Self::new(deduped.into())
    }

    #[cfg(test)]
    pub(crate) fn from_pairs(vars: &[(&str, &str)]) -> Self {
        Self::new(
            vars.iter()
                .map(|(key, value)| (OsStr::new(key).into(), OsStr::new(value).into()))
                .collect(),
        )
    }

    /// Look up `key` the way [`env::var_os`] would have when the snapshot was
    /// taken, except that a variable held twice gives its last value.
    pub(crate) fn get(&self, key: &OsStr) -> Option<&Arc<OsStr>> {
        self.vars
            .iter()
            .find(|(k, _)| is_same_key(k, key))
            .map(|(_, value)| value)
    }

    /// Make `cmd` run in this environment rather than the current one.
    ///
    /// This clears the environment `cmd` would inherit and sets every variable
    /// of the snapshot, so the child does not depend on the process environment
    /// at the time it is spawned, which another thread may be changing. It also
    /// clears anything set on `cmd`'s environment before, so call it first.
    pub(crate) fn apply(&self, cmd: &mut Command) {
        cmd.env_clear();
        cmd.envs(pairs(&self.vars));
    }
}

impl PartialEq for EnvSnapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.vars, &other.vars) || (self.hash == other.hash && self.vars == other.vars)
    }
}

impl Eq for EnvSnapshot {}

impl Hash for EnvSnapshot {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}

impl Ord for EnvSnapshot {
    fn cmp(&self, other: &Self) -> Ordering {
        if Arc::ptr_eq(&self.vars, &other.vars) {
            return Ordering::Equal;
        }
        self.vars.cmp(&other.vars)
    }
}

impl PartialOrd for EnvSnapshot {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Debug for EnvSnapshot {
    // The values may hold secrets, and a `Build` can end up in a log.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EnvSnapshot(<{} variables>)", self.vars.len())
    }
}

fn pairs(vars: &EnvVars) -> impl Iterator<Item = (&OsStr, &OsStr)> {
    vars.iter().map(|(key, value)| (&**key, &**value))
}

/// Environment variable names are case-insensitive on Windows.
fn is_same_key(a: &OsStr, b: &OsStr) -> bool {
    if cfg!(windows) {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/cc/unit/src/build_env/tests_185.rs"));
