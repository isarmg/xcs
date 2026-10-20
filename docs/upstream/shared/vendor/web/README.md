# zxs CSR 内部引擎

本目录只保存项目内维护的浏览器框架代码；公开入口在 `web/runtime` 的 `zxs-web`，应用在 `web/src` 的 `zxs-web-app`。

| 目录 | 职责 |
| --- | --- |
| `zxs-web-core` | 响应式所有权、DOM 构建、组件、控制流与挂载；项目已移除服务端渲染、服务器函数、hydration 和服务器表单依赖入口 |
| `zxs-web-macro` | `view!`、组件与属性过程宏，生成统一 `zxs_web` 路径 |

本地包、Rust 路径、DOM 标记和调试环境变量采用 zxs 命名。`reactive_graph`、`tachys`、`leptos_dom`、`leptos_hot_reload` 及其 Leptos 配套依赖已纳入 [dependencies](dependencies/README.md) 自行维护，保留真实包名和锁定版本。所有框架内部依赖直接指向仓库内路径，应用与 demo 使用同一份源码；不依赖消费者 workspace 的 patch 配置。

第三方基础来自 [Leptos 0.8.19](https://github.com/leptos-rs/leptos)。原始 MIT 版权和许可全文分别集中保留在 [core README](zxs-web-core/README.md) 与 [macro README](zxs-web-macro/README.md)。字体来源和完整许可通知保留在 [字体 README](../../web/public/fonts/README.md)。

通过项目入口验证，不依赖原框架上游 workspace：

```sh
cargo check --locked --offline --manifest-path web/Cargo.toml --target wasm32-unknown-unknown
cargo test --locked --offline --manifest-path web/Cargo.toml --workspace
```

内核与宏的定向回归使用 `cargo test --locked --offline --manifest-path web/Cargo.toml -p zxs-web-core -p zxs-web-macro --lib`。覆盖属性基础值/增量值的稳定排序、显式组件属性分界及真正可变的 children 闭包。`AnimatedShow` 的每次条件通知都会替换旧隐藏定时器；解除挂载仍由 scoped cleanup 清理当前定时器，重复的 false 通知不会留下过期回调。
