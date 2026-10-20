# Web 子系统维护性审查与落地记录

范围：`web/src/`、`web/runtime/`、`web/public/styles/`、`web/styles.css`。没有构建或重启本地服务，没有修改真实密码或业务文件。根 agent 负责整体构建和服务更新；候选 READY 后本 agent 执行受写接口拦截保护的最终真实浏览器验收。

## 已确认问题与修正

1. CSS 分散声明造成改一处漏多处。原入口仍为旧侧栏/磁盘卡/时间线/ESM 页面保留全套业务别名，并对 main 下所有 input/button/table 施加高覆盖范围样式，现代页面再逐页修正。现入口只导入、字体和产品排版；几何集中到 `components/geometry.css`，主题只配色与状态，登录原段独立 `auth.css`。删除已无 Rust/HTML 运行引用的旧页面及 `date-range.css`。
2. 显式共享角色：`zxs-stack`（块栈）、`zxs-flow`（同行与换行）、`zxs-row`（字号和首字内缩）、`zxs-frame`（灰框/G4/边框内缩）、紧凑及下划线输入。所有现有业务页面组合这些角色；业务 CSS 只保留列布局、图表、截断、表格语义和树专属布置。账户不再以页面 selector 覆盖普通输入，改为显式 compact。Select 弹层及选项也使用 frame/row，避免遗漏内缩变量。
3. 默认值保持：compact 36px / 字体 .4=14.4px / 块与折行 gap 18px / G4 E18；普通输入和头部图标 44px / 17.6px / E22；登录既定物理尺寸和字号独立保留；四列 3:2 状态卡、手机一列、最新叶在上的真实版本树均保留。
4. `files/actions.rs` 743 行承担太多职责：拆为 `actions/mod.rs` 门面、`browse.rs` 目录/历史代次隔离读取、`mutation.rs` 命名空间/目录分支/文件分支/备注事务、`operation.rs` 发布回执与未知状态轮询、`transfer.rs` 顺序上传和内容读取/下载。所有既有 crate 内调用接口保留，模块依赖使用显式 imports。提取停止未上传队列项的逻辑，未知提交和已发布回执不被改成失败。
5. Web `/v1/` 路径与备注验证和其他端重复：改为 `zxs_contract::validate_api_path`、`MAX_NOTE_BYTES`、`validate_note`；保留 Web 双语错误和新名称 UI 128 字节限制，不收紧已有长名称/旧备注展示。
6. 下载结果和改密响应缺少会话代次守卫：复用 `current_session`。清理重复的会话结束代码，退出确认和改密确认共享 `end_session`，已开始退出或新登录后的旧结果不能更新新会话。
7. 更底层的旧 401 仍会无条件清除新 TOKEN/CSRF：将两个独立 thread_local 合并为不实现 Debug 的 `Credentials`，每次替换/撤销推进 generation；`Fetched` 记录发出时的代次，401 只撤销相同代次。restore 也只为相同代次设置 CSRF。
8. AppShell 永远创建空的 notice/error 容器，占用块栈间距：改为内容存在时才渲染，并给有内容的提示和账户说明组合共享 row/stack。
9. `theme::cleanup` 仅供 WASM 生命周期调用，增加相同目标 cfg，消除原本 native 测试的死代码警告。
10. 首轮目视检查发现删除旧 generic cell CSS 后，table 的 start 对齐被 th 的 UA 居中默认值覆盖。root 在 file-manager cell 层补 `text-align:inherit`；本 agent 给真实浏览器 fileRows 同时增加 computed start/left 与单字 Range 起点守卫，按名称列实际图标宽和 gap 修正期望值。源码夹具四负／四正对照均按预期，最终候选重新构建后完整四场再跑 60/60，不复用修正前结果。

## 审查覆盖与逻辑归属

| 文件/模块 | 检查重点与结果 |
| --- | --- |
| `web/Cargo.toml`、`Cargo.lock`、`runtime/Cargo.toml` | CSR/wasm 依赖与独立 workspace；未改依赖身份或增加业务 JS |
| `src/lib.rs`、`ui.rs` | 挂载、会话隔离、导航、周期轮询、鉴权失败与改密；修正旧响应和空壳间距 |
| `src/api.rs`、`api/response.rs` | 同源/原生桥、API 路径、TOKEN/CSRF、超时、有界读取、严格响应解码、上传封存与摘要、校验下载；路径/凭据代次收敛，有限小数、重复 JSON 字段和真实 HTTP 状态语义保留 |
| `src/format.rs`、`icons.rs` | 大整数容量格式、UTC 时间、二进制单位、文件/目录图标；已有格式测试通过，未改动协议整数精度 |
| `src/files/mod.rs`、`model.rs` | 选中项、异步代次、预览取消、发布/未知/降级状态区分；保留原状态模型 |
| `src/files/actions/{mod,browse,mutation,operation,transfer}.rs` | 按读取、事务、回执和传输梳理，保留未确认保存门禁、原操作轮询及当前目录保护 |
| `src/files/{view,detail,inline,branch}.rs` | 磁盘独立目录、文件/目录差异、原位输入、128 UTF-8 字节/IME/Enter/blur、元数据更新不重建编辑器；接共享布局/验证 |
| `src/files/version_tree.rs` | 真父子边、反向拓扑、所有最新叶在上、向上箭头、跨层避让、编辑高度重测、树内部横滚；本轮只组合布局角色，算法未重写 |
| `src/history.rs` | 提交事件读取和磁盘筛选；组合 row/flow/frame，旧状态和错误不串入新会话 |
| `src/monitor.rs`、`monitor/chart.rs` | 缺失采样不当零、真实功耗/不可用、范围代次、四列/手机、短图表保持字号与真实尺度；只接共享布局，数值语义保留 |
| `runtime/src/{lib,startup,theme,i18n}.rs` | 字体先解码、启动单终态、pagehide/owner 生命周期、系统主题与手动偏好、语言路径；cleanup 目标收紧，其余行为保留 |
| `runtime/src/components/{mod,shell,header,login,account,select,dialog,state,notification}.rs` | 公共 Signal/Callback 接口、凭据不 Debug、表单 readonly/autocomplete/清密、popover/键盘/焦点、原生模态恢复、通知有界；显式组合几何，不改公有组件参数或鉴权职责 |
| `runtime/src/contours/{mod,profile,geometry,paint,controller}.rs` | profile/误差边界、缓存、CSS 输入/自绘边框、观察器/事件/退出恢复与重入保护；数值与生命周期实现未重写，既有纯测试及最终真实 G4 验收继续约束 |
| `public/styles/{tokens/*,startup.css}` | reset/无障碍/主题/启动失败相对路径；未增加缩放补偿逻辑 |
| `public/styles/components/*`、`styles.css` | 原业务别名引用核对、几何依赖、特异性、通用 token 与账户/login/Select 例外；删除逐页面尺寸兜底 |
| `web/README.md`、`public/styles/README.md`、`runtime/README.md`、`runtime/src/components/README.md`、字体 README | 对照模块职责、布局约定、构建路径与来源边界；更新前两项的新结构和使用方式 |

## 验证证据

- `cargo test --locked --offline --manifest-path web/Cargo.toml --workspace`：74 项全过，26 runtime + 43 app + 5 integration，无 warning。新增真状态测试覆盖未知保存仅停止未上传项、退出/重登后的 completion 隔离、旧 401 不能撤销新 TOKEN/CSRF；API 路径用例新增控制字符。
- `cargo fmt --manifest-path web/Cargo.toml -p zxs-web -p zxs-web-app -- --check`：通过。注意早先使用一次 `--all` 会递归 path dependencies，已告知 root；后续均限定包，没有回滚其他 agent 编辑。
- 当时的临时测试目录或日志 → 当时的临时测试目录或日志：中英文 × 1440/800/390/320 共 8 组源码 CSS 通过；详情行/灰框/分组、折行历史、状态卡 3:2/图表余高/底部采样时间，无 document 溢出。
- 当时的临时测试目录或日志 → 当时的临时测试目录或日志：同 8 组；36px/14.4px/E18、9.8px 输入内缩、504px 上限、桌面标签同行/手机分组、18px 间距、readonly/autocomplete/pending/长错误，无溢出。
- 当时的临时测试目录或日志 → 当时的临时测试目录或日志：4 个正常宽度动态把共享行高从 36→40、普通输入从 44→50，验证文件/详情/树/历史/状态/账户/普通输入/header/下拉选项的字号、G4、内缩、灰框和 gap 一起计算变化；登录整组 bounds 和字号完全相同。此检查专门证明不必逐页修改尺寸。
- 最终真实 WASM 浏览器：`tests/evidence/maintainability/browser-probe.mjs`，中文浅色 1440px、中文深色 390px、英文浅色 800px、英文深色 320px，四场 60/60 组通过。表头修正后的最终候选完整重跑一轮，`result.json` 和 `summary.json` 为这次四场统一结果；实际共享 role 完整性、默认几何、40/50px 动态公共令牌传播、登录保持原样、真实树和模拟多父树向上箭头、原位输入、历史空槽、状态 3:2、账户 dummy 改密及旧会话延迟 401 保护均通过。默认和 token40 的磁盘／文件表共 16 个表格状态、320 单元格（64 th）均为 start；单字 Range 起点与 cell 内缩的最大误差为 0.015625px。
- 四场均逐响应比对动态 16 CSS + 1 WASM 基线资源，MIME/候选 SHA-256/真实 Rust G4 数学和 CSP 通过；无 pageerror 或意外 console error。路由阶段追加的测试 CSS 有独立标记，不作为基线字节一致性。
- 每场两次真实登录及退出，共 8 次登录、8 次 HTTP 200 退出，所有会话已撤销。4 次 dummy 改密均浏览器拦截返回模拟错误，其他业务写零放行；密码、文件和配置没有修改，记录不包含凭据或输入内容。
- 独立 AnimatedShow 真实 Rust/WASM fixture 两序列通过，结果 `animated-show-result.json`；隐藏→重复隐藏→显示后不会被旧 timer 隐藏，隐藏→重复隐藏→卸载后无异步 panic。此组件当前未被业务页使用，不算业务页覆盖。
- 目视检查桌面账户和树、手机账户和树、手机及 800px 状态页，无重叠、全黑图表或页面溢出；宽树仅在自身容器横向浏览。修正后的最终1440px文件表截图已复查，表头与数据的列内起点恢复正常。

## 保留边界与风险

- 本轮未执行浏览器过大缩放或异常字号的适配，也没有增加这类代码。
- 登录 CSS 原段迁移，业务几何严格只作用于 `.zxs-app-shell`；最终浏览器已验证真实登录卡和变更共享令牌后的登录尺寸隔离。
- 核心版本树数值/连线和 runtime G4 数学没有改算法，最终真实路径与裁切已验收；未声称六平台原生 App 实机或物理盘故障验收完成。
- 无 Git 操作、无凭据输出。真实服务只写登录与退出；所有业务写被拦截，测试改密和 HTTP 401 为浏览器本地模拟。
