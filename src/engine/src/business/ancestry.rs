//! Indexed commit reachability shared by validation and history splitting.
use crate::Commit;
use std::collections::{BTreeMap, BTreeSet};

/// Include root and its descendants without repeatedly scanning all commits.
/// Callers validate parent identities/history scope before trusting the result.
pub(crate) fn descendant_commits<'a>(
    commits: impl Iterator<Item = &'a Commit>,
    root: &str,
) -> BTreeSet<String> {
    let mut children = BTreeMap::<&str, Vec<&str>>::new();
    for commit in commits {
        for parent in &commit.parent_commit_ids {
            children.entry(parent).or_default().push(&commit.commit_id);
        }
    }
    let mut reached = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        if reached.insert(id.to_owned())
            && let Some(descendants) = children.get(id)
        {
            pending.extend(descendants.iter().copied());
        }
    }
    reached
}

#[cfg(test)]
#[path = "../../../../test/engine/unit/business/ancestry/tests.rs"]
mod tests;
