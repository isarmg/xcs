# 分支操作 Web 验收

本次将文件多选、分支范围和源目标映射落到 Rust/WASM 界面；不改变既有字体比例、Apple G4 轮廓、登录样式或浏览器缩放规则。

## 已实现

- 文件／目录复选框与筛选后全选；同一文件身份的不同逻辑路径拥有独立选择及名称输入。
- 移动、复制、删除、重命名均在页面内审阅，删除不再弹窗。逐项名称采用最多 128 UTF-8 字节的下划线输入。
- 源目录分支显式勾选；移动／复制逐条选择源分支与目标分支，可添加一源多目标映射，不暗中展开为两两组合。
- 目标目录和目录分支只读加载，不临时切换 SMB 默认入口；旧命名空间可显式登记目录分支基线，GET 不写入记录。
- 每条请求包含稳定条目身份、逻辑路径和捕获的依据子树。某个选中分支缺少条目时整批拒绝，服务端 409 保留审阅内容。
- 未确认的操作绑定原操作 ID、阻止再次提交；原操作轮询确认后才清除草稿，避免复制操作重复生成副本。
- 文件历史及文件分支选择携带 `repository_id` 和逻辑 `path`。当前显示 Main 时再次点选 Main 仍提交显式目标，不因名称、节点相同而省略操作。
- 当前公共文件分支（含首次写入自动生成的分支）可在文件详情选择器旁内联改名；PATCH 只提交名称和原操作 ID，不复制正文或切换分支。
- 历史树优先使用 `nodes` 的节点身份和父边，兼容仅有 `versions` 的旧服务；路径节点和正文节点可以共享同一正文版本。路径节点仍三行，空备注显示改名前后路径，备注独立 PATCH；创建分支分别提交正文版本 ID 和来源节点 ID。
- 原有根目录 SMB 默认选择、目录分支内联改名及父下子上、等粗正交圆角箭头保留。新批量审计动作提供中英文标签。

代码按 `web/src/files/batch/{mod,request,actions,view}.rs` 分离草稿、纯请求校验、会话隔离发布和展示。公共行高、内容间距和轮廓继续使用已有组件，不新增页面专用尺寸体系。

## 验证记录

| 记录 | 结果及范围 |
| --- | --- |
| [Rust 测试](web-rust-tests.log) | 101 项通过、无警告：公共运行时 26、组件 3、响应式状态 2、应用 70 |
| [批量浏览器结果](web/browser-result.json) | 中文／1440px／浅色与英文／390px／深色各 9 组交互；含双源双目标移动、一源多目标复制、取消、冲突、删除、基线登记、未确认结果、逻辑路径选择与公共文件分支内联改名 |
| [路径节点结果](web-path-nodes/browser-result.json) | 两场各 5 组：改名复用正文、节点备注、从路径节点分支、随后写入形成 F1→路径节点→F2、正确下载名称及正文 ID |
| [目录控制结果](web-directory/browser-result.json) | 两场各 5 组：真实目录 ID 默认选择、空目录分支、128 字节原位改名及审计标签 |
| [版本树结果](web-tree/browser-result.json) | 两场各 3 种几何状态：父下子上、跨层绕行、圆角转弯、均匀实线和节点边界 |
| [候选摘要](web-candidate.json)、[构建清单](web-build-manifest.json) | 真实生产 WASM 与全部 146 个构建输入摘要；记录时与项目源码全部一致 |

浏览器加载真实生产 WASM。批量探针对所有网络请求本地响应，使用与服务一致的 CSP；其业务 API 为隔离夹具，不接触实际用户文件。目录及树回归探针使用临时候选静态服务，也将所有业务 API 拦截。截图位于各结果目录。

这些记录验证客户端行为、请求范围和显示几何；核心持久化、实际搬移、业务 API、SMB、恢复及服务更新由各自验收记录证明。Web 子任务没有替换实际预览服务或项目 `web/dist`。

## 重现

```sh
cargo test --manifest-path web/Cargo.toml --locked --offline --workspace
XCS_PLAYWRIGHT_MODULE=/absolute/path/to/playwright/index.mjs \
XCS_BROWSER_DIST=/absolute/path/to/candidate/web/dist \
node tests/evidence/branch-operations/web-browser-probe.mjs
XCS_PLAYWRIGHT_MODULE=/absolute/path/to/playwright/index.mjs \
XCS_BROWSER_DIST=/absolute/path/to/candidate/web/dist \
node tests/evidence/branch-operations/web-path-nodes-probe.mjs
```

可用 `XCS_BRANCH_WEB_EVIDENCE` 指定新输出目录。树和目录探针分别复用 `tests/evidence/version-tree-connectors/browser-probe.mjs` 与 `tests/evidence/directory-compositions/browser-probe.mjs`；通过 `XCS_PREVIEW_URL` 指向隔离静态服务，`XCS_PRODUCT_ROOT` 指向相应候选根，再用 `XCS_TREE_EVIDENCE`／`XCS_DIRECTORY_EVIDENCE` 指定输出目录。Playwright 是测试工具，不是产品构建或运行依赖。
