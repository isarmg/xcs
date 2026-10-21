# 序列化组件功能命名

## 范围

| 原组件 | 当前组件 | 原始版本 |
| --- | --- | --- |
| serde | data-serialization | 1.0.229 |
| serde_core | serialization-core | 1.0.229 |
| serde_derive | serialization-derive | 1.0.229 |
| serde_json | json-codec | 1.0.154 |
| serde_urlencoded | form-codec | 0.7.1 |
| serde_path_to_error | serialization-error-path | 0.1.20 |
| serde_spanned | source-spanned-value | 0.6.9 |

七个源码包及六个已有测试根协调改名；另外 30 个既有组件中的 50 个源码和 47 个测试文件按用途重命名，共 97 处内部路径变动。基线 11,275 个 src/tools/test 文件逐字节摘要审查并双射映射，无旧源树复活。根锁文件在当前工作区离线重建，没有复制实验工作区锁文件。

## 稳定数据与内部 API

- 派生辅助属性统一为 serialization，宏生产者、消费者、derive-where 和 zerovec 重导出路径同步调整，无旧属性兼容别名。
- 版本后缀私有模块改为稳定的 __derive_support；运行时、核心和派生实现要求相同版本，运行时对核心和派生依赖均精确固定。门禁拒绝混用版本或放宽固定约束。
- 六个既有私有序列化协议标记及出现次数保持原字节；四类 slot 与码点树的原序列化名称通过明确属性保留，Rust 类型改名不改线格式。
- random-generator-core 的旧 serde1 能力统一为 serialization 特性，依赖激活关系保留。标准 no_std/alloc/rc 能力仍验证。
- 独立外部 API（例如 value-bag from_serde1、缺失的原上游测试提供者名称）不被伪造为本仓库的新组件；既有不可运行的可选上游组合不记作通过。

## 回归与来源

- 独立复核未发现额外高/中优先级语义问题；发现的 sysinfo CI 旧特性名已改正并有生成 manifest 的回归测试。
- 真实工作区的鉴权、契约、客户端、界面和宏测试共 256 个运行用例及 57 个文档用例通过。
- 新维护门禁运行原生数据/宏测试、标准 WASM、真实文档示例、no_std 原生/WASM，并要求旧辅助属性和仅核心直接派生组合失败。它接入 foundation CI，不是未调用的测试文件。
- 原始 libc/ring 源码与测试字节不变；原有 720 项运行时来源哈希重放通过。来源、版本、许可证、原始文档和既有历史收据不改写。
- 保留两份 allocator-api2 原有 CRLF；精确 Git 属性识别 CR 行尾，其他空格检查仍启用，没有靠全局忽略空白绕过检查。
- 本地静态门禁、格式检查、Clippy、原生全目标与发布 WASM 检查通过；Linux 系统后端的真实可选组合与三个运行用例通过。完整生产打包已成功，CSR 范围检查单独执行。
- 远端 CI 以本批提交对应的运行记录为准，不把上一批的通过结果记作本批验收，也不以本地编译冒充真实浏览器执行。

规范与已实施组件见 [命名规范](../NAMING.md)。
