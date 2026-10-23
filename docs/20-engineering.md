# 20 工程结构与依赖约束

[文档目录](README.md) · [上一章：容量、配额与性能预算](19-capacity-and-performance.md) · [下一章：配置规范与默认值](21-configuration.md)

<a id="s20"></a>

本章定义已经落地的目录、代码命名、依赖与构建体系。完整功能目标以各领域规范为准，实际验证范围见[实现报告](reports/implementation.md)。

## 本章目录

- [20.1 Workspace 与目录](#s20-01)
- [20.2 命名与依赖规则](#s20-02)
- [20.3 离线工具](#s20-03)
- [20.4 构建与工程验证](#s20-04)

<a id="s20-01"></a>

## 20.1 Workspace 与目录

```text
zxs/
├── Cargo.toml / Cargo.lock       # 唯一活动工作区与统一锁定解析
├── rust-toolchain.toml
├── src/
│   ├── auth/                    # 可移植认证算法
│   ├── contract/                # 业务契约、ID、严格 JSON 和校验
│   ├── naming/                  # 命名规则、固定映射和合并后的 Unicode 规范化
│   ├── platform/                # Linux 文件安全、SQLite 和凭据基础设施
│   ├── engine/                  # 存储、历史、授权、发布及恢复
│   ├── server/                  # zxs 服务入口、HTTP、认证、流式内容和本机 SMB RPC
│   ├── client/                  # 运维 HTTP 会话、传输及完整性验证
│   ├── rescue/                  # zxs-rescue 本机维护与离线恢复入口
│   ├── web/                     # 浏览器 WASM 产品界面、声明式样式与字体
│   ├── interface/               # 生命周期、主题、组件与合并后的 CSR 原语
│   ├── macros/                  # 界面过程宏编译目标
│   ├── smb/                     # Samba 协议桥接和 SCM_RIGHTS
│   ├── smb-vfs/                 # Samba SDK 对接和 Rust C ABI
│   └── foundation/              # 同仓维护的 Rust 基础实现
├── tools/                       # 根工作区中的构建、来源和验收组件
├── docs/                        # 所有活动文档和原始上游说明
│   ├── components/              # 自有组件说明
│   ├── tools/                   # 工具说明
│   ├── validation/              # 验收说明与证据索引
│   └── upstream/                # 原始上游文档，逐文件哈希保留
├── compliance/                  # 所有许可、版权、来源和迁移收据
│   ├── licenses/                # 第一方条款、逐来源上游原文和补充通知
│   ├── provenance/              # 原始身份、清单、哈希和目录合并映射
│   ├── NOTICE.md                # 统一第三方通知索引
│   └── rust-sources.json        # 活动源码来源与完整性清单
├── deploy/                      # 部署配置及安装脚本
├── test/                        # 按组件组织的单元/集成测试、夹具与证据检查
├── build/web/                   # 生成的正式 Web 资源
├── target/                      # 本机编译缓存与局部 SDK
└── runtime/                     # 本机运行池、预览和回滚材料
```

完整迁移和集中维护约定见[单仓库结构](monorepo.md)。自有应用和工具属于根工作区；基础实现作为本地路径依赖接入，只有根 `Cargo.lock` 参与解析。`src/`、`tools/` 和 `test/` 中没有独立子工作区、活动子锁文件或嵌套 Git 仓库。迁移前的锁文件、发布清单和上游 CI 元数据仅在 `compliance/provenance/` 中作为溯源原文保存。

构建按包和目标选择：默认成员为 Linux 服务及运维工具，Web 使用 `wasm32-unknown-unknown`，VFS 显式使用 Samba SDK。所有路径依赖均位于本仓库，禁止依赖其他项目绝对路径。

### 源码与本机生成物

`.cargo/config.toml` 默认输出到根 `target/native/`，Web 与工具使用根 `target/` 下各自缓存。正式 Web 资源由统一构建器写入 `build/web/`。各类缓存、运行池和生成证据按根 `.gitignore` 的限定路径排除。

| 纳入版本管理 | 忽略的本机内容 |
| --- | --- |
| 根工作区清单、根 `Cargo.lock` 和全部本地 Rust 源码 | `target/`、`build/` 和构建临时目录 |
| `src/web`、`src/interface`、字体、schema 和固定向量 | 根 `runtime/`、`zxs-data/`、本地缓存及待提交数据 |
| `docs` 中的全部说明、固定图示和验收索引 | 验收结果 JSON、截图、日志、收据和浏览器报告 |
| `compliance` 中的许可原文、来源身份和历史原文 | 凭据、私人环境覆盖和个人工具状态 |

验收说明统一在 `docs/validation/`，引用的原始检查代码与本地产生的证据位于 `test/evidence/`。固定输入样例与所属测试一起放在根 `test/` 的组件子目录；不能按文件后缀全局忽略依赖源材料和固定样例。

文档链接检查区分活动规范与原始上游归档。活动规范、schema、固定样例和检查脚本的链接必须存在，路径越界和符号链接越界也必须拒绝。文档允许引用按规则忽略的本地生成验证结果：存在时核对目标，缺失时单独统计，不把源码副本中缺失的生成证据误记为业务验收通过。`docs/upstream/` 保持原始说明字节，由来源映射的哈希校验，不把其中历史链接当作当前手册链接。

认证算法在独立可移植 `authentication` 中只有一个实现；服务端与救援工具直接依赖 `authentication`，服务端与平台均不保留旧认证路径兼容导出，业务会话持久化仍属于服务端。

核心按责任拆分：business/repository 管仓库与共享绑定，workspace 管完整上传和提交，history 管文件历史和关系，namespace 管路径，file 管共享文件分支与备注，directory 管稳定目录分支、不可变子树、选择及默认视图。container_storage 管普通正文唯一路径、真实容器改名、继承缓存与旧 stub 清理；边界见其[README](components/engine/src/container_storage.md)。engine/{publication,recovery,reads,index,workspace_storage} 管发布、恢复、读取、索引和工作区；journal 管状态分页及证明，metadata_budget 管决定前限额，maintenance 管计划/清理/兼容拆分，history_package 管完整恢复闭包，repair/degraded_recovery 区分可修复与永久损失。入口复用核心规则，不直接操作业务盘。

文件节点的统一解析与继承标签由 `business/file_nodes` 管理；`file_tags` 新增标签子节点，`file_merges` 保存采用结果和归属，`file_publication` 按精确引用统一推进目录组成。旧 `history` 中的 `MergeLink` 仍只记关系线；新 `FileBranchMerge` 会推进归属分支与参与目录引用，两者不是同一操作。节点正文继续通过真实 `version_id` 读取，语义边界见[合并、标签与备份规则](design/file-merge-tags-backup.md)。

服务端 `file_manager.rs` 定义以磁盘、相对路径和稳定文件身份为入口的路由，`engine_backend/file_manager.rs` 将其映射到核心授权、命名空间和文件分支操作，不把磁盘当作分支。`telemetry/mod.rs` 保留最小 CPU/内存采样对象，磁盘容量由核心针对已登记盘根读取真实文件系统统计；接口只提供管理员基础状态，见 [采样边界](components/server/src/telemetry.md)。Web 的 `monitor.rs` 只展示这些真实样本，`history.rs` 展示已提交文件操作；两者不混用监测数据与文件历史。

`engine_backend/file_manager/backup_namespace.rs` 单独投影备份盘的用户逻辑路径，先过滤历史域授权，再检查路径冲突并固定该槽可用版本。它不伪造主路径上下文或提供备份写入入口；磁盘可见性按实际版本所属历史与路由判断，不能只按仓库默认一级盘推断。

Web 的 `files/{model,view,detail,inline,branch,version_tree}.rs` 分别维护浏览状态、列表、详情、原位输入及版本树；`files/actions/{browse,mutation,operation}` 分开处理读取、事务和回执。公共 `geometry.css` 的 stack/flow/row/frame/input 角色统一尺寸、内缩、边框和 G4；stack/flow 默认不增加外部间距，仅在相邻可见表面直接接触时用 surface 边界角色补距，已有文字或标签行隔开时不重复添加。业务样式只定义领域布局。详情通过顶栏菜单访问，返回文件列表时保留目录、搜索条件与选择；列表和详情共用原位备注输入，不提供正文预览。分支名称在原按钮位置输入，Enter 提交，空白、失焦或 Escape 取消，不再使用分支弹窗；前后端复用 `name-policy` 验证名称。Web 不含上传动作或正文传输任务页；保留的 HTTP 内容接口与客户端库不构成 Web 功能。公共 `Select` 管理键盘、焦点及原设计弹层；公共与业务文案均通过统一语言接口选择，CSS、令牌和图标仍为声明式资源。

<a id="s20-02"></a>

## 20.2 命名与依赖规则

### 项目命名

| 对象 | 规则 | 示例 |
| --- | --- | --- |
| 项目显示名 | `ZXS` | `ZXS 文件存储` |
| Cargo 包名 | 按职责命名，kebab-case，不加项目名前缀 | `storage-contract`、`web-interface` |
| 第一方组件目录 | 按职责组织，不重复项目前缀 | `src/contract`、`src/interface` |
| Rust crate 路径 | 对应职责包名的 snake_case | `storage_engine::Engine` |
| Rust 文件、模块、方法、字段 | snake_case，明确业务词 | `request_digest`、`commit_workspace` |
| Rust 类型与 trait | UpperCamelCase，在模块中表达职责 | `Workspace`、`ReadLease`、`CommitRequest` |
| 常量与外部错误码 | SCREAMING_SNAKE_CASE | `MAX_FRAME`、`CONTENT_PURGED` |
| 环境变量 | `ZXS_` 加职责 | `ZXS_WEB_TARGET_DIR` |
| DOM 标记、应用缓存键 | `data-zxs-` / `zxs-` | `data-zxs-runtime`、`data-zxs-theme`、`zxs-language` |
| 工具二进制 | 按职责命名；产品入口保留产品名称 | `web-build`、`contract-check`、`zxs` |
| 持久化 ID | 语义领域前缀加 32 位小写十六进制 | `ver_…`、`commit_…`、`op_…` |
| 哈希用途域 | `zxs:` 加用途 | `zxs:request\0` |

源码目录、包和模块按职责命名，不机械添加项目名前缀；第三方来源标识、产品入口和公开协议标记遵守各自约定。业务字段和持久化 ID 遵守公开契约，不因展示名称变更而更换语义。持久化记录和分段清单使用固定语义类型标识，不嵌入项目发布版本号；未支持的布局直接拒绝，不保留旧格式迁移分支。

复制并维护的框架实现属于本项目源码，遵守上述命名和独立目录规范。外部 Cargo 包、第三方仓库、SDK 函数、系统 ABI 符号和协议字段保留真实名称；不得通过文本替换破坏链接或伪称另一个依赖。第三方来源、版权和许可通知集中在 `compliance/`，原始说明集中在 `docs/upstream/`；来源、包身份和源码版权头保留真实上游名称。

### 依赖方向

```text
Web 界面 → web-interface（共享组件、生命周期和直接合入的 CSR 原语）
界面编译 → view-macros（过程宏）
服务端认证 / 平台凭据 → authentication（无 Linux 或数据库依赖）
Web → HTTPS 业务 API
本机 zxs → storage-server 服务启动
Web 管理界面 → HTTPS 管理 API（初始化、账户、磁盘和维护）
本机 zxs-rescue → 未初始化配置草稿 / 系统与历史恢复 / 离线检查与导出
保留的 storage-client 库 → HTTPS 业务 API + storage-contract（不再由 zxs 暴露）
HTTP / 本机 SMB RPC → storage-engine
Samba VFS → smb-protocol → 受保护 Unix socket
storage-engine → storage-contract + name-policy + platform-io
platform-io → 文件系统 / SQLite / 凭据算法依赖
```

`storage-contract` 不依赖操作系统或 UI；`name-policy` 将 Unicode 规范化算法与项目映射放在同一编译组件，使用固定 Unicode 数据，不跟随编译器 Unicode 版本漂移。SQLite 为可删除后重建的索引，全部配置成员日志和不可变对象承担恢复依据。复制必须显式读写完整字节，不使用硬链接、reflink 或去重冒充独立副本。

真实文件容器的目录名、文件备注和版本备注由 State 恢复，`container.json` 只是派生缓存；不可变版本描述、内容散列和发布证明留在独立证明目录。`time` 负责 UTC 时间格式，`rustix` 负责已有安全文件系统原语。旧载荷继续按缺省可选字段的旧路径读取，本次布局扩展不等于完成全盘格式迁移；详见[第 8.2 节](08-full-storage-and-disk-format.md#s08-02)。

自研业务、界面、工具和桥接逻辑采用 Rust。HTML、CSS、TOML、JSON、Unicode 表和其他声明式资源按其用途维护；WASM 生成的 JavaScript 绑定及官方平台依赖可以包含其他语言。自写业务不得迁入这些外部桥接文件。

API 与 SMB 只映射认证主体、输入、句柄和传输错误。最终授权、文件占用、命名、备份路由和发布判定均在核心执行。普通应用不直接访问底层数据盘。

<a id="s20-03"></a>

## 20.3 离线工具

`zxs-rescue offline-check <disk>` 验证本盘版本、内容清单、发布回执和带状态页包含证明的内容墓碑；不修改盘数据。`zxs-rescue offline-export <disk> <version> <output>` 独立验证后输出完整文件，拒绝覆盖已有目标。导出不修改目标父目录权限，失败不把未完成复制发布成完整文件。

`zxs-rescue --root <new-root> restore-system --disks <disk1> …` 从固定配置的至少 Q = floor(N/2)+1 个一致、可信的原元数据成员配置恢复系统目录，随后重放日志、恢复凭据灾备快照及可发现的待提交工作区。该快照不构成管理面认证权威；守护进程启动和后台重试不会自动安装管理凭据。管理凭据恢复必须在另行准备的干净系统状态目录中，通过受当前本机 bootstrap 授权保护的 Web 恢复流程明确确认旧凭据风险；保留已恢复或部分恢复目录作为核验依据，不删除或覆盖现有权威。它不会重新初始化数据盘，也不绕过设备身份核验。N ≥ 1，单成员池须核验唯一原成员；多成员池不能按幸存数降低 Q。正常写入仍须全部配置成员及发布闭包，不因一份 PUBLICATION 就放行。所有新初始化盘均保存配置锚，缺锚旧盘不兼容且不自动迁移。

离线工具不依赖在线服务或原服务器 SQLite。实际只恢复本盘持有且证明、字节均有效的版本，不推断全局最新状态。格式、设备或日志冲突必须明确失败；检查命令不得隐式修复或清理。

<a id="s20-04"></a>

## 20.4 构建与工程验证

Rust 版本由根 `rust-toolchain.toml` 指定。所有组件、目标和工具共同使用根 `Cargo.lock`；在仓库根通过 `cargo ... -p <包名>` 选择编译目标。正式构建使用本仓库 Rust 源码与 `--locked --offline`，不在构建时下载 Rust 依赖；覆盖范围见[统一来源清单](THIRD_PARTY_RUST_SOURCES.md)，许可与来源规则见[集中合规说明](../compliance/README.md)。Linux 原生链接依赖和 Samba SDK 保持原有管理方式。

单元与集成测试按组件集中在根 `test/`，模块单元测试通过显式路径接入实现；跨入口浏览器、HTTP、CLI、故障及真实 SDK 证据记录在 `test/` 与[实现报告](reports/implementation.md)。构建、模拟文件故障、真实设备断电、浏览器、SMB 互操作和发行产物是不同层级证据，禁止相互代替。

契约变更同时更新类型、schema、固定样例、Rust 校验器和入口映射；只保留一份规则实现。禁止未经证据将第 22 章全部用例一次标记为通过。

---

[文档目录](README.md) · [上一章：容量、配额与性能预算](19-capacity-and-performance.md) · [下一章：配置规范与默认值](21-configuration.md)
