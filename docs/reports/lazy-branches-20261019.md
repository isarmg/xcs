# 分支引用、目录子树与首次内容写入

实现讨论记录：2026-10-19。[文档目录](../README.md) · [当前分支规则](../06-history-and-branches.md#s06-02) · [完整逻辑](../design/branch-system-logic.md) · [实际图形实例](../../demos/branch-system/examples.md)

本文保留实现过程中的字段、缓存及恢复讨论，不作为最新语义已经实现的证明。创建目录分支直接引用父子树、首次写入才为文件建立公共分支，以及跨目录跟踪移交和批量作用范围，以[最新逻辑](../design/branch-system-logic.md#s23)为准；实现与规范需分别验收。

## 用户可见行为

创建文件或目录分支只发布元数据，直接引用创建时冻结的父文件版本，不复制正文，不新建空占位文件，不增加内容版本。切换后读取冻结的真实版本；此后父文件分支推进 HEAD，不会悄悄改变已保存的目录子树。

文件分支绑定一个稳定 `file_id`。目录分支是一套共享文件分支的组合，具有稳定的 `directory_branch_id` 和名称，不为目录里的文件另建一套私有分支。只有显式“创建目录分支”才增加命名分支；修改文件或切换子项只生成新的不可变目录子树并推进当前分支的 `head_tree_id`，不会自动增加一批命名分支。

| 操作 | 结果 |
| --- | --- |
| 创建文件分支 | 登记同文件的冻结父版本，尚未拥有自己的正文 |
| 创建目录分支 | 登记名称并直接引用父子树，不提前创建自有子树或后代文件分支，不新建空文件 |
| 首次上传或实际写入 | 为真正写入的文件生成独立 FULL，其余文件继续引用原版本 |
| 首写为空或与父相同字节 | 仍生成独立版本身份和校验，不能按未变 no-op 跳过 |
| 首次范围写入 | 以可见父版本为基础保留未修改字节，超出 EOF 的空洞补零 |
| 修改目录内一个文件 | 仍继承时首次写入才建公共文件分支；已明确选择的已有分支则继续推进，必要目录生成子树 |
| 切换目录内某个文件或子目录 | 原子更新该范围的组合，并为所在目录分支保存新子树；目录外文件不变 |
| 选择根目录默认分支 | 切换默认可见组合；普通有写权限用户可操作，SMB 经稳定共享命名空间读取此默认视图 |
| 历史另存 | 只为工作区实际 staged 的文件生成 FULL，其余引用父版本，不隐式复制 |
| 跨目录移动 | 源、目标所选分支各生成子树并移交跟踪，实体迁到目标目录，未选分支仍可读原节点 |
| 批量移动／复制／删除／重命名 | 明确作用分支及配对，统一发布；其他分支的当前子树、名称和版本不变 |

尚未拥有正文的文件在父版本可读时正常预览、下载和 SMB 读取；只读打开或无脏写关闭不产生内容版本。真实写入版本通过 `source_version_ids` 接入文件谱系；目录子树只描述组合，不冒充文件内容节点。

## 命名分支与不可变子树

`DirectoryBranch` 保存稳定身份、名称和当前 `head_tree_id`。`DirectoryTree` 保存一次不可变的组合：所属目录分支、`parent_tree_id`、对应的真实提交、冻结条目及共享文件分支映射。分支的 `entries`、`commit_id`、`file_branches`、`path`、`sequence` 是当前 head 的投影，必须与该节点一致；历史展示从树节点读取，不能把可变 head 投影当作历史快照。

修改所选目录分支后，新树通过 `parent_tree_id` 连接旧树，分支名称不变；新节点在上、旧节点在下。显式创建新分支才新增一个稳定命名对象，初始直接引用父分支已有子树，首次修改才产生自有子树，后续修改继续向同一个命名分支追加子树。

根目录稳定身份为仓库 `repo_` ID；子目录为目录条目的 `entry_` ID。目录或文件改名保留稳定身份，旧树保留旧路径；最新规则要求选择旧子树恢复其完整组成及名称，不再采用仅映射当前条目的中间方案。移动后的历史逻辑路径通过稳定文件和版本身份解析实际位置，不能因实体迁到 b 就破坏未选分支的读取。

目录选择代际保存在 `directory_selection_generations`，值采用 u64 十进制字符串。显式切换目录才推进代际；普通文件修改及其生成子树不推进选择代际。工作区捕获打开时的文件选择和目录选择代际，目的地已切换时返回 `FILE_BRANCH_CONFLICT` 或 `DIRECTORY_BRANCH_CONFLICT`（HTTP 409），保留待提交字节；无关文件的正常修改不会仅因自动生成子树而错误失效。

## 来源保留与失败

活动文件分支、有效工作区及读租约固定所需正文。选中的目录分支当前 head，以及尚未完成登记的目录分支当前 head，同样固定它们引用的真实版本。已完成且未选中的旧组合、旧子树和普通历史提交不永久固定全部旧正文；按清理权限删除正文后仍保留树和版本元数据，回看如实显示 `PURGED`。

纯创建只要求来源元数据可信、同文件同授权历史，不读取正文。来源此前已清除仍可登记；下载、预览或依赖父字节的范围编辑返回实际内容错误，不伪造空内容，也不自动换成另一个版本。完整上传可直接建立新的独立 FULL，保留旧墓碑。首次写入独立内容后，来源在没有其他活动依赖时可按规则清理。

## 权威记录与物理布局

`Branch.pending_file_versions` 记录当前尚未拥有正文的文件及冻结父版本，`Commit.pending_file_versions` 保留每个历史快照当时的继承归属。持久及泛型 Entry 的文件 `version_id`、大小一直指向可见真实版本。`State.directory_branches`、`directory_trees`、`directory_selections` 和选择代际随快照及历史清单一起受发布证明保护。

```text
histories/<history-id>/file-containers/<file-id>/<当前文件名>/
├── container.json
└── versions/
    └── v000002_<UTC时间>_<摘要前8位>.<扩展名>
```

不再新建 `pending-branches/*.pending`。已有现代容器的 `container.json` 用 `inherited_branches` 表达来源版本、自有版本为空及继承状态；它只是可重建缓存，不能替代发布记录。旧格式来源没有现代容器时，也不为引用新建空容器，不移动旧正文。兼容清理仅删除旧保留目录中合法零字节、单链接普通 stub 及其空目录；未知项、非空文件、符号链接和硬链接均保留并报错。

历史包携带真实内容、文件继承映射、目录分支、完整子树闭包、选择及代际，恢复后保持冻结组合。校验必须拒绝版本/文件/共享分支跨域、head 投影不同、悬空父边和循环。当前旧历史拆分接口对包含独立文件分支或目录组合的历史明确拒绝，不能漏掉这些对象再声称拆分成功；兼容整树历史仍按原独立恢复规则拆分。

## 文件内容接口与兼容

尚未写入的文件投影：

```json
{
  "version_id": "ver_00000000000000000000000000000001",
  "size": "12",
  "materialized": false,
  "inherited": true,
  "own_version_id": null,
  "source_version_id": "ver_00000000000000000000000000000001",
  "content_state": "PENDING",
  "visible_content_state": "AVAILABLE"
}
```

`PENDING` 只表示文件尚未拥有正文，实际读取状态看 `visible_content_state`。写后 `materialized:true`、`inherited:false`、`own_version_id=version_id`、`source_version_id:null`，两种内容状态相同；真实零字节 FULL 仍有自己的版本 ID。泛型 Entry 保持既有字段，目录没有文件正文归属承诺。

能力声明使用 `branch_creation:"record-until-write"`、`branch_content:"inherit-parent-until-first-write"`、`directory_branch_creation:"compose-shared-file-branches"`、`directory_branches:true`、`smb_default_branch:"selected-root-directory"`。新创建回执的 `copied_files`、`placeholder_files` 均为 `"0"`；目录回执返回稳定目录分支 ID，不把共享文件分支描述为此次新建私有分支。

Branch/Commit 的空继承映射，以及 State/历史清单的空目录映射，都按默认空值并省略序列化保持旧规范字节。旧 FULL 内容、已发布回执和证明不迁移、不改写。App 使用 HTTPS 业务 API；SMB 经独立的 `Samba → 服务端桥接 → 核心` 访问所绑定的稳定主命名空间，读取其选中根目录默认组合。显式默认组合选择与 SMB 共享绑定是两件操作，不把 App 描述为通过 SMB 访问。

## 验证与历史记录

业务实现证据按[测试材料索引](../../tests/README.md)分别核对；本次实际图形检查见[图示结果](../../demos/branch-system/tests/history/branch-system-visuals/result.json)。图示检查仅证明文档图片及页面展示，不能代替存储、业务 API、SMB 实际读写或物理断电验收。

此前[继承加空文件方案](../../tests/evidence/inherited-branches/README.md)和[不允许读取的占位方案](../../tests/evidence/lazy-branches/README.md)均为被后续澄清替代的历史材料，保留原日志。旧自动增加命名目录分支的中间结果也不作为当前“修改只生成子树”的通过证据。全量验收仍按[第 22 章](../22-acceptance.md#s22-03)记录，不把构建或临时 fixture 通过解释为六平台实机和发行完成。
