//! Protected local Samba bridge. Every operation is authorized by Backend;
//! mutable files remain unpublished until the normal core commit succeeds.
use crate::{ApiError, Backend, Content, auth::Identity};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{self, Read, Write},
    os::{
        fd::{AsFd, OwnedFd},
        unix::fs::{FileExt, MetadataExt, PermissionsExt},
    },
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
};
use zxs_contract::{Entry, Workspace, new_id, parse_u64_decimal};
use zxs_smb::protocol::{FrameStream, OpenResult, PROTOCOL, Request, Response};

const MAX_CONNECTIONS: usize = 64;
const MAX_HANDLES: usize = 1024;

mod handles;
mod publication;
mod session;
mod transport;
use publication::recovery_record;
use session::Session;
#[cfg(all(test, feature = "core"))]
use transport::run_connection;
pub use transport::serve;
#[cfg(all(test, feature = "core"))]
mod tests;

struct Shared {
    backend: Arc<dyn Backend>,
    identities: BTreeMap<String, Identity>,
    spool: PathBuf,
    file_writers: Mutex<BTreeSet<(String, String)>>,
}

enum Handle {
    Read(Content, String),
    Directory(File),
    Write(Writer),
}

struct Writer {
    file: File,
    source: PathBuf,
    path: String,
    item_id: String,
    workspace: Workspace,
    dirty: bool,
    committed: bool,
    operation_id: String,
    last_head: String,
    write_through: bool,
}

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a str, ApiError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::invalid(format!("missing {key}")))
}
fn contract(error: zxs_contract::Error) -> ApiError {
    ApiError::invalid(error)
}
