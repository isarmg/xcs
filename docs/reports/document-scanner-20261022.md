# 文档扫描专项简化（2026-10-22）

## 边界与结果

`walkdir 2.5.0` 只有 `tools/contract-check` 一个直接消费者，`same-file 1.0.6` 只有 walkdir 一个消费者。删除两者，不新增包装 crate 或通用遍历接口。维护文档扫描直接属于 `tools/contract-check/src/document_scan.rs`；行为测试位于 `test/tools/contract-check/`。

活动来源由 247 减至 245，foundation 包由 245 减至 243，显式退役记录由 88 增至 90。原始来源、版本、许可证和迁移证据保持不变。最后活动实现与外置测试按原摘要归档到 `compliance/walkdir-2.5.0-retirement.json.gz` 和 `compliance/same-file-1.0.6-retirement.json.gz`，不参与 Cargo 解析或产品编译。根锁文件同步移除两包；根 manifest 原本没有它们的 patch 条目。

## 保持的扫描语义

- 根路径以 symlink_metadata 检查；根目录软链接跟随一次以读取目录，但软链接本身不是普通文件。根文件软链接不输出，普通 `.md` 根文件输出。
- 子项软链接不跟随、不输出，包括断链和循环。断链或循环的根路径仍报错。
- 仅精确小写 `.md` 扩展名；非 UTF-8 文件名不丢弃。
- 排除 `docs/upstream` 精确路径和任意深度的 `dist`、`.dist-staging`、`.dist-backup`、`target` 文件或目录名；不排除其他位置同名 upstream 目录。
- 读目录、读取条目类型、根路径元数据等错误继续向调用者传播。被排除的普通目录不读取其子项；根软链接先解析元数据，因此被排除的断链根仍报错。
- 显式目录栈替代递归，每次只持有一个 read_dir 句柄。完整路径集排序；主调用方合入 README 后再次全局排序。错误发生顺序和展示文本不作兼容承诺，多错误树仍失败而不是静默跳过。

## 可复现验收

删除前，直接编译当时活动的 same-file/walkdir，与新扫描器对同一批 fixture 做差分：5/5 通过，文件描述符限制 32，包含 150 层目录。执行用户为非 root，权限拒绝分支实际执行。

`python3 test/tools/contract-check/check_document_scan.py` 从退役归档恢复到临时目录，先校验整个文件集合及摘要与退役清单一致，再重编译原 walker 运行相同差分。重放共 6 项通过，包含实际维护文档树；原实现仅存在于这一验收临时目录，不恢复任何活动依赖。该检查已加入 contracts 套件。

独立分支的初始基线存在单盘配置 schema/vector 契约不一致，真实契约检查曾返回 `3 was expected`；没有弱化扫描或跳过该检查。整合单盘契约修正后，binary 单测 18/18、scanner 5/5、gitignore 1/1、source_export 1/1 全部通过。`cargo clippy --locked --offline -p contract-check --all-targets -- -D warnings` 通过。
来源与架构：`python3 tools/rust-sources/manage.py verify --metadata`、`python3 tools/rust-sources/architecture.py verify` 均通过，确认 245 个活动来源、90 个退役来源及 2 个被替代来源。
完整静态门禁：`bash .github/scripts/check.sh static` 通过。
真实文档契约：`cargo run --locked --offline -p contract-check -- --root .` 通过，覆盖 83 个 schema/语义向量、30 条 OpenAPI 路径和 1174 个本地链接。生成证据可以在源码导出中缺失，但维护源码链接仍必须有效。

重放启动器从仓库外运行及启用 `PYTHONOPTIMIZE=1` 时均通过 6/6 差分；优化模式下篡改归档字节仍会在编译前被拒绝。完整性和路径边界使用显式异常，不依赖可关闭的 assert；子进程固定仓库工作目录以使用其 Rust 工具链，并在无限硬限制环境中仍将文件描述符软限制设为 32。

本次不声称产品运行时性能提升；收益是移除无实际用途的通用文件身份/递归遍历依赖及其活动维护面。
