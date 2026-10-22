# 服务入口与管理迁移验收清单

正常服务入口是直接运行的 `zxs`，离线工具是明确命名的命令行工具 `zxs-rescue`。旧管理 dispatcher 和其源目录已经删除，不以改名、隐藏发布物或兼容旧入口代替职责拆分。日常管理与初始化通过 Web 完成；本机配置草稿和真正离线救援保持明确边界。

## 基线审计（8a0038d8）

| 原命令 | 原基线缺口与审计要求（历史记录，不是当前状态） |
| --- | --- |
| config-template / init / init-auth / register-disk | 原 Web 无首次初始化流程。需管理面安全引导、受保护凭据初始化、配置预览/设备检查后才能删除。register-disk 只改未初始化草稿，不是在线换盘。 |
| account | 原 Web 只有本人改密，无账号授权、禁用和管理员配置。 |
| create-repo / policy / rule / smb-binding | 原 Web 缺少完整管理表单；必须保留并验证世代校验、活动工作区冲突与权限。 |
| repositories / disks / topology / list / history / graph / operation / check | 原 Web 有文件/版本、磁盘/状态和活动列表，但不是所有原始状态与完整性检查的一对一替代。 |
| branch / namespace | 原 Web 有文件/目录分支和命名空间操作，须验证对应作用域；不能据类似按钮推断整历史域能力相同。 |
| link / purge / split / historical-save / plan-commit / plan-cancel | 原 Web 缺少完整高级计划流程。需受审查预览、摘要/世代校验、明确提交和取消。 |
| export-history / check-history-package / restore-history-package | 完整独立历史包不是单版本救援导出；保留在 zxs-rescue package export/check/restore 子域，不新建 Web 文件正文传输入口。 |
| serve | 被直接 daemon 入口替代；新入口不预先打开 Engine。 |
| restore-system / recovery-preview / recovery-commit / offline-check / offline-export | 保留到独立最小 rescue。 |

## 正常启动边界

`zxs --root SYSTEM_STATE --management-root SYSTEM_MANAGEMENT --web-dir WEB_ROOT` 直接监听 Web/API。没有管理子命令分发。系统盘管理目录与数据成员分离；数据引擎打开在 HTTP 监听之后进行。数据故障不等于授权放宽，也不会自动初始化、清空池或接受损失。

systemd 样例不依赖数据成员挂载成功，不先检查池配置文件。它不会创建、挂载或格式化数据设备。`ReadWritePaths=-...` 允许缺失路径，核心仍验证设备身份。Samba/smbd 继续是独立网络服务；daemon 的可选 Unix socket 只是受信任本机桥接，不是 SMB 网络服务器。

## 救援限制

- restore-system：要求已配置原始元数据成员的严格多数认可同一配置（N/2 向下取整再加一），不能按当前可访问盘数降低门槛。单盘池须该原始盘可用；双成员池须两盘，三成员池须两盘。单盘池无磁盘冗余，唯一存储盘丢失时不能凭空恢复。
- offline-check / offline-export：只核验可访问盘的材料；只导出本盘验证通过的指定已发布版本，不保证全池或最新权限视图。
- recovery-preview / recovery-commit：针对已经获得 ACCEPT 但尚未发布、全部原始磁盘仍可访问、且内容确已不可修复的窄场景。需要独占锁、预览摘要和显式永久丢失承认。不是丢盘替换、已发布结果改写或自动 reset。

需要打开池的救援操作必须先停止该池服务并持有适当本机权限。配置草稿编辑不是在线拓扑迁移，不提供隐藏的日常管理或远程客户端入口。

## 可选备份配置

除系统盘外，最低只需一块存储盘；第二、第三、第四及后续备份层均可不配置。`zxs-rescue configuration template --disks /absolute/primary` 生成单盘草稿，不访问或初始化设备；按顺序增加路径即可生成逐级备份链，实际配置仍须通过设备与拓扑校验。草稿注册命令不能修改已初始化的池或在线变更元数据成员。

正常元数据发布仍要求所有已配置元数据成员持久化成功；多数门槛只用于相应恢复/准入流程。不能把失联成员自动移出配置。单盘模式保留校验、写入与恢复协议，但没有额外副本或故障盘容错。`package restore` 默认请求一个副本，需额外备份时明确指定 `--copies N` 并预先配置足够的备份链。

## 首次初始化部署授权

启动参数可重复设置 `--setup-disk ID=/absolute/existing/root`，只向 Web 提供本机管理员批准的磁盘 ID，不能由浏览器提交任意系统路径。初始化需要管理目录中预先配置的私有 `bootstrap.key`（0600，恰好 64 个十六进制字节、无换行，mtime 起 30 分钟有效）；daemon 不自动创建或日志输出引导密钥。空白系统不会因数据引擎打开失败而自动创建管理账号或存储池。

仅本地开发测试允许显式传入 `--allow-development-setup`，启用目录模拟模式；生产默认拒绝这种初始化。此开关不代表真实设备独立性、Btrfs 或断电验证通过。

`zxs-rescue package export HISTORY DESTINATION`、`package check PACKAGE`、`package restore PACKAGE --primary-slot SLOT --copies N` 保留原完整历史包操作。导出目标不得存在，父目录必须由当前用户所有且 0700；导入保持原校验、存储池约束与操作幂等校验，不提供覆盖现有数据或跳过验证选项。执行导出/导入前停服务，避免与守护进程争用存储锁。

## 最终能力矩阵与安全收窄

| 原能力 | 当前入口与验证 |
| --- | --- |
| serve | `storage-server` 直接二进制 `zxs`；真实进程验证无子命令启动、帮助不建运行时、缺失/损坏池仍提供 Web/health |
| config-template / register-disk | `zxs-rescue configuration template/register-disk`；原拓扑/别名/已初始化成员拒绝、备份与原子文件回归迁入 `test/rescue/unit`，另有实际命令端到端测试 |
| init 最低一块存储盘 | 受保护 Web 引导；本机磁盘白名单、引导密钥、空池、生产设备/挂载约束，不自动初始化 |
| init --config | 操作员本机预置 `--setup-config ID=/absolute/private-file`，Web 选择、预览摘要并明确提交；实际四盘/三元数据成员和非默认配置保留回归，不能传远程任意路径/JSON |
| init-auth | 仅保留可证明的中断初始化补全：绑定原配置与 setup intent、尚未发布管理权威。显式授权、凭据校验和生命周期门控；不是密码重置 |
| account / create-repo / policy / rule / smb-binding | 经认证的 Web 管理表单及同构建器实际 HTTP 回归；账户管理依赖系统盘权威，其他写操作依赖当前存储可写状态 |
| repositories / disks / topology / list / history / graph / operation / check | Web 文件管理、健康与管理审查查询；同一业务 HTTP 权限和错误边界 |
| branch / namespace | 历史分支表单及已有分支选择、文件夹/命名空间批量操作；保留非默认分支与世代/CAS 回归，不新增内容传输 |
| link / purge / split / historical-save / plan-commit / plan-cancel | Web 高级管理；实际请求构造器 HTTP 回归覆盖预览、摘要/世代冲突、明确提交与取消，已有工作区 ID 不是文件上传 |
| restore-system / recovery-preview / recovery-commit / offline-check / offline-export | `zxs-rescue`；固定配置多数恢复、明确损失承认、单版本核验边界保留 |
| export-history / check-history-package / restore-history-package | `zxs-rescue package export/check/restore`；实际跨池恢复后字节一致性、拒绝覆盖、拒绝损坏发布证明 |

有意拒绝没有可信权威快照、也没有绑定 setup intent 的孤立旧池创建新管理员。不能为逐字兼容旧 `init-auth` 引入凭据重置旁路；此场景须依赖可信恢复材料或人工离线审查，不宣称任意遗留池都可继续初始化。已有权威状态缺失/损坏时同样不自动生成管理员或回滚旧缓存。

日常界面与端点的逐项对应见[日常管理覆盖](daily-management-coverage.md)。运行时初始化/恢复测试位于 `test/server/unit/setup_recovery_tests.rs` 与管理面回归中；浏览器烟测包含数据池故障、默认初始化、预置摘要变化和中断补全场景。原生工具单测/编译不能冒充真实 Chrome 运行；默认沙箱不可用的环境由正式 GitHub CI 执行浏览器验收，不关闭浏览器沙箱。
