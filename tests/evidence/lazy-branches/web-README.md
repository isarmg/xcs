# Web 分支登记与首次写入验收

本次 Web 调整使用显式 `materialized`、`content_state` 及可空 `version_id` 区分占位与实际内容。旧服务器未提供新标志时，只凭非空真实版本 ID 判定可读内容；`source_version_id` 不作为当前内容。布局、间距、圆角、登录页面和公共组件未作改动。

## 验证结果

- [单元及组件测试](web-tests.log)：82 项通过，包含 8 项新回归测试。覆盖来源不冒充当前内容、待写入分支身份、空版本读操作守卫、队列路径及分支固定、不同本地名称写原受管文件、占位记录不入版本树和历史登记文案兼容。
- [生产构建](web-build.log)：Rust/WASM release 与指定的 wasm-bindgen 0.2.129 成功构建。[候选收据](web-candidate.json)记录 WASM、CSS、HTML 及构建清单散列。
- [真实 Chromium](web-result.json)：中文／浅色／1440px，英文／深色／390px，两场各 3 组业务检查通过。实际加载候选 WASM，其响应 SHA-256 与本地候选一致。

浏览器依次执行：在真实历史节点登记分支、切换待写入分支、检查预览／下载／从当前内容创建不可用、检查历史树仍只有来源真实节点、将不同本地名称的零字节文件写入当前分支、检查目标原路径与捕获的分支 ID、检查首次写入新增且只新增一个真实节点、检查父版本向新版本的上向连接，以及校验零字节正文后正常预览。真实历史节点的分支登记入口在待写入分支选中时仍可用。

全部 `/v1/` 请求由 [测试程序](web-probe.mjs)拦截，包括认证恢复、分支登记、选择、写入、封存、提交和内容读取。测试使用临时本机静态服务器，与运行中的业务服务无关，不读取真实账户凭据，也不修改用户文件。这验证了实际 WASM 交互与协议投影；实际占位文件、发布事务和重启恢复另由核心及服务端测试验证。

## 复现

```sh
cargo test --locked --offline --manifest-path web/Cargo.toml --workspace
XCS_WEB_BINDGEN=/absolute/path/wasm-bindgen cargo run --locked --offline --manifest-path tools/web-build/Cargo.toml -- build --product-root .
python3 -m http.server 17401 --bind 127.0.0.1 --directory web/dist
```

在另一个终端执行：

```sh
XCS_PLAYWRIGHT_MODULE=/absolute/path/playwright/index.mjs XCS_PREVIEW_URL=http://127.0.0.1:17401 node tests/evidence/lazy-branches/web-probe.mjs
```

验证结束后关闭临时静态服务。此验收不涉及浏览器过度缩放补偿，不构成六平台实机、物理盘故障或发行验证。
