# ZXS 文档目录

[项目入口](../README.md) · [工程结构](20-engineering.md)

本目录统一提供所有产品、组件、开发、运维、验收及上游原始说明；许可证与机器可读溯源统一放在 [compliance/](../compliance/README.md)。项目仅有一个根 Cargo 工作区，目录说明见[单仓库结构](monorepo.md)。编号 01—25 保留既有主题和锚点；稳定规则、架构决策、操作指南及变更证据按下面的用途索引阅读。维护说明以中文为主；原始上游资料、许可证与历史证据保留原文。机器可读契约统一放在 `contracts/`。

## 从这里开始

- 仓库与来源：[整体目录和合并规则](monorepo.md) → [统一许可与溯源](../compliance/README.md)
- 内部命名：[职责命名规范](NAMING.md) → [源码命名迁移与来源架构](architecture/source-naming.md)
- 本地开发：[运行服务与检查](getting-started.md) → [工程结构](20-engineering.md)
- 日常使用：[操作手册](25-operation-manual.md) → [配置](21-configuration.md)
- 理解设计：[术语速查](02-concepts-and-invariants.md#s02-04) → [总体架构](03-architecture.md) → [分支逻辑](design/branch-system-logic.md)
- 构建与分发：[持续集成](ci.md) → [质量门禁](quality.md) → [发布与安装](release.md) → [Rust 第三方源码来源](THIRD_PARTY_RUST_SOURCES.md)
- 核对进度：[交付状态](23-delivery-and-decisions.md#s23-05) → [实现与检查报告](reports/README.md)

## 按资料用途阅读

| 类型 | 入口 | 使用边界 |
| --- | --- | --- |
| 稳定规范 | 第 01—19、21—22 章及 [机器契约](contracts/README.md) | 定义要求、格式和验收标准；目标不等于已完成验收 |
| 统一工程规范 | [第 20 章](20-engineering.md)、[命名细则](NAMING.md)、[测试布局](architecture/test-layout.md) | 规定结构、接口、错误、并发、unsafe 与变更评审；自动化范围以 CI 实际入口为准 |
| 架构决策（ADR） | [adr/](adr/README.md) | 记录选择、代价和依据；[实现专题](architecture/README.md) 解释当前代码 |
| 操作与开发指南 | [入门](getting-started.md)、[操作手册](25-operation-manual.md)、[CI](ci.md)、[发布](release.md) | 给出执行顺序、配置和恢复入口 |
| 变更与验证证据 | [CHANGELOG](CHANGELOG.md)、[历史报告](reports/README.md)、各测试证据目录 | 绑定执行时点和范围，保留失败、未执行与待验证项 |

理解一次提交可直接阅读[写入时序与失败边界](architecture/write-flow.md)。第 23 章汇总交付计划与证据状态，第 24 章保留外部引用；它们不覆盖各主规范。

## 完整索引

| 内容 | 位置与入口 |
| --- | --- |
| 产品与架构 | [需求](01-requirements.md)、[核心概念](02-concepts-and-invariants.md)、[总体架构](03-architecture.md)、[平台与入口](04-platforms-and-access.md) |
| 文件与存储 | [命名](05-naming-and-paths.md)、[历史与分支](06-history-and-branches.md)、[数据模型](07-data-model.md)、[盘格式](08-full-storage-and-disk-format.md)、[备份拓扑](09-backup-topology.md) |
| 发布、读取与保护 | [事务恢复](10-transactions-and-recovery.md)、[写入](11-write-semantics.md)、[读取](12-reading-and-cache.md)、[保留删除](13-retention-and-deletion.md)、[权限](14-security-and-permissions.md) |
| 接口与维护 | [API](15-api-and-errors.md)、[客户端](16-client-interfaces.md)、[生命周期](17-lifecycle-and-maintenance.md)、[故障](18-failure-handling.md)、[容量](19-capacity-and-performance.md) |
| 工程与交付 | [工程结构](20-engineering.md)、[配置](21-configuration.md)、[验收](22-acceptance.md)、[交付状态](23-delivery-and-decisions.md)、[参考资料](24-references.md)、[操作手册](25-operation-manual.md) |
| 专题设计 | [design/](design/README.md)：分支完整规则及合并、标签、备份补充 |
| 实现与检查报告 | [reports/](reports/README.md)：实现范围、测试证据、重构与讨论记录 |
| 公开机器契约 | [contracts/](contracts/README.md)：schema、OpenAPI 和固定向量 |
| 组件和工具 | [第一方组件](components/README.md)、[构建与验收工具](tools/README.md)、[验证材料索引](validation.md) |
| 许可与溯源 | [统一规则](../compliance/README.md)、[第三方通知](../compliance/NOTICE.md)、[原始上游文档](upstream/README.md) |
| 报告模板 | [验收报告模板](templates/validation-report-template.md) |

新增规范按主题放入本目录；具体实现证据放入报告或对应测试材料，避免与领域规则混写。项目源码、构建缓存与运行池的边界见[工程约定](20-engineering.md)。

## 当前开发兼容策略

开发以当前代码与契约为准，不要求为旧程序版本保留接口、实现或磁盘格式兼容层。需要调整设计时，应同步当前服务端、客户端、测试和文档，而不是增加只为旧版本服务的分支。已有的跨版本读取能力属于实现事实，不构成持续兼容承诺。

用户文件及版本历史、当前权限、完整性与恢复要求继续受保护。已有运行池的重置、清理或迁移需要单独授权。`MergeLink` 是关系标记，`FileBranchMerge` 是显式内容合并，二者语义不同；现存格式读取器及兼容导出是否移除，应逐项评估数据恢复和调用方影响，而非按名称批量删除。

## 规范用语

“必须”“不得”表示强制要求；“应”表示默认应满足的要求；“可以”表示允许的能力。领域规范定义行为，操作手册说明使用顺序，报告记录指定执行时点的证据。示例中的 ID、设备名称和配置用于说明规则，部署时应核对实际命令和配置支持情况。
