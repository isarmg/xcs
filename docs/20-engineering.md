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
├── Cargo.toml                    # Linux 服务、共享契约与 CLI 工作区
├── Cargo.lock
├── rust-toolchain.toml
├── crates/
│   ├── zxs-contract/             # 领域类型、严格 JSON、JCS、ID 与校验清单
│   ├── zxs-naming/               # 固定 Unicode 16.0 数据和跨平台路径规则
│   ├── zxs-auth/                 # 可移植密码、令牌、CSRF 与准入算法
│   ├── zxs-platform/             # Linux 文件安全、SQLite、凭据基础设施
│   ├── zxs-core/                 # 存储、历史、授权、日志与恢复
│   ├── zxs-server/               # HTTP、认证、流式内容、本机 SMB RPC
│   ├── zxs-client/               # 原生客户端会话、传输和完整性验证
│   └── zxs-cli/                  # zxs 可执行入口
├── web/                         # 独立 WASM 工作区
│   ├── src/                     # zxs-web-app 产品界面
│   │   ├── files/               # 文件模型、动作与文件管理器视图
│   │   ├── ui.rs                # 会话、页面导航与公共壳接入
│   │   ├── api.rs               # HTTP、原生桥接、上传封存与校验读取
│   │   ├── monitor.rs           # CPU、内存和磁盘状态卡
│   │   ├── history.rs           # 已提交文件操作历史
│   │   ├── format.rs            # 容量、时间和响应字段格式化
│   │   └── icons.rs             # 统一文件操作 SVG 图标
│   ├── runtime/                 # zxs-web 统一框架入口
│   │   └── src/
│   │       ├── components/      # 外壳、登录、账户、Select、对话框和通知
│   │       ├── contours/        # 连续轮廓几何、绘制与生命周期
│   │       ├── lib.rs           # 字体解码、挂载和退出清理
│   │       ├── startup.rs       # 启动终态与事件
│   │       ├── theme.rs         # 主题状态与系统偏好
│   │       └── i18n.rs          # 语言准入和公共文案
│   ├── public/
│   │   ├── styles/tokens/       # 明暗令牌、reset 与无障碍
│   │   ├── styles/components/   # 共享组件外观
│   │   ├── styles/startup.css   # WASM 加载前的失败视图
│   │   └── fonts/              # 项目字体资源
│   ├── styles.css              # 仅导入、字体与基础排版；几何在 components/geometry.css
│   └── dist/                    # 生成资源
├── apps/
│   ├── bridge/                  # zxs-app-bridge 原生业务与系统交接
│   └── host/                    # zxs-app 跨平台宿主、能力清单和配置
├── integrations/
│   └── smb/                     # 独立 SMB SDK 边界
│       ├── src/                 # zxs-smb RPC、SCM_RIGHTS 协议
│       └── vfs/                 # zxs-smb-vfs Rust C ABI 实现
├── vendor/
│   ├── web/
│   │   ├── zxs-web-core/        # 本项目维护的 CSR 引擎
│   │   └── zxs-web-macro/       # 本项目维护的界面过程宏
│   └── dependencies/            # 保持上游包名的外部依赖快照
├── tools/
│   ├── contract-check/          # zxs-contract-check 契约校验
│   ├── web-build/               # zxs-web-build WASM、资源与图标构建
│   ├── unicode-build/           # zxs-unicode-build 固定 Unicode 表重现
│   ├── browser-smoke/           # Rust WebDriver 浏览器烟测
│   └── release-tools/           # 发布版本校验
├── demos/branch-system/          # 演示内容、代码、图示与检查材料
│   ├── src/                    # 无存储 I/O 的模型、视图、场景及图形
│   │   └── bin/                # build / serve / reference 三个 Cargo 工具入口
│   ├── reference/              # SVG / PNG 与无脚本查看页
│   ├── tests/                  # 浏览器检查、收据及历史材料
│   └── dist/                   # 生成的独立演示发行包
├── tests/                       # 跨模块验证说明与脱敏证据
├── docs/
│   ├── README.md               # 文档导航
│   ├── getting-started.md      # 本地运行
│   ├── 21-ci.md                # 持续集成
│   ├── release.md              # 发布与安装
│   ├── 01-…25-*.md             # 编号规范手册
│   ├── design/                 # 分支、合并、标签和备份等专题规则
│   ├── reports/                # 实现、重构和讨论报告
│   ├── templates/              # 验收报告模板
│   └── contracts/              # 唯一机器可读公开契约与固定样例
├── target/                      # 本机编译缓存与局部 SDK
├── runtime/                     # 本机运行池、预览和回滚材料，不纳入 Git
└── zxs-data/                    # 本机客户端待提交数据，不纳入 Git
```

工作区按工具链隔离：服务端测试不要求 WebKit、移动 SDK 或 Samba SDK；Web 不引入 Linux 数据盘 I/O；原生宿主单独处理平台链接。每个工作区维护对应锁文件。所有路径依赖必须位于本项目内，禁止依赖其他项目绝对路径。

### 源码与本机生成物

`.cargo/config.toml` 默认输出到根 `target/native/`，Web、App 与工具可使用根 `target/` 下各自缓存。`.gitignore` 用 `target/` 同时覆盖独立工作区的默认缓存，新增 crate 无须逐个补规则。演示三个可执行工具统一放在 `src/bin/`，通过 Cargo 名称运行，不依赖调用时的当前目录。

| 纳入版本管理 | 忽略的本机内容 |
| --- | --- |
| Cargo 源码、配置和各工作区 `Cargo.lock` | 各层 `target/`、Web/演示发行包与构建临时目录 |
| `web/runtime/`、框架快照、字体与 App 图标 | 根 `runtime/`、`zxs-data/`、本地缓存及待提交数据 |
| schema、固定向量、参考图、测试代码与验收说明 | 测试结果 JSON、截图、日志、收据、trace、浏览器报告与覆盖率缓存 |
| `.env.example` 等配置示例、共享编辑器配置 | 本地 `.env` 覆盖、Cargo 凭据和个人工具状态 |

根 `runtime/` 与 `web/runtime/` 含义不同，不能用无作用域的 `runtime/` 规则。`tests/evidence/`、演示的 `tests/evidence/` 和 `tests/history/` 只纳入 Markdown 说明及 Rust/MJS/Python/Shell 检查代码，其他文件默认为本地生成物；必要的固定输入样例放在 `tests/fixtures/`，文档固定图示仍放在演示 `reference/`。不按 `.png`、`.wasm`、数据库后缀或压缩包后缀全局忽略，避免漏掉这些样例和依赖源材料。增加本地目录时应按用途限定路径。

文档可链接本地验证结果；源码副本中未必包含这些文件。契约校验会单独统计声明验证目录中的生成物链接及缺失数量，存在时检查目标，不将缺失的本地生成物当作源码断链。规范、schema、固定样例、Markdown 说明及检查脚本的链接仍必须存在，路径越界和符号链接越界也仍拒绝。若要审阅具体执行证据，应运行对应检查或从独立归档取得材料，不能只凭源码副本的文档检查认定验收已完成。

认证算法在独立可移植 `zxs-auth` 中只有一个实现；服务端 `auth_primitives` 与平台 `admin_auth` 保留原公开路径的兼容导出，业务会话持久化仍属于服务端。

核心按责任拆分：business/repository 管仓库与共享绑定，workspace 管完整上传和提交，history 管文件历史和关系，namespace 管路径，file 管共享文件分支与备注，directory 管稳定目录分支、不可变子树、选择及默认视图。container_storage 管普通正文唯一路径、真实容器改名、继承缓存与旧 stub 清理；边界见其[README](../crates/zxs-core/src/container_storage/README.md)。engine/{publication,recovery,reads,index,workspace_storage} 管发布、恢复、读取、索引和工作区；journal 管状态分页及证明，metadata_budget 管决定前限额，maintenance 管计划/清理/兼容拆分，history_package 管完整恢复闭包，repair/degraded_recovery 区分可修复与永久损失。入口复用核心规则，不直接操作业务盘。

文件节点的统一解析与继承标签由 `business/file_nodes` 管理；`file_tags` 新增标签子节点，`file_merges` 保存采用结果和归属，`file_publication` 按精确引用统一推进目录组成。旧 `history` 中的 `MergeLink` 仍只记关系线；新 `FileBranchMerge` 会推进归属分支与参与目录引用，两者不是同一操作。节点正文继续通过真实 `version_id` 读取，语义边界见[合并、标签与备份规则](design/file-merge-tags-backup.md)。

服务端 `file_manager.rs` 定义以磁盘、相对路径和稳定文件身份为入口的路由，`engine_backend/file_manager.rs` 将其映射到核心授权、命名空间和文件分支操作，不把磁盘当作分支。`telemetry/mod.rs` 保留最小 CPU/内存采样对象，磁盘容量由核心针对已登记盘根读取真实文件系统统计；接口只提供管理员基础状态，见 [采样边界](../crates/zxs-server/src/telemetry/README.md)。Web 的 `monitor.rs` 只展示这些真实样本，`history.rs` 展示已提交文件操作；两者不混用监测数据与文件历史。

`engine_backend/file_manager/backup_namespace.rs` 单独投影备份盘的用户逻辑路径，先过滤历史域授权，再检查路径冲突并固定该槽可用版本。它不伪造主路径上下文或提供备份写入入口；磁盘可见性按实际版本所属历史与路由判断，不能只按仓库默认一级盘推断。

Web 的 `files/{model,view,detail,inline,branch,version_tree}.rs` 分别维护浏览状态、列表、详情、原位输入及版本树；`files/actions/{browse,mutation,operation,transfer}` 分开处理读取、事务、回执和传输。公共 `geometry.css` 的 stack/flow/row/frame/input 角色统一尺寸、内缩、边框和 G4；stack/flow 默认不增加外部间距，仅在相邻可见表面直接接触时用 surface 边界角色补距，已有文字或标签行隔开时不重复添加。业务样式只定义领域布局。详情通过顶栏菜单访问，返回文件列表时保留目录、搜索条件与选择；列表和详情共用原位备注输入与只读预览弹窗。分支名称在原按钮位置输入，Enter 提交，空白、失焦或 Escape 取消，不再使用分支弹窗；前后端复用 `zxs-naming` 验证名称。上传动作内部完成上传、封存和提交，公开界面不要求用户操作工作区或手动提交。公共 `Select` 管理键盘、焦点及原设计弹层；公共与业务文案均通过统一语言接口选择，CSS、令牌和图标仍为声明式资源。

<a id="s20-02"></a>

## 20.2 命名与依赖规则

### 项目命名

| 对象 | 规则 | 示例 |
| --- | --- | --- |
| 项目显示名 | `ZXS` | `ZXS 文件存储` |
| Cargo 包、组件目录 | `zxs-` 加职责，kebab-case | `zxs-contract`、`zxs-web-core` |
| Rust crate 路径 | `zxs_` 加职责，snake_case | `zxs_core::Engine` |
| Rust 文件、模块、方法、字段 | snake_case，明确业务词 | `request_digest`、`commit_workspace` |
| Rust 类型与 trait | UpperCamelCase，在模块中表达职责 | `Workspace`、`ReadLease`、`CommitRequest` |
| 常量与外部错误码 | SCREAMING_SNAKE_CASE | `MAX_FRAME`、`CONTENT_PURGED` |
| 环境变量 | `ZXS_` 加职责 | `ZXS_WEB_TARGET_DIR` |
| DOM 标记、应用缓存键 | `data-zxs-` / `zxs-` | `data-zxs-runtime`、`data-zxs-theme`、`zxs-language` |
| 构建资源与二进制 | `zxs` 或 `zxs-` 加职责 | `zxs`、`zxs-app`、`zxs-web` |
| 持久化 ID | 已版本化领域前缀加 32 位小写十六进制 | `ver_…`、`commit_…`、`op_…` |
| 哈希用途域 | `zxs:` 加用途和版本 | `zxs:request:v2\0` |

包和边界入口承担项目名前缀；其内部已限定作用域的类型不机械重复 `Zxs`。业务字段和持久化 ID 遵守公开契约，不因展示名称变更而更换语义。盘格式使用 `ZXSLOG2\0`、`ZXSCHK2\0` 等固定标识，变化必须升级格式并提供迁移方案。

复制并维护的框架实现属于本项目源码，遵守上述命名和独立目录规范。外部 Cargo 包、第三方仓库、SDK 函数、系统 ABI 符号和协议字段保留真实名称；不得通过文本替换破坏链接或伪称另一个依赖。第三方来源、版权和许可通知集中在相应 README；不保留原项目名称作为运行时标识、工作区路径或产品品牌。

### 依赖方向

```text
Web 界面 → zxs-web → zxs-web-core / zxs-web-macro
服务端认证 / 平台凭据 → zxs-auth（无 Linux 或数据库依赖）
Web / App / CLI → 公开业务 API
App / CLI 原生传输 → zxs-client → zxs-contract
HTTP / 本机 SMB RPC → zxs-core
Samba VFS → zxs-smb → 受保护 Unix socket
zxs-core → zxs-contract + zxs-naming + zxs-platform
zxs-platform → 文件系统 / SQLite / 凭据算法依赖
```

`zxs-contract` 不依赖操作系统或 UI；`zxs-naming` 使用固定 Unicode 数据，不跟随编译器 Unicode 版本漂移。SQLite 为可删除后重建的索引，三成员日志和不可变对象承担恢复依据。复制必须显式读写完整字节，不使用硬链接、reflink 或去重冒充独立副本。

真实文件容器的目录名、文件备注和版本备注由 State 恢复，`container.json` 只是派生缓存；不可变版本描述、内容散列和发布证明留在独立证明目录。`time` 负责 UTC 时间格式，`rustix` 负责已有安全文件系统原语。旧载荷继续按缺省可选字段的旧路径读取，本次布局扩展不等于完成全盘格式迁移；详见[第 8.2 节](08-full-storage-and-disk-format.md#s08-02)。

自研业务、界面、工具和桥接逻辑采用 Rust。HTML、CSS、TOML、JSON、Unicode 表和其他声明式资源按其用途维护；WASM 生成的 JavaScript 绑定及官方平台依赖可以包含其他语言。自写业务不得迁入这些外部桥接文件。

API 与 SMB 只映射认证主体、输入、句柄和传输错误。最终授权、文件占用、命名、备份路由和发布判定均在核心执行。普通应用不直接访问底层数据盘。

<a id="s20-03"></a>

## 20.3 离线工具

`zxs offline-check <disk>` 验证本盘版本、内容清单、发布回执和带状态页包含证明的内容墓碑；不修改盘数据。`zxs offline-export <disk> <version> <output>` 独立验证后输出完整文件，拒绝覆盖已有目标。导出不修改目标父目录权限，失败不把未完成复制发布成完整文件。

`zxs --root <new-root> restore-system --disks <disk1> <disk2> …` 从至少两个一致的原元数据成员配置恢复系统目录，随后重放日志、恢复凭据控制状态及可发现的待提交工作区。它不会重新初始化数据盘，也不绕过设备身份核验。缺少成员时维持受限状态，不能据单份 PUBLICATION 开放正常写入。

离线工具不依赖在线服务或原服务器 SQLite。实际只恢复本盘持有且证明、字节均有效的版本，不推断全局最新状态。格式、设备或日志冲突必须明确失败；检查命令不得隐式修复或清理。

<a id="s20-04"></a>

## 20.4 构建与工程验证

Rust 版本由根 `rust-toolchain.toml` 指定。原生工作区使用根 `Cargo.lock`；Web、App、Samba 和工具按自身工作区锁定。正式验证使用 `--locked`，依赖预取后使用 `--offline`。Linux 链接依赖、移动 SDK、Samba SDK 及其验证范围分别记在组件 README。

模块测试靠近实现；跨入口浏览器、HTTP、CLI、故障及真实 SDK 证据记录在 `tests/` 与[实现报告](reports/implementation.md)。构建、模拟文件故障、真实设备断电、浏览器、原生宿主和发行签名是不同层级证据，禁止相互代替。

契约变更同时更新类型、schema、固定样例、Rust 校验器和入口映射；只保留一份规则实现。禁止未经证据将第 22 章全部用例一次标记为通过。

---

[文档目录](README.md) · [上一章：容量、配额与性能预算](19-capacity-and-performance.md) · [下一章：配置规范与默认值](21-configuration.md)
