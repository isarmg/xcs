# 项目维护的 Leptos 依赖族

来源：[Leptos](https://github.com/leptos-rs/leptos) 发布到 crates.io 的原始 `.crate` 包。范围以迁入时 `web/Cargo.lock` 实际选中且 repository 指向该仓库的包为准，不是整个 Rust 生态依赖的离线镜像。

| 包 | 版本 |
| --- | --- |
| reactive_graph | 0.2.16 |
| tachys | 0.2.19 |
| leptos_dom | 0.8.8 |
| leptos_hot_reload | 0.8.7 |
| any_spawner | 0.3.0 |
| either_of | 0.1.9 |
| oco_ref | 0.2.1 |
| or_poisoned | 0.1.0 |
| throw_error | 0.3.1 |
| next_tuple | 0.1.0 |
| const_str_slice_concat | 0.1.0 |

## 来源、许可和本地修改

每个目录保留发布包中的全部文件，包括 `Cargo.toml.orig`、原始测试、README、包内 Cargo.lock 和 `.cargo_vcs_info.json`。发布包本身多数没有包含独立许可文件；共享 [LICENSE-MIT](LICENSE-MIT) 保留已有 Leptos 导入中的原始版权和 MIT 全文。包名、版本、作者和 license 字段不改名。内含 Cargo.lock 作为发布原件保留，不是应用构建锁文件。

[provenance.json](provenance.json) 记录每个发布包的 crates.io URL、registry source、SHA-256、原始文件哈希、上游 Git SHA/目录及原始 dirty 标记。本地源码的精确基准是经 Cargo.lock 校验的发布包，而不是声称某个 Git commit 可以完全重现 dirty 发布包。

此次导入仅修改各包规范化 `Cargo.toml`：框架族内部 normal/dev/build/target 依赖使用相对 path，并添加独立 workspace 边界。不删除 optional feature 或上游实现。`zxs-web-core`、`zxs-web-macro` 直接依赖本目录。外部通用库仍由 crates.io 解析，不在此复制。

## 检查和重现

在仓库根目录执行（Python 3.11+）：

```sh
python3 tools/vendor-leptos/vendor.py verify --check-metadata
python3 -m unittest discover -s tools/vendor-leptos -p 'test_*.py'
cargo metadata --locked --offline --manifest-path web/Cargo.toml --filter-platform wasm32-unknown-unknown --format-version 1
cargo metadata --locked --offline --manifest-path demos/branch-system/Cargo.toml --filter-platform wasm32-unknown-unknown --format-version 1
cargo check --locked --offline --manifest-path web/Cargo.toml --target wasm32-unknown-unknown
cargo check --locked --offline --manifest-path demos/branch-system/Cargo.toml --target wasm32-unknown-unknown --lib
```

`verify` 校验导入内容、额外文件/符号链接、已记录的 manifest/源码补丁、框架内部 path 边及两个消费 workspace 的锁文件，重复包条目也会失败。`--check-metadata` 进一步检查 Cargo 实际解析的 package source 和 manifest_path；仅静态检查可省略该选项。要在新的空目录重现迁入，准备相应 Cargo registry 缓存后运行：

```sh
python3 tools/vendor-leptos/vendor.py import --output /tmp/zxs-leptos-import
```

删除维护目录后重建时，先另存 provenance.json，再通过 `--seed /path/to/provenance.json` 指定该原始来源记录。

脚本校验 `.crate` SHA-256 后才提取，且拒绝覆盖既有目录；可用重现结果与维护树比较。首轮 import 可从迁入前的 web 锁文件生成来源记录；后续 import 使用来源记录中的原始 registry checksum。日常构建不需要重跑 import，也不读取 registry 中的框架源码。

## 后续维护

修改源码时新增项目回归测试，并在本文件登记行为变化；原始 `upstream_files` 哈希不可改写成新源码。审核后在 [patches.json](patches.json) 的 packages → 包名 → 相对文件名中记录 `sha256` 和非空 `reason`（删除文件时 sha256 为 null），比对原始发布包审查差异；新增文件也必须登记。verify 应用该补丁记录，不改变原始来源哈希。升级必须重新核实整个依赖族、两份消费者锁文件、各包许可及 optional/dev 依赖，不只修改根包版本。原始包内全特性测试涉及非 CSR 和其他平台，不等同于项目支持范围。

### 已登记的维护补丁

- `reactive_graph`、`tachys`、`either_of` 的 `Cargo.toml` 将停止维护的 `paste` 替换为精确固定的 `pastey = 0.2.3`，保留 `paste` 依赖别名与宏调用接口；`zxs-web-core` 同步替换。两份消费者锁文件不再解析 `paste`，没有忽略该审计告警。依据：[RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html)、[pastey 官方迁移说明](https://github.com/AS1100K/pastey)、[0.2.3 发布源码](https://docs.rs/crate/pastey/0.2.3/source/Cargo.toml.orig)。
- 这是导入后的明确维护补丁，不改写 `provenance.json` 的原始发布包或导入清单哈希，也不更改上游许可证。补丁文件的当前 SHA-256 和原因记录在 `patches.json`；验证器测试覆盖清单补丁登记、原始来源保留及后续未登记修改拒绝。

G4 几何和 CSS 不属于本次依赖维护，不作修改。
