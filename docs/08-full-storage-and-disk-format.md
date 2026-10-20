# 08 全量存储、盘格式与单盘恢复

[文档目录](README.md) · [上一章：数据模型、身份与索引](07-data-model.md) · [下一章：用户指定的分层备份拓扑](09-backup-topology.md)

<a id="s08"></a>

完整文件、副本材料、规范编码、盘上布局和离线恢复。

## 本章目录

- [8.1 完整版本与独立文件系统](#s08-01)
- [8.2 单盘布局](#s08-02)
- [8.3 版本与完整内容描述](#s08-03)
- [8.4 全量封存与验证](#s08-04)
- [8.5 单盘恢复与外部修改](#s08-05)
- [8.6 盘格式 V2、规范编码与升级](#s08-06)

<a id="s08-01"></a>

## 8.1 完整版本与独立文件系统

每盘独立 Btrfs、UUID 和设备身份，不组跨盘条带。保持文件数据校验；NOCOW 会隐含关闭文件数据校验，不能借禁止共享为由全盘启用。[S10][S16]

逻辑层记录不可变 FileVersion，内容层只保存 FULL 普通文件，放置层记录每个指定物理盘上的完整副本。修改、追加或截短后保存，最终都生成新完整文件，不依赖前一版本补齐字节。

| 场景 | 持久化结果 | 读取与恢复 |
| --- | --- | --- |
| 普通内容保存 | 变更文件的新完整版本及校验清单 | 直接范围读或离线打开，无旧版本依赖 |
| 新建文件分支 | 仅发布记录及冻结来源引用，不创建占位文件或内容版本 | 可见内容继承冻结父版本；首次写入拥有 FULL，切换不影响其他文件 |
| 目录分支与子树 | 分支引用冻结文件版本并复用共享文件分支，修改仅生成不可变子树 | 不复制正文或创建私有文件分支，命名分支 ID 和名称保持 |
| 旧关系标记 `MergeLink` | 仅持久化两个已有节点间的元数据关系 | 不产生内容文件，不推进HEAD或目录引用 |
| 文件分支合并 `FileBranchMerge` | 复用已有节点，或新增复用原正文的合并元数据节点；保存来源和新目录引用 | 归属继续写入、其余输入保留只读历史；原正文与证明不重写，见[合并规则](design/file-merge-tags-backup.md) |
| 历史树拆分 | 新树保留文件的独立完整副本及元数据 | 两树可以分别导出和恢复 |

网络范围上传、服务端读取旧版未变范围，只能作为构建工作文件的过程；封存的正式版本仍是完整文件。接口不提供其他保存模式及转换操作。

<a id="s08-02"></a>

## 8.2 单盘布局

```text
<disk-root>/
├── README.txt
├── disk.json
├── histories/<history-id>/
│   ├── versions/<version-id>/
│   │   ├── version.json
│   │   ├── checksums.bin
│   │   ├── payload.json
│   │   ├── commit-proof.json
│   │   └── publication-proof.json
│   └── file-containers/<file-id>/
│       └── 报告.docx/
│           ├── container.json
│           └── versions/
│               ├── v000001_20261018T133000Z_ab12cd34.docx
│               └── v000002_20261018T134500Z_ef56ab78.docx
├── metadata/application-log/journal.bin
├── metadata/objects/<object-type>/<hash>.json
├── metadata/pool-membership/config.json
├── metadata/publication-seals/<candidate-hash>.json
├── staging/<operation-id>/
└── quarantine/<incident-id>/
```

`versions/<version-id>/` 保留不可变版本说明、校验及发布证明；正文唯一实体位于 `file-containers/<file-id>/<当前文件名>/versions/<storage_name>`，是可以直接读取的普通文件。外层 `file-id` 为稳定身份，内层是实际目录，不是界面别名。业务改名真正重命名该内层目录；历史版本文件的内部名称及已有证明保持不变。下载当前或历史版本时使用该文件当前的业务名称，已删除文件保留最后登记的名称。

版本内部名称为 `v<文件内序号>_<UTC时间>_<内容散列前8位>.<扩展名>`，例如 `v000001_20261018T133000Z_ab12cd34.docx`；序号至少六位，按同一文件已有版本递增，不等同于全局 `version_sequence` 审计序。时间另以 RFC3339 写入 `created_at`。新版本扩展名采用当前文件名；没有扩展名时不追加句点。它们不使用容器目录名称作为版本身份。

分支的 pending_file_versions 是已发布的继承归属记录，不对应空文件。新创建、恢复和导入均不生成 pending-branches/*.pending；旧格式来源也不为引用新建空容器，正文路径及 inode 保持。兼容清理只删除合法零字节、单链接旧 stub；未知项、非空文件或链接全部保留并报错。container.json 的 inherited_branches 是来源引用缓存，可见 version_id 等于父真实 ID，自有版本为空。

文件备注、版本备注、当前名称和选中分支在已发布 State 中持久化；容器内 `container.json` 只是从该状态重建的浏览缓存，标记 `cache_only`，不替代不可变 `version.json`、校验和或本盘证明。修改备注不改写旧版本描述。名字与缓存投影在事务决定后、PUBLICATION 前执行；中断后可以按已确定决定重放，不能把单次目录重命名成功当作发布完成。

旧版本没有 `storage_name` 时继续读取 `versions/<version-id>/content/<creation_name>`。新增可选字段为空时不改变旧对象序列化；升级不隐式搬迁所有旧正文，也不把空容器当作已迁移成功。新版本的准备正文可以短暂位于原 `content/` 目录，验证后在同一文件系统上独占重命名到容器，并同步源、目标目录；正常完成后不保留第二份准备正文。

每份有效副本包括完整内容、版本说明、校验及本盘证明。commit-proof 证明候选和本盘内容准备，publication-proof 携带[第 10.10 节](10-transactions-and-recovery.md#s10-10)的公开决定及本盘验证闭包。只有前者时，不把文件标为已完成正常发布。不允许从其他盘、其他树或在线服务补齐内容或必需证明后才声称本盘可以独立恢复。

<a id="s08-03"></a>

## 8.3 版本与完整内容描述

```json
{
  "format_version": 2,
  "history_tree_id": "history-example",
  "version_id": "version-example-002",
  "file_id": "file-example-001",
  "creation_primary_slot_id": "P01",
  "logical_size": "1048576",
  "version_sequence": "2",
  "content_hash": "<完整字节BLAKE3-256>",
  "checksum_chunk_bytes": 262144,
  "checksum_manifest_hash": "<分段校验清单哈希>",
  "creation_name": "报告.docx",
  "storage_name": "v000002_20261018T134500Z_ef56ab78.docx",
  "created_at": "2026-10-18T13:45:00Z",
  "source_version_ids": ["version-example-001"],
  "created_as_full_checkpoint": false
}
```

```json
{
  "format_version": 2,
  "payload_id": "payload-example-002",
  "version_id": "version-example-002",
  "kind": "FULL",
  "size": "1048576",
  "content_hash": "<与版本描述一致的完整字节哈希>"
}
```

例中 ID 和摘要是占位符，不是真实测试向量，`storage_name` 的短散列必须与实际 `content_hash` 前八位一致。`storage_name`、`created_at` 是新增可选字段；旧版本缺少它们时按旧正文布局读取。source_version_ids 仅表示审计来源；读取不访问源版本。健康与清除状态由事务日志和事件决定，不能原地改写 version.json 掩盖损坏或删除。

<a id="s08-04"></a>

## 8.4 全量封存与验证

使用显式读写创建新普通文件，处理短写、空间不足、校验、文件与目录同步。禁止用硬链接、reflink、快照或隐式去重冒充独立完整文件。新版本封存后不原地更新；备份修复恢复相同字节，不改变版本身份。

来源可以是完整上传、服务端工作文件或已有完整版本；source_version_ids 只记录审计来源，不形成真实 FULL 的读取依赖。创建分支只发布冻结引用，不读取来源正文或新建空文件；实际写入才封存独立 FULL。目录分支复用共享文件分支，修改只生成目录子树，不自动增加命名分支。元数据登记与内容保存分别报告，不把引用当作新增内容保护成功。

新正文从准备路径移动到容器时使用目录描述符相对的独占重命名，拒绝覆盖目标；重试遇到目标已存在，先核对长度和完整散列，再清理同内容准备副本。稳定文件目录出现多个命名容器、符号链接或正文硬链接时明确失败。读取缺失正文仍报告预期路径，供既有丢失分类和修复流程处理，不凭空创建替代文件。

<a id="s08-05"></a>

## 8.5 单盘恢复与外部修改

离线工具扫描版本描述、内容墓碑、本盘清单和证明，验证后直接导出完整文件。尚未完成的提交不冒充已发布版本；文件还在但证明不全时标记未知并保留。普通数据盘只保证恢复本盘实际持有的内容，全局历史依赖完整控制元数据恢复包。

拆盘按相应 Btrfs 版本的只读且避免日志重放写入的方式挂载；单写 ro 不代表所有恢复性写入均被阻止。[S21] 外部修改后重新接入须隔离并显式导入为新版本，不接受为旧不可变版本的合法原地更新。不能两台服务器同时写管理同一盘。

<a id="s08-06"></a>

## 8.6 盘格式 V2、规范编码与升级

盘格式 V2 采用以下约束；[第 23 章](23-delivery-and-decisions.md)要求在实际使用前交付与其一致的机器可读 schema 和固定测试向量。文档中的简写 ID 只用于说明。实际 ID 为实体前缀、下划线及 32 位小写十六进制随机值；前缀固定为 pool/repo/history/topology/placement/br/commit/link/tree/entry/file/ver/payload/rep/plan/ws/tx/op/disk/lease/session/recovery/req。随机值来自密码学安全随机源，创建时仍检查唯一性；已登记ID不重新生成。用户指定的slot_id另用稳定ASCII标识（如P01/B01），限定字母开头、后接字母数字或下划线/连字符、最多64字节；大小写不敏感唯一，改展示名不改其引用身份。

| 项目 | 规范要求 |
| --- | --- |
| JSON 编码 | UTF-8，无 BOM；拒绝重复字段、孤立代理项和不符合 schema 的类型；写盘使用 RFC 8785 JCS 规范表示 [S23] |
| 字符串 | 名称在业务校验阶段检查 NFC；哈希阶段不得再次规范化、折叠大小写或改写字符串 |
| 数字 | 文件大小、偏移、日志序号、代际及 token 统一为无符号 64 位十进制字符串，范围 0～18446744073709551615，无正号或多余前导零；小型枚举、格式版本和段大小用 schema 限定的整数，禁止浮点字段 |
| 数组顺序 | Tree 条目按 lookup_key 的 UTF-8 字节序排序；同键冲突先拒绝；副本成员和证明回执按 disk_id 排序；有业务顺序的操作数组保持原顺序 |
| 对象哈希 | 元数据采用 `BLAKE3("zxs:" + object_type + ":v2" + NUL + JCS(payload))`；类型标记为 ASCII，NUL 是单个零字节；自包含的 hash 字段置于外层，不参与 payload，避免自引用 |
| 内容哈希 | 对完整内容字节计算 BLAKE3-256；不套用元数据域前缀，空文件也有确定哈希 |
| 分段清单 | 32 字节头：8 字节 ASCII 魔数 `ZXSCHK2` 后接 NUL、u16 格式版本 2、u16 算法编号 1（BLAKE3-256）、u32 段字节数、u64 逻辑长度、u64 段数；整数小端。随后为顺序 32 字节段哈希，无尾部扩展；长度须等于 `32 + 32 × 段数` 且段数符合内容长度。空文件段数零，尾段按实际长度计算，整份原始清单哈希写入描述 |
| 日志帧 | 16 字节头：8 字节 ASCII 魔数 `ZXSLOG2` 后接 NUL、u16 格式版本 2、u16 保留值 0、u32 payload 长度，整数小端；随后为 JCS payload 和 32 字节 BLAKE3(头+payload)。payload 内有记录类型及前帧哈希，首帧前哈希为 64 个零；payload 最多 16 MiB，越界先拒绝分配。尾部半帧不构成记录；中间损坏或有效分叉不能跳过 |
| 大型元数据 | Tree 分页为不可变节点，候选通过哈希引用依赖清单；日志不内嵌无限增长的文件列表，所有引用依赖须先按相同保护规则持久化 |
| 扩展与未知字段 | 仅 schema 声明的可选扩展可忽略其展示意义，但必须保留并参与哈希；未知必需特性拒绝写入和自动重放，已知普通内容可在只读导出模式单独验证 |

请求摘要也使用有类型标记的规范 payload，字段集合及排除规则见[第 15.7 节](15-api-and-errors.md#s15-07)；服务端自行计算并比对，不能信任客户端给出的摘要。JSON 网络传输可以带空白，参与哈希时必须归一到同一 JCS 字节序列。[S23][S24]

升级流程固定为：只读预检 → 导出格式与能力清单 → 验证新旧读取器和恢复工具 → 暂停写入 → 持久化迁移开始记录 → 生成新格式元数据 → 校验并切换格式代际。不得原地改写逻辑内容；完整文件导入或迁移另按[第 10 章](10-transactions-and-recovery.md)事务执行；旧读取器遇到不认识的必需格式必须停止自动修复。迁移未完成时保留旧元数据和恢复依据，只有完整验证后才允许清理。

必须提供跨 Rust、浏览器和离线工具一致的正常、边界整数、重复字段、字段重排、未知必需特性、截断帧及位翻转样例。未经这些样例验证，不能把示例 JSON 当作已冻结的可互操作格式。

当前可校验的字段契约、固定样例及验证入口见[契约附件](contracts/README.md)。附件逐项声明覆盖范围；尚未交付的 Commit/Tree 分页、完整日志回执与迁移格式仍是冻结门槛，不能因部分 schema 通过便宣称盘格式 V2 已冻结。盘格式升级也不自动证明支持任何旧格式；读取器须声明具体版本并通过对应恢复样例。

[S10]: https://btrfs.readthedocs.io/en/latest/btrfs-scrub.html
[S16]: https://btrfs.readthedocs.io/en/latest/Reflink.html
[S21]: https://btrfs.readthedocs.io/en/latest/btrfs-man5.html
[S23]: https://www.rfc-editor.org/rfc/rfc8785.html
[S24]: https://www.rfc-editor.org/rfc/rfc8259.html

---

[文档目录](README.md) · [上一章：数据模型、身份与索引](07-data-model.md) · [下一章：用户指定的分层备份拓扑](09-backup-topology.md)
