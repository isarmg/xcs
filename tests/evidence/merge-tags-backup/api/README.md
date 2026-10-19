# 文件合并、标签与备份的接口验证

所有业务发布、故障和恢复操作均使用临时存储池，没有写入正在使用的预览池。

- [服务端与客户端完整回归](merge-tags-backup-server-client-tests.txt)：85项通过，包含客户端8、服务端单元57、真实引擎HTTP19及监测接口1。
- [公开核心API回归](merge-tags-backup-core-tests.txt)：11项通过，含原有6项及新增5项文件合并场景。
- [合并兼容缓存修正回归](merge-pending-cache-tests.txt)：针对合并后主目录旧待写入缓存的最终小修重新执行5项合并场景，全部通过。
- [机器契约](contract-tests.txt)：11项通过。请求必须携带明确确认、结果归属、各参与head及备份scope身份/代数；统一节点图区分正文和元数据，真实HTTP回执也按schema校验。
- [只读别名的SMB错误映射](smb-errno-test.txt)：临时Unix socket验证`BRANCH_NOT_ACTIVE`保留诊断并映射`EROFS`。
- [真实HTTP回执采集](http-feature-wires.txt)：标签继承、重命名后历史路径、任意同文件节点合并、只读别名、权限拒绝、独立备份前缀、回填、scope身份及代数冲突。
- [备份命名空间补充回归](backup-namespace-tests.txt)：新增3项综合场景通过，覆盖B/C用户前缀的根与子目录、主目录和文件改名后路径独立、该槽固定旧版本、无live alias的历史读取、只读拒绝、授权后冲突检查，以及跨一级来源链与普通读用户磁盘可见性。HTTP文件现共22项，以上85项完整记录保留其当时19项HTTP范围；最终全仓输出见上级目录。
- [合并与标签历史图](wire/file-merge-tag-history.json)和[文件scope的备份策略](wire/scoped-backup-configuration.json)：合成临时池的真实HTTP响应。
- [固定备份路径与旧正文](wire/backup-prefix-files.json)、[无live alias的保留文件](wire/backup-prefix-retained.json)、[多条历史来源链](wire/backup-mixed-routes.json)及[按历史授权的磁盘列表](wire/backup-route-disks.json)：本次补充回归的真实响应，均参与schema校验。

新增核心场景分别覆盖：采用第三节点生成一个元数据子节点；复用祖先head时避免成环且保留全部合并来源；同文件多个路径的未参与分支保持冻结；连续两次合并后旧只读别名跟随新的归属；父目录冻结的旧子树独立派生且不带入子目录新head中的其他文件。所有情况保留原正文版本及读取能力，并重新打开临时池验证持久化。

当前接口为文件级合并。目录分支合并待用户确认，暂不实现且没有写端点。这里的自动化通过不代表独立物理盘的断电、已部署Samba或六平台设备验收。
