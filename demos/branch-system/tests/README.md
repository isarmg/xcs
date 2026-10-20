# 演示验证

原生测试直接运行相同模型与场景命令，浏览器检查实际生成的 WASM；演示业务逻辑不使用 JavaScript。浏览器验证代码仅用于自动点击和断言。

测试程序与本说明纳入 Git；`evidence/` 和 `history/` 中的结果 JSON、截图及日志属于本地生成附件，由 `.gitignore` 排除。需要这些材料时先构建演示，再运行下面的验证命令；静态参考图在 `../reference/` 中作为文档内容保留。

```sh
cargo test --locked --offline --manifest-path demos/branch-system/Cargo.toml --target-dir target/web-native
ZXS_PLAYWRIGHT_MODULE=/absolute/path/playwright/index.mjs node demos/branch-system/tests/browser.mjs
```

静态参考图可单独验证：

```sh
ZXS_PLAYWRIGHT_MODULE=/absolute/path/playwright/index.mjs node demos/branch-system/tests/reference.mjs
```

该命令重绘 27 张 PNG 并验证 81 次离线切换；更新图片后应重建演示发行包。

独立 Rust 预览服务可补测（先构建 `zxs-demo-serve`）：

```sh
node demos/branch-system/tests/serve.mjs
```

[只读预览服务检查](evidence/server-result.json)核对两个页面、HEAD、WASM MIME、安全策略、路径限制和拒绝写请求。

默认访问 `http://127.0.0.1:7400/demo/`，`ZXS_DEMO_URL` 可改为独立只读演示服务地址，例如 `http://127.0.0.1:7401/`。

[浏览器结果](evidence/browser-result.json)记录过程逐步推进、播放/暂停、接续操作、首写、标签、合并、只读别名、改名、移动、复制、删除、撤销/重做、未选分支冻结、独立备份前缀、三种屏宽及静态参考切换。检查实际 WASM 的 SHA-256、源清单、严格 CSP 与启动标记，禁止访问业务 API 或发出修改请求。[Rust 测试日志](evidence/rust-tests.log)与[构建日志](evidence/build.log)保留在同一目录。这些验证只证明独立演示行为，不能代替生产存储验收。
