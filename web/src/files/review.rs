//! Shared publication state for inline review panels. Unknown outcomes remain
//! tied to their original operation, and definitive conflicts preserve drafts.
use super::{
    actions,
    model::{Model, SaveStatus, publication_notice},
};
use crate::{
    api,
    ui::{State, current_session, fail},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use wasm_bindgen_futures::spawn_local;
use xcs_web::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Kind {
    Merge,
    Tags,
    Backup,
}
#[derive(Clone, Default)]
pub(crate) struct Publication {
    pub operation_id: Option<String>,
    pub error: String,
}
pub(crate) type Publications = BTreeMap<Kind, Publication>;

pub(crate) fn error(state: RwSignal<State>, kind: Kind, message: String) {
    state.update(|s| s.files.publications.entry(kind).or_default().error = message);
}
pub(crate) fn reset(state: RwSignal<State>, kind: Kind) {
    state.update(|s| {
        s.files.publications.remove(&kind);
    });
}
pub(crate) fn error_text(state: &State, kind: Kind) -> String {
    state
        .files
        .publications
        .get(&kind)
        .map(|publication| publication.error.clone())
        .unwrap_or_default()
}
pub(crate) fn settle(model: &mut Model, id: &str, status: SaveStatus) {
    let kinds = model
        .publications
        .iter()
        .filter(|(_, publication)| publication.operation_id.as_deref() == Some(id))
        .map(|(kind, _)| *kind)
        .collect::<Vec<_>>();
    for kind in kinds {
        match status {
            SaveStatus::Published | SaveStatus::Degraded => {
                match kind {
                    Kind::Merge => model.merge = None,
                    Kind::Backup => model.backup = None,
                    Kind::Tags => {
                        model.tag_edit = None;
                        model.checked_nodes.clear();
                    }
                }
                model.publications.remove(&kind);
            }
            SaveStatus::Rejected => {
                if let Some(publication) = model.publications.get_mut(&kind) {
                    publication.operation_id = None;
                    publication.error = publication_notice(status);
                }
            }
            _ => {}
        }
    }
}
pub(crate) fn submit(
    state: RwSignal<State>,
    kind: Kind,
    method: &'static str,
    endpoint: String,
    mut body: Value,
) {
    if state.with_untracked(|s| {
        !s.files.writable
            || kind != Kind::Backup && super::model::selected_entry_read_only(&s.files)
            || s.files.loading
            || actions::pending(s)
            || s.files
                .publications
                .get(&kind)
                .is_some_and(|publication| publication.operation_id.is_some())
    }) {
        return;
    }
    let id = api::operation_id();
    if id.is_empty() {
        return;
    }
    body["operation_id"] = id.clone().into();
    let epoch = state.with_untracked(|s| s.session_epoch);
    state.update(|s| {
        let publication = s.files.publications.entry(kind).or_default();
        publication.operation_id = Some(id.clone());
        publication.error.clear();
        s.files.mutation_pending = true;
    });
    spawn_local(async move {
        let result = api::request(method, &endpoint, Some(body)).await;
        if !current_session(state, epoch) {
            return;
        }
        match result {
            Ok(value) => actions::record_operation(state, value),
            Err(error) => {
                let unknown = error.status == 0 || error.status >= 500;
                if unknown {
                    actions::record_operation(state, json!({"operation_id":id,"state":"in_doubt"}));
                }
                state.update(|s| {
                    if let Some(publication) = s
                        .files
                        .publications
                        .get_mut(&kind)
                        .filter(|publication| publication.operation_id.as_deref() == Some(&id))
                    {
                        publication.error = error.to_string();
                        if !unknown {
                            publication.operation_id = None;
                        }
                    }
                });
                fail(state, error);
            }
        }
        state.update(|s| s.files.mutation_pending = false);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipts_settle_only_the_matching_publication_and_preserve_unknown_outcomes() {
        let mut model = Model::default();
        model.checked_nodes.insert("selected-node".into());
        model.publications.insert(
            Kind::Tags,
            Publication {
                operation_id: Some("original-tags-operation".into()),
                error: "network interrupted".into(),
            },
        );
        model.publications.insert(
            Kind::Merge,
            Publication {
                operation_id: Some("different-merge-operation".into()),
                error: String::new(),
            },
        );
        settle(&mut model, "unrelated-operation", SaveStatus::Published);
        assert_eq!(model.publications.len(), 2);
        settle(&mut model, "original-tags-operation", SaveStatus::Unknown);
        assert_eq!(
            model.publications[&Kind::Tags].operation_id.as_deref(),
            Some("original-tags-operation")
        );
        assert!(model.checked_nodes.contains("selected-node"));
        settle(&mut model, "original-tags-operation", SaveStatus::Rejected);
        assert!(model.publications[&Kind::Tags].operation_id.is_none());
        assert!(!model.publications[&Kind::Tags].error.is_empty());
        assert!(model.checked_nodes.contains("selected-node"));
        model
            .publications
            .get_mut(&Kind::Tags)
            .unwrap()
            .operation_id = Some("retry-tags-operation".into());
        settle(&mut model, "original-tags-operation", SaveStatus::Published);
        assert!(model.checked_nodes.contains("selected-node"));
        settle(&mut model, "retry-tags-operation", SaveStatus::Degraded);
        assert!(model.checked_nodes.is_empty());
        assert!(!model.publications.contains_key(&Kind::Tags));
        assert_eq!(
            model.publications[&Kind::Merge].operation_id.as_deref(),
            Some("different-merge-operation")
        );
    }
}
