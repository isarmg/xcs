//! Immutable listing projections. Selection and transfer state are deliberately
//! not inputs: they must not rebuild, sort, or filter the listing.
use super::model::{entry_key, file_kind};
use crate::ui::{State, text};
use json_codec::Value;
use std::sync::Arc;
use web_interface::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct RowKey {
    context: Arc<RowContext>,
    entry_id: String,
    path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct RowContext {
    epoch: u64,
    slot: String,
    directory: String,
}

#[derive(Clone, Debug)]
pub(super) struct Row {
    pub key: RowKey,
    pub check_id: String,
    pub name: String,
    normalized_name: String,
    pub directory: bool,
    pub selection_id: String,
    pub size: Option<u64>,
}
impl Row {
    pub fn current_context(&self, state: &State) -> bool {
        state.session_active()
            && state.session_epoch == self.key.context.epoch
            && state.files.slot == self.key.context.slot
            && state.files.directory == self.key.context.directory
    }

    pub fn current_entry<'a>(&self, state: &'a State) -> Option<&'a Value> {
        if !self.current_context(state) {
            return None;
        }
        // Event-time identity lookup intentionally reads the current snapshot.
        // Rows hold display data only, never old JSON or whole listing snapshots.
        state.files.entries.iter().find(|entry| {
            entry["entry_id"].as_str().unwrap_or_default() == self.key.entry_id
                && entry["path"].as_str().unwrap_or_default() == self.key.path
        })
    }
}
impl PartialEq for Row {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.name == other.name
            && self.selection_id == other.selection_id
            && self.directory == other.directory
            && self.size == other.size
    }
}

impl Eq for Row {}

#[derive(Clone, PartialEq)]
struct Listing {
    epoch: u64,
    slot: String,
    directory: String,
    entries: Arc<[Value]>,
}
impl Listing {
    fn capture(state: &State) -> Self {
        Self {
            epoch: state.session_epoch,
            slot: state.files.slot.clone(),
            directory: state.files.directory.clone(),
            entries: state.files.entries.clone(),
        }
    }
    fn project(&self) -> Vec<Arc<Row>> {
        #[cfg(test)]
        tests::record_sort(self.entries.len());
        let context = Arc::new(RowContext {
            epoch: self.epoch,
            slot: self.slot.clone(),
            directory: self.directory.clone(),
        });
        let mut rows = self
            .entries
            .iter()
            .map(|entry| {
                let check_id = entry_key(entry);
                let name = text(entry, &["name"]);
                Arc::new(Row {
                    key: RowKey {
                        context: context.clone(),
                        entry_id: text(entry, &["entry_id"]),
                        path: text(entry, &["path"]),
                    },
                    check_id,
                    normalized_name: name.to_lowercase(),
                    name,
                    directory: !file_kind(entry),
                    selection_id: text(entry, &["entry_id", "path"]),
                    size: crate::format::parse_u64(&entry["size"]),
                })
            })
            .collect::<Vec<_>>();
        // Stable ordering preserves the server order of equal names and keeps
        // folders before files, exactly as the original file table did.
        rows.sort_by(|a, b| {
            (!a.directory, &a.normalized_name).cmp(&(!b.directory, &b.normalized_name))
        });
        rows
    }
}

pub(super) fn visible_rows(state: RwSignal<State>) -> Memo<Vec<Arc<Row>>> {
    // Arc equality short-circuits identical allocations. An unrelated State
    // notification does not traverse or clone any entry JSON.
    let listing = Memo::new(move |_| state.with(Listing::capture));
    let sorted = Memo::new(move |_| listing.with(Listing::project));
    let query = Memo::new(move |_| state.with(|s| s.files.query.to_lowercase()));
    Memo::new(move |_| {
        #[cfg(test)]
        tests::record_filter();
        query.with(|query| {
            sorted.with(|rows| {
                rows.iter()
                    .filter(|row| row.normalized_name.contains(query))
                    .cloned()
                    .collect()
            })
        })
    })
}

#[cfg(test)]
#[path = "../../../../test/web/unit/files/rows/tests.rs"]
mod tests;
