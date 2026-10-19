//! Rebuild the branch-system SVGs, standalone viewer and Markdown image index.
//! cargo run --manifest-path demos/branch-system/Cargo.toml --bin xcs-demo-reference
use std::{fmt::Write as _, fs, path::Path};
#[path = "../entry.rs"]
mod entry;

#[derive(Clone, Copy)]
enum Tone {
    Reused,
    New,
    Chosen,
    Reference,
}
impl Tone {
    fn color(self) -> &'static str {
        match self {
            Self::Reused => "#65758b",
            Self::New => "#16845d",
            Self::Chosen => "#7753c6",
            Self::Reference => "#326bc4",
        }
    }
    fn fill(self) -> &'static str {
        match self {
            Self::Reused => "#f2f5f9",
            Self::New => "#e8f7ef",
            Self::Chosen => "#f2edfc",
            Self::Reference => "#edf4ff",
        }
    }
}
use Tone::{Chosen as C, New as N, Reference as B, Reused as R};
struct Row {
    text: &'static str,
    tone: Tone,
}
struct Node {
    id: &'static str,
    x: f64,
    y: f64,
    w: f64,
    title: &'static str,
    kind: &'static str,
    badge: &'static str,
    tone: Tone,
    rows: Vec<Row>,
    head: bool,
}
impl Node {
    fn h(&self) -> f64 {
        if self.head {
            100.
        } else {
            122. + self.rows.len() as f64 * 34.
        }
    }
    fn cx(&self) -> f64 {
        self.x + self.w / 2.
    }
}
struct Edge {
    from: &'static str,
    to: &'static str,
    label: &'static str,
    reference: bool,
}
struct Note {
    x: f64,
    y: f64,
    w: f64,
    title: &'static str,
    rows: Vec<&'static str>,
}
struct Scene {
    id: &'static str,
    title: &'static str,
    short: &'static str,
    summary: &'static str,
    w: f64,
    h: f64,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    notes: Vec<Note>,
    lanes: Vec<(f64, &'static str)>,
}
fn row(text: &'static str, tone: Tone) -> Row {
    Row { text, tone }
}
fn node(
    id: &'static str,
    x: f64,
    y: f64,
    w: f64,
    title: &'static str,
    kind: &'static str,
    badge: &'static str,
    tone: Tone,
    rows: Vec<Row>,
) -> Node {
    Node {
        id,
        x,
        y,
        w,
        title,
        kind,
        badge,
        tone,
        rows,
        head: false,
    }
}
fn head(
    id: &'static str,
    x: f64,
    y: f64,
    w: f64,
    title: &'static str,
    badge: &'static str,
) -> Node {
    Node {
        id,
        x,
        y,
        w,
        title,
        kind: "分支标签 · 不是内容节点",
        badge,
        tone: B,
        rows: vec![],
        head: true,
    }
}
fn edge(from: &'static str, to: &'static str, label: &'static str) -> Edge {
    Edge {
        from,
        to,
        label,
        reference: false,
    }
}
fn reference(from: &'static str, to: &'static str) -> Edge {
    Edge {
        from,
        to,
        label: "直接引用同一父节点",
        reference: true,
    }
}
fn note(x: f64, y: f64, w: f64, title: &'static str, rows: Vec<&'static str>) -> Note {
    Note {
        x,
        y,
        w,
        title,
        rows,
    }
}
fn scene(
    id: &'static str,
    title: &'static str,
    short: &'static str,
    summary: &'static str,
) -> Scene {
    Scene {
        id,
        title,
        short,
        summary,
        w: 1280.,
        h: 960.,
        nodes: vec![],
        edges: vec![],
        notes: vec![],
        lanes: vec![],
    }
}
fn scenes() -> Vec<Scene> {
    let mut all = vec![];
    let mut s = scene(
        "01-file-register",
        "登记文件分支：两个名称，共用一个父节点",
        "登记文件分支",
        "还没有新内容节点；两个蓝色标签都直接指向 F1。空占位文件为 0。 ",
    );
    s.nodes = vec![
        head("main", 100., 225., 440., "main", "原文件分支"),
        head("review", 740., 225., 440., "审阅稿", "新登记 · 尚未写入"),
        node(
            "F1",
            390.,
            640.,
            500.,
            "F1 · 报告.docx",
            "实际文件节点",
            "唯一正文",
            R,
            vec![
                row("内容：预算 100", R),
                row("两个分支读取相同内容", R),
                row("正文不复制，也不建空文件", R),
            ],
        ),
    ];
    s.edges = vec![reference("F1", "main"), reference("F1", "review")];
    all.push(s);

    let mut s = scene(
        "02-file-first-write",
        "文件首次写入：在父节点上方产生新节点",
        "文件首次写入",
        "审阅稿第一次保存“预算 120”，才产生真实文件节点 F2；main 留在 F1。 ",
    );
    s.nodes = vec![
        node(
            "F2",
            390.,
            190.,
            500.,
            "F2 · 报告.docx",
            "本次新增文件节点",
            "审阅稿当前",
            N,
            vec![
                row("内容：预算 120", N),
                row("来源节点：F1", R),
                row("完整正文独立保存", N),
            ],
        ),
        node(
            "F1",
            390.,
            640.,
            500.,
            "F1 · 报告.docx",
            "父文件节点",
            "main 仍在这里",
            R,
            vec![
                row("内容：预算 100", R),
                row("原节点与原分支保持不变", R),
                row("新节点位于父节点上方", R),
            ],
        ),
    ];
    s.edges = vec![edge("F1", "F2", "首次写入")];
    all.push(s);

    let mut s = scene(
        "03-directory-register",
        "登记目录分支：直接共用父目录子树",
        "登记目录分支",
        "此时没有新增文件分支、文件内容节点或自有子树；资料中的后代也直接复用。 ",
    );
    s.nodes = vec![
        head("main", 100., 225., 440., "main", "原目录分支"),
        head("draft", 740., 225., 440., "方案甲", "新登记 · 尚未修改"),
        node(
            "T0",
            365.,
            620.,
            550.,
            "T0 · 项目目录",
            "同一棵父目录子树",
            "完整可读",
            R,
            vec![
                row("报告.docx → main @ F1", R),
                row("数据.csv → main @ G1", R),
                row("资料 / → main @ S1", R),
            ],
        ),
    ];
    s.edges = vec![reference("T0", "main"), reference("T0", "draft")];
    all.push(s);

    let mut s = scene(
        "04-directory-first-write",
        "只写报告：同时产生文件节点和目录子树",
        "目录内只写一文件",
        "本次只为报告创建公共文件分支。数据 G1、资料 S1 继续复用，目录分支仍叫方案甲。 ",
    );
    s.lanes = vec![(320., "目录子树的演进"), (960., "报告文件的公共历史")];
    s.nodes = vec![
        node(
            "T1",
            45.,
            205.,
            550.,
            "T1 · 项目目录",
            "本次新增目录子树",
            "方案甲当前",
            N,
            vec![
                row("报告 → 方案甲-7c2a @ F2", N),
                row("数据 → main @ G1　复用", R),
                row("资料 / → main @ S1　复用", R),
            ],
        ),
        node(
            "T0",
            45.,
            650.,
            550.,
            "T0 · 项目目录",
            "父目录子树",
            "main 仍在这里",
            R,
            vec![
                row("报告 → main @ F1", R),
                row("数据 → main @ G1", R),
                row("资料 / → main @ S1", R),
            ],
        ),
        node(
            "F2",
            685.,
            205.,
            550.,
            "F2 · 报告.docx",
            "本次新增文件节点",
            "公共分支 方案甲-7c2a",
            N,
            vec![
                row("内容：预算 120", N),
                row("来源：F1", R),
                row("可在文件详情中独立选择", N),
            ],
        ),
        node(
            "F1",
            685.,
            650.,
            550.,
            "F1 · 报告.docx",
            "父文件节点",
            "文件 main 保持不变",
            R,
            vec![
                row("内容：预算 100", R),
                row("原始节点", R),
                row("不存在目录私有文件副本", R),
            ],
        ),
    ];
    s.edges = vec![
        edge("T0", "T1", "完成方案甲首次分化"),
        edge("F1", "F2", "首次实际写入才分叉"),
    ];
    all.push(s);

    let mut s = scene(
        "05-next-file-write",
        "报告已修改过，数据现在才第一次写入",
        "再首次修改数据",
        "目录追加 T3，数据第一次产生 G2；报告继续复用 F3，不增加新的命名目录分支。 ",
    );
    s.lanes = vec![(320., "同一目录分支：方案甲"), (960., "数据文件的公共历史")];
    s.nodes = vec![
        node(
            "T3",
            45.,
            205.,
            550.,
            "T3 · 项目目录",
            "本次新增目录子树",
            "方案甲当前",
            N,
            vec![
                row("报告 → 方案甲-7c2a @ F3　复用", R),
                row("数据 → 方案甲-91bd @ G2", N),
                row("资料 / → main @ S1　复用", R),
            ],
        ),
        node(
            "T2",
            45.,
            650.,
            550.,
            "T2 · 项目目录",
            "修改前子树",
            "仍叫方案甲",
            R,
            vec![
                row("报告 → 方案甲-7c2a @ F3", R),
                row("数据 → main @ G1　继承", R),
                row("资料 / → main @ S1", R),
            ],
        ),
        node(
            "G2",
            685.,
            205.,
            550.,
            "G2 · 数据.csv",
            "本次新增文件节点",
            "公共分支 方案甲-91bd",
            N,
            vec![
                row("这时才为数据创建公共分支", N),
                row("新数据完整保存", N),
                row("报告分支保持原样", R),
            ],
        ),
        node(
            "G1",
            685.,
            650.,
            550.,
            "G1 · 数据.csv",
            "父文件节点",
            "数据 main 保持不变",
            R,
            vec![
                row("原数据", R),
                row("之前一直被目录直接引用", R),
                row("不会因报告修改而提前分叉", R),
            ],
        ),
    ];
    s.edges = vec![
        edge("T2", "T3", "追加子树，分支名不变"),
        edge("G1", "G2", "数据首次写入"),
    ];
    all.push(s);

    let mut s = scene(
        "06-switch-file",
        "切换到已有文件分支：只新增目录子树",
        "切换已有文件分支",
        "方案乙第一次操作是选中审阅稿 F8。这是目录写入，完成首次分化，但没有新增文件正文。 ",
    );
    s.lanes = vec![(320., "目录引用发生变化"), (960., "复用已有文件节点")];
    s.nodes = vec![
        node(
            "T1",
            45.,
            205.,
            550.,
            "T1 · 项目目录",
            "本次新增目录子树",
            "方案乙当前",
            N,
            vec![
                row("报告 → 审阅稿 @ F8　明确指定", C),
                row("数据 → main @ G1　继承", R),
                row("方案乙首次分化完成", N),
            ],
        ),
        node(
            "T0",
            45.,
            650.,
            550.,
            "T0 · 项目目录",
            "父目录子树",
            "方案乙原先引用这里",
            R,
            vec![
                row("报告 → main @ F1　继承", R),
                row("数据 → main @ G1", R),
                row("尚未产生自有子树", R),
            ],
        ),
        node(
            "F8",
            685.,
            650.,
            550.,
            "F8 · 报告.docx",
            "早已存在的文件节点",
            "审阅稿当前",
            R,
            vec![
                row("内容：预算 108", R),
                row("被新目录子树直接引用", C),
                row("正文节点新增数量：0", R),
            ],
        ),
    ];
    s.edges = vec![edge("T0", "T1", "切换引用也是一次写入")];
    s.notes = vec![note(
        730.,
        240.,
        460.,
        "本次没有新的文件子节点",
        vec![
            "文件分支新增：0",
            "文件内容节点新增：0",
            "新子树只改变报告的引用目标",
        ],
    )];
    all.push(s);

    let mut s = scene(
        "07-explicit-write",
        "明确选中审阅稿后，继续写入审阅稿",
        "明确指定后的写入",
        "P3：用户已经指定已有分支，后续保存推进这个分支，不再私自另建文件分支。 ",
    );
    s.lanes = vec![(320., "目录子树"), (960., "同一个公共文件分支：审阅稿")];
    s.nodes = vec![
        node(
            "T2",
            45.,
            205.,
            550.,
            "T2 · 项目目录",
            "本次新增目录子树",
            "方案乙当前",
            N,
            vec![
                row("报告 → 审阅稿 @ F9　指定", N),
                row("数据 → main @ G1　复用", R),
                row("目录分支身份与名称不变", R),
            ],
        ),
        node(
            "T1",
            45.,
            650.,
            550.,
            "T1 · 项目目录",
            "修改前子树",
            "旧视图仍引用 F8",
            R,
            vec![
                row("报告 → 审阅稿 @ F8　指定", R),
                row("数据 → main @ G1", R),
                row("保存后 T1 仍然不变", R),
            ],
        ),
        node(
            "F9",
            685.,
            205.,
            550.,
            "F9 · 报告.docx",
            "本次新增文件节点",
            "审阅稿当前",
            N,
            vec![
                row("内容：预算 125", N),
                row("来源：F8", R),
                row("新文件分支登记：0", R),
            ],
        ),
        node(
            "F8",
            685.,
            650.,
            550.,
            "F8 · 报告.docx",
            "父文件节点",
            "原审阅稿节点",
            R,
            vec![
                row("内容：预算 108", R),
                row("其他目录冻结在 F8 的引用不变", R),
                row("不会自动追到 F9", R),
            ],
        ),
    ];
    s.edges = vec![
        edge("T1", "T2", "推进当前目录子树"),
        edge("F8", "F9", "写入用户明确指定的分支"),
    ];
    all.push(s);

    let mut s = scene(
        "08-switch-folder",
        "切换子文件夹：父目录直接引用目标子树",
        "切换子文件夹",
        "资料选择发布版 S5；说明、图片、附件仍可各用不同文件分支，不做同名批量切换。 ",
    );
    s.lanes = vec![(320., "父目录新增子树"), (960., "目标子目录保持原样")];
    s.nodes = vec![
        node(
            "T2",
            45.,
            205.,
            550.,
            "T2 · 项目目录",
            "本次新增目录子树",
            "方案甲当前",
            N,
            vec![
                row("报告 → 方案甲-7c2a @ F2　复用", R),
                row("资料 / → 发布版 @ S5　指定", C),
                row("其他目录内容保持原引用", R),
            ],
        ),
        node(
            "T1",
            45.,
            650.,
            550.,
            "T1 · 项目目录",
            "父目录子树",
            "切换前",
            R,
            vec![
                row("报告 → 方案甲-7c2a @ F2", R),
                row("资料 / → main @ S1", R),
                row("父目录引用的是旧子树", R),
            ],
        ),
        node(
            "S5",
            685.,
            650.,
            550.,
            "S5 · 资料目录",
            "早已存在的子目录子树",
            "发布版当前",
            R,
            vec![
                row("说明.txt → 审阅稿 @ H7", R),
                row("图片.png → main @ I1", R),
                row("附件.pdf → 客户版 @ J3", R),
            ],
        ),
    ];
    s.edges = vec![edge("T1", "T2", "更换子目录引用")];
    s.notes = vec![note(
        730.,
        240.,
        460.,
        "没有复制子目录里的文件",
        vec![
            "S5 本身不被改写",
            "文件正文新增：0",
            "只改变父目录对资料的引用",
        ],
    )];
    all.push(s);

    let mut s = scene(
        "09-nested-write",
        "修改深层说明：沿路径生成必要的新子树",
        "深层文件修改",
        "每列都是同一对象的父子演进。报告 F1、图片 I1、模板 K1 都复用，只有说明产生新正文。 ",
    );
    s.w = 1440.;
    s.lanes = vec![
        (195., "项目目录"),
        (545., "资料目录"),
        (895., "草稿目录"),
        (1245., "说明文件"),
    ];
    s.nodes = vec![
        node(
            "T1",
            40.,
            205.,
            310.,
            "T1 · 项目",
            "新目录子树",
            "方案甲",
            N,
            vec![
                row("资料 → S2", N),
                row("报告 → F1　复用", R),
                row("其余引用不变", R),
            ],
        ),
        node(
            "T0",
            40.,
            650.,
            310.,
            "T0 · 项目",
            "父目录子树",
            "原节点",
            R,
            vec![
                row("资料 → S1", R),
                row("报告 → F1", R),
                row("旧路径保留", R),
            ],
        ),
        node(
            "S2",
            390.,
            205.,
            310.,
            "S2 · 资料",
            "新目录子树",
            "沿修改路径",
            N,
            vec![
                row("草稿 → R2", N),
                row("图片 → I1　复用", R),
                row("其余引用不变", R),
            ],
        ),
        node(
            "S1",
            390.,
            650.,
            310.,
            "S1 · 资料",
            "父目录子树",
            "原节点",
            R,
            vec![
                row("草稿 → R1", R),
                row("图片 → I1", R),
                row("旧路径保留", R),
            ],
        ),
        node(
            "R2",
            740.,
            205.,
            310.,
            "R2 · 草稿",
            "新目录子树",
            "沿修改路径",
            N,
            vec![
                row("说明 → H2", N),
                row("模板 → K1　复用", R),
                row("其余引用不变", R),
            ],
        ),
        node(
            "R1",
            740.,
            650.,
            310.,
            "R1 · 草稿",
            "父目录子树",
            "原节点",
            R,
            vec![
                row("说明 → H1", R),
                row("模板 → K1", R),
                row("旧路径保留", R),
            ],
        ),
        node(
            "H2",
            1090.,
            205.,
            310.,
            "H2 · 说明",
            "新文件节点",
            "公共文件分支",
            N,
            vec![
                row("本次写入的内容", N),
                row("来源 H1", R),
                row("真实完整正文", N),
            ],
        ),
        node(
            "H1",
            1090.,
            650.,
            310.,
            "H1 · 说明",
            "父文件节点",
            "main",
            R,
            vec![
                row("原说明内容", R),
                row("创建目录时复用", R),
                row("此刻才发生分叉", R),
            ],
        ),
    ];
    s.edges = vec![
        edge("T0", "T1", "项目新子树"),
        edge("S1", "S2", "资料新子树"),
        edge("R1", "R2", "草稿新子树"),
        edge("H1", "H2", "写入新正文"),
    ];
    all.push(s);

    let mut s = scene(
        "10-namespace",
        "新增、删除：目录保存完整组成",
        "完整目录恢复",
        "P1：切回 main 恢复报告和数据；选择方案甲显示报告和日志，两个分支不互相混入文件。 ",
    );
    s.h = 1280.;
    s.nodes = vec![
        node(
            "T2",
            70.,
            170.,
            570.,
            "T2 · 项目目录",
            "删除数据后的新子树",
            "方案甲当前",
            N,
            vec![
                row("报告.docx → F1　复用", R),
                row("日志.txt → C1　复用", R),
                row("数据.csv 已从本子树移除", C),
            ],
        ),
        node(
            "T1",
            70.,
            570.,
            570.,
            "T1 · 项目目录",
            "新增日志时产生的子树",
            "方案甲的上一节点",
            R,
            vec![
                row("报告.docx → F1", R),
                row("数据.csv → G1", R),
                row("日志.txt → C1　新文件", R),
            ],
        ),
        node(
            "T0",
            70.,
            970.,
            570.,
            "T0 · 项目目录",
            "原目录子树",
            "main 当前",
            R,
            vec![
                row("报告.docx → F1", R),
                row("数据.csv → G1", R),
                row("还没有日志.txt", R),
            ],
        ),
    ];
    s.edges = vec![
        edge("T1", "T2", "删除目录条目"),
        edge("T0", "T1", "新增日志文件"),
    ];
    s.notes = vec![
        note(
            760.,
            235.,
            440.,
            "选择方案甲：读取 T2",
            vec!["显示报告、日志", "不显示数据", "目录分支名仍为方案甲"],
        ),
        note(
            760.,
            660.,
            440.,
            "选择 main：读取 T0",
            vec![
                "显示报告、数据",
                "不混入方案甲新增的日志",
                "移除条目不等于清除历史正文",
            ],
        ),
    ];
    all.push(s);

    let mut s = scene(
        "11-rename-move",
        "只在选中分支改名：文件身份和正文不变",
        "选中分支重命名",
        "在方案甲将报告改名为客户报告，未选 main 仍保留原名称。文件身份 A、节点 F1 和正文继续复用。 ",
    );
    s.lanes = vec![
        (320., "仅选中分支的名称形成新子树"),
        (960., "仍然是同一个文件"),
    ];
    s.nodes = vec![
        node(
            "T1",
            45.,
            205.,
            550.,
            "T1 · 项目目录",
            "本次新增目录子树",
            "方案甲当前",
            N,
            vec![
                row("客户报告.docx → F1", C),
                row("归档 / → S0　复用", R),
                row("文件身份 A 保持不变", R),
            ],
        ),
        node(
            "T0",
            45.,
            650.,
            550.,
            "T0 · 项目目录",
            "父目录子树",
            "main 当前",
            R,
            vec![
                row("报告.docx → F1", R),
                row("归档 / → 空目录 S0", R),
                row("文件身份 A", R),
            ],
        ),
        node(
            "F1",
            685.,
            650.,
            550.,
            "F1 · 文件身份 A",
            "继续复用的文件节点",
            "正文节点新增 0",
            R,
            vec![
                row("原完整正文", R),
                row("稳定文件身份不因改名重建", R),
                row("内部版本存储名保持不变", R),
            ],
        ),
    ];
    s.edges = vec![edge("T0", "T1", "仅在方案甲重命名")];
    s.notes = vec![note(
        730.,
        235.,
        460.,
        "下载名取当前目录视图",
        vec![
            "main：报告.docx",
            "方案甲：客户报告.docx",
            "历史子树仍保留各自名称",
        ],
    )];
    all.push(s);

    let mut s = scene(
        "12-smb-default",
        "SMB 默认主分支：用户可以选择方案甲",
        "切换 SMB 默认入口",
        "蓝色标记表示默认入口。切换入口不创建正文或子树；后续新打开的操作使用选定视图。 ",
    );
    s.nodes = vec![
        node(
            "T1",
            390.,
            190.,
            500.,
            "T1 · 项目目录",
            "已有子树，不是本次新建",
            "方案甲",
            R,
            vec![
                row("报告 → 方案甲-7c2a @ F2", C),
                row("数据 → main @ G1", R),
                row("现在被选为 SMB 默认视图", B),
            ],
        ),
        node(
            "T0",
            390.,
            640.,
            500.,
            "T0 · 项目目录",
            "父目录子树",
            "main",
            R,
            vec![
                row("报告 → main @ F1", R),
                row("数据 → main @ G1", R),
                row("原 main 不被改写或改名", R),
            ],
        ),
    ];
    s.edges = vec![edge("T0", "T1", "此前已有的子树演进")];
    s.notes = vec![
        note(
            35.,
            230.,
            310.,
            "切换后新操作",
            vec![
                "默认入口 → 方案甲",
                "新打开报告 → F2",
                "写入生成方案甲后续子树",
            ],
        ),
        note(
            935.,
            650.,
            310.,
            "切换前的句柄",
            vec![
                "已开只读 → 仍读 F1",
                "旧写入目标冲突 → 保留暂存",
                "不悄悄转写新默认分支",
            ],
        ),
    ];
    all.push(s);

    let mut s = scene(
        "13-empty-file",
        "真正提交空文件：0 字节也是真实新版本",
        "真正的空文件版本",
        "新建分支时没有空占位文件。这里的 F2 来自用户实际提交“清空文件”，因此是完整的真实版本。 ",
    );
    s.nodes = vec![
        node(
            "F2",
            390.,
            190.,
            500.,
            "F2 · 报告.docx",
            "本次新增文件节点",
            "方案甲-7c2a 当前",
            N,
            vec![
                row("真实内容长度：0 字节", N),
                row("来源：20 字节的 F1", R),
                row("已成功提交，不是占位文件", N),
            ],
        ),
        node(
            "F1",
            390.,
            640.,
            500.,
            "F1 · 报告.docx",
            "父文件节点",
            "main 当前",
            R,
            vec![
                row("原内容长度：20 字节", R),
                row("登记分支时直接读取本节点", R),
                row("只读关闭或取消不会产生 F2", R),
            ],
        ),
    ];
    s.edges = vec![edge("F1", "F2", "用户实际提交清空操作")];
    all.push(s);

    let mut s = scene(
        "14-label-metadata",
        "改分支名、改备注：标签变，节点不增加",
        "分支名与备注",
        "P2：公共文件分支自动命名后可改名。分支 ID 不变；标签和备注编辑只记元数据历史。 ",
    );
    s.nodes = vec![
        head(
            "directory-label",
            85.,
            225.,
            470.,
            "客户交付",
            "原名方案甲 · 分支 DB2 不变",
        ),
        head(
            "file-label",
            725.,
            225.,
            470.,
            "给客户的修订版",
            "原名方案甲-7c2a · 分支 FB7 不变",
        ),
        node(
            "T1",
            85.,
            640.,
            470.,
            "T1 · 项目目录",
            "继续使用同一目录子树",
            "没有新增子树",
            R,
            vec![
                row("报告 → 文件分支 FB7 @ F2", R),
                row("数据 → main @ G1", R),
                row("分支标签可改，身份不变", R),
            ],
        ),
        node(
            "F2",
            725.,
            640.,
            470.,
            "F2 · 报告.docx",
            "继续使用同一文件节点",
            "正文保持不变",
            R,
            vec![
                row("备注：第二轮预算修订", C),
                row("新增一条备注编辑历史", C),
                row("文件内容节点新增：0", R),
            ],
        ),
    ];
    s.edges = vec![
        reference("T1", "directory-label"),
        reference("F2", "file-label"),
    ];
    all.push(s);

    let mut s = scene(
        "15-frozen-head",
        "冻结 F1 不追赶 F9，也不把公共 HEAD 倒退",
        "冻结引用与最新 HEAD",
        "仅修改数据，报告仍读冻结的 F1。报告公共 main 之前已到 F9，保持 F9，不因兄弟文件保存而倒退。 ",
    );
    s.lanes = vec![
        (320., "当前目录的冻结引用"),
        (960., "公共文件分支的独立演进"),
    ];
    s.nodes = vec![
        node(
            "T2",
            45.,
            205.,
            550.,
            "T2 · 项目目录",
            "本次新增目录子树",
            "方案甲当前",
            N,
            vec![
                row("报告 → main @ F1　仍冻结 F1", R),
                row("数据 → 方案甲-91bd @ G2", N),
                row("写数据不影响报告公共 HEAD", R),
            ],
        ),
        node(
            "T1",
            45.,
            650.,
            550.,
            "T1 · 项目目录",
            "父目录子树",
            "修改前",
            R,
            vec![
                row("报告 → main @ F1", R),
                row("数据 → main @ G1", R),
                row("目录引用固定具体节点", R),
            ],
        ),
        node(
            "F9",
            685.,
            205.,
            550.,
            "F9 · 报告.docx",
            "此前已存在的文件节点",
            "公共 main 当前",
            R,
            vec![
                row("报告的最新内容", R),
                row("本次没有改写报告", R),
                row("HEAD 仍保持在 F9", C),
            ],
        ),
        node(
            "F1",
            685.,
            650.,
            550.,
            "F1 · 报告.docx",
            "旧文件节点",
            "目录仍可冻结引用",
            R,
            vec![
                row("报告的旧内容", R),
                row("冻结引用不自动订阅最新 HEAD", R),
                row("公共分支此前从这里继续演进", R),
            ],
        ),
    ];
    s.edges = vec![
        edge("T1", "T2", "只写数据"),
        edge("F1", "F9", "此前已有的文件演进"),
    ];
    all.push(s);

    let mut s = scene(
        "16-branch-history",
        "真正分叉与连续子树：父在下，子在上",
        "完整分叉历史",
        "方案甲、方案乙由用户显式创建。每个分支继续修改时追加子树，名称与分支 ID 保持不变。 ",
    );
    s.h = 1160.;
    s.nodes = vec![
        node(
            "T2",
            80.,
            155.,
            460.,
            "T2 · 方案甲",
            "最新目录子树",
            "方案甲当前",
            N,
            vec![row("报告 → F3", N), row("数据 → G1　复用", R)],
        ),
        node(
            "T1",
            80.,
            505.,
            460.,
            "T1 · 方案甲",
            "前一个目录子树",
            "同一命名分支",
            R,
            vec![row("报告 → F2", R), row("数据 → G1", R)],
        ),
        node(
            "T8",
            740.,
            155.,
            460.,
            "T8 · 方案乙",
            "最新目录子树",
            "方案乙当前",
            N,
            vec![row("报告 → F1　复用", R), row("数据 → G8", N)],
        ),
        node(
            "T7",
            740.,
            505.,
            460.,
            "T7 · 方案乙",
            "前一个目录子树",
            "同一命名分支",
            R,
            vec![row("报告 → F1", R), row("数据 → G7", R)],
        ),
        node(
            "T0",
            410.,
            855.,
            460.,
            "T0 · 项目目录",
            "共同父子树",
            "main 当前",
            R,
            vec![row("报告 → F1", R), row("数据 → G1", R)],
        ),
    ];
    s.edges = vec![
        edge("T0", "T1", "首次分化"),
        edge("T0", "T7", "首次分化"),
        edge("T1", "T2", "再次修改"),
        edge("T7", "T8", "再次修改"),
    ];
    all.push(s);
    all.extend(transfer_scenes());
    all.extend(rename_node_scenes());
    all.extend(merge_tag_scenes());
    all
}

fn transfer_scenes() -> Vec<Scene> {
    let mut all = vec![];
    let mut s = scene(
        "17-move-between-branches",
        "移动 a.m → b.l：两侧各记一个子树",
        "跨目录移交",
        "文件 f 实体迁到 b；a.m 记录移出并停止跟踪，b.l 记录加入并接管。a.n、b.p 保持原样。 ",
    );
    s.w = 1440.;
    s.lanes = vec![
        (195., "源 a.m · 已选"),
        (545., "目标 b.l · 已选"),
        (895., "a.n · 未选"),
        (1245., "b.p · 未选"),
    ];
    s.nodes = vec![
        node(
            "M1",
            40.,
            205.,
            310.,
            "M1 · a.m",
            "新增移出子树",
            "停止跟踪 f",
            N,
            vec![
                row("报告.docx 已移出", C),
                row("当前条目不再有 f", N),
                row("移交目标：b.l", R),
            ],
        ),
        node(
            "M0",
            40.,
            650.,
            310.,
            "M0 · a.m",
            "移动前子树",
            "父节点保留",
            R,
            vec![
                row("报告.docx → f @ F1", R),
                row("此前由 a.m 跟踪", R),
                row("历史仍可回看", R),
            ],
        ),
        node(
            "L1",
            390.,
            205.,
            310.,
            "L1 · b.l",
            "新增加入子树",
            "接管跟踪 f",
            N,
            vec![
                row("报告.docx → f @ F1", N),
                row("实际容器归属：b", C),
                row("文件身份与正文不变", R),
            ],
        ),
        node(
            "L0",
            390.,
            650.,
            310.,
            "L0 · b.l",
            "移动前子树",
            "原节点",
            R,
            vec![
                row("此前没有报告.docx", R),
                row("目标分支明确为 l", R),
                row("其余条目保持原样", R),
            ],
        ),
        node(
            "N0",
            740.,
            650.,
            310.,
            "N0 · a.n",
            "未选分支当前子树",
            "不追加子树",
            R,
            vec![
                row("a / 报告.docx", R),
                row("仍引用 f @ F1", R),
                row("旧逻辑路径仍可读取", R),
            ],
        ),
        node(
            "P0",
            1090.,
            650.,
            310.,
            "P0 · b.p",
            "未选分支当前子树",
            "保持原样",
            R,
            vec![
                row("仍然没有报告.docx", R),
                row("不因同在 b 而加入", R),
                row("当前子树仍是 P0", R),
            ],
        ),
    ];
    s.edges = vec![
        edge("M0", "M1", "本次移动 · 移出"),
        edge("L0", "L1", "同笔操作 · 加入"),
    ];
    all.push(s);

    let mut s = scene(
        "18-tracking-after-move",
        "移动后继续写：只由 b.l 跟踪更新",
        "移动后继续写入",
        "在 b.l 保存新内容，产生 F2 与 L2；a.m 不再跟踪，未选 a.n 仍从旧逻辑路径读取 F1。 ",
    );
    s.w = 1440.;
    s.lanes = vec![
        (195., "a.m · 已停止跟踪"),
        (545., "b.l · 当前跟踪者"),
        (895., "a.n · 仍冻结 F1"),
        (1245., "实际存储：目录 b"),
    ];
    s.nodes = vec![
        node(
            "M1",
            40.,
            650.,
            310.,
            "M1 · a.m",
            "移动后的当前子树",
            "保持 M1",
            R,
            vec![
                row("不含文件 f", R),
                row("本次不追加 M2", R),
                row("旧 M0 历史仍保留", R),
            ],
        ),
        node(
            "L2",
            390.,
            205.,
            310.,
            "L2 · b.l",
            "本次新增目录子树",
            "继续跟踪",
            N,
            vec![
                row("报告.docx → f @ F2", N),
                row("文件正文更新为 F2", N),
                row("只推进目标分支上下文", R),
            ],
        ),
        node(
            "L1",
            390.,
            650.,
            310.,
            "L1 · b.l",
            "移入时的父子树",
            "原加入记录",
            R,
            vec![
                row("报告.docx → f @ F1", R),
                row("从 a.m 移入", R),
                row("加入时固定的是 F1", R),
            ],
        ),
        node(
            "N0",
            740.,
            650.,
            310.,
            "N0 · a.n",
            "未选分支当前子树",
            "保持 N0",
            R,
            vec![
                row("a / 报告.docx", R),
                row("仍引用 f @ F1", R),
                row("不自动追到 F2", R),
            ],
        ),
        node(
            "F2",
            1090.,
            205.,
            310.,
            "F2 · 文件 f",
            "本次新增文件节点",
            "实体在 b 中",
            N,
            vec![
                row("内容：预算 120", N),
                row("来源节点：F1", R),
                row("继续沿明确目标分支写", R),
            ],
        ),
        node(
            "F1",
            1090.,
            650.,
            310.,
            "F1 · 文件 f",
            "父文件节点",
            "仍保存在 b 中",
            R,
            vec![
                row("内容：预算 100", R),
                row("a.n、旧 M0 仍可引用", R),
                row("按身份解析实际位置", R),
            ],
        ),
    ];
    s.edges = vec![
        edge("L1", "L2", "b.l 记录后续变化"),
        edge("F1", "F2", "实际内容写入"),
    ];
    all.push(s);

    let mut s = scene(
        "19-batch-move",
        "批量移动：源、目标逐项配对",
        "批量移动与配对",
        "映射 a.m@F1 → b.l、a.n@F2 → b.p；a.r、b.q 未选不变。文件 f 的实体只迁到 b 一次。 ",
    );
    s.w = 1440.;
    s.lanes = vec![
        (195., "源 a.m · 已选"),
        (545., "源 a.n · 已选"),
        (895., "目标 b.l · 已选"),
        (1245., "目标 b.p · 已选"),
    ];
    s.nodes = vec![
        node(
            "M1",
            40.,
            205.,
            310.,
            "M1 · a.m",
            "新增移出子树",
            "映射 1",
            N,
            vec![
                row("移出 f @ F1", C),
                row("目标：b.l", C),
                row("停止跟踪 f", N),
            ],
        ),
        node(
            "M0",
            40.,
            650.,
            310.,
            "M0 · a.m",
            "源父子树",
            "保存原引用",
            R,
            vec![
                row("报告.docx → f @ F1", R),
                row("源节点：F1", R),
                row("旧历史不删除", R),
            ],
        ),
        node(
            "N1",
            390.,
            205.,
            310.,
            "N1 · a.n",
            "新增移出子树",
            "映射 2",
            N,
            vec![
                row("移出 f @ F2", C),
                row("目标：b.p", C),
                row("停止跟踪 f", N),
            ],
        ),
        node(
            "N0",
            390.,
            650.,
            310.,
            "N0 · a.n",
            "源父子树",
            "保存原引用",
            R,
            vec![
                row("报告.docx → f @ F2", R),
                row("源节点：F2", R),
                row("不同分支可引用不同版", R),
            ],
        ),
        node(
            "L1",
            740.,
            205.,
            310.,
            "L1 · b.l",
            "新增加入子树",
            "接收映射 1",
            N,
            vec![
                row("报告.docx → f @ F1", N),
                row("接管本分支的跟踪", N),
                row("不误用映射 2 的 F2", R),
            ],
        ),
        node(
            "L0",
            740.,
            650.,
            310.,
            "L0 · b.l",
            "目标父子树",
            "原节点",
            R,
            vec![
                row("此前没有报告.docx", R),
                row("依据 L0 发布本批变化", R),
                row("其余条目继续复用", R),
            ],
        ),
        node(
            "P1",
            1090.,
            205.,
            310.,
            "P1 · b.p",
            "新增加入子树",
            "接收映射 2",
            N,
            vec![
                row("报告.docx → f @ F2", N),
                row("接管本分支的跟踪", N),
                row("同一个 f，冻结不同版", R),
            ],
        ),
        node(
            "P0",
            1090.,
            650.,
            310.,
            "P0 · b.p",
            "目标父子树",
            "原节点",
            R,
            vec![
                row("此前没有报告.docx", R),
                row("依据 P0 发布本批变化", R),
                row("本批一起发布", R),
            ],
        ),
    ];
    s.edges = vec![
        edge("M0", "M1", "映射 1 移出"),
        edge("N0", "N1", "映射 2 移出"),
        edge("L0", "L1", "映射 1 加入"),
        edge("P0", "P1", "映射 2 加入"),
    ];
    all.push(s);

    let mut s = scene(
        "20-batch-copy",
        "批量复制：源不变，选中目标加入独立副本",
        "批量复制",
        "读取 a.m 的 f@F1，复制到 b 的 l、p 分支；只产生一个实体副本 c@C1。原文件 f 与未选 b.q 不变。 ",
    );
    s.w = 1440.;
    s.lanes = vec![
        (195., "源 a.m · 只读取"),
        (545., "目标 b.l · 已选"),
        (895., "目标 b.p · 已选"),
        (1245., "b.q · 未选"),
    ];
    s.nodes = vec![
        node(
            "M0",
            40.,
            650.,
            310.,
            "M0 · a.m",
            "源子树保持原样",
            "不新增源子树",
            R,
            vec![
                row("报告.docx → f @ F1", R),
                row("实际原文件仍在 a", R),
                row("继续保留原跟踪关系", R),
            ],
        ),
        node(
            "L1",
            390.,
            205.,
            310.,
            "L1 · b.l",
            "新增副本加入子树",
            "跟踪副本 c",
            N,
            vec![
                row("报告副本.docx → c @ C1", N),
                row("c 是新的文件身份", C),
                row("复制来源：f @ F1", R),
            ],
        ),
        node(
            "L0",
            390.,
            650.,
            310.,
            "L0 · b.l",
            "目标父子树",
            "原节点",
            R,
            vec![
                row("此前没有副本条目", R),
                row("目标实际目录：b", R),
                row("仅本次选中范围更新", R),
            ],
        ),
        node(
            "P1",
            740.,
            205.,
            310.,
            "P1 · b.p",
            "新增副本加入子树",
            "同一副本初始版",
            N,
            vec![
                row("报告副本.docx → c @ C1", N),
                row("复用本次创建的副本", R),
                row("后续与原文件 f 独立", C),
            ],
        ),
        node(
            "P0",
            740.,
            650.,
            310.,
            "P0 · b.p",
            "目标父子树",
            "原节点",
            R,
            vec![
                row("此前没有副本条目", R),
                row("副本只在 b 保存一份", R),
                row("所选分支引用同一 C1", R),
            ],
        ),
        node(
            "Q0",
            1090.,
            650.,
            310.,
            "Q0 · b.q",
            "未选目标当前子树",
            "保持原样",
            R,
            vec![
                row("仍然没有副本条目", R),
                row("不因同在 b 而加入", R),
                row("当前子树仍为 Q0", R),
            ],
        ),
    ];
    s.edges = vec![
        edge("L0", "L1", "加入独立副本"),
        edge("P0", "P1", "加入同一副本引用"),
    ];
    all.push(s);

    let mut s = scene(
        "21-batch-delete",
        "批量删除：只移除选中分支，保留其他引用",
        "批量删除",
        "勾选 a.m、a.n 移除报告，两者停止跟踪。未选 a.r 仍能读取 F1，因此实际文件和所需版本不能一并清除。 ",
    );
    s.lanes = vec![
        (235., "a.m · 已选"),
        (640., "a.n · 已选"),
        (1045., "a.r · 未选"),
    ];
    s.nodes = vec![
        node(
            "M1",
            45.,
            205.,
            380.,
            "M1 · a.m",
            "新增移除子树",
            "停止跟踪 f",
            N,
            vec![
                row("报告.docx 已移除", C),
                row("保留.txt → G1　复用", R),
                row("移除不等于历史清除", R),
            ],
        ),
        node(
            "M0",
            45.,
            650.,
            380.,
            "M0 · a.m",
            "父子树",
            "原条目仍在历史中",
            R,
            vec![
                row("报告.docx → f @ F1", R),
                row("保留.txt → G1", R),
                row("旧节点仍可回看", R),
            ],
        ),
        node(
            "N1",
            450.,
            205.,
            380.,
            "N1 · a.n",
            "新增移除子树",
            "停止跟踪 f",
            N,
            vec![
                row("报告.docx 已移除", C),
                row("其他条目保持原样", R),
                row("只更新本次所选分支", R),
            ],
        ),
        node(
            "N0",
            450.,
            650.,
            380.,
            "N0 · a.n",
            "父子树",
            "原条目仍在历史中",
            R,
            vec![
                row("报告.docx → f @ F2", R),
                row("删除前的版本为 F2", R),
                row("记录本分支移除", R),
            ],
        ),
        node(
            "R0",
            855.,
            650.,
            380.,
            "R0 · a.r",
            "未选分支当前子树",
            "保持原样且可读",
            R,
            vec![
                row("报告.docx → f @ F1", R),
                row("仍可下载原版本", R),
                row("实际容器继续保留", R),
            ],
        ),
    ];
    s.edges = vec![
        edge("M0", "M1", "删除 m 中的条目"),
        edge("N0", "N1", "删除 n 中的条目"),
    ];
    all.push(s);

    let mut s = scene(
        "22-batch-rename",
        "批量重命名：选中分支换名，未选仍保留旧名",
        "批量重命名",
        "在 a.m、a.n 改成客户报告.docx；a.r 仍叫报告.docx。文件身份 f、F1/F2 正文和所有旧子树保持不变。 ",
    );
    s.lanes = vec![
        (235., "a.m · 已选"),
        (640., "a.n · 已选"),
        (1045., "a.r · 未选"),
    ];
    s.nodes = vec![
        node(
            "M1",
            45.,
            205.,
            380.,
            "M1 · a.m",
            "新增重命名子树",
            "新逻辑名称",
            N,
            vec![
                row("客户报告.docx → f @ F1", C),
                row("文件身份、正文不变", R),
                row("下载名：客户报告.docx", C),
            ],
        ),
        node(
            "M0",
            45.,
            650.,
            380.,
            "M0 · a.m",
            "父子树",
            "历史名称保留",
            R,
            vec![
                row("报告.docx → f @ F1", R),
                row("原文件身份 f", R),
                row("旧子树名称不被覆盖", R),
            ],
        ),
        node(
            "N1",
            450.,
            205.,
            380.,
            "N1 · a.n",
            "新增重命名子树",
            "新逻辑名称",
            N,
            vec![
                row("客户报告.docx → f @ F2", C),
                row("保留自己的具体版本 F2", R),
                row("下载名：客户报告.docx", C),
            ],
        ),
        node(
            "N0",
            450.,
            650.,
            380.,
            "N0 · a.n",
            "父子树",
            "历史名称保留",
            R,
            vec![
                row("报告.docx → f @ F2", R),
                row("不同分支仍可引用不同版", R),
                row("不重建文件分支", R),
            ],
        ),
        node(
            "R0",
            855.,
            650.,
            380.,
            "R0 · a.r",
            "未选分支当前子树",
            "旧名称保持原样",
            R,
            vec![
                row("报告.docx → f @ F1", R),
                row("下载名：报告.docx", R),
                row("通过身份解析实际容器", R),
            ],
        ),
    ];
    s.edges = vec![
        edge("M0", "M1", "只更改 m 的名称"),
        edge("N0", "N1", "只更改 n 的名称"),
    ];
    all.push(s);
    all
}
fn merge_tag_scenes() -> Vec<Scene> {
    let mut reuse = scene(
        "24-file-merge-reuse",
        "文件合并：复用所选分支头",
        "合并复用节点",
        "选择 A、B，采用 B 的当前 F3；结果归属 A，不新增正文或文件子节点。",
    );
    reuse.h = 1320.;
    reuse.nodes = vec![
        head(
            "owner",
            790.,
            175.,
            420.,
            "A · 合并后可写",
            "B 保留历史 → A 的只读别名",
        ),
        node(
            "F3",
            790.,
            400.,
            420.,
            "F3 · 已有节点直接复用",
            "合并结果 · 原 B 分支头",
            "不新建节点",
            C,
            vec![
                row("正文及标签取自 F3", C),
                row("A 后续保存接在这里", B),
                row("合并记录保留 A、B 来源", R),
            ],
        ),
        node(
            "F2",
            55.,
            730.,
            420.,
            "F2 · 原 A 分支头",
            "已有文件节点",
            "仍保留历史",
            R,
            vec![row("原正文和标签不改写", R), row("合并边接入 F3", B)],
        ),
        node(
            "F1",
            430.,
            1030.,
            420.,
            "F1 · 共同父节点",
            "已有正文节点",
            "原有父边不变",
            R,
            vec![row("历史目录子树仍固定旧引用", R)],
        ),
    ];
    reuse.edges = vec![
        edge("F1", "F2", "原有演进"),
        edge("F1", "F3", "原有演进"),
        edge("F2", "F3", "无环合并边"),
        reference("F3", "owner"),
    ];
    reuse.notes = vec![note(
        55.,
        200.,
        640.,
        "目录同步",
        vec![
            "引用 A／B 的当前目录分支各增子树。",
            "引用其他文件分支的条目保持原样。",
            "若加父边会成环，省略该边，来源仍完整保存。",
        ],
    )];
    let mut choose = scene(
        "25-file-merge-chosen",
        "文件合并：选任意历史节点作为结果",
        "合并选历史节点",
        "选择 A、B，但采用该文件其他节点 F0；生成元数据节点 M，复用 F0 的正文与标签。",
    );
    choose.h = 1320.;
    choose.nodes = vec![
        head(
            "owner",
            430.,
            175.,
            420.,
            "B · 用户确认的归属",
            "A 变为只读别名",
        ),
        node(
            "M",
            430.,
            400.,
            420.,
            "M · 新合并子节点",
            "只新增元数据",
            "不新增 FULL",
            N,
            vec![
                row("正文 = F0 的正文", C),
                row("标签 = F0 的标签", C),
                row("来源包含 F1、F2、F0", R),
            ],
        ),
        node(
            "F1",
            55.,
            730.,
            420.,
            "F1 · A 当前节点",
            "参与合并",
            "已有节点",
            R,
            vec![row("仍可从原历史查看", R)],
        ),
        node(
            "F2",
            805.,
            730.,
            420.,
            "F2 · B 当前节点",
            "参与合并",
            "已有节点",
            R,
            vec![row("原正文没有被覆写", R)],
        ),
        node(
            "F0",
            430.,
            1030.,
            420.,
            "F0 · 用户选中的历史",
            "该文件的任一节点",
            "结果正文来源",
            C,
            vec![row("所选节点自身保持原样", C)],
        ),
    ];
    choose.edges = vec![
        edge("F0", "F1", "原有演进"),
        edge("F0", "F2", "原有演进"),
        edge("F1", "M", "输入 A"),
        edge("F2", "M", "输入 B"),
        edge("F0", "M", "所选正文"),
        reference("M", "owner"),
    ];
    choose.notes = vec![note(
        55.,
        185.,
        315.,
        "正文来源单独保存",
        vec!["合并记录：chosen=F0。", "M 也保存 F0 父边。"],
    )];
    let mut tags = scene(
        "26-version-tags",
        "标签修改：继承、子节点与历史编辑",
        "版本标签继承",
        "标签属于具体文件节点；修改标签产生元数据子节点，正文及原节点标签保持不变。",
    );
    tags.h = 1320.;
    tags.nodes = vec![
        head(
            "head",
            55.,
            175.,
            520.,
            "main 当前引用",
            "仍是较新的正文节点 W",
        ),
        node(
            "W",
            55.,
            400.,
            520.,
            "W · 后续正文写入",
            "生成真实内容版本",
            "继承 T 的标签",
            N,
            vec![row("正文：新内容", N), row("标签：[已审阅]", C)],
        ),
        node(
            "T",
            55.,
            730.,
            520.,
            "T · 修改当前节点标签",
            "正文仍引用 V1",
            "只新增标签子节点",
            N,
            vec![row("移除 草稿，增加 已审阅", C), row("新标签：[已审阅]", C)],
        ),
        node(
            "H",
            705.,
            730.,
            520.,
            "H · 给旧 V1 改标签",
            "历史旁子节点",
            "不回退当前引用",
            N,
            vec![row("正文仍引用 V1", R), row("标签：[草稿、归档]", C)],
        ),
        node(
            "V1",
            380.,
            1030.,
            520.,
            "V1 · 原节点",
            "已有正文",
            "标签仍为 [草稿]",
            R,
            vec![row("原标签快照不原地修改", R)],
        ),
    ];
    tags.edges = vec![
        edge("V1", "T", "编辑当时的当前节点"),
        edge("T", "W", "写正文继承标签"),
        edge("V1", "H", "编辑历史标签"),
        reference("W", "head"),
    ];
    tags.notes = vec![note(
        705.,
        200.,
        520.,
        "可批量选择多个分支节点",
        vec![
            "每项单独继承旧标签，再应用相同增删。",
            "先验证所有目标；任何冲突整批失败。",
            "无标签变化不重复生成节点。",
        ],
    )];
    let mut backup = scene(
        "27-backup-prefix",
        "三份保存：每个备份盘独立逻辑前缀",
        "备份路径前缀",
        "前缀由用户指定；主盘移动或改名不会把真实主盘路径写进备份前缀。",
    );
    backup.nodes = vec![
        node(
            "P",
            55.,
            210.,
            1170.,
            "主盘 P01 · 工作/客户甲/项目/图纸/a.svg",
            "实际文件容器",
            "3 份包含这一份",
            R,
            vec![row("策略目录：工作/客户甲/项目；相对路径：图纸/a.svg", R)],
        ),
        node(
            "B",
            55.,
            510.,
            550.,
            "备份 B01",
            "用户前缀：档案/项目",
            "独立配置",
            C,
            vec![
                row("档案/项目/图纸/a.svg", C),
                row("保留同一版本 ID 和正文证明", R),
            ],
        ),
        node(
            "C",
            675.,
            510.,
            550.,
            "备份 C01",
            "用户前缀：离线/项目",
            "独立配置",
            C,
            vec![
                row("离线/项目/图纸/a.svg", C),
                row("不是主盘真实路径的拼接", R),
            ],
        ),
    ];
    backup.notes = vec![note(
        55.,
        800.,
        1170.,
        "绑定持久化",
        vec!["移动主盘文件后仍保持已设置的份数和副本路径。显式修改配置时才重设前缀和路径。"],
    )];
    vec![reuse, choose, tags, backup]
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn text(out: &mut String, x: f64, y: f64, class: &str, value: &str) {
    write!(
        out,
        "<text x='{x}' y='{y}' class='{class}'>{}</text>",
        esc(value)
    )
    .unwrap();
}
fn rename_node_scenes() -> Vec<Scene> {
    let mut s = scene(
        "23-rename-path-nodes",
        "文件夹改名：目录和每个文件都增加子节点",
        "改名与文件路径节点",
        "项目改名为客户项目：目录增加 D1，每个文件增加路径节点；正文仍是 F1、G1，两份正文没有复制。",
    );
    s.lanes = vec![
        (235., "目录分支"),
        (640., "文件 a.txt"),
        (1045., "文件 b.txt"),
    ];
    s.nodes = vec![
        node(
            "D1",
            45.,
            205.,
            380.,
            "D1 · 目录新子树",
            "客户项目/",
            "记录新名称及节点引用",
            N,
            vec![
                row("a.txt → 路径节点 P1", C),
                row("b.txt → 路径节点 Q1", C),
                row("仍是原来的命名目录分支", R),
            ],
        ),
        node(
            "D0",
            45.,
            650.,
            380.,
            "D0 · 原目录子树",
            "项目/",
            "旧名称及历史保留",
            R,
            vec![
                row("a.txt → 正文节点 F1", R),
                row("b.txt → 正文节点 G1", R),
                row("未选分支继续引用 D0", R),
            ],
        ),
        node(
            "P1",
            450.,
            205.,
            380.,
            "P1 · 文件路径节点",
            "客户项目/a.txt",
            "只记录路径变化",
            N,
            vec![
                row("父节点：F1", R),
                row("正文继续引用 F1", C),
                row("可备注、从此节点建分支", R),
            ],
        ),
        node(
            "F1",
            450.,
            650.,
            380.,
            "F1 · 原正文节点",
            "项目/a.txt",
            "第一份真实正文",
            R,
            vec![
                row("文件身份 a 不变", R),
                row("内容和校验摘要不变", R),
                row("没有新增 FULL 文件", R),
            ],
        ),
        node(
            "Q1",
            855.,
            205.,
            380.,
            "Q1 · 文件路径节点",
            "客户项目/b.txt",
            "只记录路径变化",
            N,
            vec![
                row("父节点：G1", R),
                row("正文继续引用 G1", C),
                row("后续写入接在 Q1 之上", R),
            ],
        ),
        node(
            "G1",
            855.,
            650.,
            380.,
            "G1 · 原正文节点",
            "项目/b.txt",
            "第二份真实正文",
            R,
            vec![
                row("文件身份 b 不变", R),
                row("旧路径历史仍然可读", R),
                row("正文总数仍为 2", C),
            ],
        ),
    ];
    s.edges = vec![
        edge("D0", "D1", "改名目录子树"),
        edge("F1", "P1", "新增路径节点"),
        edge("G1", "Q1", "新增路径节点"),
    ];
    vec![s]
}

fn svg(s: &Scene) -> String {
    let mut o = String::new();
    write!(o,"<svg xmlns='http://www.w3.org/2000/svg' width='{}' height='{}' viewBox='0 0 {} {}' role='img' aria-labelledby='{}-title {}-desc' data-scene='{}'><title id='{}-title'>{}</title><desc id='{}-desc'>{}</desc>",s.w,s.h,s.w,s.h,s.id,s.id,s.id,s.id,esc(s.title),s.id,esc(s.summary)).unwrap();
    write!(o,"<defs><marker id='{}-arrow' markerWidth='10' markerHeight='10' refX='7' refY='4' orient='auto' markerUnits='userSpaceOnUse'><path d='M 0 0 L 8 4 L 0 8 Z' fill='#7b8ba2'/></marker><marker id='{}-ref' markerWidth='10' markerHeight='10' refX='7' refY='4' orient='auto' markerUnits='userSpaceOnUse'><path d='M 0 0 L 8 4 L 0 8 Z' fill='#326bc4'/></marker></defs>",s.id,s.id).unwrap();
    o.push_str("<style>text{font-family:'Noto Sans CJK SC','Microsoft YaHei','PingFang SC',sans-serif}.dg-title{font-size:27px;font-weight:700;fill:#18283f}.dg-subtitle{font-size:16px;fill:#586a80}.dg-lane{font-size:18px;font-weight:500;fill:#344a67;text-anchor:middle}.dg-kind{font-size:13px;font-weight:500;fill:#596d85}.dg-node-title{font-size:23px;font-weight:700;fill:#172b46}.dg-row{font-size:18px}.dg-row-small{font-size:16px}.dg-badge{font-size:12px;font-weight:500}.dg-edge-label{font-size:14px;fill:#63758a;text-anchor:middle}.dg-legend{font-size:14px;fill:#566982}.dg-note-title{font-size:18px;font-weight:600;fill:#344a67}.dg-note-row{font-size:15px;fill:#62758b}</style>");
    write!(o,"<rect width='{}' height='{}' rx='22' fill='#f7f9fc'/><path d='M 36 110 H {}' stroke='#e3e9f1' stroke-width='1'/>",s.w,s.h,s.w-36.).unwrap();
    text(&mut o, 40., 49., "dg-title", s.title);
    text(&mut o, 40., 83., "dg-subtitle", s.summary);
    for (x, label) in &s.lanes {
        text(&mut o, *x, 147., "dg-lane", label);
    }
    for e in &s.edges {
        let p = s.nodes.iter().find(|n| n.id == e.from).expect("parent");
        let c = s.nodes.iter().find(|n| n.id == e.to).expect("child");
        assert!(
            p.y > c.y + c.h() + 35.,
            "parent must be below child: {} {}>{}",
            s.id,
            p.id,
            c.id
        );
        let (x1, y1, x2, y2) = (p.cx(), p.y - 2., c.cx(), c.y + c.h() + 7.);
        let d = if (x1 - x2).abs() < 0.1 {
            format!("M {x1} {y1} V {y2}")
        } else {
            let mid = (y1 + y2) / 2.;
            let r = 12.;
            let sign = if x2 > x1 { 1. } else { -1. };
            let (a, b) = if sign > 0. { (1, 0) } else { (0, 1) };
            format!(
                "M {x1} {y1} V {} A {r} {r} 0 0 {a} {} {mid} H {} A {r} {r} 0 0 {b} {x2} {} V {y2}",
                mid + r,
                x1 + sign * r,
                x2 - sign * r,
                mid - r
            )
        };
        write!(o,"<path class='dg-edge' data-parent='{}' data-child='{}' data-reference='{}' d='{d}' fill='none' stroke='{}' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round' vector-effect='non-scaling-stroke' marker-end='url(#{}-{})'/>",p.id,c.id,e.reference,if e.reference {"#326bc4"}else{"#7b8ba2"},s.id,if e.reference{"ref"}else{"arrow"}).unwrap();
        if !e.label.is_empty() {
            let ly = if (x1 - x2).abs() < 0.1 {
                (y1 + y2) / 2.
            } else {
                (y1 + y2) / 2. - 20.
            };
            let lx = (x1 + x2) / 2.;
            let w = e.label.chars().count() as f64 * 14. + 24.;
            write!(
                o,
                "<rect x='{}' y='{}' width='{w}' height='28' rx='10' fill='#f7f9fc'/>",
                lx - w / 2.,
                ly - 19.
            )
            .unwrap();
            text(&mut o, lx, ly, "dg-edge-label", e.label);
        }
    }
    for n in &s.nodes {
        let border = if matches!(n.tone, N) {
            "#8ecdb1"
        } else if n.head {
            "#9ab9ec"
        } else {
            "#d9e2ee"
        };
        write!(o,"<g class='dg-node' data-node='{}' data-x='{}' data-y='{}' data-width='{}' data-height='{}' data-head='{}'><rect x='{}' y='{}' width='{}' height='{}' rx='18' fill='{}' stroke='{border}' stroke-width='1.5'/>",n.id,n.x,n.y,n.w,n.h(),n.head,n.x,n.y,n.w,n.h(),if n.head {B.fill()} else {"white"}).unwrap();
        if !n.head {
            write!(
                o,
                "<rect x='{}' y='{}' width='5' height='{}' rx='2.5' fill='{}'/>",
                n.x + 1.,
                n.y + 24.,
                n.h() - 48.,
                n.tone.color()
            )
            .unwrap();
        }
        text(&mut o, n.x + 24., n.y + 27., "dg-kind", n.kind);
        text(&mut o, n.x + 24., n.y + 65., "dg-node-title", n.title);
        if !n.badge.is_empty() {
            let bw = n
                .badge
                .chars()
                .map(|c| if c.is_ascii() { 7. } else { 12. })
                .sum::<f64>()
                + 20.;
            let by = if n.head { n.y + 75. } else { n.y + 10. };
            let bx = if n.head {
                n.x + 24.
            } else {
                n.x + n.w - bw - 16.
            };
            write!(o,"<rect x='{bx}' y='{by}' width='{bw}' height='23' rx='7' fill='{}'/><text x='{}' y='{}' class='dg-badge' fill='{}'>{}</text>",n.tone.fill(),bx+10.,by+16.,n.tone.color(),esc(n.badge)).unwrap();
        }
        for (i, r) in n.rows.iter().enumerate() {
            let y = n.y + 105. + i as f64 * 34.;
            if matches!(r.tone, N | C | B) {
                write!(
                    o,
                    "<rect x='{}' y='{}' width='{}' height='29' rx='7' fill='{}'/>",
                    n.x + 20.,
                    y - 21.,
                    n.w - 40.,
                    r.tone.fill()
                )
                .unwrap();
            }
            let class = if n.w < 350. { "dg-row-small" } else { "dg-row" };
            write!(
                o,
                "<text x='{}' y='{y}' class='{class}' fill='{}'>{}</text>",
                n.x + 29.,
                r.tone.color(),
                esc(r.text)
            )
            .unwrap();
        }
        o.push_str("</g>");
    }
    for n in &s.notes {
        let h = 67. + n.rows.len() as f64 * 31.;
        write!(
            o,
            "<rect x='{}' y='{}' width='{}' height='{h}' rx='16' fill='#edf1f7'/>",
            n.x, n.y, n.w
        )
        .unwrap();
        text(&mut o, n.x + 20., n.y + 33., "dg-note-title", n.title);
        for (i, r) in n.rows.iter().enumerate() {
            text(
                &mut o,
                n.x + 20.,
                n.y + 68. + i as f64 * 31.,
                "dg-note-row",
                r,
            );
        }
    }
    let y = s.h - 30.;
    for (x, tone, label) in [
        (45., N, "本次新增节点"),
        (240., R, "已有 / 复用节点"),
        (470., C, "明确选中的引用"),
        (700., B, "分支标签 / 默认入口"),
    ] {
        write!(
            o,
            "<circle cx='{x}' cy='{}' r='5' fill='{}'/>",
            y - 5.,
            tone.color()
        )
        .unwrap();
        text(&mut o, x + 14., y, "dg-legend", label);
    }
    write!(o,"<text x='{}' y='{y}' class='dg-legend' text-anchor='end'>子节点在上 · 父节点在下</text></svg>",s.w-35.).unwrap();
    o
}
fn html(scenes: &[Scene], svgs: &[String]) -> String {
    let mut o = String::from(
        "<!doctype html><html lang='zh-CN'><head><meta charset='utf-8'><meta name='viewport' content='width=device-width,initial-scale=1'><title>分支系统 · 图形实例</title><style>*{box-sizing:border-box}body{margin:0;background:#eef2f7;color:#192a42;font:16px/1.6 'Noto Sans CJK SC','Microsoft YaHei','PingFang SC',sans-serif}input.case-radio{position:absolute;width:1px;height:1px;opacity:0}a{color:#315fa9;text-underline-offset:4px}.app{display:grid;grid-template-columns:242px minmax(0,1fr);min-height:100vh}.sidebar{padding:28px 16px;background:#fff;border-right:1px solid #dce4ef}.brand{font-size:21px;font-weight:700;padding:0 10px}.sub{font-size:13px;color:#6a7a90;padding:6px 10px 18px}.cases{display:grid;gap:5px}.cases label{display:flex;align-items:center;gap:11px;cursor:pointer;padding:10px 12px;border-radius:10px;color:#52657f;font-size:14px}.cases label:hover{background:#f1f5fa}.num{font-variant-numeric:tabular-nums;color:#8290a6;font-size:12px}.content{padding:22px;min-width:0}.topbar{display:flex;gap:20px;justify-content:space-between;align-items:center;margin:0 auto 16px;max-width:1440px;font-size:14px}.confirmed{background:#e6f4eb;color:#236747;padding:5px 12px;border-radius:20px}.panel{display:none;max-width:1440px;margin:auto}.canvas{overflow:auto;border:1px solid #dce4ef;border-radius:18px;background:#f7f9fc}.canvas svg{display:block;width:100%;height:auto;min-width:880px}.caption{display:flex;gap:20px;justify-content:space-between;align-items:center;padding:15px 4px;color:#63748b;font-size:14px}.caption p{margin:0;max-width:900px}.caption a{flex-shrink:0}.footer{font-size:12px;color:#7a899d;max-width:1440px;margin:12px auto}input.case-radio:focus-visible~.app .cases{outline:2px solid #82ace7;outline-offset:4px} @media(max-width:900px){.app{grid-template-columns:1fr}.sidebar{padding:16px;border-right:0;border-bottom:1px solid #dce4ef}.cases{grid-template-columns:repeat(4,minmax(0,1fr));max-height:205px;overflow:auto}.cases label{font-size:13px;padding:7px}.brand{font-size:19px}.sub{padding-bottom:9px}.content{padding:14px}.topbar{flex-wrap:wrap;gap:10px}.caption{align-items:flex-start}.canvas svg{min-width:960px}} @media print{.sidebar,.topbar,.caption a,.footer{display:none}.app{display:block}.content{padding:0}.panel{display:block!important;break-after:page}.canvas{border:0}.canvas svg{min-width:0;width:100%}.caption{font-size:10px}} ",
    );
    for (i, s) in scenes.iter().enumerate() {
        write!(o,"#case-{i}:checked~.app .panel[data-case='{i}']{{display:block}}#case-{i}:checked~.app label[for='case-{i}']{{background:#eaf1fc;color:#234f91;font-weight:600}}#case-{i}:checked~.app label[for='case-{i}'] .num{{color:#376db6}}/*{}*/",s.id).unwrap();
    }
    o.push_str("</style></head><body>");
    for i in 0..scenes.len() {
        write!(
            o,
            "<input class='case-radio' type='radio' name='case' id='case-{i}' aria-label='{}' {}>",
            esc(scenes[i].short),
            if scenes[i].id == "24-file-merge-reuse" {
                "checked"
            } else {
                ""
            }
        )
        .unwrap();
    }
    o.push_str("<div class='app'><aside class='sidebar'><div class='brand'>分支系统图解</div><div class='sub'>点击实例切换图示<br>父节点在下，子节点在上</div><nav class='cases' aria-label='选择图形实例'>");
    for (i, s) in scenes
        .iter()
        .enumerate()
        .skip(22)
        .chain(scenes.iter().enumerate().skip(16).take(6))
        .chain(scenes.iter().enumerate().take(16))
    {
        write!(
            o,
            "<label for='case-{i}'><span class='num'>{:02}</span>{}</label>",
            i + 1,
            esc(s.short)
        )
        .unwrap();
    }
    o.push_str("</nav></aside><main class='content'><div class='topbar'><a href='../index.html'>过程演示</a><a href='../playground.html'>自由操作</a><span><a href='../examples.md'>Markdown 图片版</a></span></div>");
    for (i, (s, svg)) in scenes.iter().zip(svgs).enumerate() {
        write!(o,"<article class='panel' data-case='{i}' aria-label='{}'><div class='canvas'>{svg}</div><div class='caption'><p>点击左侧切换实例。</p><a href='{}.svg' target='_blank' rel='noopener'>打开原尺寸 SVG ↗</a></div></article>",esc(s.title),s.id).unwrap();
    }
    write!(
        o,
        "<p class='footer'>分支系统图解 · {} 个实例</p></main></div></body></html>",
        scenes.len()
    )
    .unwrap();
    o
}
fn markdown(scenes: &[Scene]) -> String {
    let mut o = String::from(
        "# 分支系统图形实例\n\n**直接查看：[过程演示](index.html) · [自由操作](playground.html) · [27 个静态图示](reference/index.html)。**\n\n**最新补充：**[24 合并复用](#example-24) · [25 合并选历史节点](#example-25) · [26 标签继承](#example-26) · [27 备份前缀](#example-27) · [详细规则](../../docs/design/file-merge-tags-backup.md)。\n\n**此前补充：**[23 改名与文件路径节点](#example-23)。\n\n**此前新增：**[17 跨目录移交](#example-17) · [18 移交后跟踪](#example-18) · [19 批量移动](#example-19) · [20 批量复制](#example-20) · [21 批量删除](#example-21) · [22 批量重命名](#example-22)。\n\n父节点在下、子节点在上。绿色为本次新增节点，灰色为已有或复用节点，紫色为明确选择，蓝色胶囊为分支引用标签。\n\n规则：[三个已确认边界](../../docs/design/branch-system-logic.md#s21) · [移动与跟踪移交](../../docs/design/branch-system-logic.md#s23) · [批量分支操作](../../docs/design/branch-system-logic.md#s24)。设计图示不代表实现或验收结果。\n\n",
    );
    for (i, s) in scenes.iter().enumerate() {
        write!(o,"<a id=\"example-{}\"></a>\n## {:02}．{}\n\n![{}](reference/{}.png)\n\n{} [查看 SVG 原图](reference/{}.svg)\n\n",i+1,i+1,s.short,s.title,s.id,s.summary.trim(),s.id).unwrap();
    }
    o
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let demo = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = demo.join("reference");
    fs::create_dir_all(&root)?;
    let scenes = scenes();
    let mut svgs = vec![];
    let mut diagram_css = String::new();
    for s in &scenes {
        let mut drawing = svg(s);
        let start = drawing.find("<style>").expect("diagram stylesheet");
        let end = drawing[start..].find("</style>").unwrap() + start;
        diagram_css = drawing[start + 7..end].to_owned();
        drawing.replace_range(start..end + 8, "");
        fs::write(
            root.join(format!("{}.svg", s.id)),
            format!("<?xml-stylesheet type='text/css' href='diagram.css'?>{drawing}"),
        )?;
        svgs.push(drawing);
    }
    fs::write(root.join("diagram.css"), &diagram_css)?;
    let mut viewer = html(&scenes, &svgs);
    let start = viewer.find("<style>").expect("viewer stylesheet");
    let end = viewer[start..].find("</style>").unwrap() + start;
    let viewer_css = format!("{}\n{diagram_css}", &viewer[start + 7..end]);
    viewer.replace_range(
        start..end + 8,
        "<link rel='stylesheet' href='./viewer.css'>",
    );
    fs::write(root.join("viewer.css"), viewer_css)?;
    fs::write(root.join("index.html"), viewer)?;
    fs::write(demo.join("index.html"), entry::html(false, "./dist/"))?;
    fs::write(demo.join("playground.html"), entry::html(true, "./dist/"))?;
    fs::write(demo.join("examples.md"), markdown(&scenes))?;
    println!(
        "Generated {} SVG diagrams, standalone HTML viewer, and Markdown image index",
        scenes.len()
    );
    Ok(())
}
