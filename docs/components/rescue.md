# 离线救援工具

`storage-rescue` 生成 `zxs-rescue`，它是明确命名的命令行救援工具，不是日常管理客户端。正常入口是直接运行的 `zxs` 守护进程，初始化与账户、仓库、策略、计划等操作在 Web 管理面进行。SMB 网络服务仍由独立 Samba/smbd 桥接提供。

所有需要打开池的救援操作应先停止该池的服务。`--root` 指向系统状态目录，`--principal` 指定原有核心 ACL 身份，默认 `admin`；没有绕过权限的管理员重置入口。帮助、版本与参数错误不启动服务或工作线程。失败返回非零退出码。

## 恢复系统盘

```sh
zxs-rescue --root /srv/zxs/new-system restore-system \
  --disks /srv/zxs/old-metadata-1 /srv/zxs/old-metadata-2
```

上述为多成员示例。实际至少需要固定配置的 Q = floor(N/2)+1 个可信原元数据成员，N ≥ 1；单成员池可只传唯一旧盘。不能拿空盘、替换盘或任意目录替代，不能按在线数量重算成员集合。所有新初始化盘保存池配置锚；缺锚旧盘不兼容，不自动补锚或迁移。核心校验发布证据、成员与池身份并保留冲突拒绝规则。工具不格式化磁盘、不自动重置、不在多成员池中根据不足 Q 个旧成员推断整个池的最终状态。单成员盘永久丢失不具备原池恢复保证。

输出中的 `credential_recovery_snapshot_available` 只表示恢复后的池中存在凭据灾备快照，不表示已创建或替换管理面权威凭据。工具不写旧 `auth.json` 缓存，也不覆盖独立系统目录中的 `management-auth.json`。随后启动守护进程时，由管理面按权威迁移规则决定是否可以导入经恢复验证的快照；已有权威标记或更晚的管理凭据不能被旧快照静默回滚。

## 受限降级恢复

```sh
zxs-rescue --root /srv/zxs/system recovery-preview OPERATION_ID --reason '经审查的原因'
zxs-rescue --root /srv/zxs/system recovery-commit RECOVERY_ID \
  --digest REVIEWED_DIGEST --accept-permanent-loss
```

仅支持已经决定但尚未发布、且所有原始数据盘都可访问的操作。先审查持久预览和损失范围，再提交完全匹配的摘要并明确接受永久数据损失。缺盘替换、自动重试发布、自动 reset 和任意撤销已发布历史均不属于此能力。

## 单盘核验与隔离导出

```sh
zxs-rescue offline-check /srv/zxs/old-disk
zxs-rescue offline-export /srv/zxs/old-disk VERSION_ID /srv/rescue/new-file
```

在停止写入的可信原成员或一致的只读副本上操作。核验或导出只证明被检查的发布证据、单个版本与其数据完整性，不证明整个池可恢复、当前 ACL、最新历史或所有副本健康。导出目标不得已存在；不要把隔离导出结果直接当作可重新加入池的磁盘。

## 历史恢复包

```sh
zxs-rescue --root /srv/zxs/system package export HISTORY_ID /srv/private-rescue/new-package
zxs-rescue package check /srv/private-rescue/new-package
zxs-rescue --root /srv/zxs/new-system package restore /srv/private-rescue/new-package \
  --primary-slot P01 --copies 3
```

保留原有完整历史包导出、独立核验和新池恢复能力，不是 Web 文件传输。导出目标不得已存在，父目录必须为当前用户拥有的私有 0700 目录；包的路径、发布证明、摘要、对象与历史结构仍由原核心实现核验。没有外部可信成员配置时，独立包检查仅证明包内证据与内容自洽，不赋予来源池成员关系权威。恢复要求目标池已安全初始化，保留目标身份、权限、副本和新池恢复约束，导入形成目标池的新本地发布，不沿用外来发布作为本池权威，不能覆盖现有目标或绕过损坏证明；可用 `--operation-id` 指定经审查的幂等操作标识。

真实二进制回归见 `test/rescue/startup.rs`：命令边界、损失确认、双元数据恢复、跨池包恢复后的字节一致性、拒绝覆盖和拒绝损坏发布证明。

## 未初始化拓扑配置草稿

```sh
zxs-rescue configuration template --production \
  --disks /srv/zxs/disk1 /srv/zxs/disk2 /srv/zxs/disk3 --output /srv/private-layout/custom.json
zxs-rescue configuration register-disk --config /srv/private-layout/custom.json \
  --slot P02 --disk-id REVIEWED_DISK_ID --path /srv/zxs/disk4 --level 1
```

此子域只生成或编辑未初始化的配置草稿，不打开池、不初始化设备、不注册在线磁盘，也不提供账号初始化或远程客户端。`template` 生成随机池/盘 ID；必须显式指定 `--production` 才是生产草稿。`register-disk` 保留原草稿到完整文件名加 `.before-registration` 的备份，再以描述符锚定的原子写入发布 0600 文件。再次编辑保留紧前一份草稿。从读取、校验到备份和发布，整个事务持有父目录的排他建议锁，而不是会被原子替换的草稿 inode；同目录并发编辑（包括父目录别名）在读取前明确失败，须待前次编辑结束后重新运行，不会静默覆盖成功编辑或保存过时备份。锁不创建边车文件；标记检查及两次发布始终使用同一已打开的父目录描述符。此锁只协调救援草稿编辑，不能替代停止守护进程及排除其他写入者。已有池或管理凭据标记、已入池盘、复制出去的在线配置、链接、重叠根目录、非法拓扑及不可信文件均被拒绝；不能用不相关的 `--root` 绕过这些检查。

自定义草稿通过守护进程本机参数 `--setup-config ID=/absolute/private-config.json` 预置，再由受保护 Web 首次初始化选择该 ID。文件须由服务账号拥有、为单链接普通文件、权限 0400/0600；浏览器不能提交任意路径或任意 Config JSON。所有配置磁盘仍必须在 `--setup-disk` 白名单中，沿用生产挂载、设备身份、空池和引导授权检查。缺失或损坏的预置文件不能阻止管理 HTTP 服务启动。Web 向导最少选择一个存储盘，多盘时按选择顺序建立可选备份链；单盘没有冗余。预置方案保留经验证的高级配置参数。
