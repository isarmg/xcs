# Web 父版本继承与首次写入验收

本轮取代此前“未写入时无可读内容”的界面语义；上一轮 [lazy-branches](../lazy-branches/web-README.md) 证据保留为历史记录，不再代表当前行为。本轮没有修改 CSS、间距、圆角、登录页面或公共组件。

## 当前行为

`version_id` 指向当前可见的真实内容版本，可用于预览、下载和从当前内容登记分支。`materialized`、`inherited` 和 `own_version_id` 区分本分支自己的版本与继承的父版本；界面不会根据 `source_version_id` 自行拼出一个未由 API 提供的可见版本。

未写入的分支显示父版本，详情说明正在继承，版本树仅在原真实父节点标“继承的父版本”，不会制造占位内容节点或将父节点转归新分支。首次写入后显示本分支自己的真实版本。目录内尚未写入的文件与已经写入的文件各自独立，可在同一列表同时显示父内容及自身内容。

## 验证

- [Rust 测试](../../../../test/evidence/inherited-branches/web-tests.log)：84 项通过，包括公共运行时 26 项、组件 3 项、响应状态 2 项、应用 53 项。新增回归检查覆盖可读继承与正文所有权分离、真实树节点身份不变，以及同一目录继承／自身文件分别捕获写入分支。
- [生产构建](../../../../test/evidence/inherited-branches/web-build.log)：指定 wasm-bindgen 0.2.129 的 Rust/WASM release 构建成功。[候选收据](../../../../test/evidence/inherited-branches/web-candidate.json)记录 WASM、HTML、CSS 与构建清单的 SHA-256。
- [实际 Chromium 交互](../../../../test/evidence/inherited-branches/web-result.json)：中文浅色 1440px、英文暗色 390px，两场各 5 组检查通过。实际加载的 WASM 字节与候选匹配，无页面异常、无未预计的 API 请求。

每场顺序验证：

1. 登记并选中未写入分支，只出现原真实父版本节点；预览、下载、从当前可见内容再登记均可用。
2. 父正文 `abc` 经过 BLAKE3 校验后预览和下载，下载名称保持受管文件名；再登记分支仍不增加正文版本。
3. 同一目录列表同时显示一个继承父内容的 3 字节文件和一个本分支自身的零字节文件；两者均可读取，继承标记不串到已写入文件。
4. 使用不同本地名称的零字节文件写入当前受管文件，请求准确使用原路径及捕获的分支 ID，首次写入新增且只新增一个真实内容节点，父节点向新节点的连线向上。
5. 写入后不再标继承，零字节正文通过完整性校验并可预览。

截图为 [中文待写入详情](../../../../test/evidence/inherited-branches/pending-zh-light-1440.png)、[英文混合目录](../../../../test/evidence/inherited-branches/mixed-directory-en-dark-390.png)、[英文首次写入后](../../../../test/evidence/inherited-branches/written-en-dark-390.png)。

全部 `/v1/` 请求，包括认证恢复、登记、选择、写入、封存、提交和内容读取，均被 [测试程序](../../../../test/evidence/inherited-branches/web-probe.mjs)拦截。测试只连接临时本机静态服务器，不读取真实账户凭据，不访问运行中业务服务或修改用户文件。实际文件占位、发布和恢复由核心及服务端测试另行验证；本证据不构成六平台实机或发行验收。

## 复现

```sh
cargo test --locked --offline --manifest-path web/Cargo.toml --workspace
ZXS_WEB_BINDGEN=/absolute/path/wasm-bindgen cargo run --locked --offline --manifest-path tools/web-build/Cargo.toml -- build --product-root .
python3 -m http.server 17401 --bind 127.0.0.1 --directory web/dist
```

另一个终端运行：

```sh
ZXS_PLAYWRIGHT_MODULE=/absolute/path/playwright/index.mjs ZXS_PREVIEW_URL=http://127.0.0.1:17401 node test/evidence/inherited-branches/web-probe.mjs
```

结束后关闭临时静态服务器。
