# URL 与编码职责包

五个来源合并为 url-encoding，无旧包兼容别名。模块分别为 percent（字节转义）、form（表单查询）、domain（IDNA/Punycode）、address（WHATWG URL）和内部 unicode_backend（ICU 属性适配）。业务调用使用职责模块名；协议、Punycode 与 Unicode 安全检查规则未改写。

默认仅 std/alloc，不默认启用 URL 解析器。无默认特性可使用 core-only percent；alloc 增加拥有数据；form 只启用 alloc；domain 才启用 ICU 与编译数据；parser 加上 domain/form。serialization 显式启用 parser 和序列化。既有调用者按实际职责显式选择特性，optional form/domain/parser 仍由原调用者的能力开关控制。纯 percent/form 不会通过同包依赖隐式引入 ICU。

来源与版本：url 2.5.8、idna 1.1.0、idna_adapter 1.2.2、percent-encoding 2.3.2、form_urlencoded 1.2.2。原始许可证、源码身份、清单及变更前哈希保留于 compliance/provenance，五来源路由和外置测试精确对应由合并收据验证。MIT/Apache-2.0 原文件头不删除。

验证入口 .github/scripts/check-url-encoding.py：独立工作区隔离六组 feature，在 Linux amd64 与 browser WASM 编译；检查 dependency graph 无旧包、低层能力无 ICU，负向检查无 parser 时不能导入 address；运行保留 URL 单元/WPT、IDNA unitbis、percent/ICU 属性全 Unicode scalar 单元，以及字节全域往返、无效 UTF8、表单、Punycode 原 JSON 向量和安全边界回归；最后检查真实 reqwest/storage-client 及 web-app WASM。

未声称执行的覆盖：依赖 tester/assert_matches 的旧 IDNA runner 保留但此入口不执行；未运行浏览器引擎中的 WPT。原 URL 清单的 debugger_visualizer 引用不存在于本项目的 natvis 文件，该不可用集成不延续；expose_internals 仍是显式 parser 能力。

域名能力直接启用匹配的 Unicode 编译表，不再暴露只能重复转发的 compiled_data 开关，也不保留模块内恒成立的特性分支。实际路由查询和代理认证编码回归由 check-url-http-consumers.py 执行，确认 form/percent 消费路径不引入 ICU。
