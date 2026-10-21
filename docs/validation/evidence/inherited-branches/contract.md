# 分支继承与自有内容契约检查

本轮采用最新澄清：创建仍只登记记录和空占位，未写入文件读取创建时冻结父版本，首次实际写入才拥有独立 FULL。目录读取组合未写文件的父版本和已写文件的自有版本。

机器契约区分：

- `version_id`：可见真实版本；继承时等于冻结父 ID。
- `materialized:false`、`inherited:true`、`own_version_id:null`：尚未拥有正文，`content_state:PENDING` 只表示归属，`visible_content_state` 是父版本实际状态。
- 首写后 `materialized:true`、`inherited:false`，自有与可见版本 ID 相同；`source_version_id:null`，实际谱系在真实版本中保留。

固定向量和定向测试覆盖映射 ID 与作用域、快照父条目一致、可见父不能误标为已拥有、真实零字节 FULL、父内容 PURGED、目录兼容字段及旧 Branch/Commit 空映射省略后的规范字节与摘要。来源内容不可用不伪造为空；活动继承读取依赖的清理保护由存储核心专项验证。

本轮实际通过 **8 项工具测试**、**52 组字段/语义向量**、3 个 JCS 向量、2 个二进制清单向量、5 个严格 JSON 拒绝样例；2 份 schema、20 条 OpenAPI 路径与当时 541 条本地链接检查均为 PASS。

结果保存在 [check.json](../../../../test/evidence/inherited-branches/contract/check.json)、[tests.log](../../../../test/evidence/inherited-branches/contract/tests.log)；[source-receipt.json](../../../../test/evidence/inherited-branches/contract/source-receipt.json)固定本轮源文件身份。本轮新记录不覆盖[之前禁止继承读取的方案](../lazy-branches/contract.md)及其日志。

```sh
cargo test --locked --offline --manifest-path tools/contract-check/Cargo.toml --target-dir target/native
cargo run --locked --offline --manifest-path tools/contract-check/Cargo.toml --target-dir target/native
```

工具只检查 schema/OpenAPI、固定向量、配置及文档链接，不启动业务服务或访问用户数据盘，不能证明实际 API、SMB、物理断电或六平台实机。完整结果见[本轮业务验证](../inherited-branches.md)和[最新规则](../../../reports/lazy-branches-20261019.md)。
