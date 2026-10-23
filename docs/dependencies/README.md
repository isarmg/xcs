# 第三方依赖文档

[文档目录](../README.md) · [来源与许可](../../compliance/README.md) · [消费者与特性](../architecture/foundation-consumers.md)

这里按当前职责组织第三方实现的文档，不按合并前仓库或版本目录要求读者逐个查找。只索引当前活动与合入实现；退役来源保留在合规材料中，不作为可用组件推荐。

## 三层资料

1. **本项目维护说明**：中文介绍实际保留的能力、接口、约束及验证方法，链接到现有架构和组件页，不另复制一套相同说明。
2. **随源码保留的说明与发布包原件**：`docs/upstream/` 保存导入时的 README 和指南，其中个别资料含原有本地补充，不一概宣称与官方发布包逐字节相同。补充取自精确发布包的原件另有身份和摘要记录；索引明确区分两者。原件不改写，其中其他平台、未启用特性、旧包名和网络安装命令不自动适用于当前定制实现。
3. **当前源码 API**：以本仓库实际公开接口和内联 Rustdoc 为准。上游没有独立说明时明确标注，不用任意较新版本的在线文档补成“同版本原文”。

当前上游身份、维护包名和当前 API 名可能不同。组内每个组件集中列出它包含的原始来源，避免将已合入模块误当成需要重新安装的独立包。版权及许可仍以[中央通知](../../compliance/NOTICE.md)与来源材料为准。

## 按职能查找

| 职能 | 当前参考 |
| --- | --- |
| 并发与异步 | [执行器、任务、Future、同步与通道](concurrency.md) |
| 集合与所有权 | [哈希表、迭代器、借用和智能指针](collections.md) |
| 密码学 | [算法、认证、熵与共享原语](cryptography.md) |
| 数据与存储 | [序列化、编码、压缩、Schema、SQLite](data-storage.md) |
| 网络 | [HTTP、路由、中间件、URI、套接字与 TLS](networking.md) |
| 平台 | [系统调用、文件系统、CPU 和系统信息](platform.md) |
| 文本与 Unicode | [文本检索、正则、Unicode 与区域数据](text-unicode.md) |
| 代码生成 | [Rust 语法、Token、宏和模板解析](code-generation.md) |
| 工程工具 | [绑定、原生编译、命令行和终端样式](tooling.md) |
| 诊断 | [错误、日志、调试和追踪](diagnostics.md) |
| 基础类型 | [数值、日期时间、标识符与标志](core-types.md) |
| 浏览器基础层 | [DOM、响应式状态、视图与 WASM](web-runtime.md) |
| 直接集成 | [已并入命名、界面、过程宏和绑定工具的来源](integrated.md) |

可选的系统 SQLite、Samba SDK、编译工具链等外部组件的操作边界分别见[SQLite 接入](../architecture/sqlite-access.md)、[SMB](../components/smb.md)与[原生编译](../architecture/native-c-toolchain.md)。它们由指定构建或部署环境提供，不能把 Rust 包的版本当作外部系统库版本。

## 生成当前 API 文档

在根工作区、使用固定 Rust 工具链生成所需组件的本地文档。例如：

```sh
cargo doc --locked --offline -p flag-set --no-deps
```

包名使用当前 `Cargo.toml` 中的名称，不使用原始上游名字。输出位置以 Cargo 提示为准；生成的 HTML 放在构建目录，不提交回仓库。`--no-deps` 只限制文档输出，不代表不编译依赖，也不证明未启用能力可用。

浏览器组件需要匹配 `wasm32-unknown-unknown` 目标与其支持的 features；原生编译依赖和过程宏仍运行于宿主。不要为了文档生成对全部包盲目开启 `--all-features`，也不要将未执行的示例算成已验证行为。

## 维护方式

组内目录由当前来源清单、当前源码入口和已保存原文生成；新增、退役、合并或改名后更新索引并验证：

```sh
python3 tools/rust-sources/documentation.py refresh
python3 tools/rust-sources/documentation.py verify
```

中文说明只保留当前用法与边界。新取得的上游文档须匹配精确来源版本，保留原文及来源校验，不覆盖已有不可变原件。历史变更表、发布日志、讨论和单次验证报告不进入当前阅读索引。
