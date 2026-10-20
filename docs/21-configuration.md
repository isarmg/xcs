# 21 配置规范与默认值

[文档目录](README.md) · [上一章：工程结构与依赖约束](20-engineering.md) · [下一章：验收标准与证明材料](22-acceptance.md)

<a id="s21"></a>

目标配置、逐级来源示例及关键参数的约束。

## 本章目录

- [21.1 配置示例](#s21-01)
- [21.2 默认值的边界](#s21-02)
- [21.3 配置分类与权威顺序](#s21-03)
- [21.4 启动校验与变更](#s21-04)

<a id="s21-01"></a>

## 21.1 配置示例

以下 TOML 定义完整目标配置，不是现有进程直接读取的配置或可直接部署的 smb.conf。当前服务读取严格 JSON `Config`，可用 `zxs config-template --production --disks <三个绝对路径> --output <file>` 生成带随机池/盘 ID 的模板，再审查后传给 `zxs --root <state> init --config <file> --username <name>`；模板命令不访问、挂载或初始化设备。实际已接受字段、CLI 和限制见[服务端说明](../crates/zxs-server/README.md)与[实现报告](reports/implementation.md)。

Samba 配置由经版本验收的部署模板另行生成。设备清单通过登记工具产生，不在示例中填写真实设备路径。

示例是完整有效配置的展示，包含协议常量断言、运行参数和初始化目标；不表示每个字段都可由运维自由切换。分类及冲突处理见[第 21.3 节](#s21-03)。

```toml
[server]
mode = "single-writer"
local_socket = "/run/zxs/service.sock"
remote_transport = "https"
http2 = true
http3 = false
require_verified_tls = true

[smb]
enabled = true
server_backend = "samba"
business_bridge = "vfs-to-core"
supported_protocol_family = "SMB3"
allow_smb1 = false
allow_guest_write = false
require_signing = true
require_encryption = true
share_target = "history-main-branch"
basic_file_operations_only = true
allow_branch_switch = false
expose_history_payloads = false
save_boundary = "flush-write-through-or-last-writable-close"
retain_failed_server_workspaces = true

[[smb.shares]]
share_id = "share-example"
name = "personal"
history_tree_id = "history-example"
main_branch_id = "br-main"
share_generation = "1"

[storage]
filesystem = "btrfs-single-device"
disk_inventory = "registered-physical-disks"
minimum_disks_for_protected_write = 3
allow_explicit_disk_enrollment = true
content_layout = "full-file-v2"
writable_disk_format = 2
readable_disk_formats = [2]
allow_cross_version_reflink = false
allow_content_hardlinks = false
allow_implicit_content_deduplication = false
allow_cross_tree_dependencies = false

[metadata]
replica_members = 3
decision_members = 2
recovery_evidence_members = 2
publish_after_all_members = true
acknowledge_after_all_members = true
max_inflight_decision_slots = 1
require_fenced_membership_change = true
allow_automatic_degraded_writes = false
sqlite_role = "rebuildable-index"
application_log = true
member_slot_ids = ["P01", "B01", "C01"]
member_selection = "user-specified"

[versions]
immutable_after_seal = true
historical_save_creates_new_full_branch = true
default_save_mode = "full"
supported_save_modes = ["full"]
automatic_topology_expiration = false

[branches]
creation_mode = "full"
materialize_all_files_on_create = true
main_branch_required_for_smb = true
merge_enabled = true
ordinary_commit_parent_count = 1
# 以下 merge_* 字段仅描述兼容的 MergeLink 关系标记，不限制文件分支合并。
merge_behavior = "link-only"
merge_target = "selected-existing-version"
merge_creates_content_version = false
merge_updates_branch_head = false
allow_history_rewrite = false
concurrent_authenticated_sessions = true
file_write_ownership = "server-arbitrated"
serialize_branch_publication = true
exclusive_maintenance_plans = true
auto_create_conflict_branch = false
switch_requires_clean_state = true

[history]
preserve_topology_after_purge = true
explicit_content_purge = true
automatic_content_purge = false
protect_active_heads = true
check_live_references_before_purge = true

[tree_split]
enabled = true
selection = "cut-commit-and-descendants"
new_root_mode = "full"
independent_metadata_and_payloads = true
copy_full_files_without_reflink = true
preserve_purged_nodes = true
atomic_publish_both_trees = true

[naming]
policy = "PortableNameV2"
path_policy = "PortablePathV1"
unicode_data_version = "16.0.0"
canonical_form = "NFC"
case_sensitive = false
preserve_display_case = true
reject_noncanonical_new_names = true
auto_rename_invalid_names = false
component_utf8_bytes = 240
component_utf16_units = 240
relative_path_utf8_bytes = 768
relative_path_utf16_units = 200
max_directory_depth = 32

[replication]
default_target = 1
preset_targets = [1, 2, 3]
minimum_target = 1
maximum_target = "primary-route-depth"
freeze_policy_per_version = true
inherit_for_retained_history = true
strict_on_commit = true
count_client_cache_as_replica = false
placement_mode = "user-tiered-backup-topology"
allow_random_placement = false
allow_unmapped_fallback = false
allow_automatic_role_promotion = false
allow_automatic_cross_route_rebalance = false
require_all_route_levels = true
count_off_route_copies = false

[backup_topology]
id = "topology-example"
generation = "1"
require_adjacent_levels = true
one_next_target_per_source = true
require_distinct_physical_disks = true

[[backup_topology.slots]]
slot_id = "P01"
copy_level = 1
binding_generation = "1"
disk_id = "disk-example-p01"
source_slot_ids = []

[[backup_topology.slots]]
slot_id = "P02"
copy_level = 1
binding_generation = "1"
disk_id = "disk-example-p02"
source_slot_ids = []

[[backup_topology.slots]]
slot_id = "P03"
copy_level = 1
binding_generation = "1"
disk_id = "disk-example-p03"
source_slot_ids = []

[[backup_topology.slots]]
slot_id = "P04"
copy_level = 1
binding_generation = "1"
disk_id = "disk-example-p04"
source_slot_ids = []

[[backup_topology.slots]]
slot_id = "B01"
copy_level = 2
binding_generation = "1"
disk_id = "disk-example-b01"
source_slot_ids = ["P01", "P02"]

[[backup_topology.slots]]
slot_id = "B02"
copy_level = 2
binding_generation = "1"
disk_id = "disk-example-b02"
source_slot_ids = ["P03", "P04"]

[[backup_topology.slots]]
slot_id = "C01"
copy_level = 3
binding_generation = "1"
disk_id = "disk-example-c01"
source_slot_ids = ["B01", "B02"]

[[backup_topology.slots]]
slot_id = "D01"
copy_level = 4
binding_generation = "1"
disk_id = "disk-example-d01"
source_slot_ids = ["C01"]

[primary_placement]
require_user_rule = true
allow_automatic_primary_selection = false

[[primary_placement.rules]]
history_tree_id = "history-example"
path = "/"
primary_slot_id = "P01"

[[primary_placement.rules]]
history_tree_id = "history-example"
path = "/media"
primary_slot_id = "P03"

[reading]
require_full_download_before_open = false
pin_version_for_committed_read_handle = true
range_reads = true
checksum_chunk_bytes = 262144
readable_checksum_chunk_bytes = [65536, 262144, 1048576, 4194304]
verification_mode = "server-verified-range"

[writing]
workspace_model = "session-overlay"
automatic_commit_scope = "file-and-required-namespace"
serialize_commits_per_session = true
freeze_file_writes_during_commit = true
require_expected_head_for_explicit_batch = true
require_file_and_namespace_preconditions = true
allow_verified_unrelated_head_advance = true
require_idempotency_key = true
writable_shared_mmap = false
offline_auto_commit = false

[capacity]
warning_used_percent = 80
critical_used_percent = 90
growth_sample_days = 7
warning_days_remaining = 30
per_disk_reserve_percent = 10
metadata_reserve_policy = "checkpoint-plus-recovery"
reserve_space_per_target_disk = true

[garbage_collection]
enabled_for_verified_unreachable_objects = true
orphan_grace_days = 7
pause_when_recovery_incomplete = true
pause_when_transaction_in_doubt = true

[resource_limits]
workspaces_per_account = 32
uploads_per_account = 4
uploads_per_pool = 16
staging_bytes_per_account = "107374182400"
staging_bytes_per_pool = "536870912000"
handles_per_session = 256
handles_per_pool = 4096
queued_tasks_per_pool = 128
default_page_items = 200
maximum_page_items = 1000
page_snapshot_ttl_seconds = 900
event_retention_days = 7
event_retention_bytes = "1073741824"
workspace_reminder_days = 7

[observability]
decision_warning_seconds = 30
decision_critical_seconds = 120
queue_warning_seconds = 30
```

<a id="s21-02"></a>

## 21.2 默认值的边界

`component_utf16_units=240` 不代表任意 240 字符名称在任何位置都可创建；全相对路径还有 200 单元的保守限制。两个限制分别验证，错误应准确说明是哪一项。

`orphan_grace_days=7` 不授权删除历史拓扑、有效内容pin、受保护版本或未决计划。历史目录仍引用的payload可以在显式清除事务完成后回收；仅凭提交可达不能禁止已授权清除，也不能授权绕过墓碑直接删除。时间条件不替代内容状态与引用检查。

`case_sensitive=false` 只影响约定的逻辑命名空间，不要求把底层 Btrfs 变成另一种文件系统。内部对象通过 ID 定位，名称比较由核心负责。

`decision_members=2` 不能改解释为两成员可以正常写入。三个成员保护、发布和确认仍须全部完成；两个成员仅建立决定并支持指定恢复流程。`maximum_target="primary-route-depth"`按具体一级来源的连续链深计算；总盘数只给出全局上界。离线、满盘或映射缺失不能擅自降低目标或使用别的来源盘。

`supported_save_modes=["full"]` 与 `creation_mode="full"` 分别约束保存和分支创建。示例中 `merge_behavior="link-only"`、`merge_updates_branch_head=false` 等 `merge_*` 字段只描述旧 `MergeLink` 关系标记：端点固定、正文与HEAD不变，不是所有合并操作的全局约束。文件分支合并 `FileBranchMerge` 是另一个已实现能力，接口声明 `merge_behavior:"explicit-file-node-adoption"`，会推进归属与参与目录引用；目录分支合并待用户确认，暂不实现。两者见[补充规则](design/file-merge-tags-backup.md)，不能通过修改旧字段开启或关闭新语义。`preserve_topology_after_purge=true`保留图节点，不永久锁住payload。`tree_split`要求新域对象和恢复包独立，不只是分支引用改名。

`smb.shares` 固定历史树与主分支；App 切换工作分支不修改该配置。普通会话不独占整个分支，按文件核对并串行发布；显式批量计划仍锁定 HEAD，维护计划处理相关工作区后才取得排他权。`auto_create_conflict_branch=false` 禁止静默处理冲突，不影响用户从历史保存时按规则登记新分支并只保存实际写入的 FULL 版本。

上述示例有4个一级、2个二级、1个三级和1个四级盘位。primary_placement按规范目录的最长匹配规则解析，`/media`落在P03，其余由根规则落在P01；不同目录规则冲突必须拒绝，不能用规则声明顺序猜测。disk_id仅为占位，实际必须通过设备登记与物理身份校验。

`member_slot_ids`是用户另外指定的三个控制元数据成员，不等于每个业务版本都保存三份。盘位配置、业务路由和控制成员发生变更时分别校验并按需要组合迁移，普通启动不重新随机选成员。

`verification_mode` 是默认服务端校验模式，App 和 CLI 可协商完整段独立验证；SMB 客户端不需要实现本系统校验协议，由服务端在返回字节前校验。段参数仅对新版本生效；读取旧版本时以其描述为准。容量阈值与校验段默认值都是待实测的初始配置，不能宣称为适合所有设备的最优参数。

`readable_disk_formats` 是构建已实现并通过恢复测试的能力声明，不能通过编辑配置让未知读取器获得旧格式支持。这里仅列 V2；旧格式必须逐版本登记 schema、迁移流程和恢复向量。resource_limits 与 observability 的数值、计费口径和超限动作见[第 19.9 节](19-capacity-and-performance.md#s19-09)及[第 19.10 节](19-capacity-and-performance.md#s19-10)。告警秒数不是提交取消期限。

<a id="s21-03"></a>

## 21.3 配置分类与权威顺序

| 类别 | 示例 | 修改与校验规则 |
| --- | --- | --- |
| 协议常量或构建能力断言 | metadata 的 3/2/2、max_inflight_decision_slots=1、全部成员发布/确认、禁止自动降级、full-only、旧MergeLink的link-only语义、禁历史改写与跨树依赖 | schema 使用 const 或固定枚举；冲突返回 CONFIG_INVARIANT_VIOLATION，不能当作可调布尔开关；文件分支合并能力单独声明 |
| 运行参数 | 网络监听、缓存及维护并发、resource_limits、observability、容量告警与安全预留 | 进行类型、范围和交叉检查；仅明确支持的字段允许热更新，记录配置修订号 |
| 受事务管理的持久状态 | 成员集合、盘位/物理绑定、拓扑代际、共享归属、命名策略、一级规则和保护策略 | 配置只提出初始化目标或变更计划；有效检查点和日志决定当前值，必须经对应业务事务修改 |

首版未明确列为运行参数或已实现可选能力的字段，均不得通过配置扩展新语义。命名策略中的长度、Unicode 版本和比较规则是同一策略身份的一部分，不能只热改数值而保留原策略版本。协议常量文件可用于核对和能力展示，不是绕过规范的入口。

已有存储池的权威顺序为：可信成员配置链与检查点/日志 → 已发布业务状态 → 可重建索引与配置展示。启动配置与已发布状态冲突时不自动覆盖任一侧；报告具体字段及双方代际，进入 CONFIG_CONFLICT 维护状态。无盘上身份时，只有显式初始化流程才能采用初始化目标，普通启动仍禁止建空池。

<a id="s21-04"></a>

## 21.4 启动校验与变更

启动依次执行：严格解析并拒绝重复键/未知字段 → 常量及范围检查 → 账户/全池额度关系检查 → 物理身份和拓扑检查 → 权威代际比较 → 事务恢复 → 发布就绪状态。不得把校验失败自动修正成默认值后继续写入。

减少运行额度只限制新增准入，已有占用超限时显示超额并停止增长，不删除工作数据。修改告警值不改变决定状态；修改份数、成员或绑定也不能通过热更新生效。动态配置验证失败保留上一份有效运行配置，持久业务状态不变。

[配置约束附件](contracts/config-constraints.schema.json)校验最关键的协议断言、资源上限与告警参数；它是完整 TOML 的受检投影，不覆盖全部设备和业务状态。完整部署配置 schema、拓扑交叉校验以及真实启动器仍须在 P0/P1 补齐，不能把投影校验当作部署成功。

---

[文档目录](README.md) · [上一章：工程结构与依赖约束](20-engineering.md) · [下一章：验收标准与证明材料](22-acceptance.md)
