# 待写入分支文档与机器契约检查

本轮增加 Branch / Commit 的 `pending_file_versions`、泛型 Entry 和文件分支详情投影契约。创建分支只登记记录和占位，首次真实写入才拥有 FULL；旧快照中的待写入状态不随当前分支变化。文件管理 JSON 使用 `version_id:null`，泛型 Entry 的字符串字段保持原类型。

新增固定向量检查映射键/值的合法 ID、单文件作用域、快照来源与条目一致、来源不冒充当前版本；定向测试比较旧 Branch / Commit 在新增字段默认为空时的规范字节及摘要。另检查待写入响应、真实零字节版本和虚拟目录三个不同状态。

本轮实际通过 **8 项工具测试**、**47 组字段/语义向量**、3 个固定 JCS 向量、2 个二进制清单向量和 5 个严格 JSON 拒绝样例；2 份 schema、20 条 OpenAPI 路径及当时 539 条本地链接检查均为 PASS。链接数随文档变化，以本轮 JSON 输出为准。

执行结果分别保存为 [check.json](../../../../tests/evidence/lazy-branches/contract/check.json) 和 [tests.log](../../../../tests/evidence/lazy-branches/contract/tests.log)，源文件散列保存在 [source-receipt.json](../../../../tests/evidence/lazy-branches/contract/source-receipt.json)。这些文件是本轮新记录，不覆盖旧证据。工具检查 schema/OpenAPI、固定向量、配置示例和文档链接，不启动业务服务、不访问数据盘，不证明实际 I/O、断电、SMB 客户端或六平台实机。

复现：

```sh
cargo test --locked --offline --manifest-path tools/contract-check/Cargo.toml --target-dir target/native
cargo run --locked --offline --manifest-path tools/contract-check/Cargo.toml --target-dir target/native
```

完整业务范围与当前执行结果见[本轮证据](../lazy-branches.md)和[中文语义说明](../../../reports/lazy-branches-20261019.md)。
