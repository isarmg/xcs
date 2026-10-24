<a id="platform-io"></a>

# Linux 平台基础（platform-io）

ZXS 独立的 Linux 服务端基础库，在本项目维护和构建。

| 模块 | 本项目职责 |
| --- | --- |
| `fs_safety` | 持有目录句柄、拒绝链接和路径逃逸、单链接文件、文件及目录同步、无覆盖发布 |
| `payload` | 有长度上限的完整流式写入、BLAKE3 校验、新 inode、不可覆盖发布 |
| `sqlite` | SQLx 0.9 索引连接、WAL/FULL/外键、defensive、原生连接限额、完整性及快照检查 |
| `schema_identity` | 索引表结构身份和确定性指纹 |
| `secret` | 默认脱敏、不可序列化、释放时清零的秘密类型 |
| `error` | 有长度上限的错误和请求标识；支持 ZXS 大写业务错误码 |

`sqlite::block_on_sqlite_connection` 支持同步核心执行直接 `SqliteConnection` 操作；不支持连接池、网络、Tokio 定时器或递归调用。每次必须在 future 返回前显式 `Connection::close().await`。`open_index_connection` 固定索引连接策略并启用 defensive；`create_if_missing=false` 不允许隐式新建数据库。

平台库不拥有数据盘路由、ACL、产品 DDL 或跨盘事务真相。SQLite 仅为可重建索引。`payload::Receipt` 证明单文件落盘，不能被当作 ACCEPT、FINALIZE 或 PUBLICATION。

认证算法的唯一实现和对应回归测试在 [authentication](auth.md)。该独立 crate 不依赖 Linux 或 SQLite，服务端和救援工具直接依赖它；平台层不保留兼容导出，避免同一密码/令牌策略维护两份实现。

本项目定制包括：

- 所有索引池连接和直接索引连接在处理业务语句前启用 SQLite defensive。
- 错误码允许全大写或全小写，拒绝混合大小写，避免客户端歧义。
- 完整 payload 使用固定 64 KiB 缓冲区，不使用硬链接、reflink、稀疏复制或去重；摘要或长度不匹配时清理暂存文件。
- 存储核心和运维 CLI 使用此 crate 的 Linux amd64 本机能力；远程管理客户端 storage-client 和浏览器 Web 不引入 Linux 盘操作及 SQLite 本地 FFI。原生产品和构建宿主的支持范围仍仅为 Linux amd64。


平台库的 Tokio 直接生产依赖仅启用 `rt`；SQLite 依赖自行声明所需的 `sync`/`time`。测试单独启用宏和多线程运行时，以覆盖两种执行环境。这不会移除服务端实际需要的多线程运行时，也不改变 SQLite 持久化策略。
