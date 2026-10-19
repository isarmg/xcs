# 文件合并、节点标签与备份前缀 Web 验证

2026-10-19。界面代码均为 Rust/WASM；测试 JavaScript 仅加载生产候选与构造隔离 API，不是生产功能实现。

## 交付内容

- 文件合并在页面内选择至少两个可写分支、冻结各自当前节点、任选本文件节点作为正文，并明确指定保留的参与分支。消耗名称保留只读别名，完整来源独立展示。目录合并只显示“待确认，暂不提供操作”，没有操作入口。
- 每个节点自己的标签显示在原备注行；勾选节点后可批量增删。新标签节点继承正文、原节点标签不变；历史来源和预期分支头分别提交，无变化不请求写入。使用共享契约校验16个标签与每个128 UTF-8字节的上限。
- 文件／文件夹／当前目录均有页面内备份设置。按服务端 `can_manage` 控制编辑，每个目标盘独立前缀，份数受全部历史主盘链的最短容量限制，按每链前n项备份槽的并集逐盘配置；提交包含读取时的范围身份和自身代数。前缀使用768字节的路径预算。备份目录显示用户前缀；主路径只作为独立读取上下文，无主alias时不拼接假路径，所选副本始终固定其实际存储的节点和正文。
- 三类新操作共享发布状态：4xx保留审阅输入，未知结果只轮询原操作ID，成功回执只清除相应草稿。路径、标签、合并节点的备注统一使用节点接口；编辑不产生正文版本。

## 结果

| 验证 | 结果 |
| --- | --- |
| [Rust workspace测试](rust-tests.log) | 111项通过：应用80、共享运行时26、组件3、响应式状态2；无警告 |
| [新功能浏览器](browser-result.json) | 中文1440px浅色、英文390px深色各11组交互，加载的真实WASM哈希与候选一致，无页面异常、未预期请求或输入弹窗 |
| [备份浏览回归](backup-browse/browser-result.json) | 两场各3组：用户逻辑前缀、派生目录只读、主alias精确上下文、无alias历史读取、固定旧备份正文的当前标记及校验预览下载 |
| [批量文件操作回归](regression-batch/browser-result.json) | 两场各9组；移动/复制/删除/改名、冲突/未知结果、公共文件分支内联改名 |
| [路径节点回归](regression-path-nodes/browser-result.json) | 两场各5组；改名节点、备注、来源节点分支、随后真实写入及下载 |
| [目录控制回归](regression-directory/browser-result.json) | 两场各5组；目录组合、SMB默认选择、空目录、原位改名及审计 |
| [复杂历史树回归](regression-tree/browser-result.json) | 两场各3种几何状态；父下子上、跨层绕行、文本编辑和改变宽度后的等粗正交圆角线 |
| [候选回执](candidate.json)、[资源与源码清单](build-manifest.json) | 152个输入与当前源码逐项一致，资源摘要逐项一致；既有公共几何、登录和选择器源码与上次候选一致 |

候选位置为 `/tmp/xcs-web-merge-tags-candidate/web/dist`，WASM SHA-256为 `2491cb2a7a0abf2413b8c19c4f7a288393ffdccfbfa2080a302e8d6e6f4433d2`。新功能实际请求摘录保存在 [wire/](wire/)。截图在本目录及各回归子目录。

浏览器API全部由本地夹具响应；测试证明页面交互、发布恢复、请求契约和真实WASM显示几何，**不代表实际服务端合并、物理副本或SMB验收**。服务端证据由对应核心/API/存储测试提供。Web子任务没有替换项目 `web/dist` 或重启真实预览服务。

复杂树回归在新证据目录复制了旧探针，将备注编辑输入限定为 `input[type="text"]`，避免新增节点复选框与通配输入选择器冲突。旧验收目录及其产物保持原样。

## 重现

```sh
cargo test --locked --offline --manifest-path web/Cargo.toml --workspace
node tests/evidence/merge-tags-backup/web/browser-probe.mjs
node tests/evidence/merge-tags-backup/web/backup-browse-probe.mjs
```

浏览器探针默认读取上述隔离候选。可用 `XCS_BROWSER_DIST`、`XCS_WEB_EVIDENCE` 和 `XCS_PLAYWRIGHT_MODULE` 指定其他位置。复杂树探针为 `regression-tree-probe.mjs`，使用回环静态服务，需设置 `XCS_PRODUCT_ROOT`、`XCS_PREVIEW_URL`、`XCS_TREE_EVIDENCE` 和 `XCS_PLAYWRIGHT_MODULE`；详见其顶部环境变量。
