# zxs 命令行

参数声明在 `args`，本机管理与业务命令在 `local`，HTTP 命令在 `remote`；`credentials` 负责有界秘密输入，`pending` 负责配置/待重试请求，`private_io` 统一以目录句柄读写私有文件。入口 `main` 只选择执行模式和报告结果。

`zxs-cli` 构建出 `zxs`，支持本机存储管理和通过 `zxs-client` 访问远端 API。输出为 JSON，失败退出码非零。`zxs --help` 和各子命令 `--help` 列出实际参数。

## 构建与初始化

```sh
CARGO_TARGET_DIR=target/native CARGO_INCREMENTAL=0 cargo build -p zxs-cli --offline
target/native/debug/zxs config-template --production \
  --disks /srv/zxs/disk1 /srv/zxs/disk2 /srv/zxs/disk3 --output /tmp/zxs-config.json
target/native/debug/zxs --root /srv/zxs/system init \
  --config /tmp/zxs-config.json --username admin --password-file /run/credentials/zxs-admin
target/native/debug/zxs --root /srv/zxs/system serve \
  --listen 127.0.0.1:7400 --web-dir /srv/zxs/web/dist
```

`config-template` 只生成带随机 pool/disk ID 的实际 Config JSON，不接触设备；输出文件不得已存在。先审查路径、盘位、容量和限制。生产模式要求已挂载的独立 Btrfs 数据盘，不会挂载、分区或格式化磁盘。JSON 的 u64 字段必须写为规范十进制字符串。默认初始路由为 P01 → B01 → C01，三个盘都是固定元数据成员。

仅开发验证可用 `init --development-disks /绝对路径/d1 /绝对路径/d2 /绝对路径/d3 --username admin --password-file ...` 模拟三盘；开发模式不验证物理设备独立性。密码通过私有文件或未提供 `--password-file` 时的标准输入读取，至少 12 字节，不接收明文命令行参数，也没有默认密码。密码和 token 文件必须由当前用户拥有、为单链接普通文件，权限为 0400 或 0600；拒绝符号链接。权限检查与实际读取针对同一个已打开描述符。密码最多 1024 字节，文件或标准输入额外允许至多两个结尾换行字节，超限时拒绝。

本机命令使用 `--root`（默认 `./zxs-data`）定位系统目录，`--principal`（默认 admin）指定核心 ACL 身份。非 admin 初始账号须显式传入该身份。本机目录访问权属于系统管理员信任边界；本机命令不能与占用同一存储池的服务进程同时打开。

## 本机常用操作

```sh
zxs --root /srv/zxs/system create-repo 文档 --primary-slot P01 --copies 3
zxs --root /srv/zxs/system put BRANCH_ID 文档/说明.txt ./说明.txt --message 首次保存
zxs --root /srv/zxs/system list BRANCH_ID
zxs --root /srv/zxs/system get VERSION_ID ./导出.txt
zxs --root /srv/zxs/system history FILE_ID
zxs --root /srv/zxs/system graph HISTORY_ID
zxs --root /srv/zxs/system branch COMMIT_ID 独立分支
zxs --root /srv/zxs/system namespace BRANCH_ID mkdir 文档
zxs --root /srv/zxs/system namespace BRANCH_ID rename 原文件.txt 新文件.txt
```

ID 从前一步 JSON 输出取得。`get`/`export` 拒绝覆盖目标。`put` 创建工作区、保存完整文件并原子提交；可指定固定 `--operation-id op_...`，远端还支持原样提交重试。`link SOURCE_BRANCH SOURCE_COMMIT TARGET_BRANCH TARGET_COMMIT` 只添加历史关联，不自动合并文件内容。

`purge HISTORY_ID VERSION_ID...`、`split HISTORY_ID CUT_COMMIT_ID`、`historical-save BRANCH_ID SOURCE_COMMIT_ID 新分支名` 默认只生成持久预览；检查输出后用 `plan-commit PLAN_ID --digest DIGEST` 执行，或显式给原命令加 `--execute`。`plan-cancel PLAN_ID` 取消尚未执行的计划。业务状态变化可能使预览失效，需重新预览。历史保存可附加 `--source ./完整文件 --path 文档/文件.txt`，在一个事务内建立独立全量分支并发布修改；不修改原分支 HEAD。

`policy new REPOSITORY_ID --path-prefix 文档 --copies 3` 与 `rule new REPOSITORY_ID --path-prefix 文档 --primary-slot P01` 创建后续文件的默认策略；更新时用原 ID 和 `--expected-generation 当前值`。已有版本不会被这些默认策略悄悄迁移。

`smb-binding REPOSITORY_ID BRANCH_ID --enabled true --expected-history-generation 当前值` 显式绑定 SMB 分支。拆分后的仓库不会自动继承 SMB 发布状态；存在活动工作区时拒绝改绑定。

`repositories`、`disks`、`topology`、`check` 和 `operation OPERATION_ID` 分别查询仓库、盘位、拓扑、完整性与操作结果。`register-disk --config CONFIG ...` 仅编辑未初始化的配置草稿并保留备份，不能代替运行中拓扑迁移。

## 远端会话

```sh
zxs --server https://storage.example.org login --username admin \
  --password-file /run/credentials/zxs-admin --output /tmp/zxs-token
zxs --server https://storage.example.org --token-file /tmp/zxs-token repositories
zxs --root /srv/zxs/client-cache --server https://storage.example.org \
  --token-file /tmp/zxs-token put BRANCH_ID 文档/说明.txt ./说明.txt
zxs --server https://storage.example.org --token-file /tmp/zxs-token logout
```

只接受 HTTPS 或回环 HTTP，不跟随重定向。会话 token 写入新建的 0600 文件，拒绝覆盖或读取宽松权限文件；不会输出 token。`whoami` 显示当前会话。`account --username ... --password-file ... --repository REPO_ID` 管理账号，`--disabled` 撤销账号，`--admin` 设置管理员；账号更新会撤销该账号现有会话。仓库读写 ACL 通过服务端管理 API 设置，账号准入不自动授予仓库权限。

Token、配置输出和待重试请求先完整写入同目录的 0600 暂存文件，文件同步后才无覆盖发布，并同步父目录。失败清理自身暂存文件；输出父目录可使用 `/tmp` 等现有目录。配置草稿注册盘的备份和替换也以完整文件发布，拒绝通过符号链接改写其他文件。登录结果中的 `expires_at` 及拓扑代际按十进制字符串输出。

远端支持仓库、盘位查询、上传下载、历史图、分支、合并关联、命名空间操作、保护策略、SMB 绑定、计划和检查。完整上传预先计算 BLAKE3；下载校验长度和 BLAKE3，成功后发布本地文件。

远端 `put` 和 `namespace` 在发送 commit 前把完整封存请求保存在 `--root/client-pending/OPERATION_ID.json`。连接中断或结果未知时先查 `operation`，也可用 `retry-commit REQUEST_FILE` 原样重发；不要改变请求内容或换 operation_id 猜测是否保存成功。文件保留是为了显式重试，成功后可由管理员清理。

## 系统盘恢复与独立历史包

```sh
zxs --root /srv/zxs/new-system restore-system --disks /srv/zxs/disk1 /srv/zxs/disk2 /srv/zxs/disk3
zxs offline-check /srv/zxs/disk1
zxs offline-export /srv/zxs/disk1 VERSION_ID ./离线文件
mkdir -m 700 /srv/zxs/history-backups
zxs --root /srv/zxs/system export-history HISTORY_ID /srv/zxs/history-backups/package1
zxs check-history-package /srv/zxs/history-backups/package1
zxs --root /srv/zxs/new-pool restore-history-package /srv/zxs/history-backups/package1 --primary-slot P01 --copies 3
```

`restore-system` 需要至少两份相同存储池的独立成员配置与可重放数据，恢复新系统目录及权威账号/撤销状态，不重新初始化数据盘。重复传入同一块盘不算多个成员。`init-auth` 仅供已有池完全没有权威认证状态时显式初始化；认证文件丢失应正常恢复，不能用它覆盖已有授权。

对于已经作出决定、尚未发布且原定副本确实永久丢失的操作，可在停服并恢复全部原始元数据成员后使用本机维护入口：

```sh
zxs --root /srv/zxs/system recovery-preview OPERATION_ID --reason 已核实的永久丢失原因
zxs --root /srv/zxs/system recovery-commit RECOVERY_ID --digest PREVIEW_DIGEST --accept-permanent-loss
```

预览冻结原始 ACCEPT、成员日志、绑定代际和实际副本证据；必须先核对输出。执行要求同一操作者、相同摘要及显式接受永久丢失，不能用于普通新写入，也不会把已有正常发布结果改成降级。成员不可访问、证据无法核实或状态变化时拒绝继续。这两个入口只在本机可用，不提供远端 HTTP 操作。

单盘离线命令不要求打开系统目录。历史包包含独立校验所需的历史图、完整内容和 ACL；导出目标不得已存在，其父目录必须已存在、由当前用户拥有且权限 0700。导入在已初始化的新池中生成独立仓库，保留原 ACL、给予恢复操作者 manage 权限，默认关闭 SMB。包和普通导出仅限本机操作，不暴露远端任意文件系统路径。

当前没有线上换盘/排空/弹出命令，不支持远端偏移分片上传。上传文件与删除/重命名目标相互覆盖的复杂工作区会保留内容并拒绝提交。生产 Btrfs 设备、真实 Samba SDK 模块及浏览器与 SMB 客户端组合需在各自环境继续验收；开发目录测试不能代替这些验收。

已执行的脱敏验证见 [API/CLI 证据](../../tests/evidence/api/README.md)，包括 6 项 HTTP/Backend 集成回归、实际本机与远端 CLI、恢复入口守卫，以及降级发布与成功后损坏的原始 SaveOperation schema 校验。
