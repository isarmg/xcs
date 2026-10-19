//! Deterministic, memory-only teaching model. No API client, filesystem or timer.
//! Immutable nodes and explicit directory references mirror the documented rules;
//! storage proofs, permissions and replication are deliberately outside this model.
use std::collections::{BTreeMap, BTreeSet};

pub type Id = u64;
pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Content,
    Path,
    Tags,
    Merge,
}
impl Kind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Content => "正文",
            Self::Path => "路径",
            Self::Tags => "标签",
            Self::Merge => "合并",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileNode {
    pub id: Id,
    pub file: Id,
    pub parents: Vec<Id>,
    pub body: usize,
    pub path: String,
    pub note: String,
    pub tags: BTreeSet<String>,
    pub kind: Kind,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileBranch {
    pub head: Id,
    pub pending: bool,
    pub alias: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    File {
        file: Id,
        branch: String,
        node: Id,
        lazy: Option<String>,
    },
    Folder {
        directory: Id,
        branch: String,
        node: Id,
        lazy: Option<String>,
    },
}
impl Entry {
    pub fn node(&self) -> Id {
        match self {
            Self::File { node, .. } | Self::Folder { node, .. } => *node,
        }
    }
    pub fn folder(&self) -> bool {
        matches!(self, Self::Folder { .. })
    }
    fn inherit(&mut self, intent: &str) {
        match self {
            Self::File { lazy, .. } | Self::Folder { lazy, .. } => *lazy = Some(intent.into()),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryNode {
    pub id: Id,
    pub directory: Id,
    pub parents: Vec<Id>,
    pub entries: BTreeMap<String, Entry>,
    pub note: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryBranch {
    pub head: Id,
    pub seed: Option<Id>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeRecord {
    pub file: Id,
    pub inputs: Vec<String>,
    pub heads: Vec<Id>,
    pub chosen: Id,
    pub result: Id,
    pub owner: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Protection {
    pub path: String,
    pub copies: usize,
    pub prefixes: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    pub bodies: Vec<String>,
    pub files: BTreeMap<Id, FileNode>,
    pub file_branches: BTreeMap<(Id, String), FileBranch>,
    pub directories: BTreeMap<Id, DirectoryNode>,
    pub directory_branches: BTreeMap<(Id, String), DirectoryBranch>,
    pub active: String,
    pub default_branch: String,
    pub merges: Vec<MergeRecord>,
    pub protection: Vec<Protection>,
    next: Id,
}
#[derive(Clone, Debug)]
struct Ancestor {
    directory: Id,
    branch: String,
    node: Id,
    lazy: Option<String>,
    name: String,
}
#[derive(Clone, Debug)]
pub struct Context {
    pub directory: Id,
    pub branch: String,
    pub node: Id,
    lazy: Option<String>,
    ancestors: Vec<Ancestor>,
}
#[derive(Clone, Debug)]
pub enum Action {
    CreateFileBranch {
        path: String,
        name: String,
    },
    SwitchFileBranch {
        path: String,
        name: String,
    },
    CreateDirectoryBranch {
        path: String,
        name: String,
    },
    SwitchDirectoryBranch {
        path: String,
        name: String,
    },
    Write {
        path: String,
        content: String,
    },
    Rename {
        path: String,
        name: String,
    },
    Move {
        path: String,
        target: String,
    },
    Copy {
        path: String,
        target: String,
    },
    Delete {
        path: String,
    },
    Tags {
        path: String,
        source: Id,
        tags: BTreeSet<String>,
    },
    Note {
        path: String,
        source: Id,
        text: String,
    },
    Merge {
        path: String,
        branches: Vec<String>,
        chosen: Id,
        owner: String,
    },
    Default {
        name: String,
    },
    Protect {
        path: String,
        copies: usize,
        prefixes: Vec<String>,
    },
}
fn name(value: &str) -> Result<()> {
    if value.len() > 128 {
        return Err("名称最多 128 字节".into());
    }
    xcs_naming::validate_name(value).map_err(|e| e.message)
}
fn path_parts(path: &str) -> Result<(&str, &str)> {
    xcs_naming::validate_path(path).map_err(|e| e.message)?;
    Ok(path.rsplit_once('/').unwrap_or(("", path)))
}
impl Default for Workspace {
    fn default() -> Self {
        Self::seed()
    }
}
impl Workspace {
    pub fn seed() -> Self {
        let mut s = Self {
            bodies: vec![],
            files: BTreeMap::new(),
            file_branches: BTreeMap::new(),
            directories: BTreeMap::new(),
            directory_branches: BTreeMap::new(),
            active: "main".into(),
            default_branch: "main".into(),
            merges: vec![],
            protection: vec![],
            next: 1,
        };
        let report = s.new_file("A/报告.txt", "预算 100", BTreeSet::from(["草稿".into()]));
        let readme = s.new_file("A/说明.txt", "项目说明", BTreeSet::new());
        let a = s.new_directory(
            BTreeMap::from([("报告.txt".into(), report), ("说明.txt".into(), readme)]),
            "初始目录 A",
        );
        let b = s.new_directory(BTreeMap::new(), "初始目录 B");
        let entries = BTreeMap::from([("A".into(), a), ("B".into(), b)]);
        let root_node = s.id();
        s.directories.insert(
            root_node,
            DirectoryNode {
                id: root_node,
                directory: 0,
                parents: vec![],
                entries,
                note: "初始目录组成".into(),
            },
        );
        s.directory_branches.insert(
            (0, "main".into()),
            DirectoryBranch {
                head: root_node,
                seed: None,
            },
        );
        s
    }
    fn id(&mut self) -> Id {
        let id = self.next;
        self.next += 1;
        id
    }
    fn new_file(&mut self, path: &str, content: &str, tags: BTreeSet<String>) -> Entry {
        let file = self.id();
        let node = self.id();
        let body = self.bodies.len();
        self.bodies.push(content.into());
        self.files.insert(
            node,
            FileNode {
                id: node,
                file,
                parents: vec![],
                body,
                path: path.into(),
                note: "初始内容".into(),
                tags,
                kind: Kind::Content,
            },
        );
        self.file_branches.insert(
            (file, "main".into()),
            FileBranch {
                head: node,
                pending: false,
                alias: None,
            },
        );
        Entry::File {
            file,
            branch: "main".into(),
            node,
            lazy: None,
        }
    }
    fn new_directory(&mut self, entries: BTreeMap<String, Entry>, note: &str) -> Entry {
        let directory = self.id();
        let node = self.id();
        self.directories.insert(
            node,
            DirectoryNode {
                id: node,
                directory,
                parents: vec![],
                entries,
                note: note.into(),
            },
        );
        self.directory_branches.insert(
            (directory, "main".into()),
            DirectoryBranch {
                head: node,
                seed: None,
            },
        );
        Entry::Folder {
            directory,
            branch: "main".into(),
            node,
            lazy: None,
        }
    }
    /// Failed operations leave the entire model unchanged, including counters.
    pub fn apply(&mut self, action: Action) -> Result<String> {
        if self.files.len()
            + self.directories.len()
            + self.file_branches.len()
            + self.directory_branches.len()
            > 300
        {
            return Err("演示已达到 300 个节点，请撤销或重置后继续".into());
        }
        let mut candidate = self.clone();
        let message = candidate.execute(action)?;
        candidate.validate()?;
        *self = candidate;
        Ok(message)
    }
    pub fn root_branches(&self) -> Vec<String> {
        self.directory_branches
            .keys()
            .filter(|(id, _)| *id == 0)
            .map(|(_, n)| n.clone())
            .collect()
    }
    pub fn context(&self, path: &str) -> Result<Context> {
        let root = self
            .directory_branches
            .get(&(0, self.active.clone()))
            .ok_or("目录分支不存在")?;
        let mut ctx = Context {
            directory: 0,
            branch: self.active.clone(),
            node: root.head,
            lazy: None,
            ancestors: vec![],
        };
        if path.is_empty() {
            return Ok(ctx);
        }
        xcs_naming::validate_path(path).map_err(|e| e.message)?;
        for component in path.split('/') {
            let Entry::Folder {
                directory,
                branch,
                node,
                lazy,
            } = self
                .entries(&ctx)
                .get(component)
                .cloned()
                .ok_or("目录不存在")?
            else {
                return Err("目标不是目录".into());
            };
            let ancestor = Ancestor {
                directory: ctx.directory,
                branch: ctx.branch,
                node: ctx.node,
                lazy: ctx.lazy,
                name: component.into(),
            };
            ctx = Context {
                directory,
                branch,
                node,
                lazy,
                ancestors: {
                    let mut a = ctx.ancestors;
                    a.push(ancestor);
                    a
                },
            };
        }
        Ok(ctx)
    }
    pub fn entries(&self, ctx: &Context) -> BTreeMap<String, Entry> {
        let mut entries = self.directories[&ctx.node].entries.clone();
        let seed = self
            .directory_branches
            .get(&(ctx.directory, ctx.branch.clone()))
            .and_then(|b| b.seed);
        if let Some(intent) = ctx
            .lazy
            .as_deref()
            .or_else(|| (seed == Some(ctx.node)).then_some(ctx.branch.as_str()))
        {
            for entry in entries.values_mut() {
                entry.inherit(intent);
            }
        }
        entries
    }
    pub fn entry(&self, path: &str) -> Result<Entry> {
        let (parent, base) = path_parts(path)?;
        self.entries(&self.context(parent)?)
            .get(base)
            .cloned()
            .ok_or("文件或目录不存在".into())
    }
    pub fn paths(&self) -> Vec<(String, Entry)> {
        fn walk(s: &Workspace, parent: &str, out: &mut Vec<(String, Entry)>) {
            let Ok(ctx) = s.context(parent) else {
                return;
            };
            for (name, entry) in s.entries(&ctx) {
                let path = if parent.is_empty() {
                    name
                } else {
                    format!("{parent}/{name}")
                };
                out.push((path.clone(), entry.clone()));
                if entry.folder() {
                    walk(s, &path, out);
                }
            }
        }
        let mut out = vec![];
        walk(self, "", &mut out);
        out
    }
    pub fn file_at(&self, path: &str) -> Result<(Id, String, Id)> {
        match self.entry(path)? {
            Entry::File {
                file, branch, node, ..
            } => Ok((file, branch, node)),
            _ => Err("请选择文件".into()),
        }
    }
    pub fn content(&self, node: Id) -> &str {
        &self.bodies[self.files[&node].body]
    }
    pub fn branch_node(&self, file: Id, branch: &str) -> Result<Id> {
        let mut branch = branch;
        let mut seen = BTreeSet::new();
        loop {
            if !seen.insert(branch) {
                return Err("分支别名成环".into());
            }
            let b = self
                .file_branches
                .get(&(file, branch.into()))
                .ok_or("文件分支不存在")?;
            if let Some(alias) = &b.alias {
                branch = alias;
            } else {
                return Ok(b.head);
            }
        }
    }
    fn append_directory(
        &mut self,
        ctx: &Context,
        entries: BTreeMap<String, Entry>,
        message: &str,
    ) -> Id {
        let node = self.id();
        self.directories.insert(
            node,
            DirectoryNode {
                id: node,
                directory: ctx.directory,
                parents: vec![ctx.node],
                entries,
                note: message.into(),
            },
        );
        if ctx.lazy.is_none() {
            if let Some(b) = self
                .directory_branches
                .get_mut(&(ctx.directory, ctx.branch.clone()))
                .filter(|b| b.head == ctx.node)
            {
                b.head = node;
            }
        }
        node
    }
    fn publish(&mut self, ctx: Context, entries: BTreeMap<String, Entry>, message: &str) {
        let mut new = self.append_directory(&ctx, entries, message);
        for parent in ctx.ancestors.into_iter().rev() {
            let parent_ctx = Context {
                directory: parent.directory,
                branch: parent.branch,
                node: parent.node,
                lazy: parent.lazy,
                ancestors: vec![],
            };
            let mut entries = self.entries(&parent_ctx);
            if let Some(Entry::Folder { node, lazy, .. }) = entries.get_mut(&parent.name) {
                *node = new;
                *lazy = None;
            }
            new = self.append_directory(&parent_ctx, entries, message);
        }
    }
    fn replace(&mut self, path: &str, entry: Option<Entry>, message: &str) -> Result<()> {
        let (parent, base) = path_parts(path)?;
        let ctx = self.context(parent)?;
        let mut entries = self.entries(&ctx);
        if let Some(entry) = entry {
            entries.insert(base.into(), entry);
        } else {
            entries.remove(base);
        }
        self.publish(ctx, entries, message);
        Ok(())
    }
    fn writable(&self, file: Id, branch: &str) -> Result<()> {
        if self
            .file_branches
            .get(&(file, branch.into()))
            .ok_or("文件分支不存在")?
            .alias
            .is_some()
        {
            return Err("合并后的别名只读，请选择归属分支".into());
        }
        Ok(())
    }
    fn child(
        &mut self,
        source: Id,
        parents: Vec<Id>,
        kind: Kind,
        path: String,
        note: String,
        body: Option<String>,
        tags: Option<BTreeSet<String>>,
    ) -> Id {
        let source = self.files[&source].clone();
        let id = self.id();
        let body = if let Some(content) = body {
            let body = self.bodies.len();
            self.bodies.push(content);
            body
        } else {
            source.body
        };
        self.files.insert(
            id,
            FileNode {
                id,
                file: source.file,
                parents,
                body,
                path,
                note,
                tags: tags.unwrap_or(source.tags),
                kind,
            },
        );
        id
    }
    fn effective_branch(
        &mut self,
        file: Id,
        source: Id,
        branch: String,
        lazy: Option<String>,
    ) -> String {
        let Some(intent) = lazy else {
            return branch;
        };
        let mut name = intent.clone();
        let mut suffix = 2;
        while self.file_branches.contains_key(&(file, name.clone())) {
            name = format!("{intent}-{suffix}");
            suffix += 1;
        }
        self.file_branches.insert(
            (file, name.clone()),
            FileBranch {
                head: source,
                pending: true,
                alias: None,
            },
        );
        name
    }
    fn execute(&mut self, action: Action) -> Result<String> {
        match action {
            Action::CreateFileBranch { path, name: new } => {
                name(&new)?;
                let (file, _, node) = self.file_at(&path)?;
                if self.file_branches.keys().any(|(f, n)| {
                    *f == file && xcs_naming::lookup_key(n) == xcs_naming::lookup_key(&new)
                }) {
                    return Err("同名文件分支已存在".into());
                }
                self.file_branches.insert(
                    (file, new.clone()),
                    FileBranch {
                        head: node,
                        pending: true,
                        alias: None,
                    },
                );
                Ok(format!(
                    "登记 {new} → F{node}；未创建正文或占位文件，尚未切换"
                ))
            }
            Action::SwitchFileBranch { path, name } => {
                let (file, _, _) = self.file_at(&path)?;
                let target = self.branch_node(file, &name)?;
                let entry = Entry::File {
                    file,
                    branch: name.clone(),
                    node: target,
                    lazy: None,
                };
                if self.entry(&path)? == entry {
                    return Ok("已经显式选择此文件分支，未增加节点".into());
                }
                self.replace(&path, Some(entry), "切换文件分支")?;
                Ok(format!("{path} 选择 {name}；新增目录子树，未复制正文"))
            }
            Action::CreateDirectoryBranch { path, name: new } => {
                name(&new)?;
                let ctx = self.context(&path)?;
                if self.directory_branches.keys().any(|(d, n)| {
                    *d == ctx.directory && xcs_naming::lookup_key(n) == xcs_naming::lookup_key(&new)
                }) {
                    return Err("同名目录分支已存在".into());
                }
                self.directory_branches.insert(
                    (ctx.directory, new.clone()),
                    DirectoryBranch {
                        head: ctx.node,
                        seed: Some(ctx.node),
                    },
                );
                Ok(format!(
                    "登记目录分支 {new} → D{}；所有文件仍引用父节点",
                    ctx.node
                ))
            }
            Action::SwitchDirectoryBranch { path, name } => {
                let ctx = self.context(&path)?;
                let head = self
                    .directory_branches
                    .get(&(ctx.directory, name.clone()))
                    .ok_or("目录分支不存在")?
                    .head;
                if path.is_empty() {
                    self.active = name.clone();
                } else {
                    self.replace(
                        &path,
                        Some(Entry::Folder {
                            directory: ctx.directory,
                            branch: name.clone(),
                            node: head,
                            lazy: None,
                        }),
                        "切换子目录分支",
                    )?;
                }
                Ok(format!("查看目录分支 {name}；读取其冻结组成"))
            }
            Action::Write { path, content } => {
                if content.len() > 4096 {
                    return Err("演示正文最多 4096 字节".into());
                }
                let Entry::File {
                    file,
                    branch,
                    node: source,
                    lazy,
                } = self.entry(&path)?
                else {
                    return Err("只能写入文件".into());
                };
                let branch = self.effective_branch(file, source, branch, lazy);
                self.writable(file, &branch)?;
                let node = self.child(
                    source,
                    vec![source],
                    Kind::Content,
                    path.clone(),
                    "写入完整正文".into(),
                    Some(content),
                    None,
                );
                let b = self.file_branches.get_mut(&(file, branch.clone())).unwrap();
                b.head = node;
                b.pending = false;
                self.replace(
                    &path,
                    Some(Entry::File {
                        file,
                        branch: branch.clone(),
                        node,
                        lazy: None,
                    }),
                    "文件写入",
                )?;
                Ok(format!(
                    "写入 {branch} → F{node}；新增 1 份 FULL，目录追加子树"
                ))
            }
            Action::Rename { path, name: new } => {
                name(&new)?;
                let (parent, old) = path_parts(&path)?;
                if old == new {
                    return Ok("名称未变，未增加节点".into());
                }
                let ctx = self.context(parent)?;
                let mut entries = self.entries(&ctx);
                if entries
                    .keys()
                    .any(|n| xcs_naming::lookup_key(n) == xcs_naming::lookup_key(&new))
                {
                    return Err("目标名称已存在".into());
                }
                let entry = entries.remove(old).ok_or("文件或目录不存在")?;
                let target = if parent.is_empty() {
                    new.clone()
                } else {
                    format!("{parent}/{new}")
                };
                let entry = self.repath(entry, &target)?;
                entries.insert(new, entry);
                self.publish(ctx, entries, "重命名：追加文件路径节点");
                Ok(format!("{path} → {target}；每个文件增加路径节点，复用正文"))
            }
            Action::Move { path, target } => self.transfer(&path, &target, false),
            Action::Copy { path, target } => self.transfer(&path, &target, true),
            Action::Delete { path } => {
                self.entry(&path)?;
                self.replace(&path, None, "删除目录引用")?;
                Ok(format!("当前分支移除 {path}；旧子树及其他分支仍保留"))
            }
            Action::Tags { path, source, tags } => {
                if tags.len() > xcs_contract::MAX_FILE_TAGS {
                    return Err("最多 16 个标签".into());
                }
                for tag in &tags {
                    xcs_contract::validate_tag(tag).map_err(|e| e.message)?;
                }
                let (file, branch, current) = self.file_at(&path)?;
                self.writable(file, &branch)?;
                let old = self.files.get(&source).ok_or("所选节点不存在")?;
                if old.file != file {
                    return Err("所选节点不属于这个文件".into());
                }
                if old.tags == tags {
                    return Ok("标签未变化，未增加节点".into());
                }
                let node = self.child(
                    source,
                    vec![source],
                    Kind::Tags,
                    old.path.clone(),
                    "更改标签，复用正文".into(),
                    None,
                    Some(tags),
                );
                if self.file_branches[&(file, branch.clone())].head == source {
                    self.file_branches
                        .get_mut(&(file, branch.clone()))
                        .unwrap()
                        .head = node;
                }
                self.rewrite_all(file, &BTreeSet::from([branch]), Some(source), node)?;
                Ok(if source == current {
                    format!("新增标签节点 F{node}，复用正文；后续子节点继承")
                } else {
                    format!("从历史 F{source} 增加旁支 F{node}；较新正文未回退")
                })
            }
            Action::Note { path, source, text } => {
                let (file, branch, _) = self.file_at(&path)?;
                self.writable(file, &branch)?;
                xcs_contract::validate_note(&text).map_err(|e| e.message)?;
                let node = self
                    .files
                    .get_mut(&source)
                    .filter(|n| n.file == file)
                    .ok_or("所选节点不属于这个文件")?;
                node.note = text;
                Ok(format!("更新 F{source} 的备注；未新增正文或子树"))
            }
            Action::Merge {
                path,
                branches,
                chosen,
                owner,
            } => {
                let (file, _, _) = self.file_at(&path)?;
                let inputs: BTreeSet<_> = branches.into_iter().collect();
                if inputs.len() < 2 || !inputs.contains(&owner) {
                    return Err("至少选择两个分支，归属必须在参与分支中".into());
                }
                let mut heads = vec![];
                for input in &inputs {
                    self.writable(file, input)?;
                    heads.push(self.file_branches[&(file, input.clone())].head);
                }
                let selected = self.files.get(&chosen).ok_or("结果节点不存在")?.clone();
                if selected.file != file {
                    return Err("结果节点必须属于同一文件".into());
                }
                let reused = heads.contains(&chosen);
                let result = if reused {
                    chosen
                } else {
                    let mut parents = heads.clone();
                    parents.push(chosen);
                    parents.sort_unstable();
                    parents.dedup();
                    self.child(
                        chosen,
                        parents,
                        Kind::Merge,
                        selected.path,
                        "合并，复用所选正文与标签".into(),
                        None,
                        None,
                    )
                };
                if reused {
                    for head in &heads {
                        if *head != result
                            && !self.ancestor(result, *head)
                            && !self.files[&result].parents.contains(head)
                        {
                            self.files.get_mut(&result).unwrap().parents.push(*head);
                        }
                    }
                }
                for ((f, n), b) in &mut self.file_branches {
                    if *f != file {
                        continue;
                    }
                    if inputs.contains(n) {
                        if n == &owner {
                            b.head = result;
                            b.pending = false;
                        } else {
                            b.alias = Some(owner.clone());
                        }
                    } else if b.alias.as_ref().is_some_and(|n| inputs.contains(n)) {
                        b.alias = Some(owner.clone());
                    }
                }
                self.merges.push(MergeRecord {
                    file,
                    inputs: inputs.iter().cloned().collect(),
                    heads,
                    chosen,
                    result,
                    owner: owner.clone(),
                });
                self.rewrite_all(file, &inputs, None, result)?;
                Ok(format!(
                    "{} F{result} → {owner}；其他参与名称成为只读别名，相关目录同步追加子树",
                    if reused {
                        "复用"
                    } else {
                        "新增合并节点"
                    }
                ))
            }
            Action::Default { name } => {
                if !self.root_branches().contains(&name) {
                    return Err("根目录分支不存在".into());
                }
                self.default_branch = name.clone();
                Ok(format!("SMB 默认组合改为 {name}；演示中不连接 SMB"))
            }
            Action::Protect {
                path,
                copies,
                prefixes,
            } => {
                if !path.is_empty() {
                    self.entry(&path)?;
                }
                if !(1..=3).contains(&copies) || prefixes.len() != copies - 1 {
                    return Err("演示有 P01 / B01 / C01 三个盘位，请指定相应前缀".into());
                }
                for prefix in &prefixes {
                    xcs_naming::validate_path(prefix).map_err(|e| e.message)?;
                }
                self.protection.retain(|p| p.path != path);
                self.protection.push(Protection {
                    path: path.clone(),
                    copies,
                    prefixes,
                });
                Ok(format!(
                    "为 {path} 设置 {copies} 份；备份前缀不取自主盘真实路径"
                ))
            }
        }
    }
    pub fn ancestor(&self, ancestor: Id, child: Id) -> bool {
        let mut todo = vec![child];
        let mut seen = BTreeSet::new();
        while let Some(id) = todo.pop() {
            if id == ancestor {
                return true;
            }
            if seen.insert(id) {
                if let Some(node) = self.files.get(&id) {
                    todo.extend(&node.parents);
                }
            }
        }
        false
    }
    fn repath(&mut self, entry: Entry, path: &str) -> Result<Entry> {
        match entry {
            Entry::File {
                file,
                branch,
                node: source,
                lazy,
            } => {
                self.writable(file, &branch)?;
                let node = self.child(
                    source,
                    vec![source],
                    Kind::Path,
                    path.into(),
                    "路径变化，复用正文".into(),
                    None,
                    None,
                );
                if lazy.is_none() && self.file_branches[&(file, branch.clone())].head == source {
                    self.file_branches
                        .get_mut(&(file, branch.clone()))
                        .unwrap()
                        .head = node;
                }
                Ok(Entry::File {
                    file,
                    branch,
                    node,
                    lazy,
                })
            }
            Entry::Folder {
                directory,
                branch,
                node,
                lazy,
            } => {
                let ctx = Context {
                    directory,
                    branch: branch.clone(),
                    node,
                    lazy,
                    ancestors: vec![],
                };
                let mut children = BTreeMap::new();
                for (name, child) in self.entries(&ctx) {
                    children.insert(name.clone(), self.repath(child, &format!("{path}/{name}"))?);
                }
                let new = self.append_directory(&ctx, children, "文件夹改名：后代路径变化");
                Ok(Entry::Folder {
                    directory,
                    branch,
                    node: new,
                    lazy: None,
                })
            }
        }
    }
    fn duplicate(&mut self, entry: Entry, path: &str) -> Result<Entry> {
        match entry {
            Entry::File { node, .. } => {
                let old = self.files[&node].clone();
                let content = self.content(node).to_owned();
                Ok(self.new_file(path, &content, old.tags))
            }
            Entry::Folder {
                directory,
                branch,
                node,
                lazy,
            } => {
                let ctx = Context {
                    directory,
                    branch,
                    node,
                    lazy,
                    ancestors: vec![],
                };
                let mut children = BTreeMap::new();
                for (name, child) in self.entries(&ctx) {
                    children.insert(
                        name.clone(),
                        self.duplicate(child, &format!("{path}/{name}"))?,
                    );
                }
                Ok(self.new_directory(children, "复制目录，独立文件身份"))
            }
        }
    }
    fn transfer(&mut self, path: &str, target: &str, copy: bool) -> Result<String> {
        let (_, base) = path_parts(path)?;
        self.context(target)?;
        if target == path || target.starts_with(&format!("{path}/")) {
            return Err("不能移入或复制到自身及子目录".into());
        }
        let destination = if target.is_empty() {
            base.into()
        } else {
            format!("{target}/{base}")
        };
        let (parent, _) = path_parts(&destination)?;
        if self
            .entries(&self.context(parent)?)
            .keys()
            .any(|n| xcs_naming::lookup_key(n) == xcs_naming::lookup_key(base))
        {
            return Err("目标已存在同名条目".into());
        }
        let entry = self.entry(path)?;
        let entry = if copy {
            self.duplicate(entry, &destination)?
        } else {
            entry
        };
        if !copy {
            self.replace(path, None, "移动：源分支停止跟踪")?;
        }
        self.replace(
            &destination,
            Some(entry),
            if copy {
                "复制：新文件身份"
            } else {
                "移动：目标分支开始跟踪"
            },
        )?;
        Ok(format!(
            "{} {path} → {destination}；其他分支保持原样",
            if copy { "复制" } else { "移动" }
        ))
    }
    // Rewrite each named head independently from its frozen composition. Historical
    // subtrees are never edited, nor replaced with another branch's mutable HEAD.
    fn rewrite_all(
        &mut self,
        file: Id,
        branches: &BTreeSet<String>,
        source: Option<Id>,
        result: Id,
    ) -> Result<()> {
        let heads: Vec<_> = self
            .directory_branches
            .iter()
            .map(|(key, b)| (key.clone(), b.head))
            .collect();
        let owner = self
            .merges
            .last()
            .filter(|m| m.result == result)
            .map(|m| m.owner.clone());
        for ((directory, branch), head) in heads {
            let ctx = Context {
                directory,
                branch: branch.clone(),
                node: head,
                lazy: None,
                ancestors: vec![],
            };
            let new = self.rewrite_tree(ctx, file, branches, source, result, owner.as_deref());
            if new != head {
                self.directory_branches
                    .get_mut(&(directory, branch))
                    .unwrap()
                    .head = new;
            }
        }
        Ok(())
    }
    fn rewrite_tree(
        &mut self,
        ctx: Context,
        file: Id,
        branches: &BTreeSet<String>,
        source: Option<Id>,
        result: Id,
        owner: Option<&str>,
    ) -> Id {
        let mut entries = self.entries(&ctx);
        let mut changed = false;
        for entry in entries.values_mut() {
            match entry {
                Entry::File {
                    file: f,
                    branch,
                    node,
                    lazy,
                } if *f == file
                    && branches.contains(branch)
                    && source.is_none_or(|source| source == *node)
                    && (owner.is_some() || lazy.is_none()) =>
                {
                    if *node != result || owner.is_some_and(|o| o != branch) {
                        *node = result;
                        if let Some(owner) = owner {
                            *branch = owner.into();
                            *lazy = None;
                        }
                        changed = true;
                    }
                }
                Entry::Folder {
                    directory,
                    branch,
                    node,
                    lazy,
                } => {
                    let new = self.rewrite_tree(
                        Context {
                            directory: *directory,
                            branch: branch.clone(),
                            node: *node,
                            lazy: lazy.clone(),
                            ancestors: vec![],
                        },
                        file,
                        branches,
                        source,
                        result,
                        owner,
                    );
                    if new != *node {
                        *node = new;
                        *lazy = None;
                        changed = true;
                    }
                }
                _ => {}
            }
        }
        if changed {
            // Do not advance nested named heads while deriving a frozen view.
            let id = self.id();
            self.directories.insert(
                id,
                DirectoryNode {
                    id,
                    directory: ctx.directory,
                    parents: vec![ctx.node],
                    entries,
                    note: "同步文件引用".into(),
                },
            );
            id
        } else {
            ctx.node
        }
    }
    pub fn validate(&self) -> Result<()> {
        for node in self.files.values() {
            if node.body >= self.bodies.len() {
                return Err("正文引用失效".into());
            }
            for parent in &node.parents {
                if self.files.get(parent).is_none_or(|n| n.file != node.file)
                    || self.ancestor(node.id, *parent)
                {
                    return Err("文件父边无效或成环".into());
                }
            }
        }
        for node in self.directories.values() {
            for parent in &node.parents {
                if self
                    .directories
                    .get(parent)
                    .is_none_or(|n| n.directory != node.directory || *parent >= node.id)
                {
                    return Err("目录父边无效".into());
                }
            }
            for entry in node.entries.values() {
                match entry {
                    Entry::File { file, node, .. }
                        if self.files.get(node).is_none_or(|n| n.file != *file) =>
                    {
                        return Err("文件引用失效".into());
                    }
                    Entry::Folder {
                        directory, node, ..
                    } if self
                        .directories
                        .get(node)
                        .is_none_or(|n| n.directory != *directory) =>
                    {
                        return Err("子目录引用失效".into());
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
