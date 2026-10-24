//! Directory cleanup is tied to both the host slot and its remote handle identity.
use std::{collections::BTreeMap, io};

pub(super) fn take_directory_open<T>(
    files: &mut BTreeMap<usize, T>,
    owner: usize,
    handle: u64,
    identity: impl FnOnce(&T) -> (u64, bool),
) -> io::Result<T> {
    if !files
        .get(&owner)
        .is_some_and(|opened| identity(opened) == (handle, true))
    {
        return Err(io::Error::from_raw_os_error(libc::EINVAL));
    }
    Ok(files.remove(&owner).expect("directory owner was checked"))
}

#[cfg(test)]
#[path = "../../../../test/smb-vfs/unit/module/directory_ownership_tests.rs"]
mod tests;
