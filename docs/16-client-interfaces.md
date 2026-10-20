# 16 App、网页与命令行界面

[文档目录](README.md) · [上一章：API、操作结果与错误契约](15-api-and-errors.md) · [下一章：初始化、启停与硬盘维护](17-lifecycle-and-maintenance.md)

<a id="s16"></a>

界面信息、文件交接、任务状态和命令行目标接口。

## 本章目录

- [16.1 用户界面](#s16-01)
- [16.2 文档打开方式](#s16-02)
- [16.3 拟定 CLI 示例](#s16-03)

客户端 App 由本项目自行实现，覆盖 Linux、Windows、macOS、Android、iOS、iPadOS，文件操作及获授权的高级功能均通过业务 API 提供。桌面与移动界面分别适配，业务规则一致。网页和 CLI 提供相同授权体系下的管理入口。

<a id="s16-01"></a>

## 16.1 用户界面

| 区域 | 展示内容 | 必要约束 |
| --- | --- | --- |
| 连接与磁盘入口 | 服务器身份、在线状态、各独立磁盘目录 | 未认证不泄露私有内容；磁盘不作为分支选择器 |
| 文件列表 | 当前磁盘、面包屑、文件夹、文件、大小、搜索和常用动作 | 上传、新建、重命名、删除、预览、下载均围绕当前目录；副本盘明确只读 |
| 文件详情 | 所选文件容器、备注、分支、节点历史、标签、文件分支合并和保存状态 | 分支绑定该文件；合并显式确认参与head、结果与归属，规则见[补充说明](design/file-merge-tags-backup.md)；历史下载使用所选逻辑名称 |
| 目录分支 | 稳定命名分支和不可变子树 | 创建引用共享文件分支，修改/子项选择只追加子树；默认根组合可由有写权限用户选择并供 SMB 读取 |
| 状态 | 简单 CPU 使用率、内存用量和各磁盘容量/在线状态 | 只显示真实采样；未知值显示不可用，不伪造零值或趋势 |
| 历史记录 | 文件路径、动作、作者、顺序和可用时间 | 不拿传输任务或硬盘列表代替文件操作历史；旧记录无时间时明确未知 |
| 高级历史工具 | 历史域、旧MergeLink关系线、拆分来源、PURGED节点 | 不在默认文件页面展示低层仓库/计划表单；旧关系标记与文件详情的FileBranchMerge分别处理，目录分支合并待用户确认 |
| 提交历史 | 版本父节点、合并关系、作者、时间、操作摘要、目录差异 | 内容已删仍显示节点；时间不代替日志顺序 |
| 传输与工作区 | 获取、上传、封存、复制、提交、冲突、待处理 | 本地保存与服务端成功分开显示 |
| 管理与清除 | 来源映射树、盘位/换盘、容量来源占比、清除和拆分 | 用户审查固定目标及迁移范围；均衡只建议不自行改路由 |
| 恢复 | 文件/目录恢复预览、容量和名称冲突 | 历史保存登记新分支，仅实际写入产生 FULL，其余继承父版本，不覆盖主分支 |
| 错误中心 | 原操作、错误码、可恢复状态和请求 ID | 不只弹一次短暂通知后丢失错误 |

<a id="s16-02"></a>

## 16.2 文档打开方式

双击或“打开”由入口能力决定：桌面 App 可以预览或调用本地关联编辑器，移动 App 按系统能力预览或交给外部应用；独立 SMB 入口按协议提供文件读写，由访问方处理本地应用交接。App 内预览读取固定版本，外部编辑后的本地保存与服务器保存状态分别跟踪。显式下载和导出是另一个操作，不作为打开所有文件的前置条件。

不能因为用户点击“打开”便绕过权限或改成从物理盘直接共享路径。

<a id="s16-03"></a>

## 16.3 拟定 CLI 示例

以下命令用于表达目标接口，盘位/设备均为占位，不是已发布命令。内容清除和树拆分先返回预览计划，提交时须携带所审查计划的摘要及代际；历史清除只删除payload并保留拓扑。

```bash
# 查看服务端配置的独立 SMB 共享。
zxs smb shares --repository personal

# 从历史提交元数据创建引用分支，不复制来源正文。
zxs branch create plan-a --repository personal --from-commit commit-example-0012

# 查看历史并比较两个提交，不修改分支。
zxs history --repository personal --branch main --path '文档/报告.docx'
zxs compare --repository personal --left commit-example-0012 --right commit-example-0015

# 超时后查询原操作状态。
zxs operation status op-example-0007

# 从历史节点另存新分支，只有工作区实际写入的文件生成 FULL。
zxs history save preview --history history-example --from-commit commit-example-0012 --new-branch recovered-work --workspace ws-example-recovery

# 查看硬盘和副本健康状态。
zxs disk list
zxs replica status --repository personal

# 查看用户指定的级别与来源映射。
zxs backup-topology show topology-example

# 预览来源分配，并以已确认计划的摘要执行。
zxs backup-topology preview --target-slot B01 --level 2 --sources P01,P02
zxs plan commit plan-topology-example --expected-digest digest-example

# 指定目录后续新版本的一级盘位。
zxs primary-rule preview --history history-example --path / --primary-slot P01

# 为既定盘位更换物理盘，保留来源职责。
zxs slot replace preview --slot B01 --new-disk disk-example-replacement

# 仅记录两个指定版本的关系线，不保存文件或推进分支。
zxs merge link --history history-example --from-branch plan-a --from-commit commit-example-a2 --to-branch main --to-commit commit-example-0012

# 检查活动引用后清除历史内容，保留历史节点。
zxs history purge preview --history history-example --versions version-example-001
zxs plan commit plan-purge-example --expected-digest digest-example

# 按选定节点及其实际后代拆分为独立历史树。
zxs tree split preview --history history-example --at commit-example-0012 --new-name independent-tree
zxs plan commit plan-split-example --expected-digest digest-example

```

创建分支、拓扑/盘位迁移、合并连线记录、树拆分、内容墓碑发布、物理清理、下载、提交和逐级副本修复完成必须是不同事件。CLI 的结构化输出应包含明确状态，便于脚本判断而不是解析自然语言“成功”。

---

[文档目录](README.md) · [上一章：API、操作结果与错误契约](15-api-and-errors.md) · [下一章：初始化、启停与硬盘维护](17-lifecycle-and-maintenance.md)
