# 单仓库综合审查与精简（2026-10-20）

> 后续范围说明：独立分支演示及其图示、构建和素材采集已于 2026-10-20 移除。本文涉及这些内容的路径、命令、计数及产物仅作历史记录；业务分支规则与存储验收分别保留。见[演示清理记录](demo-removal-20261020.md)。

[报告索引](README.md) · [仓库结构](../monorepo.md) · [来源与许可](../../compliance/README.md)

本轮以 `faf11044cb7e999ae1c99f141691da3ac1fee802` 为基线，检查 Web／SMB 范围、模块引用、依赖与特性、构建入口、CI、交付文档和溯源。修复保存在工作区，未提交、推送、部署或发布。

现有职责分层合理：存储与平台能力、HTTP 服务、浏览器界面、独立 Samba 桥接和运维 CLI 各有明确边界。没有发现原生 App 回流或未挂接的第一方 Rust 源模块；空的 `from_form_data` 模块已删除。仍有可以继续精裁的基础源码超集，因此本次不声称“所有第三方源码已最小化”或“全仓无冗余、无警告”。

## 已修复的问题

| 问题 | 本轮处理 | 验证 |
| --- | --- | --- |
| 界面内核遗留无法启用的 islands、nightly、tracing 和 shadow-dom 条件分支；两个 build.rs 产生的 cfg 不参与有效实现 | 删除无效构建探测、空模块和不可达分支，`unexpected_cfgs` 从 allow 改为 deny | 接口和宏单元／集成测试、严格 Clippy、正式 WASM 与真实浏览器 |
| 宏保留 island/SSR 水合、类型擦除和追踪实验开关，部分展开会引用消费者根本没有的符号 | 删除这些开关及生成路径；保留普通／透明组件、TypedBuilder props、children、template、泛型和生命周期 | 新增实际执行组件调用链的回归，验证字符串 props、透明组件及嵌套 children |
| 组件直接依赖与实际引用不一致 | 删除 12 条冗余直接边：server 2、web 1、interface 6、macros 3，含两个无效 build 依赖 | 冻结锁文件检查、完整原生测试及 Web 测试 |
| admin/server 使用 Tokio full，开启未使用能力 | 统一继承根特性列表，实际 Tokio 1.53.3 不再启用 full、io-std、process、parking_lot | 当前 CLI 构建、真实 HTTP／SMB RPC 测试、浏览器对当前 CLI 验收 |
| 分支演示继承原生 release 配置，与正式 Web 的体积策略不一致 | 两个构建器共用 `tools/web-build/src/wasm_release.rs`，统一 s／fat LTO／symbols；共享文件纳入两份构建输入收据 | 主 Web 与演示均完成实际 release WASM 构建，核对全部输入和资源摘要 |
| Web 测试入口只运行 lib，漏跑接口集成回归；性能工具语义测试未接入 CI | `check.sh web` 和 `web-build test` 均使用 `--tests`；build-tools 增加 perf-check Rust／Python 单测；新增 3 项编排回归 | 169 项 Web 测试、工具与编排测试 |
| 来源门禁只比较已有清单，没有重新解析目标闭包 | rust-sources 入口执行 `verify --metadata`，使用临时空 Cargo home 重解析登记目标 | 8 个入口／目标组合校验通过 |
| Release 不依赖独立 CI 的质量任务 | release verify 直接加入 quality，失败阻断后续组装与公开 | 工作流依赖与脚本复核；未实际发布 |
| 发布包仅携带少数文档，安装说明及 SMB／运维链接缺页；INSTALL 移动后相对链接基准错误 | 两包携带完整 docs；中央模板生成独立 INSTALL，校验后分别解压到新目录 | 真实 tar 回归、最小环境执行安装命令；损坏归档不创建目录，已有目录不被覆盖 |
| 当前文档仍含旧 crates/web-dist/构建锁路径 | 修正组件、工具、演示及 ADR 中的当前指引 | 文档契约和源码导出检查 |

`src/interface` 与 `src/macros` 两目录合计净减少 579 行，删除 3 个文件。删除直接边不意味着对应基础包可删：这些实现仍有必要的传递使用者，最终锁文件仍解析 420 个包。本轮没有升级依赖版本。

Portal 继续使用普通 DOM 挂载并随 owner 清理；兼容参数 `use_shadow=true` 现在显式拒绝。产品没有启用该模式。原始许可证、上游说明和 provenance 字节未改，当前源码与清单分别记录修改后摘要。

## 结构与依赖复核

- 一个根 Git 仓库、一个根 Cargo workspace、一个活动 Cargo.lock，21 个成员；未发现子仓库、子工作区或活动子锁文件。
- 420 个解析包均由仓库内 path 提供，外部 registry／Git 来源为 0；403 个登记源码树、2 个已合并实现、405 个原始身份保持可追溯。
- 登记闭包覆盖 Linux x86_64、Linux aarch64 与 wasm32-unknown-unknown，合计 8 个入口／目标组合。aarch64 本轮为元数据解析验证，未做实际交叉编译。
- `src/client` 是运维 CLI 使用的 HTTP 客户端库；浏览器调用同源业务 API；SMB 经 Samba → 本机 RPC 接入核心。浏览器桌面宽度截图不属于原生 App。
- `docs/` 与 `compliance/` 继续集中管理文档及法律／来源材料；生成发行目录中的分发副本不成为另一份权威来源。
- `.gitignore` 本轮不需再扩大；契约测试证明源码导出保留必要源码、固定 fixture 与中央材料，并排除生成验证附件和本地状态。

实际特性树的复核命令为：

```sh
cargo tree --workspace --frozen --offline --target all \
  -e normal,build --prefix none --format '{p}'
python3 tools/rust-sources/manage.py verify --metadata
```

`cargo tree` 的全成员、全目标结果选择 380 个登记树。metadata 是解析候选上界；二者都不等于最终链接代码或浏览器实际加载字节。应保留这一区别，不能以锁文件、源码目录数推导产品体积或实际启用功能。

## 本轮验证

| 范围 | 实际结果 |
| --- | --- |
| 原生默认成员，空 Cargo home，`cargo test --frozen --offline` | 550 个独立测试通过，0 失败，2 项原有 ignored；排除了一个隔离子进程重复打印的结果 |
| Web／接口／宏，`bash .github/scripts/check.sh web` | 169 项通过：宏 7、Web 108、接口 lib 46、接口 integration 8 |
| 分支演示 | 19 项通过；正式演示 WASM 构建成功 |
| 来源策略 | verify --metadata 成功；Python 来源工具 53、离线构建 5、编排 3 项通过 |
| 构建工具 | Rust 12、离线 Python 5、性能报告 Python 5 项通过；8 种发布模拟、真实 tar／安装与 Unicode 重现通过 |
| 契约与导出 | Rust 17 项通过，含 gitignore 和真实源码导出；文档链接、71 个语义向量和 30 条 API 路径检查通过 |
| 质量 | 第一方全部成员的格式和严格 Clippy 通过；Samba SDK 模块由原有独立任务负责 |
| 正式 Web | 离线构建成功；747 个分发文件、21,898 个输入摘要全部与实际文件一致 |
| 演示产物 | 807 个分发文件、21,924 个输入摘要全部一致；两份产物的中央法律材料副本均逐字节核对 |
| 真实浏览器 | 普通 sarmg 用户，当前新构建 debug CLI 与本轮正式 Web，Chrome 16 项 PASS；包含真实上传保存、下载字节一致性、会话撤销、焦点／快照 DOM、生命周期和字体失败场景 |

本轮实际产物：

| 产物 | 字节 | gzip 字节 | SHA-256 |
| --- | ---: | ---: | --- |
| 正式 Web WASM | 2,205,628 | 662,454 | `e2cb85b66da3faef93764980d66346040e04117539f6ed465d13a23af21d91f7` |
| 演示 WASM | 977,037 | 352,147 | `becd9721f1b4c733d546284f67d175aaad108e2bce03bbeef44a1f8ed8a6ab43` |

两份 WASM 的 name 自定义段分别为 1,671／1,662 字节，未携带多兆字节的函数名段。源码减少不保证二进制同比例减少，本轮没有作这种性能承诺。

执行日志保存在本机 `/tmp/storage-review-*.log`、`/tmp/storage-source-review-*.log`；计数、依赖图和产物复核分别见 `/tmp/storage-source-review-native-tests-summary.json`、`/tmp/storage-source-review-active-trees-after.json`、`/tmp/storage-review-artifacts.json`。这些临时日志不随源码交付，上述命令及 CI 可重新执行。

## 仍可优化的内容与边界

1. **非目标平台源码超集**：44 个登记树没有进入既定 profile，其中 Windows、Apple、WASI 等其他平台 37 树共 326,270,872 源码字节。20 个保留 manifest 仍有 26 条目标依赖边引用它们；直接删目录会使离线解析断裂。若进一步收窄基础库的平台支持，应同时裁剪目标表及特性转发，增加显式退役来源记录并保留全部原始身份、法律材料，再验收三个目标。本轮保留此兼容／解析超集，没有把它们误认作桌面 App。
2. **未激活可选源码与上游警告**：全目标实际特性树未激活的 23 树约 5.35 MiB。Foundation 还有被裁特性名称残留、上游自定义 cfg、弃用 API 等诊断。第一方严格 Clippy 通过不代表所有基础实现无警告；后续应逐包删除确定无用途的实现或修正合法 cfg，避免全局 allow 掩盖问题。
3. **宏文档示例的验证方式**：上游示例依赖消费者 web_interface 上下文，作为独立宏 crate doctest 时有 21 例失败，2 个 compile_fail 还可能因缺依赖产生假通过。当前使用 `--tests` 明确执行单元和集成回归，保留原文且不将其计入测试证据。后续可把适用例子迁入真正的消费者编译 fixture，并删除已经不适用的当前示例。
4. **维护性**：少数服务和 VFS 文件仍较大，可随对应业务改动按领域拆分；本轮没有为缩小文件行数而迁移稳定 API。私有 Unicode 历史算法与回归语料继续用于验证固定 Unicode 行为，未因暂未进入生产调用而盲删。
5. **既有法律复核**：中央 NOTICE 中原始独立许可资料缺失的 3 项状态仍待复核；本轮不补造许可证，也不把来源哈希通过视作法律问题关闭。

本轮没有重跑原生 release、真实 Samba SDK、真实 SMB 客户端互操作、物理盘掉电或部署验收，也没有重新执行在线安全公告扫描。原有大于 64 MiB 历史包专项和手工延迟测试按默认规则忽略，未据默认测试结果宣称本轮已执行。历史验收与本轮静态、构建、测试及浏览器证据应分别阅读。
