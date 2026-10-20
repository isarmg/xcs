# 存储、保护路径与历史包验证

2026-10-19；均使用 Rust 临时池，未写入预览服务池。新增生产代码为 Rust；目录模拟盘不构成独立物理设备断电验收。

| 检查 | 本次结果 |
| --- | --- |
| `cargo test --locked --offline -p storage-engine protection::backup_paths --lib` | 6/6 通过 |
| `cargo test --locked --offline -p storage-engine business::file_tags --lib` | 4/4 通过 |
| `cargo test --locked --offline -p storage-engine degraded_recovery --lib` | 6/6 通过 |
| `cargo test --locked --offline -p storage-engine container_storage --lib` | 11/11 通过 |
| `cargo test --locked --offline -p storage-engine history_package --lib` | 既有 8/8 通过；新增综合用例起初仅临时导出根权限错误，补齐0700后下列单项通过 |
| `cargo test --locked --offline -p storage-engine metadata_merges_labels --lib` | 1/1 通过 |

保护测试覆盖：单文件、目录、空目录的规则继承；保留独立子规则；全部 AVAILABLE 版本补足副本且版本 ID／不可变元数据不变；每槽独立字面路径；主盘文件及父目录改名后保持备份路径，新写入继续沿用副本数；显式改前缀保留正文 inode；满容量已有副本不重复计费；固定链4份；范围身份、配置代次、缺前缀与不足路由原子拒绝。

故障测试在 `after_finalize`、`after_container_names`、`after_commit_proofs`、`after_publication_1`、`after_local_publications` 重启并重试同一操作。旧版本补保护的本次证明按候选哈希单独保存，首次发布证明保留。既有受限恢复六项回归同时通过。

新增历史综合测试执行：创建标签节点及备注、两个文件分支分别写入、采纳第三节点形成合并元数据、保留合并别名、为全部版本回填备份、确认原 FULL 与首次证明不变、清除仅被历史来源引用的旧正文、离线导出与恢复、检查标签／备注／合并来源／别名／备份路径和绑定、三个槽离线验证、再次重启。还验证混合节点环、别名环及跨范围合并来源均被拒绝。旧版本清除保留节点与 PURGED 记录，历史元数据不会永久阻止正文清除。

空扩展状态与旧 ProtectionPolicy 的 JCS 往返保持字节不变，避免新可选字段改变旧快照或历史清单的哈希。

## 备份目录浏览补充

新增服务端 `engine_backend/file_manager/backup_namespace.rs` 将实际放置在所选备份槽、内容为 AVAILABLE 且当前用户有读取权限的文件容器投影到其用户前缀。未配置前缀的文件兼容旧 `P01/...` 目录。合成目录只用于浏览，文件保留真实仓库身份与可见路径，历史和下载使用稳定文件／版本身份。主盘当前版本未存于该槽时，显示该槽保留的版本；主路径已删除时仍可读取保留容器，并明确标记 `historical_only`。

所有备份条目为只读，列表不提供目录分支操作或写上下文。重叠目录可合并；不同文件占用同一规范化路径或文件占用目录路径时，GET 明确返回409 `BACKUP_PATH_CONFLICT`。授权过滤先于投影与冲突检查，隐藏仓库不参与可见结果。

`cargo check --locked --offline -p storage-server` 与既有真实 file_manager HTTP 大场景通过；新增自定义前缀、主盘改名、历史下载、只读与权限场景的HTTP记录由 API 验证目录补齐。

新增备份命名空间真实HTTP三项已全部通过：用户前缀目录及固定槽版本、授权后的路径冲突、跨主盘链实际放置可见性。覆盖先写入三份、后续只写主盘而B/C保留旧版本，双次主盘改名、删除后历史容器、history及下载、备份写入403、规范化文件／目录冲突409，以及未授权仓库不参与列表与冲突。服务端与Web兼容字段已固定为 `backup`、`read_only`、`historical_only` 与显式真实 `repository_path`；最终线缆记录见同级 API 证据目录。
