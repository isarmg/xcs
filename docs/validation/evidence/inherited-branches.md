# 待写入分支继承父版本验证

本轮按 2026-10-19 最新确认的规则更新：创建只登记记录和零字节占位；切换后读取登记时冻结的父版本，首次写入自动生成自己的完整版本。目录按文件分别继承或拥有内容，未写文件不跟随父分支后续 HEAD 改动。当前语义见[说明](../../reports/lazy-branches-20261019.md)。

继承文件的 `version_id` 是当前可见父版本，`inherited:true`、`materialized:false`、`own_version_id:null`；写入后可见版本和自有版本相同，不再标记继承。占位文件仍是零字节投影，不作为下载正文或虚构版本节点。

- 核心和 HTTP 检查三文件目录：全部继承 → 只写 A → A 自有、B/C 仍继承，父文件不变，重启保留混合视图。
- 范围写及 SMB 编辑基于父内容，打开后不写直接关闭不会物化；显式空截断及同字节首写仍生成自己的版本。
- 继承中的父正文是活动读依赖，正常清理会拒绝；已在登记前清除的父版本如实不可读，但仍可用完整新内容替换。
- 历史快照、导出、恢复和拆分保留各文件的继承标记；来源和自有内容分开。
- [Web 验证](inherited-branches/web-README.md)使用真实 WASM、全部 API 拦截的独立夹具；[契约检查](inherited-branches/contract.md)单独记录结构、向量与中文规范检查。

## 实际检查结果

| 检查 | 结果与证据 |
| --- | --- |
| 原生全工作区 | **239 项通过**，含核心 74、服务端 52 和实际路由 HTTP 集成 12；见 [完整日志](../../../tests/evidence/inherited-branches/native-workspace.log) |
| Linux 预览二进制 | `cargo build --locked --offline -p storage-admin` 通过；见 [构建日志](../../../tests/evidence/inherited-branches/native-build.log) |
| SMB 服务端及客户端 | 服务端 12 项包含在原生工作区结果中；独立桥接客户端 6 项通过，见 [服务端日志](../../../tests/evidence/inherited-branches/smb-server-tests.log)、[客户端日志](../../../tests/evidence/inherited-branches/smb-client-tests.log) |
| 存储专项 | 容器 6、历史包 4、维护 7，共 17 项通过，属于核心测试子集；见 [专项日志](../../../tests/evidence/inherited-branches/storage-tests.log) |
| Web | 84 项 Rust 测试、生产 WASM 构建和两场浏览器共 10 组行为通过；见 [Web 说明](inherited-branches/web-README.md) |
| 机器契约 | 8 项工具测试及 52 组字段／语义向量通过；见 [契约说明](inherited-branches/contract.md) |

HTTP 场景验证实际父正文下载、目录混合视图和首次写入；SMB 场景验证从父正文进行局部编辑，无写入打开／关闭及中止不增加自有版本，显式空截断增加自有版本。核心检查进一步覆盖父 HEAD 后续变化、混合视图重启、同字节首写、历史快照与写入目标冲突。[服务端复核](inherited-branches/review/server.md)、[存储复核](inherited-branches/review/storage.md)分别记录这些边界。

首次全量运行遇到 HTTP 测试丢弃 Router 后立即恢复与后台采样释放锁的时序竞态；只对测试夹具增加最多 2 秒、仅针对实际占用锁的等待，没有改变生产逻辑或业务断言。[原始失败](../../../tests/evidence/inherited-branches/native-workspace-initial-lock-race.log)保留，[修复说明](inherited-branches/http-reopen-review.md)及修复后的专项和最终全量日志分别记录，不将失败改写为成功。

[候选身份](../../../tests/evidence/inherited-branches/candidate-receipt.json)记录本轮源码与 Linux 二进制、WASM、Web 构建清单摘要；Web 及契约另有各自收据。这是本地检查和预览候选身份，不是完整源码分发包或发行证明。现有预览服务只做候选替换和只读核对，不使用真实用户数据作为业务测试夹具；实际业务测试均使用临时目录，浏览器业务接口全部拦截。

预览服务已在 `http://127.0.0.1:7400` 完成恢复并返回 HTTP 200。[实际交付核对](../../../tests/evidence/inherited-branches/served-assets.json)确认运行中二进制与候选摘要一致，实际返回的 16 份 CSS、1 份 WASM 与构建字节一致，MIME 正确，CSP 保持严格。该检查只读取首页、静态资源和进程身份，没有用现有账户执行业务写入。

此前 `lazy-branches/` 的不可读／空视图记录属于被本次纠正取代的中间实现，不作为当前验收；原文件保留，不改写历史证据。以上结果不代表六平台实机、Samba 守护进程或物理断电验收。
