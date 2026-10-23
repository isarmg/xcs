<a id="schema-capability-boundary"></a>

# Schema 能力边界

`schema-validation` 整合 schema 值、ECMA 模式转换、引用注册表及离线 JSON Schema 验证。实现模块私有，消费者使用 `values`、`patterns`、`references` 或根验证 API。

<a id="features"></a>

## 特性

- 无特性：空的 `no_std` 能力边界。
- `value`：泛型类型化 JSON 访问、比较及数值语义，不含验证器、引用注册表、URL 依赖或正则引擎。`json-codec` 启用 JSON 值后端。
- `pattern`：需分配器的 ECMA 语法验证/转换；正则依赖仅启用语法和 Unicode 表，不启用 `std`、匹配器或自动机。
- `reference`：URI 引用、draft 检测、注册表及自定义同步检索。
- 默认 `validator`：离线验证器，使用 value、pattern、reference 能力。
- `arbitrary-precision`：通过现有 numeric 和 JSON codec 实现精确大整数/有理数语义，不自动选择验证器。
- `jsonb`：类型化 PostgreSQL JSONB 表示；`jsonb-testkit` 另导出测试夹具使用的真实保留字节编码器。
- `resolve-async`：真实异步检索与引用解析，仅启用 futures 分配和辅助能力，不提供执行器、网络客户端或阻塞桥接。
- `idna`：通过 `url-encoding/domain` 显式启用域名规范化，不使用旧 URL/IDNA 包。
- `conformance`：类型化值访问器一致性辅助能力。

URL 编码依赖可选，只由 reference/validator 能力启用其 `std` 边；域名表需另行选择。默认检索拒绝外部 HTTP 和文件资源；需要时显式实现 `Retrieve` 或 `AsyncRetrieve`。没有实际实现的 Python、Ruby、过程验证宏与 reqwest 桥接不是可用特性或 API。

<a id="retained-evidence"></a>

## 验证与来源

内置 metaschema JSON 保持原始字节与标准标识；原始 manifest、许可证和来源证明由集中合规材料管理。

原始外置测试逐字节保存在 `test/foundation/schema-validation/upstream`。自写运行器在 `test/schema-validation`，与归属明确的历史材料分开。规范集成目标保留原 `run_case` 断言，仅用普通运行器替换不可用的注册宏；执行 31 份 JSON 夹具并断言共 1,509 个用例，不使用空实现或弱化断言。原宏依赖的单元/集成套件作为来源材料保留，不宣称专项门禁完整执行了这些套件。

运行 `.github/scripts/check-schema-validation.py` 检查能力构建、依赖图、默认及任意精度规范用例，以及真实异步/类型化 JSONB 运行测试。数值精度和模式匹配消费者门禁使用整合后的 API。
