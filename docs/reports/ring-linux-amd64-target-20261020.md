# ring Linux amd64 构建输入收敛

[报告目录](README.md) · [测试材料恢复](ring-test-restoration-20261020.md) · [风格来源证明](ring-style-diagnostics-20261020.md)

## 范围与保留边界

在 runtime 正式提交 `3f3841df0f129e49ac2467d05ecd768239e90c59` 上（与准备快照 `ef39c6773264679d56fc6601fc2f7e9970b62624` 同树），仅退役 ring 的 25 份非 amd64 原生构建输入和 91 份非目标预生成材料，合计 116 文件、3,845,917 字节。文件级收敛不是密码算法优化或性能提升，也不是删除第三方身份。

保留全部算法正文、全部测试入口与向量、通用 C fallback、x86_64 生成器、两个 fiat ADX 汇编输入、所有头文件和五份共享 perlasm helper/translator。含 Windows/ARM 字符串不构成删除理由。12 份 Linux amd64 ELF 汇编和全部三份前缀头原样保留，包括 NASM 前缀头；算法、ABI、CPU 运行时分派与原有 lint 策略不改。

build.rs 的目标集合仅保留 Linux/x86_64/elf，删除 25 条非目标源入口及 ARM SHA-256 合成生成入口；28 条 Linux 选中输入顺序不变，x86_64 SHA-256/SHA-512 双输出保留。入口显式要求 Linux、x86_64、64 位、小端，不限定 libc 或 vendor；不支持的平台明确失败，不静默降级为不完整构建。其他无关编译器分支暂保留，不扩大到共享代码重构。

项目 Web 支持与 ring 支持分开：五份真实闭包中，ring 仅出现在 root-native（alloc/default/dev_urandom_fallback），Web、SMB、构建工具和验证工具 profile 均不依赖 ring；Web profile 也没有 getrandom 0.2。因此 Web 构建通过不是 ring 的 Web 验证。

不保留失效平台兼容承诺：删除 JS、ESP-IDF、两项 ARM testing 以及仅 os=none 有效的 custom-or-rdrand 共五个 feature；删除两项 ARM/Apple libc target 表，保留 Linux amd64 测试 libc。清除 include 中 35 项不存在的路径/空模式（含本次25项、此前集中许可证和空示例等），不删除中央法律资料。src/lib.rs 当前说明仅列维护中的目标/选项；src/rand.rs 将原已包含 Linux 的 cfg 列表收敛为 Linux，getrandom 调用及错误映射表达式不变。ARM Rust 算法文件不扩大修改，原 docs/upstream 文档保持历史原文。

## 历史来源与当前源码分别校验

原 test-materials、original-overlap、style-patches 三份证明逐字节不变。新增[目标裁剪证明](../../compliance/provenance/supplemental/ring-0.17.14-linux-amd64-target.json)记录精确删除/保留清单、当前哈希、build.rs 八段可逆替换、manifest/当前说明/rand cfg 的三份精确可逆记录和三个历史证明哈希；[历史只读归档](../../compliance/provenance/supplemental/ring-0.17.14-linux-amd64-retired.zip)仅保存 116 份确已退役字节，压缩后 729,998 字节，不参与 Cargo 构建。

[校验器](../../tools/rust-sources/check_ring_tests.py)固定第四证明摘要，先校验当前 331 份活动文件（328 份未改、3 份有精确补丁）和 build.rs after-hash，要求所有退役路径不存在，拒绝新增、遗漏、修改或符号链接。随后才校验归档固定压缩长度/SHA-256、唯一成员集合、普通文件类型、规范相对路径、压缩方式、总展开长度及逐项长度/SHA-256/Git blob。成员按预期长度加一个字节有限读取，不向源码树解压。

历史视图仅能读取明确退役的 116 个路径，build.rs 及三份元数据/条件说明补丁通过唯一精确反向替换重建；其余活动源仍从真实路径读取。此后继续执行原 69 材料、391 原包文件、380 历史实现及 Git commit/tree 证明。缺失的活动源码或 fixture 不能从归档兜底；旧历史摘要不被改成当前源码摘要。当前来源收据单独描述本地裁剪状态。

## 验证与覆盖边界

详细命令、退出码、日志摘要及逐文件等价记录见[机器证据](../validation/evidence/foundation/ring-linux-amd64-target-20261020.json)。

基于 `5815ea78569719d80e25ab89484ede73bdae1880` 的完整隔离工作区，构建输入裁剪候选的十组 GNU 矩阵全部通过：packaged/localgen 各执行 debug/default、debug/std、release/default、release/std、release/std+slow_tests。每组94库测试、9 doctest，集成计数为88/89/92/93/93；每组仅保留上游原有1项ignored文档示例，无失败或过滤。该矩阵在最终metadata清理之前运行，不冒充最终提交的十矩阵终验。

正式整合额外增加64位guard、收整派发空白以及上述metadata/纯说明/cfg清理。default/std/no-default三份真实native展开，仅反向恢复精确crate说明段后与候选基线逐字节一致；未删除cfg或算法输出以制造一致。最终28条有序原生输入、54份已观察编译依赖文件摘要仍相同，实际正式预生成入口的12 ELF+3 prefix全部逐字节相同；四种不支持目标明确拒绝，build.rs以-Dwarnings编译通过。

最终正式树已通过42项ring来源回归、完整static、严格quality（含workspace/all-targets及Web release）、ring std回归、strict ring all-target std Clippy、storage-server编译、20项storage-client库回归、Web production、文档契约与来源治理检查。第一次Web production在metadata仍变化时被自身source-stability门禁拒绝；源码冻结后重试通过，初次不计为验收成功。新增负测由现有unittest discover自动接入CI，未改写CI/check脚本。

普通no-default build成功，但有40项既有unused/dead-code警告，与同命令的5815候选基线逐项同序，无新增；no-default展开有同样1项unused-import诊断。它们不冒充零警告门禁，未为隐藏诊断添加allow。default/std严格门禁通过。最终提交仍须由其精确commit的CI执行完整十矩阵后才算终验，本报告不提前声称CI通过。

来源治理保持 339 个活动身份、64 个退役身份、5 个真实根闭包；五 profile 的包集合、feature、role 与 runtime 基线逐值一致，唯一根工作区及 ring 非默认验证成员不变。根锁仅删除 getrandom 0.2.17 已不可达的 js-sys/wasm-bindgen 两条可选边，包身份集合不变。不新增包、独立工作区、测试过滤或全局 lint 允许项。

本机实际执行边界仍为 GNU/Linux amd64 当前 CPU。不宣称 musl 运行、CPU 特征屏蔽、其他硬件、性能收益、CAVP 认证或新 TLS 网络握手；保留通用 fallback 不等于已覆盖所有运行时 CPUID 路径。
