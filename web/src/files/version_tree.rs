//! File history ancestry, including body and path nodes, laid out once per identity/edge change. Notes never
//! participate in layout keys, so refreshing metadata preserves active editors.
use super::model::{
    Model, history_node_id, history_nodes, inherited_content, materialized, visible_content,
};
use super::{actions, branch, inline};
#[cfg(test)]
use crate::ui::list;
use crate::{
    format,
    ui::{State, text},
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use wasm_bindgen::{JsCast, closure::Closure};
use xcs_web::{html, i18n::t, prelude::*};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Version {
    id: String,
    created: String,
    parents: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Layout {
    levels: Vec<Vec<String>>,
    edges: Vec<(String, String)>,
}

fn metadata(records: &[Value], file_id: &str) -> Vec<Version> {
    let mut versions = records
        .iter()
        .filter_map(|record| {
            let id = history_node_id(record);
            let owner = text(record, &["file_id"]);
            if id.is_empty()
                || !(materialized(record)
                    || (matches!(record["kind"].as_str(), Some("rename" | "tags" | "merge"))
                        && visible_content(record)))
                || (!owner.is_empty() && !file_id.is_empty() && owner != file_id)
            {
                return None;
            }
            let mut parents = record
                .get("parent_node_ids")
                .and_then(Value::as_array)
                .or_else(|| record.get("source_version_ids").and_then(Value::as_array))
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .filter(|parent| !parent.is_empty() && *parent != id)
                .map(str::to_owned)
                .collect::<Vec<_>>();
            parents.sort();
            parents.dedup();
            let created = record["created_at"]
                .as_str()
                .map(str::to_owned)
                .or_else(|| record["created_at"].as_u64().map(|v| v.to_string()))
                .unwrap_or_default();
            Some(Version {
                id,
                created,
                parents,
            })
        })
        .collect::<Vec<_>>();
    versions.sort();
    versions.dedup();
    versions
}

fn newer_first(a: &Version, b: &Version) -> std::cmp::Ordering {
    a.created
        .is_empty()
        .cmp(&b.created.is_empty())
        .then_with(
            || match (a.created.parse::<u64>(), b.created.parse::<u64>()) {
                (Ok(a), Ok(b)) => b.cmp(&a),
                (Ok(_), Err(_)) => std::cmp::Ordering::Less,
                (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
                (Err(_), Err(_)) => b.created.cmp(&a.created),
            },
        )
        .then_with(|| a.id.cmp(&b.id))
}

/// This marker belongs to the selected branch projection, not the immutable
/// parent version. Its tree node remains one real historical node.
fn inherited_current(model: &Model, version: &str) -> bool {
    (inherited_content(&model.file_history) && history_node_id(&model.file_history) == version)
        || model
            .selected
            .as_ref()
            .is_some_and(|entry| inherited_content(entry) && history_node_id(entry) == version)
}

fn reachable(start: &str, goal: &str, children: &BTreeMap<String, BTreeSet<String>>) -> bool {
    let mut pending = vec![start.to_owned()];
    let mut seen = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if id == goal {
            return true;
        }
        if seen.insert(id.clone()) {
            pending.extend(children.get(&id).into_iter().flatten().cloned());
        }
    }
    false
}

/// Only declared, visible parents produce edges. Cyclic edges are isolated
/// rather than inventing chronology or making the traversal recurse forever.
fn layout(versions: &[Version]) -> Layout {
    let mut nodes = BTreeMap::<String, Version>::new();
    // Stable conflict handling also makes duplicate identities harmless.
    let mut sorted = versions.to_vec();
    sorted.sort();
    for version in sorted {
        nodes.entry(version.id.clone()).or_insert(version);
    }
    let mut children = BTreeMap::<String, BTreeSet<String>>::new();
    for version in nodes.values() {
        for parent in &version.parents {
            if parent != &version.id && nodes.contains_key(parent) {
                children
                    .entry(parent.clone())
                    .or_default()
                    .insert(version.id.clone());
            }
        }
    }
    let mut edges = Vec::new();
    for (parent, descendants) in &children {
        for child in descendants {
            if !reachable(child, parent, &children) {
                edges.push((parent.clone(), child.clone()));
            }
        }
    }
    // Walk from all branch tips towards old ancestors. This is a reverse
    // topological layout, not a visual reversal of the old root-first layers:
    // tips of both short and long branches belong on the first layer.
    let mut remaining_children = nodes
        .keys()
        .map(|id| (id.clone(), 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut accepted_parents = BTreeMap::<String, Vec<String>>::new();
    for (parent, child) in &edges {
        *remaining_children.get_mut(parent).expect("visible parent") += 1;
        accepted_parents
            .entry(child.clone())
            .or_default()
            .push(parent.clone());
    }
    let mut ready = nodes
        .values()
        .filter(|v| remaining_children[&v.id] == 0)
        .collect::<Vec<_>>();
    ready.sort_by(|a, b| newer_first(a, b));
    let mut depths = BTreeMap::<String, usize>::new();
    let mut levels = Vec::<Vec<String>>::new();
    while !ready.is_empty() {
        let version = ready.remove(0);
        let depth = depths.get(&version.id).copied().unwrap_or(0);
        if levels.len() <= depth {
            levels.resize(depth + 1, Vec::new());
        }
        levels[depth].push(version.id.clone());
        for parent in accepted_parents.get(&version.id).into_iter().flatten() {
            depths
                .entry(parent.clone())
                .and_modify(|d| *d = (*d).max(depth + 1))
                .or_insert(depth + 1);
            let count = remaining_children.get_mut(parent).expect("visible parent");
            *count -= 1;
            if *count == 0 {
                ready.push(&nodes[parent]);
            }
        }
        ready.sort_by(|a, b| newer_first(a, b));
    }
    if let Some(tips) = levels.first_mut() {
        tips.sort_by(|a, b| newer_first(&nodes[a], &nodes[b]));
    }
    for depth in 1..levels.len() {
        let positions = levels[depth - 1]
            .iter()
            .enumerate()
            .map(|(index, id)| (id.as_str(), index))
            .collect::<BTreeMap<_, _>>();
        // Group ancestors by the positions of their newer children on the
        // preceding display layer; all cross-layer source edges remain intact.
        let centers = levels[depth]
            .iter()
            .map(|id| {
                let children = edges
                    .iter()
                    .filter(|(parent, _)| parent == id)
                    .filter_map(|(_, child)| positions.get(child.as_str()).copied())
                    .collect::<Vec<_>>();
                let key = if children.is_empty() {
                    (usize::MAX as u128, 1u128, usize::MAX)
                } else {
                    (
                        children.iter().map(|index| *index as u128).sum(),
                        children.len() as u128,
                        *children.iter().min().expect("nonempty children"),
                    )
                };
                (id.clone(), key)
            })
            .collect::<BTreeMap<_, _>>();
        levels[depth].sort_by(|a, b| {
            let (a_sum, a_count, a_min) = centers[a];
            let (b_sum, b_count, b_min) = centers[b];
            (a_sum * b_count)
                .cmp(&(b_sum * a_count))
                .then_with(|| a_min.cmp(&b_min))
                .then_with(|| newer_first(&nodes[a], &nodes[b]))
        });
    }
    Layout { levels, edges }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Geometry {
    width: f64,
    height: f64,
    paths: Vec<(String, String, String)>,
}

#[derive(Clone, Copy, Debug)]
struct NodeBounds {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
    depth: usize,
}
impl NodeBounds {
    fn center(self) -> f64 {
        (self.left + self.right) / 2.
    }
}

/// Arrows retain their old-parent → new-child meaning while travelling up.
/// The target gap follows the entire newer layer so a short target's connector
/// cannot pass through a taller sibling before arriving at its bottom edge.
fn gap_heights(parent: NodeBounds, child: NodeBounds, boxes: &[NodeBounds]) -> (f64, f64) {
    let exit = boxes
        .iter()
        .filter(|node| node.depth == parent.depth)
        .map(|node| node.top)
        .fold(parent.top, f64::min)
        - 6.;
    let entry = boxes
        .iter()
        .filter(|node| node.depth == child.depth)
        .map(|node| node.bottom)
        .fold(child.bottom, f64::max)
        + 6.;
    (exit, entry)
}

fn long_route(parent: NodeBounds, child: NodeBounds, boxes: &[NodeBounds]) -> Vec<(f64, f64)> {
    let (exit, entry) = gap_heights(parent, child, boxes);
    let sx = parent.center();
    let tx = child.center();
    let left = boxes
        .iter()
        .map(|node| node.left)
        .fold(parent.left, f64::min)
        - 9.;
    let right = boxes
        .iter()
        .map(|node| node.right)
        .fold(parent.right, f64::max)
        + 9.;
    let lane = if (sx - left).abs() + (tx - left).abs() <= (sx - right).abs() + (tx - right).abs() {
        left
    } else {
        right
    };
    vec![
        (sx, parent.top),
        (sx, exit),
        (lane, exit),
        (lane, entry),
        (tx, entry),
        (tx, child.bottom + 3.),
    ]
}

const EDGE_CORNER_RADIUS: f64 = 4.;

/// Draw only horizontal/vertical runs and circular quarter turns. Adjacent
/// corners each consume at most half their shared run, so even short routes
/// cannot double back or overlap their own rounded turns.
fn rounded_route(points: &[(f64, f64)], radius: f64) -> Option<String> {
    if !radius.is_finite() || radius <= 0. || points.is_empty() {
        return None;
    }
    let mut route = Vec::<(f64, f64)>::new();
    for &(x, y) in points {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        if let Some(&(px, py)) = route.last() {
            if x == px && y == py {
                continue;
            }
            if x != px && y != py {
                return None;
            }
        }
        // Remove straight-through waypoints; keep every real corner and reject
        // reversals instead of inventing a circular U-turn through a node.
        while route.len() >= 2 {
            let a = route[route.len() - 2];
            let b = route[route.len() - 1];
            let incoming = (b.0 - a.0, b.1 - a.1);
            let outgoing = (x - b.0, y - b.1);
            if incoming.0 * outgoing.1 != incoming.1 * outgoing.0 {
                break;
            }
            if incoming.0 * outgoing.0 + incoming.1 * outgoing.1 <= 0. {
                return None;
            }
            route.pop();
        }
        route.push((x, y));
    }
    let &(x, y) = route.first()?;
    if route.len() < 2 {
        return None;
    }
    let mut path = format!("M{x:.3},{y:.3}");
    let mut cursor = (x, y);
    let line = |path: &mut String, cursor: &mut (f64, f64), point: (f64, f64)| {
        if point == *cursor {
            return;
        }
        if point.0 == cursor.0 {
            path.push_str(&format!(" V{:.3}", point.1));
        } else {
            path.push_str(&format!(" H{:.3}", point.0));
        }
        *cursor = point;
    };
    for turn in route.windows(3) {
        let [previous, corner, next] = [turn[0], turn[1], turn[2]];
        let incoming_length = (corner.0 - previous.0).abs() + (corner.1 - previous.1).abs();
        let outgoing_length = (next.0 - corner.0).abs() + (next.1 - corner.1).abs();
        let incoming = (
            (corner.0 - previous.0) / incoming_length,
            (corner.1 - previous.1) / incoming_length,
        );
        let outgoing = (
            (next.0 - corner.0) / outgoing_length,
            (next.1 - corner.1) / outgoing_length,
        );
        let radius = radius.min(incoming_length / 2.).min(outgoing_length / 2.);
        let start = (
            corner.0 - incoming.0 * radius,
            corner.1 - incoming.1 * radius,
        );
        let end = (
            corner.0 + outgoing.0 * radius,
            corner.1 + outgoing.1 * radius,
        );
        line(&mut path, &mut cursor, start);
        let clockwise = u8::from(incoming.0 * outgoing.1 - incoming.1 * outgoing.0 > 0.);
        path.push_str(&format!(
            " A{radius:.3},{radius:.3} 0 0 {clockwise} {:.3},{:.3}",
            end.0, end.1
        ));
        cursor = end;
    }
    line(&mut path, &mut cursor, *route.last()?);
    Some(path)
}

fn edge_path(parent: NodeBounds, child: NodeBounds, boxes: &[NodeBounds]) -> Option<String> {
    if parent.depth <= child.depth
        || parent.top <= child.bottom
        || !boxes.iter().all(|node| {
            [node.left, node.top, node.right, node.bottom]
                .iter()
                .all(|value| value.is_finite())
        })
    {
        return None;
    }
    if parent.depth > child.depth + 1 {
        return rounded_route(&long_route(parent, child, boxes), EDGE_CORNER_RADIUS);
    }
    let (exit, entry) = gap_heights(parent, child, boxes);
    let middle = (exit + entry) / 2.;
    let sx = parent.center();
    let tx = child.center();
    let sy = parent.top;
    let ty = child.bottom + 3.;
    rounded_route(
        &[(sx, sy), (sx, middle), (tx, middle), (tx, ty)],
        EDGE_CORNER_RADIUS,
    )
}

fn measure(root: &web_sys::Element, layout: &Layout) -> Geometry {
    let bounds = root.get_bounding_client_rect();
    let depths = layout
        .levels
        .iter()
        .enumerate()
        .flat_map(|(depth, ids)| ids.iter().map(move |id| (id.as_str(), depth)))
        .collect::<BTreeMap<_, _>>();
    let mut boxes = BTreeMap::new();
    if let Ok(nodes) = root.query_selector_all(".xcs-version-node[data-version-id]") {
        for index in 0..nodes.length() {
            if let Some(element) = nodes
                .item(index)
                .and_then(|node| node.dyn_into::<web_sys::Element>().ok())
            {
                if let Some(id) = element.get_attribute("data-version-id") {
                    if let Some(&depth) = depths.get(id.as_str()) {
                        let rect = element.get_bounding_client_rect();
                        boxes.insert(
                            id,
                            NodeBounds {
                                left: rect.left() - bounds.left(),
                                right: rect.right() - bounds.left(),
                                top: rect.top() - bounds.top(),
                                bottom: rect.bottom() - bounds.top(),
                                depth,
                            },
                        );
                    }
                }
            }
        }
    }
    let all = boxes.values().copied().collect::<Vec<_>>();
    let paths = layout
        .edges
        .iter()
        .filter_map(|(parent, child)| {
            Some((
                parent.clone(),
                child.clone(),
                edge_path(*boxes.get(parent)?, *boxes.get(child)?, &all)?,
            ))
        })
        .collect();
    Geometry {
        width: bounds.width().max(1.),
        height: bounds.height().max(1.),
        paths,
    }
}

fn observe(root: NodeRef<html::Div>, layout: Memo<Layout>, geometry: RwSignal<Geometry>) {
    root.on_load(move |element| {
        let element: web_sys::Element = element.unchecked_into();
        let update_root = element.clone();
        let initially_centered = std::rc::Rc::new(std::cell::Cell::new(false));
        let update = move || {
            let next = measure(&update_root, &layout.get_untracked());
            if geometry.with_untracked(|old| old != &next) {
                geometry.set(next);
            }
            // A wide tree initially reveals its newest tip rather than an empty
            // left corner. This belongs to this mounted tree only: refreshes, note
            // edits and later resizes never take the user's scroll position.
            if !initially_centered.get() {
                if let (Some(scroll), Ok(Some(first_tip))) = (
                    update_root.parent_element(),
                    update_root.query_selector(
                        ".xcs-version-tree-level[data-version-depth=\"0\"] .xcs-version-node",
                    ),
                ) {
                    let rect = first_tip.get_bounding_client_rect();
                    let viewport = scroll.client_width();
                    if rect.width() > 0. && rect.height() > 0. && viewport > 0 {
                        let frame = scroll.get_bounding_client_rect();
                        let center = rect.left() - frame.left()
                            + f64::from(scroll.scroll_left())
                            + rect.width() / 2.;
                        let maximum = (scroll.scroll_width() - viewport).max(0);
                        scroll.set_scroll_left(
                            (center - f64::from(viewport) / 2.)
                                .clamp(0., f64::from(maximum))
                                .round() as i32,
                        );
                        initially_centered.set(true);
                    }
                }
            }
        };
        update();
        let resize_update = update.clone();
        let callback = Closure::wrap(Box::new(
            move |_: js_sys::Array, _: web_sys::ResizeObserver| resize_update(),
        )
            as Box<dyn FnMut(js_sys::Array, web_sys::ResizeObserver)>);
        let Ok(resize) = web_sys::ResizeObserver::new(callback.as_ref().unchecked_ref()) else {
            return;
        };
        resize.observe(&element);
        let observe_nodes = {
            let resize = resize.clone();
            let element = element.clone();
            move || {
                if let Ok(nodes) = element.query_selector_all(".xcs-version-node") {
                    for index in 0..nodes.length() {
                        if let Some(node) = nodes
                            .item(index)
                            .and_then(|node| node.dyn_into::<web_sys::Element>().ok())
                        {
                            resize.observe(&node);
                        }
                    }
                }
            }
        };
        observe_nodes();
        let mutation_callback = Closure::wrap(Box::new(
            move |_: js_sys::Array, _: web_sys::MutationObserver| {
                observe_nodes();
                update();
            },
        )
            as Box<dyn FnMut(js_sys::Array, web_sys::MutationObserver)>);
        let mutation =
            web_sys::MutationObserver::new(mutation_callback.as_ref().unchecked_ref()).ok();
        if let Some(observer) = &mutation {
            let options = web_sys::MutationObserverInit::new();
            options.set_child_list(true);
            options.set_subtree(true);
            let _ = observer.observe_with_options(&element, &options);
        }
        let cleanup =
            send_wrapper::SendWrapper::new((resize, callback, mutation, mutation_callback));
        on_cleanup(move || {
            cleanup.0.disconnect();
            if let Some(observer) = &cleanup.2 {
                observer.disconnect();
            }
            drop(cleanup);
        });
    });
}

pub(super) fn view(state: RwSignal<State>) -> impl IntoView {
    let versions = Memo::new(move |_| {
        state.with(|s| {
            let id = s
                .files
                .selected
                .as_ref()
                .map(|entry| text(entry, &["file_id", "entry_id"]))
                .unwrap_or_default();
            metadata(&history_nodes(&s.files.file_history), &id)
        })
    });
    let tree = Memo::new(move |_| layout(&versions.get()));
    let geometry = RwSignal::new(Geometry::default());
    let root = NodeRef::<html::Div>::new();
    observe(root, tree, geometry);
    view! {
        <div class="xcs-version-tree-scroll" aria-label=t("文件版本树","File version tree")>
            <div class="xcs-version-tree-canvas" node_ref=root>
                <svg class="xcs-version-tree-links" aria-hidden="true" focusable="false"
                    viewBox=move ||geometry.with(|g|format!("0 0 {} {}",g.width.max(1.),g.height.max(1.))) preserveAspectRatio="none">
                    <defs><marker id="xcs-version-arrow" class="xcs-version-tree-arrow" markerWidth="8" markerHeight="8" refX="8" refY="4" orient="auto" markerUnits="userSpaceOnUse"><path d="M0,0 L8,4 L0,8 Z"/></marker></defs>
                    <For each=move ||geometry.with(|g|g.paths.clone()) key=|edge|(edge.0.clone(),edge.1.clone()) children=move |(parent,child,_)| {
                        let key=StoredValue::new((parent.clone(),child.clone()));
                        view!{<path class="xcs-version-tree-edge" data-parent-version=parent data-child-version=child d=move ||geometry.with(|g|g.paths.iter().find(|edge|key.with_value(|key|edge.0==key.0&&edge.1==key.1)).map(|edge|edge.2.clone()).unwrap_or_default()) marker-end="url(#xcs-version-arrow)" stroke-dasharray="none" stroke-linecap="round" stroke-linejoin="round" vector-effect="non-scaling-stroke"/>}
                    }/>
                </svg>
                <For each=move ||tree.with(|tree|tree.levels.iter().enumerate().map(|(depth,ids)|(depth,ids.clone())).collect::<Vec<_>>()) key=|level|level.0 children=move |(depth,_)| {
                    view!{<div class="xcs-version-tree-level" data-version-depth=depth.to_string()>
                        <For each=move ||tree.with(|tree|tree.levels.get(depth).cloned().unwrap_or_default()) key=|id|id.clone() children=move |id|node(state,id)/>
                    </div>}
                }/>
            </div>
        </div>
    }
}

fn node(state: RwSignal<State>, version_id: String) -> impl IntoView {
    let id = StoredValue::new(version_id);
    let version = Memo::new(move |_| {
        state.with(|s| {
            history_nodes(&s.files.file_history)
                .into_iter()
                .find(|v| history_node_id(v) == id.get_value())
                .unwrap_or(Value::Null)
        })
    });
    let can_modify = Signal::derive(move || {
        state.with(|s| s.files.writable && !super::model::selected_entry_read_only(&s.files))
    });
    let disabled = Signal::derive(move || !can_modify.get() || state.with(actions::pending));
    let inherited =
        Signal::derive(move || state.with(|s| inherited_current(&s.files, &id.get_value())));
    let note_target = version.with_untracked(|node| {
        if matches!(node["kind"].as_str(), Some("rename" | "tags" | "merge")) {
            inline::Target::NodeNote(id.get_value())
        } else {
            inline::Target::VersionNote(text(node, &["version_id"]))
        }
    });
    let (note, edit) = inline::version_note(state, note_target, id.get_value(), disabled);
    view! {
        <article class="xcs-content-panel xcs-version-node xcs-frame xcs-stack xcs-stack--flush" data-version-id=id.get_value() data-node-id=id.get_value() data-body-version-id=move ||version.with(|node|text(node,&["version_id"])) data-node-kind=move ||version.with(|node|text(node,&["kind"]))
            aria-label=move ||version.with(|v|if v["current"]==true{if inherited.get(){t("继承的父版本","Inherited parent version")}else{t("当前版本","Current version")}}else{t("历史版本","Historical version")})>
            <div class="xcs-version-date xcs-row" title=id.get_value()>
                <Show when=move ||can_modify.get()><input class="xcs-node-checkbox" type="checkbox" aria-label=t("选择此节点","Select this node") disabled=move ||disabled.get()||state.with(|s|s.files.tag_edit.is_some()) prop:checked=move ||state.with(|s|s.files.checked_nodes.contains(&id.get_value())) on:change=move |event|super::tags::toggle(state,id.get_value(),event_target_checked(&event))/></Show>
                <time>{move ||version.with(|v|v["created_at"].as_str().map(|value|if value.contains('T'){format::timestamp(value)}else{format::timestamp_seconds(value)}).unwrap_or_else(||t("未记录时间","Time not recorded")))}</time>
                <Show when=move ||version.with(|v|v["current"]==true)><span class="xcs-version-current">{move ||if inherited.get(){t("继承的父版本","Inherited parent version")}else{t("当前","Current")}}</span></Show>
            </div>
            {note}
            <div class="xcs-version-actions xcs-flow xcs-flow--start">
                {edit}
                <Show when=move ||can_modify.get() fallback=move ||view!{<span class="xcs-row">{t("只读","Read only")}</span>}>
                <button class="xcs-button" disabled=move ||disabled.get() on:click=move |_|super::tags::edit_node(state,id.get_value())>{t("编辑标签","Edit tags")}</button>
                {branch::control(state,t("创建分支","Create branch"),branch::Target::HistoryNode(id.get_value()),disabled)}
                </Show>
            </div>
        </article>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rename_nodes_reuse_body_versions_but_keep_their_own_ancestry() {
        let records = vec![
            serde_json::json!({"file_id":"file","node_id":"ver-one","version_id":"ver-one","parent_node_ids":[],"kind":"content","created_at":"1"}),
            serde_json::json!({"file_id":"file","node_id":"fnode-rename","version_id":"ver-one","parent_node_ids":["ver-one"],"kind":"rename","created_at":"2"}),
            serde_json::json!({"file_id":"file","node_id":"ver-two","version_id":"ver-two","parent_node_ids":["fnode-rename"],"source_version_ids":["ver-one"],"kind":"content","created_at":"3"}),
        ];
        let tree = layout(&metadata(&records, "file"));
        assert_eq!(
            tree.levels,
            vec![
                vec!["ver-two".to_owned()],
                vec!["fnode-rename".to_owned()],
                vec!["ver-one".to_owned()]
            ]
        );
        assert_eq!(
            tree.edges,
            vec![
                ("fnode-rename".into(), "ver-two".into()),
                ("ver-one".into(), "fnode-rename".into())
            ]
        );
        let model = Model {
            file_history: serde_json::json!({"node_id":"fnode-rename","version_id":"ver-one","inherited":true}),
            ..Default::default()
        };
        assert!(inherited_current(&model, "fnode-rename"));
        assert!(!inherited_current(&model, "ver-one"));
    }
    fn v(id: &str, created: u64, parents: &[&str]) -> Version {
        Version {
            id: id.into(),
            created: created.to_string(),
            parents: parents.iter().map(|p| (*p).into()).collect(),
        }
    }
    fn ids(tree: &Layout) -> BTreeSet<String> {
        tree.levels.iter().flatten().cloned().collect()
    }

    #[test]
    fn a_real_fork_has_two_siblings_and_a_merge_keeps_both_parent_edges() {
        let tree = layout(&[
            v("root", 1, &[]),
            v("left", 2, &["root"]),
            v("right", 3, &["root"]),
            v("merge", 4, &["left", "right"]),
        ]);
        assert_eq!(
            tree.levels,
            vec![vec!["merge"], vec!["right", "left"], vec!["root"]]
        );
        assert_eq!(tree.edges.len(), 4);
        assert!(tree.edges.contains(&("left".into(), "merge".into())));
        assert!(tree.edges.contains(&("right".into(), "merge".into())));
    }
    #[test]
    fn timestamps_without_parent_evidence_never_invent_a_chain() {
        let tree = layout(&[
            v("new", 30, &[]),
            v("old", 1, &[]),
            v("missing", 2, &["invisible"]),
        ]);
        assert_eq!(tree.levels, vec![vec!["new", "missing", "old"]]);
        assert!(tree.edges.is_empty());
    }
    #[test]
    fn topology_wins_over_misordered_timestamps_and_input_order_is_irrelevant() {
        let records = vec![
            v("late-parent", 99, &[]),
            v("early-child", 1, &["late-parent"]),
            v("sibling", 2, &["late-parent"]),
        ];
        let tree = layout(&records);
        let mut reversed = records;
        reversed.reverse();
        assert_eq!(layout(&reversed), tree);
        assert_eq!(
            tree.levels,
            vec![vec!["sibling", "early-child"], vec!["late-parent"]]
        );
    }
    #[test]
    fn cycles_and_duplicates_are_safe_without_losing_unrelated_edges_or_nodes() {
        let tree = layout(&[
            v("a", 1, &["b"]),
            v("b", 2, &["a"]),
            v("c", 3, &["a", "a", "c"]),
            v("c", 3, &["a", "a", "c"]),
            v("d", 4, &[]),
        ]);
        assert_eq!(
            ids(&tree),
            BTreeSet::from(["a".into(), "b".into(), "c".into(), "d".into()])
        );
        assert_eq!(tree.edges, vec![("a".into(), "c".into())]);
        assert_eq!(tree.levels, vec![vec!["d", "c", "b"], vec!["a"]]);
    }
    #[test]
    fn old_ancestors_group_by_their_newer_children_before_timestamps_and_ids() {
        let records = vec![
            v("root", 1, &[]),
            v("left", 2, &["root"]),
            v("right", 3, &["root"]),
            v("z-left-one", 30, &["left"]),
            v("z-left-two", 50, &["left"]),
            v("a-right-one", 4, &["right"]),
            v("a-right-two", 20, &["right"]),
            v("merge", 60, &["z-left-two", "a-right-one"]),
        ];
        let tree = layout(&records);
        assert_eq!(tree.levels[2], vec!["left", "right"]);
        assert_eq!(tree.edges.len(), 8);
        assert_eq!(ids(&tree).len(), records.len());
        let mut reversed = records;
        reversed.reverse();
        assert_eq!(layout(&reversed), tree);
    }
    #[test]
    fn mixed_legacy_dates_keep_a_total_stable_order_and_unknown_dates_are_last() {
        let versions = [
            v("two", 2, &[]),
            v("ten", 10, &[]),
            Version {
                id: "legacy".into(),
                created: "2020-01-01T00:00:00Z".into(),
                parents: vec![],
            },
            Version {
                id: "unknown".into(),
                created: String::new(),
                parents: vec![],
            },
        ];
        for a in &versions {
            for b in &versions {
                for c in &versions {
                    if newer_first(a, b).is_le() && newer_first(b, c).is_le() {
                        assert!(newer_first(a, c).is_le());
                    }
                }
            }
        }
        let tree = layout(&versions);
        assert_eq!(tree.levels, vec![vec!["ten", "two", "legacy", "unknown"]]);
    }
    #[test]
    fn every_branch_tip_is_above_its_ancestors_even_when_branches_have_unequal_depths() {
        let tree = layout(&[
            v("root", 1, &[]),
            v("long-mid", 2, &["root"]),
            v("long-tip", 3, &["long-mid"]),
            v("short-tip", 100, &["root"]),
        ]);
        assert_eq!(
            tree.levels,
            vec![
                vec!["short-tip", "long-tip"],
                vec!["long-mid"],
                vec!["root"]
            ]
        );
        for (parent, child) in &tree.edges {
            let depth = |id: &String| {
                tree.levels
                    .iter()
                    .position(|level| level.contains(id))
                    .unwrap()
            };
            assert!(
                depth(parent) > depth(child),
                "every real ancestry edge points upwards"
            );
        }
        assert!(tree.edges.contains(&("root".into(), "short-tip".into())));
    }
    #[test]
    fn upward_cross_layer_routes_avoid_nodes_and_enter_below_the_tallest_target_sibling() {
        let child = NodeBounds {
            left: 100.,
            right: 394.,
            top: 0.,
            bottom: 110.,
            depth: 0,
        };
        let tall_sibling = NodeBounds {
            left: 430.,
            right: 724.,
            top: 0.,
            bottom: 182.,
            depth: 0,
        };
        let middle = NodeBounds {
            left: 100.,
            right: 394.,
            top: 200.,
            bottom: 310.,
            depth: 1,
        };
        let parent = NodeBounds {
            left: 100.,
            right: 394.,
            top: 328.,
            bottom: 438.,
            depth: 2,
        };
        let boxes = [child, tall_sibling, middle, parent];
        let route = long_route(parent, child, &boxes);
        assert_eq!(middle.top - tall_sibling.bottom, 18.);
        assert_eq!(parent.top - middle.bottom, 18.);
        assert_eq!(route.first(), Some(&(parent.center(), parent.top)));
        assert_eq!(route[1].1, parent.top - 6.);
        assert_eq!(route[3].1, tall_sibling.bottom + 6.);
        assert!(route.windows(2).all(|segment| segment[1].1 <= segment[0].1));
        assert_eq!(route.last(), Some(&(child.center(), child.bottom + 3.)));
        assert!(route[2].0 < child.left);
        for segment in route.windows(2) {
            let [(x1, y1), (x2, y2)] = [segment[0], segment[1]];
            assert!(x1 == x2 || y1 == y2);
            for node in boxes {
                let enters = if x1 == x2 {
                    x1 > node.left
                        && x1 < node.right
                        && y1.min(y2) < node.bottom
                        && y1.max(y2) > node.top
                } else {
                    y1 > node.top
                        && y1 < node.bottom
                        && x1.min(x2) < node.right
                        && x1.max(x2) > node.left
                };
                assert!(
                    !enters,
                    "an upward route must never enter a node's interior: {segment:?} / {node:?}"
                );
            }
        }
        let across = edge_path(parent, child, &boxes).unwrap();
        assert!(across.contains('H') && across.contains('V') && across.contains('A'));
        assert!(!across.contains(['C', 'L', 'Q']));
        let adjacent = edge_path(middle, child, &boxes).unwrap();
        assert_eq!(adjacent, "M247.000,200.000 V113.000");
        assert!(edge_path(child, parent, &boxes).is_none());
    }
    #[test]
    fn an_eighteen_pixel_layer_gap_places_rounded_right_angles_between_nodes() {
        let child = NodeBounds {
            left: 18.,
            right: 312.,
            top: 0.,
            bottom: 110.,
            depth: 0,
        };
        let sibling = NodeBounds {
            left: 330.,
            right: 624.,
            top: 0.,
            bottom: 182.,
            depth: 0,
        };
        let parent = NodeBounds {
            left: 330.,
            right: 624.,
            top: 200.,
            bottom: 310.,
            depth: 1,
        };
        let boxes = [child, sibling, parent];
        assert_eq!(parent.top - sibling.bottom, 18.);
        let (exit, entry) = gap_heights(parent, child, &boxes);
        assert_eq!((exit, entry), (194., 188.));
        assert!(parent.top > exit && exit > entry && entry > child.bottom + 3.);
        let path = edge_path(parent, child, &boxes).unwrap();
        assert_eq!(
            path,
            "M477.000,200.000 V195.000 A4.000,4.000 0 0 0 473.000,191.000 H169.000 A4.000,4.000 0 0 1 165.000,187.000 V113.000"
        );
        assert!(191. - EDGE_CORNER_RADIUS > sibling.bottom);
        assert!(191. + EDGE_CORNER_RADIUS < parent.top);
    }
    #[test]
    fn orthogonal_turns_have_circular_arcs_and_short_runs_limit_both_corner_radii() {
        assert_eq!(
            rounded_route(&[(10., 40.), (10., 20.), (50., 20.), (50., 0.)], 4.).unwrap(),
            "M10.000,40.000 V24.000 A4.000,4.000 0 0 1 14.000,20.000 H46.000 A4.000,4.000 0 0 0 50.000,16.000 V0.000"
        );
        assert_eq!(
            rounded_route(&[(0., 10.), (0., 5.), (2., 5.), (2., 0.)], 4.).unwrap(),
            "M0.000,10.000 V6.000 A1.000,1.000 0 0 1 1.000,5.000 A1.000,1.000 0 0 0 2.000,4.000 V0.000"
        );
        assert_eq!(
            rounded_route(&[(3., 10.), (3., 10.), (3., 5.), (3., 0.)], 4.).unwrap(),
            "M3.000,10.000 V0.000"
        );
        for points in [
            &[(0., 0.)][..],
            &[(0., 0.), (1., 1.)][..],
            &[(0., 0.), (0., 1.), (0., 0.)][..],
            &[(0., 0.), (f64::NAN, 1.)][..],
        ] {
            assert!(rounded_route(points, 4.).is_none());
        }
        assert!(rounded_route(&[(0., 0.), (0., 1.)], f64::INFINITY).is_none());
    }
    #[test]
    fn metadata_ignores_notes_current_markers_and_foreign_file_records() {
        let mut records = vec![
            serde_json::json!({"file_id":"selected","version_id":"a","created_at":"1","source_version_ids":[],"note":"initial"}),
            serde_json::json!({"file_id":"other","version_id":"foreign","created_at":"0"}),
            serde_json::json!({"file_id":"selected","version_id":"b","created_at":"2","source_version_ids":["foreign","a","a",null]}),
        ];
        let before = metadata(&records, "selected");
        records[0]["note"] = "typing stays mounted".into();
        records[0]["current"] = true.into();
        assert_eq!(metadata(&records, "selected"), before);
        let tree = layout(&before);
        assert_eq!(tree.edges, vec![("a".into(), "b".into())]);
        assert_eq!(ids(&tree), BTreeSet::from(["a".into(), "b".into()]));
    }
    #[test]
    fn placeholder_records_never_create_tree_nodes_or_source_aliases() {
        let records = vec![
            serde_json::json!({"file_id":"selected","version_id":"source","created_at":"1"}),
            serde_json::json!({"file_id":"selected","branch_id":"pending","materialized":false,"content_state":"PENDING","version_id":null,"source_version_id":"source"}),
            serde_json::json!({"file_id":"selected","version_id":"not-real","materialized":false,"source_version_ids":["source"]}),
            serde_json::json!({"file_id":"selected","version_id":"first-write","materialized":true,"created_at":"2","size":"0","source_version_ids":["source"]}),
        ];
        let tree = layout(&metadata(&records, "selected"));
        assert_eq!(
            ids(&tree),
            BTreeSet::from(["source".into(), "first-write".into()])
        );
        assert_eq!(tree.edges, vec![("source".into(), "first-write".into())]);
    }
    #[test]
    fn inherited_current_markers_do_not_change_real_version_identity_or_ancestry() {
        let versions = vec![
            serde_json::json!({"file_id":"file","version_id":"parent","created_at":"1","current":true}),
        ];
        let mut model = Model {
            file_history: serde_json::json!({"version_id":"parent","materialized":false,"inherited":true,"own_version_id":null,"source_version_id":"parent","versions":versions}),
            ..Default::default()
        };
        let before = metadata(&list(&model.file_history, "versions"), "file");
        assert!(inherited_current(&model, "parent"));
        assert!(!inherited_current(&model, "other"));
        model.file_history["materialized"] = true.into();
        model.file_history["inherited"] = false.into();
        model.file_history["own_version_id"] = "parent".into();
        assert!(!inherited_current(&model, "parent"));
        assert_eq!(
            metadata(&list(&model.file_history, "versions"), "file"),
            before
        );
        assert_eq!(layout(&before).levels, vec![vec!["parent"]]);
    }
}
