//! Process scripts use the same commands as the free-operation page.
use super::model::{Action, Id, Result, Workspace};
use std::collections::BTreeSet;

pub struct Step {
    pub title: &'static str,
    pub detail: &'static str,
    pub action: Action,
    pub focus: &'static str,
}
pub struct Scenario {
    pub id: &'static str,
    pub title: &'static str,
    pub steps: Vec<Step>,
}
fn step(title: &'static str, detail: &'static str, action: Action, focus: &'static str) -> Step {
    Step {
        title,
        detail,
        action,
        focus,
    }
}
fn branch(path: &str, name: &str) -> Action {
    Action::CreateFileBranch {
        path: path.into(),
        name: name.into(),
    }
}
fn select(path: &str, name: &str) -> Action {
    Action::SwitchFileBranch {
        path: path.into(),
        name: name.into(),
    }
}
fn directory(path: &str, name: &str) -> Action {
    Action::CreateDirectoryBranch {
        path: path.into(),
        name: name.into(),
    }
}
fn view(path: &str, name: &str) -> Action {
    Action::SwitchDirectoryBranch {
        path: path.into(),
        name: name.into(),
    }
}
fn write(path: &str, body: &str) -> Action {
    Action::Write {
        path: path.into(),
        content: body.into(),
    }
}
pub fn all() -> Vec<Scenario> {
    let seed = Workspace::seed();
    let source = seed.file_at("A/报告.txt").unwrap().2;
    let mut merge = vec![
        step(
            "登记审阅分支",
            "只有分支记录，正文数量不变。",
            branch("A/报告.txt", "审阅"),
            "A/报告.txt",
        ),
        step(
            "main 写入 120",
            "main 增加 FULL；审阅仍引用初始的 100。",
            write("A/报告.txt", "预算 120"),
            "A/报告.txt",
        ),
        step(
            "选择审阅",
            "目录追加子树，当前文件重新显示 100。",
            select("A/报告.txt", "审阅"),
            "A/报告.txt",
        ),
        step(
            "审阅写入 110",
            "首写完成审阅分支；两个分支各有自己的正文。",
            write("A/报告.txt", "预算 110"),
            "A/报告.txt",
        ),
    ];
    let mut ready = Workspace::seed();
    for s in &merge {
        ready.apply(s.action.clone()).unwrap();
    }
    let chosen = ready.file_at("A/报告.txt").unwrap().2;
    let mut arbitrary = merge.clone_steps();
    merge.push(step(
        "复用审阅头，归属 main",
        "不新增文件节点；审阅转为只读别名，目录引用同步更新。",
        Action::Merge {
            path: "A/报告.txt".into(),
            branches: vec!["main".into(), "审阅".into()],
            chosen,
            owner: "main".into(),
        },
        "A/报告.txt",
    ));
    arbitrary.push(step(
        "选择历史正文作为结果",
        "新建合并元数据节点；复用初始正文及标签，两个分支头都成为父节点。",
        Action::Merge {
            path: "A/报告.txt".into(),
            branches: vec!["main".into(), "审阅".into()],
            chosen: source,
            owner: "审阅".into(),
        },
        "A/报告.txt",
    ));
    let mut tags = vec![
        step(
            "从初始节点改标签",
            "新标签节点复用正文；初始节点的草稿标签保持原样。",
            Action::Tags {
                path: "A/报告.txt".into(),
                source,
                tags: BTreeSet::from(["已审阅".into()]),
            },
            "A/报告.txt",
        ),
        step(
            "写入下一版",
            "新正文自动继承父节点的已审阅标签。",
            write("A/报告.txt", "预算 130"),
            "A/报告.txt",
        ),
    ];
    tags.push(step(
        "为历史初始节点加归档标签",
        "新增旁支；当前较新正文仍是 130，不退回 100。",
        Action::Tags {
            path: "A/报告.txt".into(),
            source,
            tags: BTreeSet::from(["草稿".into(), "归档".into()]),
        },
        "A/报告.txt",
    ));
    vec![
        Scenario {
            id: "file",
            title: "文件分支：登记 → 切换 → 首写",
            steps: vec![
                step(
                    "登记审阅分支",
                    "main 和审阅引用同一个父节点；没有空文件。",
                    branch("A/报告.txt", "审阅"),
                    "A/报告.txt",
                ),
                step(
                    "切换到审阅",
                    "读取父节点的预算 100；切换本身只增加目录子树。",
                    select("A/报告.txt", "审阅"),
                    "A/报告.txt",
                ),
                step(
                    "首写预算 110",
                    "此时才新增完整正文，完成审阅分支创建。",
                    write("A/报告.txt", "预算 110"),
                    "A/报告.txt",
                ),
                step(
                    "返回 main",
                    "main 仍显示预算 100；审阅内容和历史保留。",
                    select("A/报告.txt", "main"),
                    "A/报告.txt",
                ),
            ],
        },
        Scenario {
            id: "directory",
            title: "目录分支：按文件逐个首写",
            steps: vec![
                step(
                    "登记目录分支 review",
                    "直接引用整个父目录组成，不批量创建文件分支。",
                    directory("", "review"),
                    "A/报告.txt",
                ),
                step(
                    "切换到 review",
                    "两个文件都显示父节点内容。",
                    view("", "review"),
                    "A/报告.txt",
                ),
                step(
                    "只写报告",
                    "报告产生公共文件分支；说明仍引用父节点，目录只追加子树。",
                    write("A/报告.txt", "预算 140"),
                    "A/报告.txt",
                ),
                step(
                    "再写说明",
                    "说明现在才创建自己的公共文件分支；目录分支名称仍为 review。",
                    write("A/说明.txt", "补充说明"),
                    "A/说明.txt",
                ),
                step(
                    "选择 main 中的报告",
                    "切换文件也算目录写入：追加子树，不新建命名目录分支。",
                    select("A/报告.txt", "main"),
                    "A/报告.txt",
                ),
            ],
        },
        Scenario {
            id: "folder",
            title: "子目录分支与 SMB 默认组合",
            steps: vec![
                step(
                    "登记 A 的实验分支",
                    "A 的新分支引用原目录节点。",
                    directory("A", "实验"),
                    "A/报告.txt",
                ),
                step(
                    "根目录选择 A/实验",
                    "根目录追加子树，A 的所有文件仍复用正文。",
                    view("A", "实验"),
                    "A/报告.txt",
                ),
                step(
                    "写入实验报告",
                    "只有被写入的报告产生文件分支，说明保持父内容。",
                    write("A/报告.txt", "实验预算 160"),
                    "A/报告.txt",
                ),
                step(
                    "登记根目录 release",
                    "冻结当前的子目录组成。",
                    directory("", "release"),
                    "A/报告.txt",
                ),
                step(
                    "切换 SMB 默认组合",
                    "SMB 默认读取 release；普通页面选择与默认组合分别显示。",
                    Action::Default {
                        name: "release".into(),
                    },
                    "A/报告.txt",
                ),
            ],
        },
        Scenario {
            id: "namespace",
            title: "改名、移动、复制与删除",
            steps: vec![
                step(
                    "保留根目录 archive",
                    "先保留一个不参与后续修改的分支。",
                    directory("", "archive"),
                    "A/报告.txt",
                ),
                step(
                    "A 改名为 项目",
                    "每个文件新增路径节点，目录追加子树；正文数量不变。",
                    Action::Rename {
                        path: "A".into(),
                        name: "项目".into(),
                    },
                    "项目/报告.txt",
                ),
                step(
                    "报告从 项目 移动到 B",
                    "源停止跟踪，B 开始跟踪；archive 仍在旧位置引用旧节点。",
                    Action::Move {
                        path: "项目/报告.txt".into(),
                        target: "B".into(),
                    },
                    "B/报告.txt",
                ),
                step(
                    "向 项目 复制报告",
                    "复制得到独立文件身份和完整正文，不共享后续写入。",
                    Action::Copy {
                        path: "B/报告.txt".into(),
                        target: "项目".into(),
                    },
                    "项目/报告.txt",
                ),
                step(
                    "删除当前分支中的副本",
                    "只删除当前组成的引用；原文件与历史仍存在。",
                    Action::Delete {
                        path: "项目/报告.txt".into(),
                    },
                    "B/报告.txt",
                ),
                step(
                    "查看未修改的 archive",
                    "A、报告和说明都保持最初的名称、位置与正文。",
                    view("", "archive"),
                    "A/报告.txt",
                ),
            ],
        },
        Scenario {
            id: "merge-head",
            title: "合并文件分支：复用现有头节点",
            steps: merge,
        },
        Scenario {
            id: "merge-history",
            title: "合并文件分支：选择历史节点",
            steps: arbitrary,
        },
        Scenario {
            id: "tags",
            title: "标签继承与历史节点旁支",
            steps: tags,
        },
        Scenario {
            id: "backup",
            title: "副本份数与各盘独立前缀",
            steps: vec![
                step(
                    "为 A 配置三份",
                    "份数包含主盘；B01、C01 分别使用用户指定的前缀。",
                    Action::Protect {
                        path: "A".into(),
                        copies: 3,
                        prefixes: vec!["档案/项目".into(), "异地/项目".into()],
                    },
                    "A/报告.txt",
                ),
                step(
                    "写入新版本",
                    "演示显示固定的相对路径映射；不执行真实复制或保护证明。",
                    write("A/报告.txt", "预算 180"),
                    "A/报告.txt",
                ),
                step(
                    "主目录改名",
                    "已有备份绑定保持原前缀；主盘路径不决定备份前缀。",
                    Action::Rename {
                        path: "A".into(),
                        name: "新项目".into(),
                    },
                    "新项目/报告.txt",
                ),
            ],
        },
    ]
}
trait CloneSteps {
    fn clone_steps(&self) -> Vec<Step>;
}
impl CloneSteps for Vec<Step> {
    fn clone_steps(&self) -> Vec<Step> {
        self.iter()
            .map(|s| step(s.title, s.detail, s.action.clone(), s.focus))
            .collect()
    }
}
pub fn replay(index: usize, count: usize) -> Result<(Workspace, String, Id, String)> {
    let scenarios = all();
    let scene = scenarios.get(index).ok_or("演示场景不存在")?;
    if count > scene.steps.len() {
        return Err("步骤超出范围".into());
    }
    let mut model = Workspace::seed();
    let mut message = "初始状态：A 中有两个文件，B 为空".into();
    let mut focus = "A/报告.txt".to_owned();
    for step in scene.steps.iter().take(count) {
        message = model.apply(step.action.clone())?;
        focus = step.focus.into();
    }
    let node = model.file_at(&focus).map(|v| v.2).unwrap_or(0);
    Ok((model, message, node, focus))
}
