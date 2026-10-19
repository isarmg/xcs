# SMB SDK 与真实客户端验证

2026-10-18，Linux x86_64，Rust 1.99.0，官方 Samba 4.23.0 / VFS 51。使用本项目 `xcs-smb-vfs` release 模块、本地真实核心开发池及官方 `smbclient`，连接限制在 `127.0.0.1:14445`。

[结果清单](result.json)记录官方 tar 摘要、模块摘要、字节比较与未测范围。SDK 已完成 configure、实际服务端及客户端编译；下载 tar 的 SHA-256 已记录，未验证签名。

| 证据 | 执行结果 |
| --- | --- |
| [SDK 测试](sdk-tests.txt) | 4 项通过，包含事件循环/取消、路径隔离及失败文件名查询的指针保留 |
| [配置解析](testparm.txt) | 退出 0；私有目录权限浏览警告保留，不当作没有警告 |
| [模块/协议/关闭](module-protocol-close.txt) | 实际加载 `xcs.so`、`SMB3_11`、CLOSE `NT_STATUS_OK` |
| [客户端操作](client.txt) | 新文件、3407872 字节往返、43 字节覆盖、大小写改名与折叠打开；下载 cmp 及 SHA-256 一致 |

专用共享目录保持为空。测试使用 root Unix guest 身份代理到非管理员 XCS 账户，不证明生产身份部署、普通 Unix 用户权限切换、签名/加密策略或跨平台编辑器矩阵。目录模拟盘不证明三个物理设备的独立持久性。实际客户端 FLUSH、write-through、断网重连、真实掉电未执行；mtime 与按保护路由报告剩余容量尚未完整实现。

本轮隔离 `smbd` 已停止，没有修改系统服务。构建参数、模块语义及恢复边界见[组件 README](../../../integrations/smb/README.md)。
