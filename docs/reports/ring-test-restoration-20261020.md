# ring 上游回归材料恢复（2026-10-20）

[报告目录](README.md) · [前批基线缺口](linux-amd64-foundation-second-20261020.md) · [机器证据](../validation/evidence/foundation/ring-restoration-20261020.json)

## 范围

本批只恢复可信测试材料、真实测试入口与离线校验，不裁剪 ring 平台源码，不修改密码算法、测试断言、C/汇编、生成器或既有预生成文件。前批保留的 381 个文件中仅 Cargo.toml 增加验证入口；其余 380 个文件哈希全部保持。新增文件是必要回归材料，不以源码体积增长判断精简失败。

- 34 份库内向量，补齐原先 42 个 include 诊断所指的缺口。
- 24 份集成测试向量，包括宏拼接的 AEAD、QUIC 路径。
- 官方 cavp 的原始 shavs.rs 与实际引用的 8 份 NIST SHA-1/256/384/512 长短消息向量；SHA-224 文件不被该测试引用，没有把它计为已测算法。
- NIST README 和原压缩包校验和保持原文，按项目布局放入 docs/upstream，而不是散落在源码树。

合计恢复 69 份材料、6,737,972 字节；其中 67 份进入 ring 源码树，两份集中保存为上游文档。没有加入独立 cavp Cargo.toml、Cargo.lock、Git 仓库或 workspace。

## 来源与 dirty 状态的处理

唯一获取来源是原 manifest 声明的官方 [briansmith/ring 精确提交](https://github.com/briansmith/ring/tree/2723abbca9e83347d82b056d5b239c6604f786df)。没有使用 main、其他版本或手写替代向量。原发布包的 dirty:true 原样保留；原 package receipt、原始逐文件哈希和许可证没有改写。

通过官方 GitHub 只读接口取得精确 commit/tree/blob，而非把当前分支当作来源。重建 commit 原始字节得到同一 Git SHA-1，75 个 Git tree 对象重新计算均匹配；新增材料逐个校验 Git blob、长度和 SHA-256。该提交未签名，这些对象哈希与新观察到的 SHA-256 是完整性证据，不是额外的上游数字签名；本批没有声称运行了完整历史 Git 仓库的 fsck。

391 份原发布包文件的已保存 SHA-256 全部复核，其中 282 份对应精确提交的相同 Git blob；其余 109 份是规范化 Cargo.toml、发布包 Cargo.lock/.cargo_vcs_info.json 和 106 份打包生成材料。上游 Cargo.toml 与保存的 Cargo.toml.orig 完全相同。当前代码与原发布包的差异逐一解释：manifest 已内化，另 6 个 Rust 文件仅有此前显式补充的匿名生命周期，表达式体未变；这 6 份取得的上游原文同时匹配发布包原哈希与 Git blob。未发现未解释的算法、测试、C/汇编或生成器差异。

新增独立记录：

- [恢复材料与 Git 对象证明](../../compliance/provenance/supplemental/ring-0.17.14-test-materials.json)：每份 origin URL、commit、blob、字节长度、SHA-256、关联测试源码哈希，以及原有 380 文件的冻结摘要。
- [发布包重合文件复核](../../compliance/provenance/supplemental/ring-0.17.14-original-overlap.json)：391 文件的原始哈希、Git blob 映射与已解释差异；必要的 6 份上游原文保存在新记录中，供离线复核。

[校验器](../../tools/rust-sources/check_ring_tests.py) 从这些持久记录离线重建证明，核对恢复字节、原始文件、关联测试源和精确入口集合。测试前再次校验；错误哈希、树/commit 漂移、重复材料、路径越界、遗漏来源与意外实现改动都失败。

## 唯一工作区与入口

ring 现在是唯一 root workspace 的第 21 个显式成员，但不在 9 个 default-members 中，是基础库验证对象而不是新增业务组件。Cargo 不允许含 dev-dependencies 的非成员直接受测；显式登记避免伪造生产依赖或创建另一个工作区。没有给 storage-client 或 contract-check 增加测试耦合。

manifest 保持 autotests=false，并精确注册 14 个现有上游 tests/*.rs 和原文 cavp/tests/shavs.rs，共 15 个集成入口。Linux amd64 测试所需 libc 使用已经存在的本地源码；不恢复仅供 wasm 测试的 wasm-bindgen-test 或基准测试依赖，不删改原测试断言。

根锁文件实际增加 getrandom 0.2.17 指向既有 js-sys、wasm-bindgen 的可选依赖边，以便工作区成员的保留特性完整可解析；没有新增包身份。五个既有运行/工具闭包的包集合、features、角色与前批完全相同，慢测试不是默认特性。治理记录按实际 manifest、锁文件、新材料和验证成员声明刷新，339 个活动源码身份、64 个退役身份、5 个闭包保持。

## 可重复验证与覆盖边界

```sh
python3 tools/rust-sources/check_ring_tests.py --verify-only
python3 tools/rust-sources/check_ring_tests.py
python3 tools/rust-sources/manage.py verify --metadata
```

完整脚本默认在 GNU/Linux amd64 运行两种模式，每种分别执行 default/std 的 debug/release 和单独 std+slow_tests release。所有命令使用唯一根 manifest/锁文件、--locked --offline、jobs=2，不筛除测试。预生成模式使用正式源码；本地生成模式复制同一已登记工作区，在临时 ring 副本添加上游 build.rs 识别的开发标记，强制重新运行 Perl 生成器和原生编译，不触碰正式工作树。两个模式使用独立 target 目录。

隔离准备阶段十组矩阵通过；正式 root 工作区十组也全部通过。每组均执行 94 个库测试和 9 个 doctest。两个模式的 default/std 集成计数在 debug 分别为 88/89，在 release 分别为 92/93；release 原测试会额外编译 4 项大数据 digest 测试。std+slow_tests 的集成计数同 release/std，实际执行百万次 X25519 迭代。CAVP 的 8 项已经包含在集成计数中，不重复相加。每组保留上游明确 ignore 的一个文档示例，不计为已执行测试，没有额外过滤测试。

本地生成模式实际产生的 12 个 GNU ELF 汇编和 3 个前缀头文件，在 debug 的 2 套、release 的 3 套构建输出中全部与原预生成材料逐字节一致。严格 workspace/all-targets 编译诊断门禁无警告通过；20 项 storage-client 回归、no-default-features 库构建、完整 static 套件（含 99 项来源工具回归）及空 Cargo home 离线 metadata 验证通过。文档契约检查通过，核对 985 个本地链接；历史生成物缺失不冒充当前验收材料。消费者验证包含 TLS 依赖图编译，不冒充新完成的 TLS 网络握手验收。

首次进入正式本地生成阶段时，/tmp 的小容量 tmpfs 无法容纳工作区副本，复制阶段在测试启动前报 ENOSPC。脚本已改在 ignored target/ring-regression 的磁盘文件系统下建立临时副本，再以 --mode localgen 完整完成五组；没有以该环境错误跳过测试。机器证据保留这个已恢复尝试和各实际成功命令。

本机证据仅覆盖当前 GNU amd64 CPU。没有 musl 工具链/运行、CPU 特征屏蔽矩阵、其他真实硬件、wasm 测试 harness、性能基准或完整 CAVP 认证声明。编译器 target-feature 开关不能代替 ring 的运行时 CPUID 覆盖。本次不因此开始算法/平台裁剪。
