//! One layout for both directory and file graphs. Coordinates are topological,
//! never chronological: reused merge heads may acquire newer parents.
use super::model::{Id, Workspace};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
};
pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
#[derive(Clone)]
struct Card {
    id: Id,
    parents: Vec<Id>,
    rows: [String; 3],
    selected: bool,
    new: bool,
}
pub struct Drawing {
    pub html: String,
    #[cfg(test)]
    pub positions: BTreeMap<Id, (usize, usize)>,
    #[cfg(test)]
    pub edges: Vec<(Id, Id)>,
}
pub fn file(model: &Workspace, file: Id, selected: Id, previous: Option<&Workspace>) -> Drawing {
    let cards = model
        .files
        .values()
        .filter(|n| n.file == file)
        .map(|n| {
            let names = model
                .file_branches
                .iter()
                .filter(|((f, name), _)| {
                    *f == file && model.branch_node(file, name).ok() == Some(n.id)
                })
                .map(|((_, name), b)| {
                    format!(
                        "{name}{}",
                        if b.alias.is_some() {
                            "（只读别名）"
                        } else if b.pending {
                            "（待首写）"
                        } else {
                            ""
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join(" / ");
            let tags = n.tags.iter().cloned().collect::<Vec<_>>().join(" · ");
            Card {
                id: n.id,
                parents: n.parents.clone(),
                rows: [
                    format!("F{} · {} · 正文 #{}", n.id, n.kind.label(), n.body + 1),
                    format!(
                        "{}{}",
                        model.content(n.id),
                        if tags.is_empty() {
                            String::new()
                        } else {
                            format!("  [{tags}]")
                        }
                    ),
                    if names.is_empty() {
                        n.path.clone()
                    } else {
                        names
                    },
                ],
                selected: n.id == selected,
                new: previous.is_some_and(|p| !p.files.contains_key(&n.id)),
            }
        })
        .collect();
    draw(cards, "F", "文件节点，点击选择操作来源")
}
pub fn directory(
    model: &Workspace,
    directory: Id,
    selected: Id,
    previous: Option<&Workspace>,
) -> Drawing {
    let cards = model
        .directories
        .values()
        .filter(|n| n.directory == directory)
        .map(|n| {
            let entries = n
                .entries
                .iter()
                .map(|(name, e)| {
                    format!(
                        "{}{} → {}{}",
                        if e.folder() { "▱ " } else { "" },
                        name,
                        if e.folder() { "D" } else { "F" },
                        e.node()
                    )
                })
                .collect::<Vec<_>>()
                .join("；");
            let names = model
                .directory_branches
                .iter()
                .filter(|((d, _), b)| *d == directory && b.head == n.id)
                .map(|((_, name), _)| name.clone())
                .collect::<Vec<_>>()
                .join(" / ");
            Card {
                id: n.id,
                parents: n.parents.clone(),
                rows: [
                    format!("D{} · {}", n.id, n.note),
                    if entries.is_empty() {
                        "空目录".into()
                    } else {
                        entries
                    },
                    if names.is_empty() {
                        "旧组成保持不变".into()
                    } else {
                        format!("当前分支：{names}")
                    },
                ],
                selected: n.id == selected,
                new: previous.is_some_and(|p| !p.directories.contains_key(&n.id)),
            }
        })
        .collect();
    draw(cards, "D", "目录子树，保存文件和子目录的固定引用")
}
fn draw(cards: Vec<Card>, prefix: &str, label: &str) -> Drawing {
    let parents: BTreeMap<_, _> = cards.iter().map(|n| (n.id, n.parents.clone())).collect();
    fn level(
        id: Id,
        parents: &BTreeMap<Id, Vec<Id>>,
        cache: &mut BTreeMap<Id, usize>,
        visiting: &mut BTreeSet<Id>,
    ) -> usize {
        if let Some(level) = cache.get(&id) {
            return *level;
        }
        if !visiting.insert(id) {
            return 0;
        }
        let value = parents
            .get(&id)
            .into_iter()
            .flatten()
            .filter(|p| parents.contains_key(p))
            .map(|p| level(*p, parents, cache, visiting) + 1)
            .max()
            .unwrap_or(0);
        visiting.remove(&id);
        cache.insert(id, value);
        value
    }
    let mut levels = BTreeMap::new();
    for card in &cards {
        level(card.id, &parents, &mut levels, &mut BTreeSet::new());
    }
    let max = levels.values().copied().max().unwrap_or(0);
    let mut columns = BTreeMap::<usize, usize>::new();
    let mut positions = BTreeMap::new();
    let edges: Vec<_> = cards
        .iter()
        .flat_map(|c| {
            c.parents
                .iter()
                .filter(|p| parents.contains_key(p))
                .map(|p| (*p, c.id))
        })
        .collect();
    for c in &cards {
        let depth = levels[&c.id];
        let column = columns.entry(depth).or_default();
        positions.insert(c.id, (18 + *column * 342, 18 + (max - depth) * 162));
        *column += 1;
    }
    let max_columns = columns.values().copied().max().unwrap_or(1);
    for card in &cards {
        positions.get_mut(&card.id).unwrap().0 += (max_columns - columns[&levels[&card.id]]) * 171;
    }
    let width = max_columns * 342 + edges.len() * 12 + 24;
    let height = (max + 1) * 162;
    let mut html = format!(
        r#"<svg class="zxs-demo-graph" viewBox="0 0 {width} {height}" width="{width}" height="{height}" role="group" aria-label="{label}"><defs><marker id="zxs-demo-arrow-{prefix}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto"><path d="M1 1 L9 5 L1 9" fill="none" stroke="currentColor" stroke-width="1.5"/></marker></defs>"#
    );
    for (index, (parent, child)) in edges.iter().enumerate() {
        let (px, py) = positions[parent];
        let (cx, cy) = positions[child];
        let sx = px + 153;
        let sy = py;
        let tx = cx + 153;
        let ty = cy + 108;
        let route = if sx == tx && sy - ty <= 80 {
            format!("M{sx} {sy} V{ty}")
        } else {
            let lane = width - 18 - index * 12;
            let start = sy - 18;
            let end = ty + 18;
            let r = 7;
            format!(
                "M{sx} {sy} V{} A{r} {r} 0 0 1 {} {start} H{} A{r} {r} 0 0 0 {lane} {} V{} A{r} {r} 0 0 0 {} {end} H{} A{r} {r} 0 0 1 {tx} {} V{ty}",
                start + r,
                sx + r,
                lane - r,
                start - r,
                end + r,
                lane - r,
                tx + r,
                end - r
            )
        };
        write!(html,r#"<path class="zxs-demo-edge" d="{route}" data-parent="{parent}" data-child="{child}" marker-end="url(#zxs-demo-arrow-{prefix})"/>"#).unwrap();
    }
    for card in cards {
        let (x, y) = positions[&card.id];
        write!(html,r#"<foreignObject x="{x}" y="{y}" width="306" height="108"><button xmlns="http://www.w3.org/1999/xhtml" type="button" class="zxs-frame zxs-demo-node" data-demo-node="{}" data-demo-kind="{prefix}" data-new="{}" aria-pressed="{}" aria-label="{}">"#,card.id,card.new,card.selected,escape(&card.rows.join("；"))).unwrap();
        for row in card.rows {
            write!(
                html,
                r#"<span class="zxs-row" title="{}">{}</span>"#,
                escape(&row),
                escape(&row)
            )
            .unwrap();
        }
        html.push_str("</button></foreignObject>");
    }
    html.push_str("</svg>");
    Drawing {
        html,
        #[cfg(test)]
        positions,
        #[cfg(test)]
        edges,
    }
}
