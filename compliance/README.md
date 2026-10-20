# ZXS 统一许可证、版权与来源说明

本仓库统一维护 Web 和 SMB 产品源码、构建工具及所需的 Rust 基础实现。所有许可证、版权通知和机器可读来源材料集中在本目录；产品、开发、运维与上游说明文档集中在 [`docs/`](../docs/README.md)。源码目录中的 crate 是同一构建依赖图中的编译组件，不是独立仓库；自有包归根工作区，基础组件通过本地路径依赖和根锁文件接入。

## 许可证边界

第一方工作区的默认许可证为 Apache-2.0，全文见 [`licenses/project/LICENSE-APACHE-2.0.txt`](licenses/project/LICENSE-APACHE-2.0.txt)。集中归档、目录合并和 crate 合并不会改变第三方源码的原始权利或许可证；第三方实现继续适用其各自保留的许可条件。源码中的版权、SPDX 和作者头继续保留。

逐项来源和许可索引见 [`NOTICE.md`](NOTICE.md)，活动源码收据见 [`rust-sources.json`](rust-sources.json)。原始许可证、版权、作者名单和通知分别按来源包标识存放于 `licenses/upstream/`；多个组件共用的通知放于 `licenses/shared/`，此前补充取得的上游通知全文放于 `licenses/supplemental/`。界面基础实现合入 `src/interface/` 后，Leptos 的 MIT 全文与原始版权通知仍见 [`zxs-web-core`](licenses/upstream/zxs-web-core-0.8.19/UPSTREAM-NOTICE.md)；过程宏对应通知见 [`zxs-web-macro`](licenses/upstream/zxs-web-macro-0.8.19/UPSTREAM-NOTICE.md)。

字体资源的原始版权与三组 SIL OFL 1.1 全文见 [`licenses/fonts/UPSTREAM-NOTICES.md`](licenses/fonts/UPSTREAM-NOTICES.md)。字体文件本身保留在 `src/web/public/fonts/`，原始字节和资源收据继续由 Web 构建校验。

## 集中材料与可追溯关系

| 位置 | 含义 |
| --- | --- |
| `rust-sources.json` | 当前源码位置、使用关系、本地改造和逐文件收据，由统一源码管理工具维护 |
| [`provenance/origins.json`](provenance/origins.json) | 导入前的包身份、上游 URL／版本／提交／归档校验和、原始文件哈希及中央许可映射 |
| [`provenance/document-migration.json`](provenance/document-migration.json) | 原始文档和元数据路径到中央路径的映射及迁移前 SHA-256 |
| `provenance/packages/` | 原始发布清单、原始子锁文件、发布校验和、VCS 身份及上游 CI 元数据；仅作溯源，不是工作区或构建入口 |
| `provenance/merged-components/` | 被合并组件的原始清单与入口，保存合并前的证据 |
| `provenance/pre-monorepo/` | 本次整体目录改造之前的来源收据和关键构建输入快照 |
| `provenance/shared/` | 原先共享的上游身份、补充通知收据和补丁记录 |
| [`../docs/upstream/`](../docs/upstream/) | 原始上游说明文档，保持归档字节，历史路径不表示当前目录 |

归档中的 `vendor/`、`crates/`、`integrations/`、`demos/`、独立 `Cargo.lock` 及历史工作区字段是迁移前的来源事实，不是当前构建指令。当前构建只使用根 `Cargo.toml` 与根 `Cargo.lock`。不得在归档目录执行 Cargo，也不得为了通过完整性检查覆盖原始来源哈希。

## 尚未关闭的许可资料复核

以下三项在既有上游固定提交树中没有找到独立许可／版权文件。源码与发布包的许可元数据、来源版本／提交及历史复核说明继续保留；集中整理不代表已取得缺失原文，也不代表这些项目已经完成独立许可复核：

- `convert_case_extras@0.2.0`（`d523585a01afddd13944be1d88faa4d1db6a3eb4`）。
- `drain_filter_polyfill@0.1.3`（`9dc585f3eec048bfc3faf8656e07c4a19273c38f`）。
- `erased@0.1.2`（`60eebad28ab2578a6322199099b8a07035438008`）。

原始说明见 [`provenance/shared/vendor/rust-notices/provenance.json`](provenance/shared/vendor/rust-notices/provenance.json)。发布材料必须携带本目录中的实际许可、版权和通知，并如实列出尚未关闭事项；许可证声明本身不构成维护状态、安全审计或性能验收证据。

## 安全补丁升级

同一路径的真实上游升级通过显式升级计划导入；当前清单中的 `superseded` 保存被替换活动包的完整收据和新来源关系。历史来源与中央材料保持不可变，活动审计只匹配实际保留的版本。首次应用见 [2026-10 emSign TLS 安全更新](../docs/security/TLS_EMSIGN_2026.md)。
