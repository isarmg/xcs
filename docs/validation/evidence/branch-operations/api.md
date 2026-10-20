# 分支操作的 API 与 SMB 验证

这里保存临时存储池下的自动化验证，不对正在使用的预览池写入测试文件。

验证范围包括：目录分支登记不复制正文和不创建占位文件；冻结子树恢复；共享 SMB 默认入口与旧写会话隔离；按路径区分同一文件的多个别名；显式分支映射的移动、独立复制、删除、重命名；幂等、过期依据和权限拒绝；旧读取描述符与未选分支保持原版本；对应的 HTTP 契约及客户端适配。

先前完整回归记录：服务端与客户端 81 项通过（客户端 8、服务端单元 57、HTTP 15、监测接口 1）；独立 SMB SDK 6 项通过。最新机器契约检查 10 项通过，包含路径节点与真实正文 ID 分离、混合父边和独立节点备注。

随后补充的公开核心 API 集成测试 6 项通过，覆盖文件夹共享复制身份、别名重命名、无效目标的原子拒绝、部分删除以及 P2 文件分支标签改名。新增的文件分支标签 HTTP 回归 1 项通过，当时 HTTP 测试文件共 16 项；随后新增的路径节点 HTTP 综合回归也已通过：文件夹及后代文件改名复用原正文、独立备注、来源节点建分支和后续写入的真实父边、错误身份及只读权限拒绝。HTTP 测试文件现共 17 项；最终整体执行记录由上级证据目录保存。

- [服务端与客户端输出](../../../../tests/evidence/branch-operations/api/server-client-tests.txt)
- [独立 SMB SDK 输出](../../../../tests/evidence/branch-operations/api/smb-sdk-tests.txt)
- [机器契约输出](../../../../tests/evidence/branch-operations/api/contract-tests.txt)
- [公开核心 API 别名回归](../../../../tests/evidence/branch-operations/api/core-alias-tests.txt)
- [文件分支标签 HTTP 回归](../../../../tests/evidence/branch-operations/api/file-label-http-test.txt)
- [路径节点 HTTP 回归](../../../../tests/evidence/branch-operations/api/path-node-http-tests.txt)
- [统一文件历史图](../../../../tests/evidence/branch-operations/api/wire/file-path-history.json)
- [批量移动回执](../../../../tests/evidence/branch-operations/api/wire/branch-namespace-batch.json)

`wire/` 仅包含合成测试池的操作回执。Unix 描述符测试覆盖本机桥接协议，不代表部署的 smbd 或六平台设备验收。
