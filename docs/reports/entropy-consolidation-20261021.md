# 系统熵实现合并（2026-10-21）

## 变更边界

将 ahash、jsonschema、jsonschema-value、referencing 的 getrandom 0.3.4 依赖合并到已保留的 0.4.3；锁文件不再解析 0.3.4。ahash 的默认 `std` / `runtime-rng` 不变，三个 schema 组件的 wasm32 专用 `wasm_js` 启用边不变。

保留 getrandom 0.2.17：ring 和 random-generator-core 仍使用旧 `getrandom()` API，后者还依赖旧错误码转换契约。不能仅改版本或目录后宣称兼容。原始身份、版本、许可证与来源继续保留在合规记录中。

## 安全契约

- 非空请求仅使用现有 OS CSPRNG / Web Crypto；不加入时间种子、Math.random、固定值或其他非密码学替代。
- Linux 仍保留现有 getrandom 系统调用和经就绪检查的 OS `/dev/urandom` 兼容路径。失败继续返回错误；ahash 默认初始化失败继续 panic，不静默降级。
- 浏览器仍使用 `globalThis.crypto.getRandomValues`、65,536 字节分块与异常捕获。缺少 `wasm_js` 时拒绝编译，不自动扩大默认功能。
- 空缓冲区返回成功的既有契约不变。`fill`、`fill_uninit`、`u32`、`u64` 的迁移调用兼容；0.4 新增 `core::error::Error` 支持和公开 RawOsError，不用于替换 0.2 数值错误契约。
- 0.4 的最低 Rust 版本为 1.85，低于本仓库 1.99 基线。已检查没有活动配置依赖已移除的 `getrandom_backend="wasm_js"` 选择器。

## 已验证证据

在独立源码副本、离线工具链和最多两并发任务下：

1. 原生熵 API、ahash/schema 验证和错误转换共 3 项测试通过。
2. 新维护的一方 `test/ci/fixtures/entropy-fail-closed.rs` 由 `check-entropy-consumers.py` 运行，2 项测试通过：强制 unsupported 后端时缓冲区不变、非空 API 返回错误、空请求保持成功；ahash 默认种子初始化明确 panic。
3. 新门禁验证 wasm32 未启用 wasm_js 时只有预期编译错误；启用后成功编译。四个迁移组件另外已在不额外启用测试入口 wasm feature 的情况下成功编译，证明既有消费者功能启用边仍有效。
4. CI suite 接线契约 7 项测试通过。foundation suite 明确执行新门禁，不是未接入测试。

这些是独立副本结果，不替代合并后的完整集成验证。WASM 检查是编译检查，不声称已执行浏览器 Web Crypto 或浏览器失败注入。浏览器运行时 smoke 仍是独立集成门禁。

## 退役与历史证据

活动后端矩阵调整为保留的 0.2/0.4：9 个原生模式、2 个 WASM 编译检查、3 个已退役后端拒绝检查。原始 runtime/platform 证明不能修改、跳过或因路径缺失静默放行。

0.3 活动源码 19 文件、65,595 字节，外置测试 1 文件、8,132 字节。退役必须使用精确上一份 package/external-owner 回执，保存有固定摘要的离线历史字节归档，并继续重放原有平台测试迁移与 runtime 哈希证明；历史归档不得重新成为构建依赖。退役文件删除、目录登记与归档验证由同批治理流程完成，不能仅凭锁文件合并视为完成。

## Applied accounting and integrated gates

The exact retirement plan is applied:323 active source trees and79 retired identities. Native consumers pass both maintained failure/API tests; the actual backend matrix passes9 native modes,2 WASM compilation checks and3 expected retired-backend rejections. Metadata verification passes. The 20-file final source/test corpus is retained only as a bounded authenticated historical archive (73,727 raw bytes,35,499 compressed); all720 unchanged historical runtime hashes still replay, and original proof files remain byte-identical. OS/Web Crypto failure remains fail-closed; no non-cryptographic fallback was introduced. Browser runtime behavior is still a separate CI result.

完整静态门禁、维护包格式/Clippy、宿主全部目标诊断及 release WASM 检查在整合后通过。上述是当前源码的本地结果；对应提交的真实浏览器与完整远端 CI 分别核对。
