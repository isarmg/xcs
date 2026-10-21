# 架构实现专题

[文档目录](../README.md) · [架构决策](../adr/README.md)

这些页面解释当前代码的实现边界与优化，不替代编号规范。现有英文专题保留术语和证据引用；此中文索引统一入口，新读者可先阅读写入流程，再按源码研究专题。

- [源码命名迁移与来源架构](source-naming.md)：活动身份、原始来源与互操作边界；配套[职责命名规范](../NAMING.md)。
- [Foundation 按职能整合方案](foundation-package-consolidation.md)：包数口径、分批合并候选、技术边界与验收标准。
- [Foundation CI 分组](foundation-ci.md)：职责分组、单次覆盖、平台准备和 Release 验证屏障。
- [HTTP 请求体整合](../components/http-body.md)：单包请求体接口、可选工具与通道边界。
- [SQLite 访问层整合](sqlite-access.md)：查询引擎、驱动与 facade 合并边界、可选能力及原始来源。
- [TOML 职能组件](../components/toml.md)：能力隔离、编辑语义、保留测试和原始来源。
- [一次写入的完整流程](write-flow.md)：规划、准备、决定、发布和失败边界。
- [分离工作区准备](detached-workspace-commits.md)：引擎锁、冻结基线、取消和校验凭据。
- [写入准入](detached-write-admission.md)：准备期间允许和阻止的并发行为。
- [恢复检查点](verified-recovery-checkpoints.md)：校验与回放条件。
- [历史增长优化](history-growth-optimization.md)：分页与历史成本。
- [优化状态](optimization-status.md)：已实现项与仍存在的限制。

描述实现时链接实际代码；测量和检查结果保留执行时点，进入 [reports](../reports/README.md) 或对应测试证据目录。未经执行的设备实验保持待验证。

- [Numeric traits and value types](numeric.md): opt-in arithmetic capabilities and retained upstream identities.
- [Pattern matching capabilities and maintained verification](pattern-matching.md)
