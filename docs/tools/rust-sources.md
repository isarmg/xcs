# Rust 源码治理工具

正式入口为 Web 和 SMB；运维 CLI 是内部管理工具。所有 Rust 包共享根 `Cargo.toml` 和根 `Cargo.lock`，活动依赖只使用仓库内 path。

- 当前源码、目标闭包、单锁文件与文件摘要：[compliance/rust-sources.json](../../compliance/rust-sources.json)。
- 不可覆盖的原始 archive 身份、原始文件摘要及材料位置：[origins.json](../../compliance/provenance/origins.json)。
- 统一法律材料：[中央通知索引](../../compliance/NOTICE.md)。
- 逐项用途与改造说明：[第三方源码清单](../THIRD_PARTY_RUST_SOURCES.md)。
- 唯一工作区及 native、Web、SMB 和工具入口：[解析策略](../../tools/rust-sources/workspaces.json)。

```sh
python3 tools/rust-sources/manage.py verify --metadata
python3 -m unittest discover -s tools/rust-sources -p 'test_*.py'
```

`verify` 检查源码、原始来源及中央法律材料的摘要，拒绝源码链接/特殊文件、仓库外依赖、registry/Git 依赖、独立工作区、独立锁文件、嵌套 Git 和散落的仓库脚手架。它同时检查所有活动 manifest 中的特性边，包括尚未启用的可选特性、`dep:alias`、`alias/feature`、`alias?/feature` 和依赖声明的 `features`；目标包或特性不存在时立即失败，不能靠默认编译没有触及来掩盖残留声明。

`--metadata` 另用空 Cargo home 离线解析五种入口、三个目标组成的七个入口／目标组合，核对 normal/build 传递闭包、编译／运行角色与已解析 features。所有活动源码树和已合并实现都必须至少进入一个登记闭包；dev 边不被混入产品运行闭包。此规则约束源码树是否必要，不表示每个可选源码文件都会链接到最终产品。元数据解析也不等于实际编译、浏览器或 Samba 行为验收。

有意修改当前源码后，先保留来源身份和许可，更新明确的改造/合并说明并完成对应回归，再执行：

```sh
python3 tools/rust-sources/manage.py refresh
python3 tools/rust-sources/manage.py verify --metadata
```

`refresh` 显式登记当前文件摘要，并重新生成来源文档；它不能重新接受被篡改的原始身份、原始法律材料或原始归档文档。修改后的源码不再冒充原始发布包。整合进命名和界面组件的 Unicode/CSR 实现没有独立 Cargo 包，但仍有合并来源、当前文件摘要及漏洞匹配身份。

移除非目标平台或未使用的可选实现时，先同步删除依赖边、特性转发和相应源码分支，再通过明确的退役计划登记。计划必须匹配当前清单的完整最后源码收据，并包含移除原因、分类、文件数和字节数；执行时源码目录必须已经移除。普通 `refresh` 不会把目录丢失自动解释成一次合法精简。

计划是 `schema: 1` 的 JSON，`packages` 数组中每项包含以下字段。应在删除目录前从当前已验证源码保存计划，审查后再删除和执行；不要从改造前 archive 冒充当前最后收据。

| 字段 | 内容 |
| --- | --- |
| `directory` | 当前清单中待移除的 `src/foundation/...` 键 |
| `package` | 从当前清单 `packages[directory]` 完整复制的对象，包含最后逐文件摘要 |
| `reason`、`category` | 明确移除理由及分类，例如非目标平台或未使用可选实现 |
| `file_count`、`bytes` | 最后收据中的文件数及这些源码文件的大小总和，均为整数 |

```sh
python3 tools/rust-sources/manage.py refresh --retire-plan /tmp/reviewed-rust-retirements.json
python3 tools/rust-sources/manage.py verify --metadata
```

当前清单使用 schema 3，以 `packages`、`merges`、`retired` 分别记录活动源码、组件内合并实现和已移除实现。退役记录保留最后源码摘要、原始身份及原因，记录摘要单独锁定，后续 `refresh` 不能覆盖。重新创建退役目录会使验证失败，重新引入必须经过显式来源登记。三类记录必须恰好覆盖全部原始身份，中央原始材料继续保留。

原始发布包只在仓库外复核。重建器默认使用不可变的改造前发布包收据，保持 archive SHA-256、原始逐文件摘要、tar 路径/链接/特殊文件限制、大小上限与输出暂存校验；不能用当前整合后的清单替代原始收据，不能覆盖仓库内源码：

```sh
python3 tools/rust-sources/import_sources.py --output /tmp/rust-original-review
# 显式允许获取固定的原始发布包时才追加 --fetch。
python3 tools/rust-sources/collect_notices.py --output /tmp/rust-notice-review
```

许可收集器默认只列出待复核项；`--fetch` 才从已登记的 Git commit 获取候选通知。结果写到仓库外，由维护者审查后再追加来源材料。安全入口为 [security.sh](../../.github/scripts/security.sh)：统一根锁与全部活动、已合并实现的原始上游身份均接受扫描。已经验证移除的实现只保留历史记录，不冒充当前产品依赖；改名、path 化、合并及本地修改均不形成漏洞、维护警告或扫描不完整的豁免。
