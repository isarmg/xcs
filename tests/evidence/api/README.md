# API 与 CLI 验收记录

记录日期：2026-10-18。环境为 Linux 开发目录模拟三盘，不代表独立生产 Btrfs、断电硬件、跨平台应用或正式 Samba 服务的验收。所有测试数据为临时合成内容；此目录不保存密码、token、Cookie、认证快照或临时 Python 探针。

## 可重复的仓库内测试

| 命令或测试 | 实测结果 |
| --- | --- |
| `cargo test -p xcs-server -p xcs-client -p xcs-cli --offline -- --skip smb_rpc::tests` | 初轮 client 2 项、server 非 SMB 单元测试 22 项、HTTP 4 项通过；CLI 编译通过，无单元测试。之后新增的 1 项授权及 2 项 Backend 回归分别列于下方 |
| `cargo test -p xcs-server --test http_engine --offline` | 扩展后的 6 项通过（4 项 HTTP + 2 项真实 EngineBackend 发布证据） |
| `cargo test -p xcs-core historical_save::tests --offline` | 2 项通过 |
| `cargo test -p xcs-server --offline authorization_tests --lib` | 1 项通过：拆分仓库的新账号范围或原范围可准入，仍须新仓库 ACL |
| `cargo check -p xcs-cli --offline` | 通过 |

构建使用 `CARGO_TARGET_DIR=target/native CARGO_INCREMENTAL=0`。SMB RPC 测试由集成模块单独记录，不把这里跳过的测试计为通过。最终工程全量测试由根项目记录。

`crates/xcs-server/tests/http_engine.rs` 的 6 个用例均使用真实 Engine 和三个数据目录，不使用业务 mock。4 项 HTTP 用例通过 Router 请求完整 Axum 路由和持久认证；另 2 项直接检查 EngineBackend 的公开操作响应：

| 用例 | 验证结果 |
| --- | --- |
| HTTP 上传准入 | 核心文件限制在接收 body 前生效，返回 413，暂存目录没有数据文件 |
| HTTP 完整保存 | 同工作区 mkdir + 子文件上传、持久范围、SEALED/JCS、三盘 TARGET_MET、原操作幂等、变更请求拒绝、Range 与校验表 |
| HTTP 对象授权 | 私有仓库直接对象 ACL、拒绝未授权写入、禁用账号后旧会话失效 |
| HTTP 历史保存与管理 | 一个事务创建独立全量分支且原 HEAD 不变、策略/规则 generation CAS、显式 SMB 重绑 |
| Backend 永久丢失恢复 | after_accept_2 中断后丢失某文件三份固定内容及暂存源，显式恢复返回降级发布和真实 UNAVAILABLE 槽 |
| Backend 成功后损坏 | 正常发布后损坏一槽，原成功 TARGET_MET 证据保持，当前健康度变为 DEGRADED |

两份实际操作响应均按未修改的原始 SaveOperation schema 验证通过，见 [校验结果](save-wire-validation.json)、[降级发布](degraded-save-operation.json) 与 [成功后损坏](damaged-published-save-operation.json)。前者冻结三槽 UNAVAILABLE、返回 PUBLISHED_DEGRADED/COMMITTED_DEGRADED/BELOW_TARGET；后者保留原 PUBLISHED/TARGET_MET/COMPLETE_VERIFIED，当前健康度单独为 DEGRADED，不重写历史成功证据。

另外 2 项历史保存核心测试验证不变文件也重新生成独立完整副本（不同 inode、nlink=1）；PURGED 历史内容缺失时拒绝，显式完整替换后可建立新分支。

## 真实 TCP 与 CLI

已在仅回环的临时服务上执行实际客户端请求：账号登录、建仓、589824 字节完整上传、seal/JCS/发布、全量下载 BLAKE3 一致、单段 Range、历史查询和原操作查询。保留的 [save-operation.json](save-operation.json) 已按原始 `docs/contracts/core.schema.json` 的 SaveOperation 定义、Draft 2020-12 与格式检查验证通过；没有修改规范来迁就实现。该文件只包含随机业务 ID、发布保护证据及检查时间。

另一轮真实 TCP 验证通过：完整分支、仅关联 merge-link 且 HEAD 不变、命名空间独立 mkdir/rename、独立 JCS/BLAKE3 实现生成摘要、compare、purge 后内容 410 且保留历史图、登录不会使业务预览失效、只读权限拒写、重复 JSON 键错误带 request_id、取消后禁止执行、split、Cookie CSRF 与跨源拒绝、禁用账号撤销已有 token。随后新增的历史计划、策略和准入检查由上述最终 4 项真实 Engine HTTP 测试覆盖。

最终二进制再次经过实际 Rust CLI/client → TCP → HTTP → 三盘 Engine：544000 字节完整保存、原始 SaveOperation schema 验证、校验下载、磁盘保留的固定请求重发、历史计划原子分支、Range 和退出均通过，见 [native-tcp.json](native-tcp.json)。

CLI 已通过实际二进制验证：开发池初始化、建仓、完整保存、固定版本读取；新系统盘从数据盘恢复并读回原内容及认证；重复输入同盘不能凑成元数据多数；删除本地认证后从权威快照恢复；单盘离线检查和离线导出字节一致。最终扩展命令实测另见 [cli-smoke.json](cli-smoke.json)。

浏览器 WASM、同工作区中文目录上传、下载、窄屏、退出刷新和静态交付证据独立保存在 [Web 验收目录](../web/README.md)。
