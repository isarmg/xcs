<a id="shared-form-float-formatting"></a>

# 表单浮点格式化复用

`form-codec` 使用 JSON/schema 已依赖的 `zmij` 直接格式化 f32/f64，移除唯一的 `ryu` 依赖；不提供转发包、复制格式化器或兼容适配器。两种类型保持各自精度，不把 f32 提升为 f64。

<a id="intentional-canonical-wire-spelling"></a>

## 明确采用的规范编码

项目尚未发布，当前有意采用带正指数加号的规范形式：`1e16` 变为 `1e+16`，表单线上值变为 `1e%2B16`；f32::MAX 同理变为 `3.4028235e%2B38`。表单中的字面 `+` 会解码为空格，因此百分号编码属于契约，不是外观调整。负指数、有符号零与非有限值拼写保留。不承诺保留旧的可选正号形式。

<a id="evidence-and-boundaries"></a>

## 证据与边界

实施审计将 10,097,974 个边界生成及随机 f32/f64 输入与旧格式化器比较。每个有限候选都能解析回精确原始位模式，观察到的拼写差异仅为正指数可选加号。全部 16,777,216 个 f32 非有限编码和 12 个代表性 f64 非有限编码保持拼写。这些有限值抽样不构成覆盖全部 f64 输入的证明。

`python3 .github/scripts/check-form-floats.py` 检查五项专项行为测试、十项保留序列化测试与九项保留反序列化测试。它执行真实原生 reqwest 表单/查询构造及 http-routing 消费，覆盖百分号编码与保持位模式的解码；同时将表单/路由测试编译为浏览器 WASM。此测试中 reqwest 明确只用于原生目标。解析后的依赖图必须包含 zmij，且不能包含 ryu。

<a id="provenance-and-size-accounting"></a>

## 来源与大小口径

原 ryu 1.0.23 身份、许可证及来源收据保持不变。`compliance/ryu-1.0.23-retirement.json.gz` 保存全部 19 份生产文件（107,241 字节，其中 18 份 Rust 文件共 106,487 字节）及十份外置测试（32,919 字节）。归档与退役目录记录、完整路径集及逐文件原始 SHA-256 核验绑定。

这些数值表示移除的活动源码，不是整个仓库节省的空间：完整不可变归档和原始来源仍保留。归档的上游 ryu 测试是保存的证据，不代表已执行这些退役套件。

检查器在 foundation-language 分片中登记一次，并由 aggregate foundation 门禁调用一次；显式命令清单和套件路由回归约束此安排。SDK、平台、SQLite、Samba、特性隔离和浏览器门禁保持不变。

完整审计重放使用 `python3 .github/scripts/check-form-floats.py --differential`，而非快速消费者门禁。它只从经认证的退役归档恢复 ryu，并运行 `test/architecture/form-floats/differential.rs` 与当前 zmij/form-codec 比较，无需 Git 历史或联网。原输出保留在 `test/evidence/form-floats/differential-native.txt`；退役后的归档参考重放再次得到相同边界/随机及非有限值数量。此可选审计不替代默认原生/WASM 消费者检查，也不会将退役代码加入生产依赖图。
