# 历史验证材料与当前回归的边界

本目录保留历次检查脚本、说明和结果的来源，**不是当前生产验收套件**。
[完整脚本清单](../../test/evidence/probes.json) 逐一登记 26 个原始 JavaScript 模块，新增或移除脚本必须同步分类。
历史结论只适用于原报告中的源码、构建、输入和运行环境；静态检查通过不表示浏览器验收通过。

当前维护的实际生产 WASM / Chrome 验收见 [browser-smoke](../tools/browser-smoke.md)。
CI 的质量门禁另外执行 `node .github/scripts/check-evidence-probes.mjs`，只做 Node 语法、清单完整性、
路径配置及安全配置单测，**不安装 Playwright、不运行历史探针、不访问现有服务**。

## 分类

- `historical-service`：旧真实服务或 API 拦截探针，静态资源仍需独立 HTTP 服务。包括分支、目录、树、账号和维护性旧候选。
- `historical-intercepted-wasm`：本地响应拦截下的历史 WASM 行为比较，不证明真实后端存储或事务语义。
- `historical-source-comparison`：独立 HTML/CSS 或另行编译的组件夹具。旧像素值、CSS 哈希和 DOM 不自动成为当前要求。
- `historical-reference`：四份独立 G4 数学参考。保持原始字节，清单保存 SHA-256；本次不改 G4 实现、参考公式或像素期望。

AnimatedShow 脚本需要另行提供匹配其 DOM/按钮约定的编译后夹具；仓库没有把当时的临时构建目录当作可复现输入。
其他历史探针也可能依赖当时的 seed、构建收据或旧 UI/API 契约。缺少匹配输入时只能阅读/静态检查，
不得将未运行、跳过或历史成功记为本轮通过。当前组件回归以 Web workspace 的维护测试为准。

## 显式配置与安全重跑

任何历史探针运行都必须指定：

- `ZXS_PLAYWRIGHT_MODULE`：已经准备好的 Playwright 模块的绝对文件路径、file URL，或可从辅助模块解析的包名；不再引用个人目录或另一个私有项目的 node_modules。
- `ZXS_EVIDENCE_OUTPUT_DIR`：归档目录之外的输出父目录。每次自动建立唯一子目录并打印路径，不覆盖历史证据或之前的重跑结果。
- `ZXS_PRODUCT_ROOT`：可选，匹配的源码/构建仓库根目录；默认由公共辅助模块自身位置推导，不受启动目录或脚本嵌套层数影响。

使用服务的探针还必须设置 `ZXS_EVIDENCE_FIXTURE_URL` 和 `ZXS_EVIDENCE_DISPOSABLE=1`。
URL 仅允许显式端口的 HTTP 回环地址 `127.0.0.1` 或 `[::1]`，不允许凭据、路径、查询或片段。
该确认意味着调用者已独立启动一次性夹具；**不要指向正在使用的业务预览，即使它位于回环地址**。
探针可能登录、修改业务数据及退出会话。代码不会替调用者创建或验证服务的数据隔离。
旧 `ZXS_PREVIEW_URL`、`ZXS_TREE_BASE` 等隐式入口和分散的输出变量不再使用。

额外输入按脚本需要显式提供：

- 账号/维护性服务比较：`ZXS_BROWSER_SEED`、`ZXS_BROWSER_PASSWORD_FILE`。
- version-tree：`ZXS_TREE_SEED`，密码文件由 `ZXS_TREE_PASSWORD_FILE` 或该显式 seed 的 `password_file` 指定。
- merge-tags-backup 本地 WASM：`ZXS_BROWSER_DIST`；served-browser-check：`ZXS_EVIDENCE_RECEIPT`。
- AnimatedShow：`ZXS_ANIMATED_FIXTURE`。

这些变量不得存储密码正文，只指定受保护的输入文件。不要提交凭据或运行生成物。
辅助配置的测试只使用临时目录和无网络配置，不导入或执行历史脚本。

本次只修复脚本可移植性、误访问/覆盖风险和维护边界；没有伪造新的历史运行、重写旧报告结论，
也没有为了通过测试修改生产 G4 或旧像素期望。需要扩展当前回归时，应在维护套件里明确新增需求和夹具，
而不是把整套历史脚本假称为当前验收。

## 维护工具与历史归档分离

历史清单仍精确覆盖 `test/evidence` 中除 `_support` 外的所有 `.mjs`，不用于登记当前维护的工具。
可选下载 helper 位于 `test/tools/browser-smoke/downloads/browser-probe.mjs`，仅供准备好的 Playwright runner 显式导入；它不自行启动浏览器，登记或语法检查不表示已执行浏览器验收。实际 Chrome CI 继续使用已有 browser-smoke runner 的下载夹具。
可选 SIMD 正确性工具位于 `test/tools/web-blake3-simd`，需通过 `run-check.sh` 提供 SIMD WASM、portable WASM 和 native reference 输入，不是无参数 CI 入口，也不证明浏览器性能。
`test/ci/test_manual_web_tools.py` 在静态 CI 中检查这两个工具的语法和仓库相对路径，但不会执行浏览器或编译 SIMD 夹具。
