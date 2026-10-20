# 分支登记与首次写入验证

> 历史范围说明：2026-10-20 起产品仅保留 Web 与 SMB；本文 App/Tauri 的路径、命令和结果属于移除前记录，不再作为当前构建或验收要求。

本轮实现 2026-10-19 确认的分支语义：创建只发布记录和零字节占位；首次真实写入才产生该分支自己的完整版本。详细规范见[分支说明](../../reports/lazy-branches-20261019.md)。

切换到待写入分支后，保存文件自动完成首次内容发布及状态转换，不需要再次创建或确认；仅暂存或已放弃的工作区不生成正文，结果未明时查询原操作。

权威状态为已发布的 `Branch.pending_file_versions` 和历史 `Commit.pending_file_versions`，占位文件是可重建投影。没有把空文件伪装成内容版本；首次明确保存空内容会生成合法的零字节版本。已有旧分支及旧快照省略新字段，读取后规范字节保持兼容。

## 验证范围

- 核心：无复制登记、目录逐文件登记、选择、原来源谱系、首写相同内容／空内容、旧会话目的分支隔离、范围写从空开始、历史编辑、重启和可重建占位。
- 存储：占位权限、名称跟随、拒绝非空文件和链接、发布断点恢复、历史包与独立拆分；首次写入后回读旧登记快照仍无正文。
- HTTP / SMB：待写入状态、不回退来源内容、打开后直接关闭不生成版本、实际写入和显式空截断；冲突保留暂存内容。
- Web：实际生产 WASM、全部 API 拦截的独立场景，见[Web 检查](lazy-branches/web-README.md)。检查未访问真实账户或用户文件。
- 机器契约与中文规范：[契约检查](lazy-branches/contract.md)。旧候选验收记录保留在原目录，不作为本轮结果。

## 已执行结果

| 范围 | 结果 | 记录 |
| --- | --- | --- |
| 原生工作区 | 238 项通过：含核心 73、服务端 52、真实 HTTP 12 及其余共享层与 CLI | [原始日志](../../../tests/evidence/lazy-branches/native-workspace.log) |
| 原生 App 桥接 | 11 项通过，共享契约变更后重新编译及执行 | [原始日志](../../../tests/evidence/lazy-branches/app-bridge.log) |
| Web | 82 项测试通过；两场实际候选 WASM 共 6 组行为通过 | [Web 验收](lazy-branches/web-README.md) |
| SMB 服务端 / 协议客户端 | 12／6 项通过；服务端结果已包含在原生工作区中 | [服务日志](../../../tests/evidence/lazy-branches/smb-server-tests.log)、[客户端日志](../../../tests/evidence/lazy-branches/smb-client-tests.log) |
| 快照与存储专项 | 11 项通过；属于核心回归子集，不重复累计 | [专项日志](../../../tests/evidence/lazy-branches/snapshot-storage-tests.log) |
| 机器契约 | 8 项测试通过，47 组字段/语义向量及文档链接检查通过 | [契约验收](lazy-branches/contract.md) |
| 原生构建 / 可移植 server | 锁定离线原生构建、无默认功能检查通过 | [构建日志](../../../tests/evidence/lazy-branches/native-build.log)、[可移植检查](../../../tests/evidence/lazy-branches/server-portable-check.log) |

独立审查记录：[核心边界复核](lazy-branches/review/cross-review.md)、[Server/SMB 适配](lazy-branches/review/server.md)、[占位存储](lazy-branches/review/storage.md)。复核发现的选择代理、范围基底、历史编辑兼容及历史占位回读问题均已修复，并进入最终回归。

[候选收据](../../../tests/evidence/lazy-branches/candidate-receipt.json)记录本轮核心源文件、原生二进制、WASM 和构建清单的 SHA-256；[Web 收据](../../../tests/evidence/lazy-branches/web-candidate.json)及[契约收据](../../../tests/evidence/lazy-branches/contract/source-receipt.json)分别记录各自输入与输出身份。最终补充 Commit 快照字段后，Web 重建的 WASM 字节与已经验收的候选完全一致，仅构建清单输入摘要更新，没有据此增加浏览器执行次数。

验证使用临时目录、临时 HTTP/Unix 套接字及独立静态浏览器夹具。需要主机权限的 FIFO、所有权及 Unix socket 测试获批后运行，不将沙箱拒绝算作通过。SDK、物理盘和六平台实机验收不因本轮回归而被声明完成。

本地预览已加载最终原生二进制及 Web 包，并成功恢复已有历史；[实际资源检查](../../../tests/evidence/lazy-branches/served-assets.json)确认 HTTP 200、进程二进制及 17 项 CSS/WASM 响应均匹配候选。未在用户数据上创建测试分支或上传测试内容。
