# 合并、标签与备份响应 fixture

这八份 JSON 是受版本管理的契约测试输入，不是 `tests/evidence/` 下可缺省的本地验收附件。它们于 2026-10-19 从实际服务端 HTTP 测试完整捕获，保留原始响应字段、ID、时间及整数编码，未用手写的空数组或最小样本替换。生成它们的 22 个 `http_engine` 测试全部通过。

来源是 [HTTP 测试入口](../../../../../crates/xcs-server/tests/http_engine.rs) 的 `capture_wire`：

- [file_metadata_operations.rs](../../../../../crates/xcs-server/tests/http_engine/file_metadata_operations.rs)：`file-merge-tag-history.json` 覆盖 content、rename、tags、merge 节点及多父合并记录；`scoped-backup-configuration.json` 覆盖文件继承目录策略、scope identity 和 generation。
- [backup_namespace.rs](../../../../../crates/xcs-server/tests/http_engine/backup_namespace.rs)：`backup-prefix-configuration.json` 覆盖目录前缀；`backup-prefix-files.json` 覆盖主文件改名后的固定备份路径；`backup-prefix-retained.json` 覆盖删除后的历史副本及 nullable 字段；`backup-prefix-acl.json` 覆盖授权过滤后的只读列表；`backup-mixed-routes.json` 与 `backup-route-disks.json` 覆盖历史/当前路由和读者可见磁盘。

独立 `cargo test --manifest-path tools/contract-check/Cargo.toml` 直接加载这些文件，逐份使用当前 OpenAPI 完整 200 响应 schema 校验。它不要求启动服务或预先生成 evidence。运行时业务语义仍由上述服务端测试验证，固定 fixture 的 schema 校验不能代替服务端测试。

## 更新方法

在仓库根目录将真实 HTTP 响应捕获到临时目录：

```sh
wire_dir=$(mktemp -d)
XCS_TEST_WIRE_EVIDENCE_DIR="$wire_dir" cargo test --locked --offline \
  --target-dir target/native -p xcs-server --test http_engine
for fixture in tools/contract-check/tests/fixtures/merge-tags-backup/*.json; do
  cp "$wire_dir/$(basename "$fixture")" "$fixture"
done
cargo test --locked --offline --manifest-path tools/contract-check/Cargo.toml \
  --target-dir target/contract
cargo run --locked --offline --manifest-path tools/contract-check/Cargo.toml \
  --target-dir target/contract
```

只有 HTTP 测试通过后才更新；检查完整 diff 与场景覆盖，不要把 schema 失败通过删除字段、清空集合或跳过缺失 fixture 掩盖掉。每次运行产生新的 ID、时间和磁盘可用空间属于预期，测试不要求它们与另一轮运行相同。
