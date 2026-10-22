# 核心契约附件

[文档目录](../README.md) · [盘格式](../08-full-storage-and-disk-format.md#s08-06) · [API](../15-api-and-errors.md#s15-10) · [状态登记](../23-delivery-and-decisions.md#s23-05)

本附件为 `0.1.0-draft`，用于使部分规范可以执行字段与向量校验。仓库已有核心、服务端和客户端实现；本附件的覆盖仍限于下表，不表示完整 API 或盘格式已冻结，也不替代实际 I/O 和平台验收。

## 文件与覆盖范围

| 文件 | 当前覆盖 | 尚未证明 |
| --- | --- | --- |
| [core.schema.json](core.schema.json) | UINT64 精确上界、实体 ID、Branch / Commit 待写入映射、泛型 Entry 及文件分支投影、版本描述、内容提交请求、操作结果、完整上传状态、内容保存 PUBLICATION 字段、当前分页历史根/索引/引用/叶页形状 | 完整命名规则、父边和来源闭合、授权、实际 I/O、Tree/日志完整格式及所有管理操作 |
| [openapi.json](openapi.json) | 保存、结果查询、版本/校验/内容/上传状态、文件管理（含零正文分支创建、冻结父版本与自有版本分开的继承投影、目录批量分支、共同分支交集和128字节单行备注）及管理员系统状态、真实功耗和监测历史 | 认证签发刷新、工作区创建/上传写接口、分页/事件及其余拟定管理接口 |
| [config-constraints.schema.json](config-constraints.schema.json) | 第 21 章关键常量、分支冻结引用/首写物化、资源上限和告警字段的受检投影 | 当前进程 JSON Config、完整部署配置、物理身份、拓扑和启动状态验证 |
| [vectors.json](vectors.json) | 正反字段样例、分支映射键/值和文件作用域拒绝、旧 Branch / Commit 编码兼容、分页历史图固定映射/引用种类/深度/未知字段拒绝、规范字节与 BLAKE3、空文件和短文件分段清单 | 跨 Rust/Web/离线工具的一致实现及断电恢复 |
| [contract-check](../tools/contract-check.md) | Rust 实现的 schema/OpenAPI 有效性、两者定义一致性、固定向量、配置投影、文档本地链接和 JSON/TOML 语法校验 | 模型检查、平台测试、网络端点行为和性能 |

成员数不是固定三盘限制：配置投影支持任意非空固定成员集，决策与恢复多数均为 floor(N/2)+1；正常发布和确认仍要求全部已配置成员。单存储盘是最小配置，后续备份层可选。配置中的三个成员只是示例，不允许按当前可访问盘数自动缩减成员或门槛。

PUBLICATION 的结构允许非空、按身份排序且无重复身份的回执集；不同内容但相同 disk_id 仍须拒绝。Schema 和独立向量没有可信成员配置，不能证明回执集完整；运行时必须对照持久化的可信配置验证精确成员身份和数量、全部成员完成屏障及成员代际，不能用回执数量推导新成员集。单/双成员字段向量不是三成员池缺失回执的授权。

PUBLICATION 中回执使用合成身份与摘要；通过字段或覆盖集合检查不证明磁盘曾同步。展开清单只是字段语义，生产日志须使用有界分页引用并保留完整证明闭包。成功保存后当前健康变为不可用的样例，只验证这两个维度可以分别表达，不模拟真实故障。

文件大小等大整数使用十进制字符串；schema 的 UInt64 正则精确限制最大值。语义检查另验证盘位连续、物理成员不重复、所需内容证明覆盖、上传完整性及账户/全池额度关系。OpenAPI 内嵌与 core 相同的定义供工具离线使用，校验器会拒绝二者漂移。

持久状态、写请求和固定向量的 JSON 解析先拒绝重复字段、BOM、孤立代理项和浮点数，再做 schema 和 JCS；解析后只检查普通字典无法找回重复字段信息。`/api/system/status` 与 `/api/system/history` 的临时监测响应允许有限小数，不放宽持久状态或写请求规则。固定向量中的 JCS 属性排序包含非 BMP 键样例，用于检测错误地按 Unicode 标量顺序排序；该向量不表示 PortableName 允许 emoji 文件名。

备注写接口通过 `x-maxUtf8Bytes: 128` 标明 UTF-8 字节上限，配合控制字符模式和服务端字节计数校验；JSON Schema 的 `maxLength` 本身只计算 Unicode 字符数。读响应保留历史备注原文，允许旧记录超过新写入上限或包含换行，不能按128字节截断。

## 运行校验

使用仓库 Rust 工具链，从仓库根执行：

```bash
CARGO_INCREMENTAL=0 cargo run -p contract-check \
  --locked --offline --target-dir target/native
```

Rust 依赖源码已包含在仓库中，首次构建也使用唯一根锁文件和 `--locked --offline`，不要求预先缓存或下载 registry 依赖；Rust 工具链及必要系统构建工具须另行准备。校验过程不访问网络、不写数据盘，也不生成业务对象。OpenAPI 官方元 schema 已固定随工具保存，JSON Schema 解析禁用网络与文件远程引用功能；自研规则只有 Rust 实现。校验失败返回非零退出码；输出 PASS 仅表示上表声明的文档契约范围通过。

## 冻结前仍需完成

1. 补齐全部拟定 API 的请求、响应、分页、错误及权限作用域，定义认证与会话生命周期。
2. 补齐 Commit/Tree 分页、日志帧 payload、成员回执、PUBLICATION 分页清单及各类恢复记录 schema；覆盖截断日志、中间损坏和未知必需特性。
3. 固定 Unicode 16.0.0 表及校验值，交付可复用命名向量；完成 Rust、浏览器与离线工具交叉验证。
4. 按[第 22 章](../22-acceptance.md#s22-10)执行发布证据模型、真实 I/O 与 SMB 样机，并使用[报告模板](../templates/validation-report-template.md)保存证据。

字段与文本变更须同步更新固定向量和 OpenAPI；未经变更说明不得用重新生成期望值来掩盖兼容变化。外部规格依据为 [JSON Schema 2020-12](https://json-schema.org/draft/2020-12)、[OpenAPI 3.1.0](https://spec.openapis.org/oas/v3.1.0.html) 和 [RFC 8785](https://www.rfc-editor.org/rfc/rfc8785.html)，这些资料不为本系统业务协议提供正确性证明。

分支创建只登记元数据与冻结来源引用，`pending_file_versions` 的键/值分别为 FileId/VersionId；Commit 逐快照保留此状态，当前分支首写不会改变旧快照。泛型 Entry 及文件管理的 version_id 都指可见真实版本；继承时为冻结父 ID，own_version_id 为 null，首次写入后才为自有 ID。新增校验同时拒绝 pending 来源逃逸文件作用域、快照来源与条目不符，以及可见父内容被误标为已拥有内容；旧 Branch / Commit 的空映射省略序列化并核对规范字节与摘要。最新执行见[本轮契约记录](../validation/evidence/inherited-branches/contract.md)，不覆盖旧向量验收快照。

目录分支以 `DirectoryBranch`、`DirectoryTree` 和 `DirectoryReference` 区分命名身份、不可变组成及子目录精确引用；`DirectoryNamespaceBatch` 每项要求源分支、依据子树与稳定条目 ID，非删除操作还要求明确目标分支及依据子树。登记目录分支不提前创建文件分支或占位文件。路径、跨分支冲突与实际归属由核心统一校验，schema 合法不等于业务可发布。

`FilePathNode` 描述重命名的路径节点并引用现有正文，`FileHistoryNode` 把内容与路径节点放在同一历史图；`FileNodeId` 允许 `ver_`、`fnode_` 与 `fmeta_`，正文读取仍只接受真实 `VersionId`。`FileContext.node_id` 和 `Branch.file_node_id` 冻结具体图节点，创建分支可以传 `source_node_id` 保留此来源。路径备注的写入单独限制为128 UTF-8字节，读历史不截断。

`FileMetadataNode` 保存不可变标签/合并子节点，`FileBranchMerge` 独立保存全部合并来源和安全图边。文件合并请求要求明确确认、结果归属及每个参与head，标签批量请求固定源节点与branch head，scope备份配置同时固定稳定身份和自身代数。标签的16项/128字节限制、无控制字符和全部预检由共享Rust验证及业务事务执行；schema检查不代替真实发布。目录分支合并待用户确认，暂不实现，能力为false。

`PagedHistoryRoot`、`HistoryIndex`、`HistoryGraphReference` 和 `HistoryLeaf` 描述实际实验存储图的字段形状；所有历史图结构均不带项目格式代际字段；根内仓库/历史身份使用 `HistoryRepository`、`HistoryIdentity`。原 `FileFeatureStateFields` 仍只是 State / 逻辑 HistoryManifest 的局部投影，未改成分页根。新增定义仅镜像进 OpenAPI 的离线组件，不新增网络端点或改变现有请求。

新字段向量使用合成摘要，只检查 schema，不能证明引用对象存在或摘要正确。规范编码字节上限（128 项、64 KiB 软目标、不可分单实体页 64 MiB 上限）、逐位置前缀、图闭包、重复实体及完整历史语义由核心图读取器和测试验证；扁平投影误作根及未知字段拒绝、真实超过 64 MiB 的聚合图往返、正常/降级恢复、独立包和篡改检查亦属于核心测试，不冒充 JSON Schema 的能力。PackageProof 的完整候选/回执闭包尚未在本附件全部建模，发行格式仍未冻结；版本边界见[第 8.7 节](../08-full-storage-and-disk-format.md#s08-07)。
