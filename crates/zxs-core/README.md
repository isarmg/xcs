# ZXS 存储核心

`zxs-core` 实现单写者、完整文件版本、不可变发布证明与离线恢复。对外入口由 `lib.rs` 导出；实现模块默认私有，不允许调用方绕过 `Engine` 的权限、操作幂等与发布流程。

| 目录 / 模块 | 职责 |
| --- | --- |
| `engine.rs` | 生命周期、权限、操作查询、在克隆状态上组合一次业务变更 |
| `engine/config.rs` | 拓扑与额度配置、独占操作系统锁 |
| `engine/publication.rs` | 正文准备、精确元数据预检、持久决定与发布屏障 |
| `engine/recovery.rs` | 仅沿已验证发布前缀回放，协调未完成决定 |
| `engine/reads.rs` | 副本验证、读租约、导出、磁盘容量与状态 |
| `engine/workspace_storage.rs` | 工作区清单的双位置持久化与重启协调 |
| `engine/index.rs` | 可重建 SQLite 投影 |
| `business/` | 仓库、命名空间、上传、文件分支、目录批量分支与历史查询 |
| `business/namespace.rs` | 唯一的路径校验、父目录约束、命名空间改动与工作区冲突规则 |
| `business/ancestry.rs` | 基于子节点索引的提交后代查询，供拆分与历史包校验复用 |
| `journal.rs` / `metadata_budget.rs` | 哈希对象、日志帧、证明与实际序列化大小限制 |
| `container_storage.rs` | 普通版本文件、真实容器改名、继承引用缓存及旧空 stub 的安全兼容清理 |
| `historical_save.rs` / `maintenance.rs` | 历史另存分支、计划、清理与历史拆分 |
| `history_package.rs` | 可独立校验、导出、恢复的完整历史包 |
| `repair.rs` / `degraded_recovery.rs` | 修复原始候选与显式永久丢失恢复 |
| `wire.rs` / `protection.rs` | 冻结上传清单、请求摘要、后续版本放置与保护规则 |

## 权威数据与投影

业务操作先在 State 克隆上组合修改；分支创建只登记冻结引用，不新建空文件。实际内容写入才准备 FULL 和固定路由副本。DirectoryBranch 保持稳定 ID/名称，DirectoryTree 保存不可变组合；文件修改或子项选择只增加子树并推进 head。PREPARE 不表示可见，ACCEPT → FINALIZE → PUBLICATION 及同步收据共同决定公开状态，恢复沿原候选继续。

SQLite、容器 container.json 及当前名称是可重建投影，不能替代发布记录和不可变证明。新建分支不生成 pending-branches/*.pending，也不为 legacy 来源创建空容器；只安全清理旧零字节单链接 stub。活动文件分支、工作区和读租约固定正文；目录只固定选中及尚未完成分支的当前 head，不永久固定旧子树。

读取继承冻结父版本，范围编辑保留父未改字节；零字节或同摘要首写仍建立独立版本。历史另存只为 staged 文件生成 FULL。Branch/Commit 空映射及新增目录 map 省略序列化，旧规范字节和 FULL 证明保持。语义、目录子树与默认视图见[分支说明](../../docs/reports/lazy-branches-20261019.md)。

## 共享规则与扩展位置

- 名称与路径只调用 `zxs-naming`，固定 Unicode 16.0 NFC / case folding。显示拼写可以不同，祖先关系必须按规范化键判断；移动子树时按路径组件处理显示名称，不能用另一种拼写的字节长度截断 UTF-8。
- 新写入备注调用 `zxs_contract::validate_note`，最多 `MAX_NOTE_BYTES = 128` UTF-8 字节，空串清空，拒绝控制字符。历史包读取明确保留旧 4096 字节上限，不隐式重写旧证明。
- 校验块大小由 `zxs_contract::SUPPORTED_CHECKSUM_CHUNK_BYTES` 统一提供，编码器、配置和历史包使用同一集合。
- 新业务操作在 `business/` 或相应维护模块组合状态，经 `mutate` / `mutate_digest` 发布；不得自行写日志、提交 SQLite、改写既有版本正文。
- 新上传检查放在不可变代次复制之前，流式复制同时受到文件额度和剩余工作区额度约束。失败应保留已接受代次，避免留下一个没有清单引用、又阻止合法重试的代次。

## 验证

```sh
cargo test --locked --offline -p zxs-core -p zxs-contract -p zxs-naming --lib
cargo check --locked --offline -p zxs-core
```

测试覆盖发布屏障、幂等、恢复、读租约、兼容树拆分及完整包恢复。新分支回归检查无正文复制或空 stub、真实空首写、缓存/改名/旧 inode、来源清除、目录组合和不可变子树闭包、共享文件身份、选择代际、目录 head 固定与历史清理；含目录组合的旧拆分明确拒绝。所有 fixture 使用临时目录，不代表物理设备故障或发行验收。
