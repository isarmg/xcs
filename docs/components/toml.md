<a id="toml-configuration"></a>

# TOML 配置

一个 `toml` Cargo 包统一维护五个原始组件。通过模块划分能力，不保留别名包：

- `datetime`：日期、时间和偏移，始终可用。
- `span`：附有原始字节区间的值，始终可用。
- `writer`：由 `write` 启用的底层编码；`alloc` 增加分配式输出辅助能力。
- `editor`：由 `edit` 启用的保留格式模型；`parse` 引入 winnow，`display` 引入 writer。
- 根级 `Value`、`Table`、序列化/反序列化和转换 API 由 `serialization` 启用；默认保留 serialization、parse 和 display。

禁用默认特性后，datetime/span 没有第三方生产依赖。两者均不需要 editor、indexmap、winnow 或序列化引擎。单独序列化不要求编辑能力。维护原生 Linux amd64 和浏览器 WASM。统一包是基于 std 的组件，不宣称保留旧独立 writer 的嵌入式/no_std 支持。`alloc` 和 `std` 选择 writer 辅助能力，不代表整个包保证 no_std。

受支持的 `preserve_order` 和 `unbounded` 选项保留原有语义。旧 `perf` 选项引用缺失的 kstring 提供方，因此有意移除，而非模拟实现。序列化的六个私有 TOML/span 标记字符串精确保留。错误区间仍为字节偏移。editor 原有行为会将 CRLF 规范化为 LF 并补充末尾换行；整合不承诺逐字节保留 CRLF。

editor 保留原有、范围严格受限的 UTF-8 快速路径，其输入由解析器边界保证有效。所有其他实现模块禁止 unsafe 代码。公共解析器接受 `&str`，不接受任意字节缓冲。

<a id="sources"></a>

## 来源

toml 0.8.23、toml_datetime 0.6.11、toml_edit 0.22.27、toml_write 0.1.2 和 serde_spanned 0.6.9（此前以 source-spanned-value 维护）的不可变来源记录、原始清单、上游 README 和 MIT/Apache-2.0 许可证均保留。现有 serde_spanned 重命名凭据不重写。新的整合凭据将这一精确历史沿革绑定到新的归属方。

<a id="test-scope"></a>

## 测试范围

原始 16 个外部片段通过经核验的可逆迁移保留原始源码路径/哈希及归属。原始 snapbox 比较改为精确的 `assert_eq!`，输入保留。两个原始 proptest 生成器因缺失 proptest 提供方，仍作为保留来源但未启用的源码保存。另有确定性测试检查每个 Unicode 标量和显式边界字符串；这些测试不能复现随机生成、字符串组合覆盖或收缩过程。

专项测试覆盖源字节区间、格式化、畸形 TOML、Unicode 边界、日期时间精度/偏移及全部六个序列化标记名。原生/WASM 特性检查验证最小能力闭包。实际消费者为 release-tools 和 contract-check。不声称执行过上游基准或不可用的外部语料测试驱动。

foundation 套件运行 `.github/scripts/check-toml-consolidation.py`。它在全部 30 个目标/特性构建（包括完整特性并集）中拒绝 Rust 警告，检查能力反向编译和无依赖原语图，并在字节相同的临时源码副本中使用真实开发依赖执行保留的单元、边界和文档测试。只有临时夹具生成自己的锁文件；产品根锁文件及源码清单均不修改。仅解析器使用的区间存储和仅格式化使用的辅助项只随所属特性编译，而不是抑制未使用代码诊断。
