# Git 忽略策略补充

2026-10-19，用户确认：排除验证生成物，只保留测试代码、说明及必要固定样例。本轮修改忽略规则、对应回归及文档附件校验，不删除本机文件，也不修改存储服务行为。

- `tests/evidence/`、演示的 `tests/evidence/` 与 `tests/history/` 按目录排除结果 JSON、截图、日志、trace、下载文件及其他生成物；Markdown 说明与 Rust/MJS/Python/Shell 检查程序继续入库。
- 各工作区 `target/`、根 `build/dist/out/artifacts`、发布临时目录、Cargo 下载缓存、覆盖率和浏览器缓存、本地环境、签名私钥、临时文件及嵌套编辑器状态均排除。
- Cargo 锁文件、共享编辑器配置、环境示例、公开验签密钥、`web/runtime/`、字体、App 图标、schema、固定输入样例和静态参考图保留；上游 `Cargo.toml.orig` 作为来源材料保留。
- 文档检查单独统计本地生成附件链接及缺失数量。源码副本不含验证附件时可校验规范；Markdown、源码、检查程序、schema 和固定样例缺失仍报错，路径越界仍拒绝。

复查命令：

```sh
cargo test --locked --offline --manifest-path tools/contract-check/Cargo.toml --target-dir target/native
cargo run --locked --offline --manifest-path tools/contract-check/Cargo.toml --target-dir target/native
git status --short --untracked-files=all
git check-ignore -v tests/evidence/merge-tags-backup/candidate-receipt.json
git check-ignore -v web/runtime/src/lib.rs
```

最后一条命令应返回未忽略。规则测试在临时 Git 仓库执行，同时覆盖应排除与应保留的路径；源码导出检查会验证没有生成附件的副本，并确认故意添加的源码断链仍失败。具体路径数量、文件体积变化及本机执行结果记录在本目录的生成附件中，附件由 Git 忽略。

项目放置约定见[工程规范](../../20-engineering.md)，测试生成物与独立归档边界见[测试说明](../../validation.md)。

本次复查通过 17 项 Rust 测试，其中忽略策略覆盖 166 个正反路径；源码导出检查在不含生成附件的副本中通过，并拒绝故意添加的源码断链。检查了 252 个真实源码和固定资源路径，没有误忽略。共有 1,694 个验证生成文件、236,525,433 字节被排除，75 份验证说明与检查代码仍可入库；本机文件均保留。
