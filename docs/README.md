# XCS 文档目录

[项目入口](../README.md) · [工程结构](20-engineering.md) · [过程演示](../demos/branch-system/index.html)

编号 01—25 的文件是连续的产品规范手册，保留稳定的章节路径。专题设计、实现报告和模板分别归档，机器可读契约仍由 `contracts/` 唯一维护。

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
| 报告模板 | [验收报告模板](templates/validation-report-template.md) |
| 演示、图示及其测试 | [demos/branch-system/](../demos/branch-system/README.md)，统一存放在独立演示目录 |

新增规范按主题放入本目录；具体实现证据放入报告或对应测试材料，避免与领域规则混写。项目源码、构建缓存与运行池的边界见[工程约定](20-engineering.md)。
