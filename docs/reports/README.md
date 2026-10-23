# 实现与检查报告

[文档目录](../README.md) · [交付状态](../23-delivery-and-decisions.md) · [变更索引](../CHANGELOG.md)

| 报告 | 范围 |
| --- | --- |
| [首发准备专项审查](first-release-review-20261023.md) | 发行归档、安装候选、内容绑定、认证/SMB/Web 边界及首发待验收项 |
| [系统性工程规范与代码质量审查](systematic-engineering-review-20261023.md) | 第一方架构门禁、中文规范与文档、认证/传输/SMB/Web 边界修复及验证限制 |
| [语法、排版与 CPU 检测功能命名](syntax-cpu-naming-20261021.md) | 三个功能包名、生成路径与内部文件协调迁移、来源与 ABI 例外 |
| [密码学公共接口整合](crypto-consolidation-20261021.md) | 单一摘要接口、BLAKE2 适配、合并来源与独立向量回归 |
| [语法解析与 CPU 检测整合](syntax-cpu-consolidation-20261021.md) | 三组重复实现合并、AST 适配、实际绑定生成与维护回归 |
| [序列化组件功能命名](serialization-naming-20261021.md) | 七包与 97 个路径、派生宏、数据格式保护、无版本私有模块与回归 |
| [熵源依赖整合](entropy-consolidation-20261021.md) | 单代实现合并、失败关闭、Linux/WASM 后端验证、不可变历史归档 |
| [哈希表依赖整合](hash-table-consolidation-20261021.md) | 单实现合并、碰撞/顺序/LRU/析构回归、真实 SQLx 缓存与来源归档 |
| [界面组件功能命名](interface-naming-20261021.md) | 六个包、内部模块/类型与宏路径、来源双射证明、追踪宏修复与验证 |
| [功能命名首批迁移](functional-naming-20261021.md) | 六个源码/测试根、四个职责包名、内部引用、不可变来源与验证边界 |
| [Base64 单实现整合](base64-consolidation-20261021.md) | 455 万组差异验证、标量特性、旧实现及外置测试归档、调用方和剩余限制 |
| [Foundation 项目定制与架构优化](foundation-customization-20261021.md) | c6421fb5 后的新一轮消费者地图、微型职责归并、构建依赖裁剪、量化结果与验证边界 |
| [架构与本轮改造说明](engineering-refactor-20261020.md) | 当前模块、存储真相、正确性/资源边界、foundation职责与验收限制的完整汇总 |
| [ring Linux amd64 构建输入收敛](ring-linux-amd64-target-20261020.md) | 116 非目标文件退役、选中输入与生成字节等价、不可变历史来源与当前源码双视图校验 |
| [Linux amd64 runtime 最终收敛](linux-amd64-runtime-final-20261020.md) | 60 平台实现退役、熵源显式拒绝、Linux/Web 源码等价与严格离线消费者；保留 docsrs 壳和已知环境限制 |
| [ring 上游回归材料恢复](ring-test-restoration-20261020.md) | 精确 commit/blob 溯源、34 库向量与完整集成/CAVP 入口、唯一工作区离线回归矩阵 |
| [Linux amd64 基础源码第二批收敛](linux-amd64-foundation-second-20261020.md) | rustix/libc 架构与 OS 分派、选中源码等价、GNU ABI 和双后端回归；ring 只读生成证据与完整回归缺口 |
| [Linux amd64 基础源码收敛](linux-amd64-foundation-20261020.md) | 原生平台边界、MSVC 构建依赖退役、Linux UAPI 架构裁剪、未来 API 保留与离线回归 |
| [演示代码清理](demo-removal-20261020.md) | 删除独立演示、素材采集、构建与CI入口，保留真实分支业务与历史来源 |
| [综合审查后续整改](review-completion-20261020.md) | 非目标／可选源码退役、全特性边与活动闭包门禁、零警告编译、真实消费者宏文档测试及模块职责拆分 |
| [基础实现行为兼容复核](foundation-compatibility-20261020.md) | IDNA 全标量与原版对照、历史 Crypt 编码、数值和辅助实现回归，保留旧语料的代际差异 |
| [单仓库综合审查与精简](comprehensive-review-20261020.md) | 无效 CSR／宏分支、依赖特性、测试和发布门禁、离线与浏览器实测，以及平台源码超集等后续优化边界 |
| [单仓库整合与集中溯源](monorepo-integration-20261020.md) | 根工作区、组件合并、整体目录、统一文档与许可、原始哈希和剩余复核事项 |
| [Rust 源码自主维护与入口收敛](rust-source-autonomy-20261020.md) | Web／SMB 范围、405 个本地源码目录、业务定制、离线重建、实际 SDK／浏览器验证及未关闭许可事项 |
| [实现报告](implementation.md) | 源码映射、入口、构建、已验收范围和待完成边界 |
| [分支系统实现](branch-system-implementation.md) | 文件/目录分支、合并、标签、备份与对应验证材料 |
| [代码审查与优化](code-review-20261019.md) | 全项目调用链、模块职责、维护性问题和验证 |
| [架构复核与修复](architecture-review-20261020.md) | 工作区重放、原生上传能力、异步阻塞边界、资源生命周期及验证范围 |
| [第二轮架构复核](architecture-followup-20261020.md) | 只读发布证据、下载交付、SMB 一致视图、通知资源与构建输入 |
| [静态审阅整改追踪](review-remediation-20261020.md) | 框架自维护、存储增长、授权、类型边界、SDK 与依赖审计，含尚未完成项 |
| [延迟创建分支讨论](lazy-branches-20261019.md) | 分支规则演变与已经确认的语义 |

报告中的“本轮”“当前环境”和测试计数仅指该报告记录的执行时点，不表示最新提交或发布状态。后续 CI 与发布结果需核对对应工作流及提交。

报告保留对应执行时点的证据，当前规则以[专题设计](../design/README.md)及编号手册为准。验收日志、截图与收据继续保留在原证据目录，便于回查。

- [Linux amd64 系统与网络后端精简](linux-amd64-system-backends-20261020.md)：sysinfo/socket2/mio 互斥平台后端退役、实际选择与消费者回归证据。

- [ring Clippy 成员诊断窄修复](ring-style-diagnostics-20261020.md)
