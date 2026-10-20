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
mod tests {
    use super::*;

    fn commit(id: &str, parent: Option<&str>) -> Commit {
        Commit {
            commit_id: id.into(),
            history_tree_id: "history".into(),
            kind: "content".into(),
            parent_commit_ids: parent.into_iter().map(str::to_owned).collect(),
            tree_id: "tree".into(),
            sequence: 0,
            operation_id: "operation".into(),
            author: "author".into(),
            message: String::new(),
            entries: Vec::new(),
            pending_file_versions: Default::default(),
            provenance: None,
        }
    }

    #[test]
    fn reachability_is_independent_of_order_and_excludes_detached_cycles() {
        let commits = [
            commit("child", Some("root")),
            commit("root", None),
            commit("grandchild", Some("child")),
            commit("sibling", Some("root")),
            commit("cycle1", Some("cycle2")),
            commit("cycle2", Some("cycle1")),
        ];
        assert_eq!(
            descendant_commits(commits.iter().rev(), "root"),
            BTreeSet::from(["root", "child", "grandchild", "sibling"].map(str::to_owned))
        );
        assert_eq!(
            descendant_commits(commits.iter(), "child"),
            BTreeSet::from(["child", "grandchild"].map(str::to_owned))
        );
        assert_eq!(descendant_commits(commits.iter(), "cycle1").len(), 2);
    }

    #[test]
    fn long_reverse_order_chain_needs_no_recursive_stack_or_full_table_rescan() {
        let mut commits = vec![commit("0", None)];
        for id in 1..12_000 {
            commits.push(commit(&id.to_string(), Some(&(id - 1).to_string())));
        }
        assert_eq!(
            descendant_commits(commits.iter().rev(), "0").len(),
            commits.len()
        );
    }
}
