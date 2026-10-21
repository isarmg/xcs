//! Protected local Samba bridge. Every operation is authorized by Backend;
//! mutable files remain unpublished until the normal core commit succeeds.
use crate::{ApiError, Backend, Content, auth::Identity};
use serde_json::{Value, json};
use smb_protocol::protocol::{FrameStream, OpenResult, PROTOCOL, Request, Response};
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
use storage_contract::{Entry, Workspace, new_id, parse_u64_decimal};

const MAX_CONNECTIONS: usize = 64;
const MAX_HANDLES: usize = 1024;

mod handles;
mod namespace;
mod publication;
mod session;
mod transport;
use publication::recovery_record;
use session::Session;
#[cfg(all(test, feature = "core"))]
use transport::run_connection;
pub use transport::serve;
#[cfg(all(test, feature = "core"))]
#[path = "../../../test/server/unit/smb_rpc/tests.rs"]
mod tests;

struct Shared {
    backend: Arc<dyn Backend>,
    identities: BTreeMap<String, Identity>,
    spool: PathBuf,
    file_writers: Mutex<BTreeSet<(String, String)>>,
    namespace_operations: Mutex<BTreeSet<String>>,
}

enum Handle {
    Read(Content, String),
    Directory(File),
    Write(Box<Writer>),
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
fn contract(error: storage_contract::Error) -> ApiError {
    ApiError::invalid(error)
}
