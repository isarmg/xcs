# 存储与独立恢复专项

最新记录：[storage-generation-tests.log](../../../../tests/evidence/composed-branches/storage-generation-tests.log)，20 项通过（容器 7、历史包 5、维护 8）。源码摘要见 [storage-source-receipt.json](../../../../tests/evidence/composed-branches/storage-source-receipt.json)。全部操作使用临时 fixture，不读取或更改真实运行数据。

- 分支登记仅引用父真实版本，不新建 `.pending` 文件或空 legacy 容器；重复投影、改名、决定后恢复和导入都保持该约束，源正文内容与 inode 不变。
- `container.json` 的 `inherited_branches` 记录可见来源版本与空自有版本；缓存缺失可由发布记录重建。
- 兼容清理先验证旧目录全部项目，仅删除合法零字节单链接普通占位及其空目录。未知项、非空内容、符号链接和硬链接全部保留并报错。
- 历史清单携带目录组合、选择和字符串代际；验证稳定目录身份、冻结来源、共享文件分支同域、映射覆盖、父闭包及循环，导入后重启仍保留。共享文件分支后来推进不更改旧组合引用。
- 选中或尚未完成的组合固定来源正文；已完成且未选中的旧组合允许按现有清理权限清正文，保留元数据并如实返回 `CONTENT_PURGED`。工作区及活动文件分支原有固定逻辑继续保留。
- 含目录组合的历史拆分明确返回 `DIRECTORY_HISTORY_SPLIT_UNSUPPORTED`，防止丢失尚未实现重映射的组合引用。旧整树拆分仍检查继承映射和独立恢复。

[storage-composition-tests.log](../../../../tests/evidence/composed-branches/storage-composition-tests.log) 保留首次扩展运行中的测试错误：测试误把嵌套文件的父目录行当作文件。按条目类型定位文件后，[storage-composition-final.log](../../../../tests/evidence/composed-branches/storage-composition-final.log) 与最新含代际运行均通过。[storage-tests.log](../../../../tests/evidence/composed-branches/storage-tests.log) 是尚未加入组合记录前的 18 项验证。

本记录只覆盖上述存储边界；不代表整个工作区、浏览器、SMB 实机或物理断电验收。此前 lazy-branches/inherited-branches 证据按原方案保留，不覆盖。
