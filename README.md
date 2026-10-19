# XCS 文件存储系统

本手册规定六平台自研 App、独立 SMB 文件访问、文件版本与文件分支、关系标记、分层硬盘备份、独立历史树拆分和运行维护的目标行为。服务端由一台 Linux 主机统一管理多块独立硬盘。

当前 Web 入口以文件管理器为中心：磁盘分别显示为独立目录，进入后浏览文件和文件夹，详情独立展示历史和分支。文件分支创建仅登记冻结父版本，首次实际写入才产生独立 FULL。目录分支复用共享文件分支形成组合，只有显式创建才新增命名分支；修改文件或切换子项生成不可变子树并推进原分支 head，名称和 ID 不变。创建不复制正文、不生成冻结来源引用文件。根目录默认组合也供 SMB 读取，普通有写权限用户可选择。用户文件对应真实版本容器；文件／文件夹改名增加目录子树及每个文件的路径节点，复用正文，改名移动容器，历史下载使用当前名称。CPU、内存、磁盘状态与操作历史在独立页面展示。

规范覆盖功能、数据与接口契约、保存和恢复条件、运行维护以及验收标准。示例中的接口、配置、命令、ID 和设备名称用于表达目标设计；实现与测试情况以交付材料为准。

文件分支合并、不可变版本标签和每盘自定备份前缀的规则见[最新补充](docs/design/file-merge-tags-backup.md)，配套[实例 24—27](demos/branch-system/examples.md#example-24)。合并明确选择结果节点和归属分支，相关目录引用一起更新；目录分支合并暂不实现。

仓库包含独立构建的 Rust 存储核心、API、CLI、Web/WASM 界面、原生 App 宿主及 Samba 桥接代码。实现范围、测试证据和剩余验收见[实现报告](docs/reports/implementation.md)与[状态登记](docs/23-delivery-and-decisions.md#s23-05)。规范仍是完整产品目标，源码构建通过不代表物理盘、Samba 和六平台发行全部通过验收。 全项目的调用链梳理、维护性重构、具体问题修复及验证见[代码审查与优化记录](docs/reports/code-review-20261019.md)。

## 代码与统一命名

自研业务、界面、构建与校验工具全部采用 Rust。项目包使用 `xcs-*`、Rust 路径使用 `xcs_*`、环境变量使用 `XCS_*`，资源和构建产物遵循相同前缀。目录、依赖方向、错误与身份规范见[工程体系](docs/20-engineering.md)。

| 目录 | 内容 |
| --- | --- |
| `crates/` | 可移植契约、固定 Unicode 命名、共享认证算法、Linux 基础库、存储核心、API、客户端和 CLI |
| `web/` | Rust/WASM 应用、统一 CSR 入口、样式与字体 |
| `apps/` | Rust 原生业务桥接及 Tauri 宿主 |
| `integrations/smb/` | 本机 RPC、文件描述符传输与目标 Samba SDK 的 Rust VFS |
| `vendor/web/` | 项目独立维护并统一命名的 CSR 引擎与过程宏 |
| `vendor/dependencies/` | 保持真实包名的外部依赖快照 |
| `tools/` | Rust Web 构建、契约校验和固定 Unicode 表重现工具 |
| `deploy/systemd/` | Linux 服务单元与运行配置说明 |
| `demos/branch-system/` | 独立 Rust/WASM 过程演示、自由操作、静态图示与验证材料 |
| `tests/` | 跨模块验证材料与证据；模块测试随源码维护 |
| `docs/` | 编号规范手册，专题设计在 `design/`、报告在 `reports/`、模板在 `templates/`、机器契约在 `contracts/` |

本项目已复制所需基础代码并独立维护，构建不读取其他项目目录。外部 Cargo 包、原生 SDK 和外部仓库保留真实名称；复制框架的来源、版权及许可通知集中在相关 README，其中 [CSR 引擎说明](vendor/web/README.md)和[Unicode 数据说明](crates/xcs-naming/README.md)列出具体来源。

完整导航见[文档目录](docs/README.md)。各工作区的 Cargo 缓存集中在根 `target/` 下；`web/dist/`、演示 `dist/`、根 `runtime/` 和 `xcs-data/` 为本机生成或运行数据。`.gitignore` 保留源码、锁文件、字体、固定参考图及验收说明；测试生成的截图、日志、结果 JSON 和收据保存在本机，不纳入 Git。具体约定见[工程结构](docs/20-engineering.md)。

## 本地运行

需要 `rust-toolchain.toml` 指定的 Rust 1.99.0。先构建服务与 CLI；密码从权限为 `0600` 的私人文件或标准输入读取，至少 12 字节。

```sh
cargo build --locked --target-dir target/native -p xcs-cli
target/native/debug/xcs --root "$PWD/runtime/pool" init \
  --development-disks "$PWD/runtime/disks/P01" "$PWD/runtime/disks/B01" "$PWD/runtime/disks/C01" \
  --username admin --password-file /secure/path/xcs-password
target/native/debug/xcs --root "$PWD/runtime/pool" serve --listen 127.0.0.1:7400
```

`--development-disks` 明确使用目录模拟三个物理设备，仅用于开发。生产配置必须使用三个独立物理设备上的单设备 Btrfs，启动时核对盘位与物理身份。服务运行时，CLI 使用 `--server http://127.0.0.1:7400` 访问同一核心；直接打开本地池的第二写者会被拒绝。默认服务仅监听本机，外部访问配置 HTTPS 反向代理和正确的外部 origin。

Web 构建、资源服务及真实浏览器验证见 [Web README](web/README.md)。原生平台依赖及平台矩阵见 [App README](apps/README.md)。Samba 的目标 SDK、受限用户映射和构建边界见 [Samba README](integrations/smb/README.md)。

```sh
cargo test --locked --offline --target-dir target/native --workspace
cargo run --locked --offline --manifest-path tools/contract-check/Cargo.toml --target-dir target/native
cargo run --locked --offline --manifest-path tools/unicode-build/Cargo.toml -- --check
cargo run --locked --offline --manifest-path tools/web-build/Cargo.toml -- check --product-root .
```

首次构建需要取得锁文件中的依赖；缓存齐全后可使用 `--offline`。本机 Unix 套接字和所有权隔离测试需要相应系统能力，不能把隔离环境拒绝执行当作业务测试通过。

[持续集成说明](docs/21-ci.md)列出根工作区之外的独立测试、正式 WASM 构建、三桌面宿主检查与浏览器烟测，以及 SDK 和真机验证边界。 标签自动发布、下载校验与安装方式见[发布说明](docs/release.md)。

## 阅读路线

先看[分支过程演示](demos/branch-system/index.html)、[自由操作页](demos/branch-system/playground.html)或 [Markdown 图片实例](demos/branch-system/examples.md)，再查[分支系统运作逻辑详解](docs/design/branch-system-logic.md)。图中父节点在下、子节点在上。三个边界已按用户意见确定：完整记录目录组成、公共文件分支自动命名、显式选择已有分支后写入该分支。对应核心、接口、Web 与验证范围见[本次分支实现记录](docs/reports/branch-system-implementation.md)。

最新实例 17—22 展示[跨目录移动与跟踪移交](docs/design/branch-system-logic.md#s23)及[批量选择分支进行移动、复制、删除、重命名](docs/design/branch-system-logic.md#s24)：只更新选中分支，实际文件移到目标目录，其他分支保留自己的逻辑路径和版本引用。

| 阅读目的 | 建议顺序 |
| --- | --- |
| 把握产品与功能 | [需求与边界](docs/01-requirements.md) → [总体架构](docs/03-architecture.md) → [六平台入口](docs/04-platforms-and-access.md) → [历史与分支](docs/06-history-and-branches.md) |
| 日常使用与管理 | [操作手册](docs/25-operation-manual.md) → 按操作跳转到对应领域规范 |
| 开发与集成 | [对象与不变量](docs/02-concepts-and-invariants.md) → [数据模型](docs/07-data-model.md) → [事务与恢复](docs/10-transactions-and-recovery.md) → [API 契约](docs/15-api-and-errors.md) → [验收标准](docs/22-acceptance.md) |
| 部署与故障恢复 | [配置](docs/21-configuration.md) → [生命周期与维护](docs/17-lifecycle-and-maintenance.md) → [故障矩阵](docs/18-failure-handling.md) → [单盘恢复](docs/08-full-storage-and-disk-format.md#s08-05) |

## 核心规则

| 方向 | 统一要求 |
| --- | --- |
| App | Linux、Windows、macOS、Android、iOS、iPadOS 客户端均由本项目自行实现，文件与管理功能均通过业务 API 访问 |
| SMB | 独立连接 Samba，仅提供共享主分支的基础文件操作；App 不内置 SMB 通道，两个入口在服务端共用核心 |
| 总体架构 | [一张完整架构图](docs/03-architecture.md#s03-01)展示 App 与 SMB 的独立链路、共同存储核心、持久化和分层硬盘 |
| 全量保存 | 内容变化的文件形成新完整版本，旧版本不原地修改；普通保存不重写所有未变文件 |
| 文件及目录分支 | 文件分支引用冻结父版本，首写才拥有 FULL；目录分支组合共享文件分支，修改只新增不可变子树并推进 head，不自动创建命名分支或空文件 |
| 文件分支合并 | 选择输入分支、该文件任意结果节点及归属；复用输入头或新建复用正文的合并节点，推进归属及相关目录引用；目录合并待确认 |
| 关系标记 | 旧 `MergeLink` 固定指定端点，只保存关系，不推进 HEAD；与文件分支合并分开 |
| 节点标签 | 绑定文件具体节点，子节点继承；增删标签新增复用正文的子节点，支持批量，历史编辑不回退较新正文 |
| 历史清除 | 可以清除获准历史内容，节点、名称、摘要、版本父边与合并标记继续保留 |
| 树拆分 | 按版本父边拆出切点及全部实际后代，两树的内容、权限和恢复材料分别独立 |
| 分层备份 | 用户指定一级归属和逐级来源，份数包含第一份，各备份盘独立用户前缀；不从主盘真实路径推导前缀，不任意换盘、跳级或自动改变角色 |
| 保存结果 | 仅当指定各级副本、控制元数据保护和公开发布均完成时，才确认正常保存成功 |
| 控制元数据 | 三个不同物理设备承担控制成员；业务文件的一至多份保护与控制元数据三份分别核算 |

## 规范用语与适用范围

“必须”“不得”表示强制要求；“应”表示默认应满足的要求；“可以”表示允许的能力。产品功能与数据不变量以领域规范为准，操作手册用于说明这些规则在实际任务中的使用顺序。

完整内容版本、历史提交、合并关系、备份份数、缓存与工作区分别建模。术语和状态以[第 02 章](docs/02-concepts-and-invariants.md)为准，操作结果按[第 15 章](docs/15-api-and-errors.md#s15-06)区分，验收材料应覆盖[需求与验收对应](docs/01-requirements.md#s01-06)。

## 第一部分：产品、架构与入口

| 章节 | 内容 |
| --- | --- |
| [01 需求与系统边界](docs/01-requirements.md) | 系统目标、功能要求、访问入口及保护范围。 |
| [02 术语、对象与核心不变量](docs/02-concepts-and-invariants.md) | 对象身份、历史关系、内容状态和正确性约束。 |
| [03 总体架构与模块边界](docs/03-architecture.md) | 一张完整总架构图，以及 App、SMB、核心和持盘边界。 |
| [04 六平台客户端与访问入口](docs/04-platforms-and-access.md) | 六平台自研 App、独立 SMB、网页和命令行的职责。 |

## 第二部分：文件命名与历史规则

| 章节 | 内容 |
| --- | --- |
| [05 统一命名与路径规范](docs/05-naming-and-paths.md) | 字符、Unicode、大小写、长度、导入校验和错误提示。 |
| [06 版本、分支引用、合并连线与树拆分](docs/06-history-and-branches.md) | 版本演进、分支引用与首次写入、历史另存、关系标记和历史树拆分。 |

## 第三部分：数据、备份与持久化

| 章节 | 内容 |
| --- | --- |
| [07 数据模型、身份与索引](docs/07-data-model.md) | 核心实体、稳定标识、对象关系及索引重建依据。 |
| [08 全量存储、盘格式与单盘恢复](docs/08-full-storage-and-disk-format.md) | 完整文件、副本材料、规范编码、盘上布局和离线恢复。 |
| [09 用户指定的分层备份拓扑](docs/09-backup-topology.md) | 一级归属、固定来源、逐级副本、等级调整和拓扑迁移。 |
| [10 事务、持久化与崩溃恢复](docs/10-transactions-and-recovery.md) | 保存状态、三成员保护、决定证明和确定性恢复。 |

## 第四部分：文件访问、安全与接口

| 章节 | 内容 |
| --- | --- |
| [11 文件写入与保存语义](docs/11-write-semantics.md) | 工作区、保存边界、原子替换、多文件并发和失败保留。 |
| [12 读取、完整性与缓存一致性](docs/12-reading-and-cache.md) | 固定版本、范围读取、分段校验、缓存及断网规则。 |
| [13 历史内容清除与安全回收](docs/13-retention-and-deletion.md) | 删除边界、保留引用、墓碑、物理清理及回收。 |
| [14 身份、权限与运行隔离](docs/14-security-and-permissions.md) | 入口认证、授权范围、本机隔离和恢复访问边界。 |
| [15 API、操作结果与错误契约](docs/15-api-and-errors.md) | 业务接口、幂等、分类型结果、错误及能力协商。 |
| [16 App、网页与命令行界面](docs/16-client-interfaces.md) | 界面信息、文件交接、任务状态和命令行目标接口。 |

## 第五部分：运行、故障与容量

| 章节 | 内容 |
| --- | --- |
| [17 初始化、启停与硬盘维护](docs/17-lifecycle-and-maintenance.md) | 设备登记、就绪状态、安全拔盘、换盘和成员恢复。 |
| [18 故障处理与降级行为](docs/18-failure-handling.md) | 故障矩阵、可读条件、写入限制和恢复动作。 |
| [19 容量、配额与性能预算](docs/19-capacity-and-performance.md) | 完整版本成本、逐盘准入、维护预留、告警和基准。 |

## 第六部分：工程、配置与验收

| 章节 | 内容 |
| --- | --- |
| [20 工程结构与依赖约束](docs/20-engineering.md) | Rust 模块、平台桥接、服务边界及离线工具。 |
| [21 配置规范与默认值](docs/21-configuration.md) | 目标配置、逐级来源示例及关键参数的约束。 |
| [22 验收标准与证明材料](docs/22-acceptance.md) | 功能、平台、格式、事务、故障和独立恢复验收。 |
| [23 交付路线、设计决策与验证边界](docs/23-delivery-and-decisions.md) | 实施依赖、阶段交付、技术取舍和待验证条件。 |
| [24 技术资料与引用范围](docs/24-references.md) | 规范所引用的官方资料及其适用边界。 |

## 第七部分：操作手册

| 章节 | 内容 |
| --- | --- |
| [25 用户与管理员操作手册](docs/25-operation-manual.md) | 连接、保存、历史、合并标记、清除、拆分、备份和维护流程。 |

## 常用问题入口

| 问题 | 规范位置 |
| --- | --- |
| App 和 SMB 如何区分 | [接入架构](docs/03-architecture.md#s03-01)、[职责边界](docs/04-platforms-and-access.md#s04-03) |
| 从旧版本继续保存到哪里 | [历史保存](docs/06-history-and-branches.md#s06-04)、[操作步骤](docs/25-operation-manual.md#s25-04) |
| 文件合并与关系画线有何区别 | [文件合并和标签](docs/design/file-merge-tags-backup.md)、[旧关系标记](docs/06-history-and-branches.md#s06-08) |
| 删内容后历史是否还在 | [节点保留](docs/13-retention-and-deletion.md#s13-02)、[清除流程](docs/25-operation-manual.md#s25-06) |
| 拆分是否会沿合并线带走其他分支 | [拆分范围与独立性](docs/06-history-and-branches.md#s06-09) |
| 二级盘满而其他盘有空闲怎么办 | [固定来源限制](docs/09-backup-topology.md#s09-11)、[故障处理](docs/18-failure-handling.md) |
| 保存超时是否需要重传 | [幂等与结果查询](docs/15-api-and-errors.md#s15-03)、[用户处理流程](docs/25-operation-manual.md#s25-10) |
| 单盘能恢复什么 | [单盘恢复](docs/08-full-storage-and-disk-format.md#s08-05)、[故障访问权限](docs/14-security-and-permissions.md#s14-10) |
