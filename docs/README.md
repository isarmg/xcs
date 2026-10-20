# XCS 文档目录

[项目入口](../README.md) · [工程结构](20-engineering.md) · [过程演示](../demos/branch-system/index.html)

本目录集中提供使用、开发和设计资料。编号 01—25 的手册定义产品目标与规则；实现范围和测试结果见交付状态及报告。机器可读契约统一放在 `contracts/`。

## 从这里开始

- 本地开发：[运行服务与检查](getting-started.md) → [工程结构](20-engineering.md)
- 日常使用：[操作手册](25-operation-manual.md) → [配置](21-configuration.md)
- 理解设计：[总体架构](03-architecture.md) → [分支逻辑](design/branch-system-logic.md) → [图形实例](../demos/branch-system/examples.md)
- 构建与分发：[持续集成](21-ci.md) → [发布与安装](release.md)
- 核对进度：[交付状态](23-delivery-and-decisions.md#s23-05) → [实现与检查报告](reports/README.md)

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
| 报告模板 | [验收报告模板](templates/validation-report-template.md) |
| 演示、图示及其测试 | [demos/branch-system/](../demos/branch-system/README.md)，统一存放在独立演示目录 |

新增规范按主题放入本目录；具体实现证据放入报告或对应测试材料，避免与领域规则混写。项目源码、构建缓存与运行池的边界见[工程约定](20-engineering.md)。

## 当前开发兼容策略

开发以当前代码与契约为准，不要求为旧程序版本保留接口、实现或磁盘格式兼容层。需要调整设计时，应同步当前服务端、客户端、测试和文档，而不是增加只为旧版本服务的分支。已有的跨版本读取能力属于实现事实，不构成持续兼容承诺。

这不等于删除用户文件或文件版本历史，也不放宽当前版本的权限、数据完整性和故障恢复要求。已有运行池的重置、清理或迁移是独立操作，不因本策略自动执行。

## 规范用语

“必须”“不得”表示强制要求；“应”表示默认应满足的要求；“可以”表示允许的能力。领域规范定义行为，操作手册说明使用顺序，报告记录指定执行时点的证据。示例中的 ID、设备名称和配置用于说明规则，部署时应核对实际命令和配置支持情况。
