# Linux 服务部署审阅样例

[zxs.service](zxs.service) 是供部署前修改和审阅的 systemd 单元，没有安装、启用或启动。它默认只提供回环 HTTP API；通过配置正确的 HTTPS 反向代理开放 App/Web 接入。`zxs.example.com`、程序路径和三个盘位路径必须换成实际配置。

## 部署前条件

- 创建专用 `zxs` 服务账户；程序和 Web 发行件安装在其不可写的 `/opt/zxs` 下。`/var/lib/zxs`、受保护 socket 父目录及每个已登记盘根要求服务账户拥有的 `0700` 权限，文件默认 `0600`。初始化在服务停止时以 `zxs` 身份使用正式生产配置执行，不能把开发模式目录当作三块物理盘。
- 各盘是独立物理设备上的独立 Btrfs，已经由管理员按 UUID 挂载。按真实配置调整 `RequiresMountsFor=` 与 `ReadWritePaths=`，所有已登记数据盘都要列入；不能把挂载失败后的普通空目录当作磁盘。核心另外核对设备、UUID、根目录身份和登记封口。
- 日常账户不能打开原始块设备或数据盘目录。部署者需要单独设置设备访问和自动挂载策略，关闭会把业务盘重新暴露给普通用户的桌面自动挂载流程。`PrivateMounts=yes` 不会自动撤销宿主机原有目录或设备权限。
- SQLite 是可重建索引；不要只备份 SQLite。保护元数据、配置和凭据的恢复材料均按项目恢复流程处理。敏感内容不通过环境变量、命令行或 systemd 单元传入。

单元以非特权账户运行，不授予挂载或原始块设备读写能力。`DevicePolicy=closed` 与只读系统目录限制设备和路径访问；业务文件 I/O 只写明确列出的目录。配置需要读取 `/proc/self/mountinfo`、`/sys` 和设备 UUID 元数据，因此未使用隐藏整个设备树的 `PrivateDevices=yes`。此样例不包含挂载辅助进程、AppArmor/SELinux 策略或硬件验收结果。

检查单元语法可执行 `systemd-analyze verify deploy/systemd/zxs.service`；只有替换完示例路径、准备对应程序和挂载环境后，才可执行目标主机的实际启动验收。检查语法不能证明设备隔离、供电故障恢复或上线可用性。

## 可选 Samba 本机桥接

必须先完成 [Samba 模块构建和实际加载验收](../../integrations/smb/README.md)。可在单元 override 中重置 `ExecStart=`，加入实际受信任进程 UID 和显式的非管理员账户映射，例如：

```ini
[Service]
ExecStart=
ExecStart=/opt/zxs/bin/zxs --root /var/lib/zxs/state serve --listen 127.0.0.1:7400 --external-origin https://zxs.example.com --web-dir /opt/zxs/web --smb-socket /run/zxs/smb.sock --smb-peer-uid 0 --smb-user SambaAlice=alice
```

示例的 `0` 只适用于确认桥接在 Samba 可信 root 进程身份下连接的部署。该连接时机必须在目标 Samba 验证，不能放宽 socket 权限来适应未认证的进程。`/run/zxs` 及 socket 分别维持 `0700`、`0600`，映射用户仍需当前账户准入和仓库授权。共享仓库拆分、恢复后必须显式重新绑定，不能自动接入迁移后的历史域。

服务使用 `SIGINT` 触发当前 CLI 的正常退出路径。停止时释放 socket 和文件句柄；未成功保存的完整工作文件及恢复记录保留。崩溃遗留的 socket 不自动删除，管理员确认路径身份和无存活服务后再清理。不要配置无条件删除 socket 或清空暂存目录的启动脚本。
