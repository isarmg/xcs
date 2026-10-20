//! One presentation graph for content, path changes and metadata revisions.
use super::*;

/// Metadata UUID order is unrelated to graph order. Resolve a path through the
/// explicit source chain and memoize it without recursive stack growth.
struct NodePaths<'a> {
    engine: &'a Engine,
    commits: &'a BTreeMap<&'a str, &'a zxs_contract::Commit>,
    chosen: BTreeMap<&'a str, &'a str>,
    cache: BTreeMap<String, String>,
}
impl NodePaths<'_> {
    fn get(&mut self, node: &str) -> String {
        let state = self.engine.state();
        let mut current = node.to_owned();
        let mut trail = Vec::new();
        let mut seen = BTreeSet::new();
        let path = loop {
            if let Some(path) = self.cache.get(&current) {
                break path.clone();
            }
            if !seen.insert(current.clone()) {
                break String::new();
            }
            trail.push(current.clone());
            if let Some(path) = state.file_path_nodes.get(&current) {
                break path.path.clone();
            }
            if let Some(version) = state.versions.get(&current) {
                break self
                    .commits
                    .get(current.as_str())
                    .and_then(|commit| {
                        commit.entries.iter().find(|entry| {
                            entry.file_id == version.file_id && entry.version_id == current
                        })
                    })
                    .map(|entry| entry.path.clone())
                    .unwrap_or_else(|| version.creation_name.clone());
            }
            if let Some(metadata) = state.file_metadata_nodes.get(&current) {
                let source = if metadata.kind == "merge" {
                    self.chosen.get(current.as_str()).copied()
                } else {
                    metadata.parent_node_ids.first().map(String::as_str)
                };
                if let Some(source) = source {
                    current = source.into();
                    continue;
                }
                break state
                    .versions
                    .get(&metadata.version_id)
                    .map(|body| body.creation_name.clone())
                    .unwrap_or_default();
            }
            break String::new();
        };
        for id in trail {
            self.cache.insert(id, path.clone());
        }
        path
    }
}

pub(super) fn unified_history(
    engine: &Engine,
    file: &str,
    history: &str,
    versions: &[Value],
    first_commits: &BTreeMap<&str, &zxs_contract::Commit>,
    current_node: Option<&str>,
) -> Vec<Value> {
    let state = engine.state();
    let mut paths = NodePaths {
        engine,
        commits: first_commits,
        chosen: state
            .file_branch_merges
            .values()
            .filter(|merge| !merge.reused_node)
            .map(|merge| (merge.result_node_id.as_str(), merge.chosen_node_id.as_str()))
            .collect(),
        cache: BTreeMap::new(),
    };
    let version_metadata = versions
        .iter()
        .filter_map(|version| version["version_id"].as_str().map(|id| (id, version)))
        .collect::<BTreeMap<_, _>>();
    let path_nodes = state
        .file_path_nodes
        .values()
        .filter(|node| node.file_id == file && node.history_tree_id == history)
        .collect::<Vec<_>>();
    let visible = versions
        .iter()
        .filter_map(|version| version["version_id"].as_str())
        .chain(path_nodes.iter().map(|node| node.node_id.as_str()))
        .chain(
            state
                .file_metadata_nodes
                .values()
                .filter(|node| node.file_id == file && node.history_tree_id == history)
                .map(|node| node.node_id.as_str()),
        )
        .collect::<BTreeSet<_>>();
    let mut nodes = versions
        .iter()
        .map(|version| {
            let mut node = version.clone();
            let id = version["version_id"].as_str().unwrap_or("");
            let commit = first_commits.get(id).copied();
            let parents = engine.file_node_parents(file, id).unwrap_or_default();
            node["node_id"] = id.into();
            node["kind"] = "content".into();
            node["tags"] = json!(engine.file_node_tags(file, id).unwrap_or_default());
            node["parent_node_ids"] = json!(
                parents
                    .into_iter()
                    .filter(|parent| visible.contains(parent.as_str()))
                    .collect::<Vec<_>>()
            );
            node["path"] = commit
                .and_then(|commit| {
                    commit
                        .entries
                        .iter()
                        .find(|entry| entry.file_id == file && entry.version_id == id)
                })
                .map(|entry| entry.path.clone())
                .unwrap_or_else(|| version["download_name"].as_str().unwrap_or("").to_owned())
                .into();
            node["previous_path"] = Value::Null;
            node["sequence"] = commit
                .map(|commit| commit.sequence.to_string())
                .unwrap_or_else(|| version["sequence"].as_str().unwrap_or("0").to_owned())
                .into();
            node["current"] = (current_node == Some(id)).into();
            node
        })
        .collect::<Vec<_>>();
    for path in path_nodes {
        let Some(body) = state.versions.get(&path.version_id) else {
            continue;
        };
        let metadata = version_metadata.get(path.version_id.as_str()).copied();
        nodes.push(json!({
            "node_id":path.node_id,"version_id":path.version_id,"file_id":file,
            "parent_node_ids":engine.file_node_parents(file,&path.node_id).unwrap_or_default().into_iter().filter(|parent|visible.contains(parent.as_str())).collect::<Vec<_>>(),
            "kind":"rename","path":path.path,"previous_path":path.previous_path,
            "branch_id":path.branch_id,"created_at":path.created_at.to_string(),"sequence":path.sequence.to_string(),
            "current":current_node==Some(path.node_id.as_str()),
            "note":state.file_path_notes.get(&path.node_id).map(String::as_str).unwrap_or(""),
            "size":body.logical_size.to_string(),"content_hash":body.content_hash,
            "content_state":state.content_states.get(&path.version_id).map(String::as_str).unwrap_or("LIVE"),
            "download_name":metadata.and_then(|version|version.get("download_name")).cloned().unwrap_or_else(||body.creation_name.clone().into()),
            "storage_name":body.storage_name,
            "operation_id":path.operation_id,"author":path.author,"message":""
            ,"tags":engine.file_node_tags(file,&path.node_id).unwrap_or_default()
        }));
    }
    for revision in state
        .file_metadata_nodes
        .values()
        .filter(|node| node.file_id == file && node.history_tree_id == history)
    {
        let Some(body) = state.versions.get(&revision.version_id) else {
            continue;
        };
        let metadata = version_metadata.get(revision.version_id.as_str()).copied();
        let path = paths.get(&revision.node_id);
        nodes.push(json!({"node_id":revision.node_id,"version_id":revision.version_id,"file_id":file,
            "kind":revision.kind,"parent_node_ids":engine.file_node_parents(file,&revision.node_id).unwrap_or_default().into_iter().filter(|parent|visible.contains(parent.as_str())).collect::<Vec<_>>(),
            "tags":revision.tags,"path":path,"previous_path":null,"branch_id":revision.branch_id,
            "created_at":revision.created_at.to_string(),"sequence":revision.sequence.to_string(),"current":current_node==Some(revision.node_id.as_str()),
            "note":state.file_path_notes.get(&revision.node_id).map(String::as_str).unwrap_or(""),
            "size":body.logical_size.to_string(),"content_hash":body.content_hash,"content_state":state.content_states.get(&revision.version_id).map(String::as_str).unwrap_or("LIVE"),
            "download_name":metadata.and_then(|version|version.get("download_name")).cloned().unwrap_or_else(||body.creation_name.clone().into()),
            "storage_name":body.storage_name,"operation_id":revision.operation_id,"author":revision.author,"message":""}));
    }
    nodes.sort_by(|left, right| {
        let sequence = |node: &Value| {
            node["sequence"]
                .as_str()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(0)
        };
        sequence(right)
            .cmp(&sequence(left))
            .then_with(|| left["node_id"].as_str().cmp(&right["node_id"].as_str()))
    });
    nodes
}
