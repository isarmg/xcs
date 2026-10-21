# 功能命名首批迁移记录

本批接续 Base64 合并后的来源图。仅在助手自己的云电脑修改；没有使用用户电脑或用户 Codex。

## 已实施

| 原活动目录 | 当前目录 / Cargo 包 | 保留的上游身份 |
| --- | --- | --- |
| src/foundation/base64-0.23.1 | src/foundation/base64 / base64 | base64 0.23.1 |
| src/foundation/bitflags-2.13.2 | src/foundation/flag-set / flag-set | bitflags 2.13.2 |
| src/foundation/itertools-0.14.0 | src/foundation/iterator-toolkit / iterator-toolkit | itertools 0.14.0 |
| src/foundation/rand_core-0.6.4 | src/foundation/random-generator-core / random-generator-core | rand_core 0.6.4 |
| src/foundation/sha2-0.11.1 | src/foundation/sha2 / sha2 | sha2 0.11.1 |
| src/foundation/shlex-2.0.1 | src/foundation/shell-tokenizer / shell-tokenizer | shlex 2.0.1 |

六个源码根与六个对应测试根同步迁移。四个包采用职责名称，SHA-2 和 Base64 保留算法/编码的功能名称；数字属于算法名时不误删。Cargo 解析所需版本及原始版本元数据仍保留。内部依赖键、库目标、Rust 引用、宏使用路径、特性转发和文档示例同步更新，没有新增一整套旧名称兼容别名。

## 来源与治理

改名记录保存精确迁移前包/测试清单及审阅后的目标清单，原始身份、归档哈希、许可证、历史收据和外置片段的原始 source 标签不改写。普通 refresh 不允许伪装改名或丢失文件；后续审阅后的实现修改更新当前清单而不覆写历史。当前仍为 325 个活动来源树，本批改名不是减少了六个依赖。

## 验证

- 离线 Cargo metadata 与五个登记闭包通过。
- Linux amd64 工作区编译通过；Samba VFS 需要真实 SDK，远端单独验收。
- 浏览器 WASM debug/release 编译通过。
- 维护包格式、Clippy、全部宿主目标诊断及 release WASM 检查通过。
- 静态检查和来源治理回归通过；远端最终 CI 以对应提交结果为准。
- shell-tokenizer 的七个直接单元测试通过。
- random-generator-core 的大种子文档示例原有死代码警告在新编译器严格模式下报错，现改为实际构造并检查 seed；五个文档测试通过，一个原有 ignored 保留。

后续界面和其他组件族仍未在本批改名，见[命名规范](../NAMING.md)。

## 同批验证器修复

远端基础库检查暴露直接调用 rustc 的测试没有 Cargo 提供的 CARGO_MANIFEST_DIR。Linux UAPI、系统调用和目标契约、libc ABI 验证器现显式设置各自真实包目录，并覆盖未设置或错误继承环境变量的回归。测试实现和断言没有删减。UAPI 的 std/no_std 各三个断言、libc ABI 的三组各 345 项核对及目标边界、27 个系统目标契约探测均已在故意设置错误继承值时通过。

首批依赖消费者定向测试最终通过：authentication 13、browser-smoke 10、password-hash 36、random-generator-core 9；其五个文档测试通过，一个原有 ignored。
