# Rust 语法、源码排版与 CPU 特性检测命名迁移

三套重复实现先完成合并和行为回归，再对存续实现整体改名。本次改名不再按消费者臆造不同角色，也不使用版本、代际或字母后缀。

| 当前目录 / Cargo 包 | Rust crate | 原始身份 |
| --- | --- | --- |
| rust-syntax | rust_syntax | syn 3.0.9 |
| rust-source-format | rust_source_format | prettyplease 0.3.0 |
| cpu-feature-detect | cpu_feature_detect | cpufeatures 0.3.1 |

## 实施边界

同步更新活动依赖键、包名、库名、特性转发、宏生成路径、私有重导出、示例、外置测试、维护门禁和消费者清单。语法解析辅助函数及错误别名采用 syntax 命名；attribute-derive 的 syntax_impls 模块和排版示例 output.rust_source_format.rs 同步搬迁。生成的 token CSS 选择器与新 crate 路径保持一致。bindgen 的排版选择项同步改为 Formatter::RustSourceFormat，配置字符串为 rust_source_format，并通过真实 C 绑定生成回归。

排版器的 Cargo links 改为 rust_source_format，并以固定功能名校验；它用于构建元数据互斥和路由，不是原生链接库 ABI。包版本及原始来源版本保留；不再将版本号拼接进 links 名。

## 明确保留的边界

- 上游 URL、许可证、版权、原始文档、归档版本和哈希、不可变来源键及历史合并/搬迁收据不变。来源链接继续指向版本化的上游原件。CPU 检测库的活动 rustdoc 使用维护文档中的新名称示例，避免直接编译旧品牌的上游示例。
- rust-syntax 中 syn-internal-codegen 的 generated-by 注释记录实际原始生成器，不是当前构建依赖或当前活动模块，保留该来源标记。生成后的 Rust 路径和 CSS 已使用当前名称。
- linux-raw-sys 的 syn 与 syn_raw 是 Linux TCP SYN 协议位访问器，与语法解析库无关，保持系统接口语义。
- CPUID 指令、AVX2 特性字符串、OSXSAVE/XSAVE 与 XCR0 检查、标准密码算法名称、数据库及 WASM/JavaScript ABI 均不因品牌匹配而改动。此命名提交不改变安全分派策略。
- synstructure 是独立的宏构造组件，并非本次三项存续实现的内部路径；没有将其名称当作字符串前缀误改。

## 验证约束

维护门禁必须实际运行 tracing 不安全函数与 impl Trait 插桩、typed-builder 泛型默认值和 mutator 测试、SQLite FnPtr 真实生成、排版 golden、CPU 检测一致性及 Argon2 PHC 测试；axum 借用 receiver 必须得到预期拒绝。另检查 browser WASM 编译、Linux amd64 消费者组合及来源元数据。

来源管理为三项根目录改名生成精确前后清单，其中排版示例有显式文件搬迁映射；attribute-derive 内部文件另有搬迁收据。所有当前外置测试内容摘要重新计算，历史来源摘要不变。版本和来源始终可由 compliance/provenance/origins.json 与生成的 THIRD_PARTY_RUST_SOURCES.md 追溯。

## 本批复验结果

主工作区的 86 个迁移运行用例、五个真实消费者用例、313 个源码治理测试、106 个 CI 工具测试和五个性能工具测试通过。新消费者用例直接解析完整 AST 节点，防止 syntax-full 特性仅改 manifest 而实现分支未启用。格式、Clippy、原生全目标及发布 WASM 检查通过；候选还通过 112 个文档用例。

全量 UAPI 检查发现并拦截了 TCP SYN 同名误匹配，相关 Linux 绑定文件已恢复为原始字节，未改写不可变检查或把误改提交出去。其他被修改组件已逐项核对为实际语法/排版/CPU 消费者或相关文档引用。
