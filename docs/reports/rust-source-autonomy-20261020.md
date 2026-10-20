# Rust 第三方源码自主维护与入口收敛记录

> 历史快照：本报告记录 2026-10-20 单仓库整体整合之前的源码自主化与入口收敛工作。文中的 11 个独立工作区、405 个来源目录、旧目录或当时测试计数仅属于该时点，不代表当前目录、工作区数量或本轮执行结果。部分链接已迁到中央文档位置，原始来源身份保持不变。当前结构及后续验证见[单仓库整合与集中溯源记录](monorepo-integration-20261020.md)和[现行单仓库说明](../monorepo.md)。


记录日期：2026-10-20。当前用户入口为 **Web 与独立 SMB**；CLI 保留为内部管理、运维和自动化工具。本报告记录本地代码和验证结果，未提交、推送、部署或发布。

## 交付范围与源码管理

Web 经 HTTPS 业务 API 访问；SMB 独立经 Samba → 服务端桥接 → 存储核心访问。移除原生 App 工作区、host/bridge、原生测试和打包入口，以及仅供这些入口使用的六个本地 fork：glib、glib-macros、gtk3-macros、tauri-plugin-opener、proc-macro-error2、proc-macro-error-attr2。浏览器兼容和实际 SMB 客户端组合仍需验收。能力接口现在精确声明 `platforms={"web":true,"smb":true}`，HTTP 集成测试固定该边界。

最终纳管 **390 个 registry 源码目录，加 15 个本地 fork／提取组件，共 405 个目录**；登记 11 个工作区、13 个工作区与目标组合。13 个组合不是 13 种已验证平台。直接、传递、平台条件、构建脚本及过程宏依赖均按真实 Cargo 闭包、角色和 features 登记，Git 来源依赖当前为零。

- [统一来源清单](../THIRD_PARTY_RUST_SOURCES.md)记录原仓库、发布标识、许可、用途、本地位置和修改；[精确回执](../../compliance/rust-sources.json)记录逐文件、manifest、锁文件和配置哈希。
- [维护工具](../tools/rust-sources.md)拒绝未登记漂移；Cargo 通过仓库内源码替换离线构建。元数据闭包检查使用空 Cargo home。
- registry 区保持登记的原始发布内容；定制放入独立 fork／内部组件。重导入按原始归档 SHA-256、安全路径及逐文件回执重建到独立空目录，不覆盖维护树；只重现清单中已有的 Cargo VCS 元数据省略。
- Rust 工具链源码不纳管。SQLite、Samba、系统库及 SDK 继续按既有原生方式管理；本轮未安装、重建或升级 Samba SDK。

## 具有业务价值的定制

| 组件 | 修改及实际用途 | 验证边界 |
| --- | --- | --- |
| [Web CSR fork](../upstream/zxs-web-core-0.8.19/README.md) | `ForSnapshot` 整合逐行 owner 与 keyed DOM 调和：稳定 key 下只更新变化快照，保持传输行、展开详情和焦点；重复 key 在发布前拒绝 | 自有快照回归及真实 Chrome 上传状态更新检查 |
| [Unicode fork 与命名](../components/naming.md) | NFC → 固定 Unicode 16 全量折叠 → NFC 改为迭代器流水线，配合 ASCII 快路径与最终输出预留，减少完整中间字符串；保留既有键字节语义 | 1,112,064 个 Unicode 标量及 38,925 组折叠上下文比较通过；组合字符耗时存在波动，测量及限制见该 README |
| [ZXS Web 绑定生成器](../upstream/zxs-bindgen-0.2.129/README.md) | 从 wasm-bindgen-cli 0.2.129 提取正式绑定命令，移除通用测试服务器、其他命令和网络依赖；保留同版支持库及输入/schema 校验 | 来源逐文件映射、自有参数回归、冻结 WASM 检查、正式 Web 构建与真实浏览器运行 |

稳定且仍必要的通用实现保留原有算法与安全、错误和平台分支。编译辅助代码不作形式化的大规模重写；来源可控、运行时定制、实际构建和设备验收分别记录。

## 已执行验证

以下均为本轮本地结果；临时日志位于当前执行环境的 `/tmp`，不是已归档的发行证明。长期可复现入口为仓库中的维护脚本、测试及工作流。

| 检查 | 结果与范围 |
| --- | --- |
| 全量归档重导入 | 390 个 registry 包从现有归档缓存离线重建通过；逐文件及 Cargo 回执匹配。日志：`/tmp/zxs-all-rust-restore.log` |
| 精确来源与闭包 | 405 目录、锁文件、同步来源文档及全部登记目标元数据通过。日志：`/tmp/zxs-final-source-verification.log` |
| 根工作区 | 测试输出共 532 项通过（含隔离子进程结果）；另将真实超过 64 MiB 读取上限的历史编码回归单独串行执行，1 项通过。大历史日志：`/tmp/zxs-large-history-test.log` |
| Web | 当前 161 项通过；冻结 WASM 检查、生产构建和使用最终 CLI＋本地精简绑定器的真实 Chrome 16 项通过；优化 release 服务端另跑同一 16 项也通过（`/tmp/zxs-final-release-browser.log`） |
| SMB | 协议测试 6 项通过；实际缓存 Samba 4.23.0 SDK 下 VFS 7 项及严格 Clippy 通过。日志：`/tmp/zxs-local-smb-vfs-tests.log`、`/tmp/zxs-local-smb-vfs-quality.log` |
| 来源及脚本回归 | Python 来源／审计／导入安全 39 项、构建脚本 8 项通过；拒绝路径逃逸、链接、归档篡改、来源漂移及隐式在线获取 |
| 发行流程模拟 | 8 种场景通过；JSON 守卫使用 Python，覆盖外来或格式错误的草稿拒绝。只测试模拟发布流程，未创建或发布 Release |
| 安全与质量 | 11 个消费者锁文件及 15 个本地上游身份扫描无匹配发现，未忽略公告；全部规定质量检查通过。结果仅针对本轮取得的公告数据库和已登记身份，不表示零漏洞或已完成人工全面审计 |
| 文档契约 | 2 份 schema、30 条 OpenAPI 路径、71 组语义向量通过；本报告写入后复查 838 个本地链接通过，生成附件缺失 0；不代替存储故障或平台实测 |

最终 Web WASM：**2,207,731 字节**，SHA-256：

```text
11b39e282be95f61197cbc6c4c35ee34dafe8d28762151582aa9d82e824eabc1
```

SDK 验证复用现有配置树及生成头文件，额外头文件通过 `-idirafter target/sysroot/usr/include` 和对应架构目录提供，库路径使用实际 `bin/default/lib/{tevent,talloc,replace}`。最终 VFS 共享库已确认导出 `samba_init_module`。没有改写旧 SDK 缓存符号链接、变更 ABI 或重新配置原生获取方式。这证明该缓存配置下的 VFS 构建与回归，不证明所有 Samba SDK、客户端或物理盘环境通过。

## 尚未关闭的事项与发布状态

`convert_case_extras 0.2.0`、`drain_filter_polyfill 0.1.3`、`erased 0.1.2` 在登记的上游 commit 中未提供独立许可文件。完整源码及包内许可元数据已保留，人工版权／通知复核仍为 **OPEN**；没有生成缺乏来源依据的版权声明，也不声称许可审查完成。

| 优化发行构建 | 最终本地结果 | 执行日志 |
| --- | --- | --- |
| Linux 服务端／内部管理 CLI release 构建 | 空 Cargo home、`--frozen --release` 通过；可执行文件 `--help` 通过 | `/tmp/zxs-final-server-release.log` |
| Samba VFS release 构建 | 原有 SDK 配置下，空 Cargo home、`--frozen --release` 通过 | `/tmp/zxs-final-smb-vfs-release.log` |

本轮工作期间 `media/` 被其他进程删除；该删除不计入本次原生 App 清理成果，也未据此重建或重渲染历史视频。历史报告保留其执行时点与范围，不能将旧宿主记录解释为当前产品承诺。

真实物理设备故障、掉电、完整浏览器与 SMB 客户端矩阵继续按[验收标准](../22-acceptance.md)交付。已通过的源码、模型、构建和本地测试不等于已部署或正式发行。
