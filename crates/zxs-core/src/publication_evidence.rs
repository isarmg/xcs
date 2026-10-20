//! Read-only replica selection for hash-bound publication evidence.
//!
//! Callers supply trusted metadata members and authoritative journal/candidate
//! hashes. A replica is never selected by voting, and reads never repair data.
use crate::{disk::*, journal::*, *};

// The journal supplies the authoritative digest. A replica is usable only if
// its decoded object matches that exact digest, never by majority agreement.
pub(crate) fn verified_object<T: serde::Serialize + serde::de::DeserializeOwned>(
    members: &[DiskConfig],
    kind: &str,
    hash: &str,
) -> Result<T> {
    let mut first_error = None;
    for member in members {
        match load_object(member, kind, hash) {
            Ok(value) => return Ok(value),
            Err(error) => {
                first_error.get_or_insert(error);
            }
        }
    }
    Err(first_error.unwrap_or_else(|| Error::new("RECOVERY_REQUIRED", "no trusted object replica")))
}

// A candidate binds one snapshot root. Try only replicas of that exact root;
// never substitute a different state or advance past a failed dependency.
pub(crate) fn verified_snapshot(members: &[DiskConfig], hash: &str) -> Result<(State, DiskConfig)> {
    let mut first_error = None;
    for member in members {
        match load_snapshot(member, hash) {
            Ok(state) => return Ok((state, member.clone())),
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
    }
    Err(first_error
        .unwrap_or_else(|| Error::new("RECOVERY_REQUIRED", "no trusted snapshot replica")))
}
/// A seal has no independently supplied digest: validate its complete receipt
/// barrier against the exact candidate and publication before accepting a copy.
pub(crate) fn verified_publication_seal(
    members: &[DiskConfig],
    candidate: &Candidate,
    publication: &Publication,
) -> Result<PublicationSeal> {
    let hash = object_hash("candidate", candidate)?;
    let mut first_error = None;
    for member in members {
        let result = read_json(&publication_seal_path(member, &hash)).and_then(|seal| {
            verify_publication_seal(candidate, publication, &seal)?;
            Ok(seal)
        });
        match result {
            Ok(seal) => return Ok(seal),
            Err(error) => {
                first_error.get_or_insert(error);
            }
        }
    }
    Err(first_error
        .unwrap_or_else(|| Error::new("RECOVERY_REQUIRED", "no trusted publication seal replica")))
}
