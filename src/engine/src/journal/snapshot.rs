//! Page-at-a-time snapshot encoding for the single current wire format.
use super::*;

struct SnapshotBuilder {
    maximum: usize,
    pages: BTreeMap<String, Vec<String>>,
    objects: Vec<PreparedMetadataObject>,
    #[cfg(test)]
    peak_page_entries: usize,
}
impl SnapshotBuilder {
    fn map<K: AsRef<str> + Ord + Serialize, V: Serialize>(
        &mut self,
        field: &str,
        entries: impl IntoIterator<Item = (K, V)>,
    ) -> Result<()> {
        let mut page = BTreeMap::new();
        let mut hashes = Vec::new();
        for (key, value) in entries {
            let boundary = blake3::hash(key.as_ref().as_bytes()).as_bytes()[0] & 31 == 0;
            if !page.is_empty() && (page.len() == STATE_PAGE_ENTRIES || boundary) {
                self.flush(&mut page, &mut hashes)?;
            }
            page.insert(key, value);
            #[cfg(test)]
            {
                self.peak_page_entries = self.peak_page_entries.max(page.len());
            }
        }
        if !page.is_empty() {
            self.flush(&mut page, &mut hashes)?;
        }
        self.pages.insert(field.into(), hashes);
        Ok(())
    }
    fn flush<K: Serialize, V: Serialize>(
        &mut self,
        page: &mut BTreeMap<K, V>,
        hashes: &mut Vec<String>,
    ) -> Result<()> {
        let object = PreparedMetadataObject::new("state-page", page, self.maximum)?;
        hashes.push(object.hash.clone());
        self.objects.push(object);
        page.clear();
        Ok(())
    }
}

pub(super) fn prepare(state: &State, maximum: usize) -> Result<PreparedSnapshot> {
    let mut builder = SnapshotBuilder {
        maximum,
        pages: BTreeMap::new(),
        objects: Vec::new(),
        #[cfg(test)]
        peak_page_entries: 0,
    };
    // One list drives both exhaustive State matching and serialization: adding
    // a field cannot compile until its encoding category is declared here.
    macro_rules! map_fields {
        (required { $($required:ident,)* } optional { $($optional:ident,)* }) => {{
            let State {
                sequence: _, business_generation: _, topology_generation: _,
                control_auth: _, events: _, directory_selection_generations: _,
                file_selection_generations: _,
                $($required,)* $($optional,)*
            } = state;
            $(builder.map(stringify!($required), $required.iter())?;)*
            $(if !$optional.is_empty() {
                builder.map(stringify!($optional), $optional.iter())?;
            })*
        }};
    }
    map_fields! {
        required {
            repositories,
            histories,
            branches,
            commits,
            versions,
            placements,
            merge_links,
            history_boundaries,
            content_states,
            operations,
            plans,
            placement_rules,
            protection_policies,
            operation_principals,
        }
        optional {
            file_selections,
            directory_branches,
            directory_trees,
            file_write_origins,
            file_contexts,
            file_path_nodes,
            file_metadata_nodes,
            file_branch_merges,
            file_branch_aliases,
            file_node_parents,
            file_path_notes,
            directory_tree_selections,
            file_locations,
            file_backup_paths,
            file_backup_policies,
            directory_selections,
            file_notes,
            version_notes,
            file_names,
        }
    }
    let sequence = state.sequence;
    let business_generation = state.business_generation;
    let topology_generation = state.topology_generation;
    let control_auth = &state.control_auth;
    let events = &state.events;
    let directory_selection_generations = &state.directory_selection_generations;
    let file_selection_generations = &state.file_selection_generations;
    // These State maps use decimal strings, including values above 2^53.
    if !directory_selection_generations.is_empty() {
        builder.map(
            "directory_selection_generations",
            directory_selection_generations
                .iter()
                .map(|(key, value)| (key, value.to_string())),
        )?;
    }
    if !file_selection_generations.is_empty() {
        builder.map(
            "file_selection_generations",
            file_selection_generations
                .iter()
                .map(|(key, value)| (key, value.to_string())),
        )?;
    }
    builder.map(
        "events",
        events
            .iter()
            .enumerate()
            .map(|(index, event)| (format!("{index:020}"), event)),
    )?;
    let mut scalars = json_codec::Map::new();
    scalars.insert("sequence".into(), sequence.to_string().into());
    scalars.insert(
        "business_generation".into(),
        business_generation.to_string().into(),
    );
    scalars.insert(
        "topology_generation".into(),
        topology_generation.to_string().into(),
    );
    // Preserve the old encoder's treatment of any top-level object as pages,
    // including optional authentication JSON; other JSON shapes remain scalars.
    if let Some(json_codec::Value::Object(map)) = control_auth {
        builder.map("control_auth", map.iter())?;
    } else {
        scalars.insert("control_auth".into(), json_codec::to_value(control_auth)?);
    }
    let descriptor = Snapshot {
        sequence,
        pages: builder.pages,
        scalars,
    };
    let root = PreparedMetadataObject::new("snapshot", &descriptor, maximum)?;
    let hash = root.hash.clone();
    builder.objects.push(root);
    #[cfg(test)]
    SNAPSHOT_PEAK_PAGE_ENTRIES.with(|peak| peak.set(builder.peak_page_entries));
    Ok(PreparedSnapshot {
        descriptor,
        hash,
        objects: builder.objects,
    })
}

#[cfg(test)]
thread_local! {
    pub(super) static SNAPSHOT_PEAK_PAGE_ENTRIES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
