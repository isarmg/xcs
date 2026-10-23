<a id="unicode-normalization-and-property-families"></a>

# Unicode 规范化与属性组件

两组独立治理的运行时/数据配对分别合为一个包：

- `unicode-normalizer`：ICU4X `icu_normalizer` 2.3.0 与 `icu_normalizer_data` 2.3.0。
- `unicode-properties`：ICU4X `icu_properties` 2.3.0 与 `icu_properties_data` 2.3.0。

Rust crate 名为 `unicode_normalizer`、`unicode_properties`，不保留旧 crate 别名。共享 `icu_provider`、collections、locale-core 和零拷贝类型仍独立。normalizer → properties 依赖仍可选，properties 不反向依赖 normalizer。

<a id="source-and-provenance-boundaries"></a>

## 源码与来源边界

运行时位于 `src/foundation/text-unicode/unicode-*`，原外置测试在 `test/foundation/unicode-*`，产品专项检查在 `test/unicode-data`。生产代码和生成表位于 `src`；继承构建脚本仍在 Cargo 包入口。

`src/compiled_data.rs` 仅在 `compiled_data` 下包含 `src/data/mod.rs`。所有原生成表文件、文件名、marker 版本、序列化键及内容保持字节不变。`v1` 后缀表示 ICU 数据格式，不是项目发布版本，不得改名。provider 的本地 `icu` 模块是生成表实现命名空间，不是兼容 crate。

准确来源、manifest、许可和版本历史保留于 `compliance/provenance`、`compliance/licenses`、`docs/upstream` 和不可变多来源收据。生成数据原注释记录 CLDR 48.2.1、ICU release-78.1rc、LSTM v0.1.0。继承的 `ICU4X_DATA_DIR` 构建输入及 Cargo rerun 触发器逐字节保留。

<a id="supported-product-and-ci-scope"></a>

## 支持的产品与 CI 范围

仅 provider 构建保持无编译表的 `no_std`。`compiled_data` 加入真实内置数据/provider；属性 `alloc` 通过其行为图独立测试。两个最小图均编译 Linux amd64 与浏览器 WASM。规范化默认启用编译表和 UTF-8 迭代；原默认 `utf16_iter` 所需 `utf16_iter` / `write16` 依赖早已缺失，不再默认选择这个坏配置。

不支持的历史 UTF-16、datagen、第三方 trait Cargo 接口已在独立限定后续变更中退役，序列化仍可选。完整历史 normalizer 测试文件保留精确逆向迁移证明，但因为需要不可用 UTF-16 提供者，不宣称已执行；不以假提供者或空实现制造通过。

`.github/scripts/check-unicode-data-family.py` 独立支持每个组件族：

- 按整合输入收据核对表字节与构建输入。
- 隔离检查仅 provider / compiled-data 图、原生/WASM 编译，以及仅 provider 不暴露 `Baked` 的负向测试。
- 使用未变生成表编译真实自定义数据路径。
- 遍历保留语料每行五列及四种规范化形式，并覆盖补充平面、Hangul、组合字符和 UTS-46 向量。
- 验证属性类别/script/bidi/二元属性向量；在内置 provider 与常量构造器间比较全部 1,112,064 个 Unicode 标量，并使用真实依赖运行 11 项保留属性单元测试。

URL 整合后，其门禁在 Unicode 后端运行保留的 `property_masks_preserve_all_unicode_scalars`。删除生成数据独立包不改变 ICU 标量值、属性位掩码或 IDNA 线上语义。

<a id="explicit-optional-capability-limits"></a>

## 可选能力的明确限制

normalizer 不再声明 `utf16_iter`、`write16`、`harfbuzz_traits`、`datagen`；properties 不再声明 `harfbuzz_traits`、`unicode_bidi`、`datagen`。七个包/特性组合均在编译前的 Cargo 特性解析阶段失败，无兼容别名或替代提供者。normalizer-to-properties 的 datagen 转发随特性删除，所有保留 manifest 都扫描消费者及间接转发。

休眠实现字节和历史测试保持完整。显式 `check-cfg` 承认历史名称但不启用它们。上游文档保持不可变归档；包说明及活动 crate 文档描述当前能力。

维护的 Unicode 门禁检查准确支持特性集、仅 provider 序列化、带 normalizer/property 集成或 property `alloc` 的序列化，并在原生/WASM 下检查全部支持特性组合；另执行七项隔离负向解析探针。这是支持配置的编译验证，不是序列化线上格式/运行互操作认证，也不是休眠 UTF-16、datagen、trait 集成测试的执行。标量、规范化、单元及可执行文档检查独立保留。

此变更是在相同来源所有者下的普通内容刷新，不是包退役、改名、移动或上游升级。原始身份/法律材料、整合收据及认证输入历史不变；未来改变源码所有者生命周期需独立显式适配器。

可执行文档使用真实公开 crate/模块路径。normalizer 文档配置选择 `compiled_data,utf8_iter,unicode-properties`；properties 选择 `compiled_data,alloc,unstable`，因为文档中的解析命名空间仅在 `unstable` 下公开。最小/仅 provider 图测试独立维护。仅 alloc 的隐藏解析模块文档配置不在已认证范围；原忽略示例继续忽略，不宣称执行休眠历史测试。
