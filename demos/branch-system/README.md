# ZXS 分支演示

[过程演示](index.html) · [自由操作](playground.html) · [27 个静态参考图](reference/index.html) · [图片实例索引](examples.md) · [项目规则](../../docs/design/branch-system-logic.md)

演示的内容、Rust 代码、页面、样式、生成器、构建产物和本次验证统一放在本目录。过程页与自由操作页调用同一个内存模型；不调用业务 API，不使用登录凭据，不读写真实文件。刷新后重置。Web 组件、Apple G4 轮廓、行高与字体比例复用项目公共运行环境，来源说明见 [Web README](../../web/README.md)。

过程页有 8 个场景：文件首写、目录按文件首写、子目录和 SMB 默认组合、改名/移动/复制/删除、复用头合并、选择历史节点合并、标签继承、备份前缀。支持上一步、下一步、播放/暂停、重播、跳到任一步，并可把当前场景和步骤带入自由操作页。

自由页先点击列表中的文件、文件夹或根目录，再选择操作；输入均为页面内下划线输入。可登记/选择分支、写入正文（包括真实空内容）、改名、移动、复制、删除引用、改节点标签和备注、合并文件分支、选择 SMB 默认组合、设置演示备份前缀。可撤销、重做、重置。点击文件图节点或节点下拉框选择历史来源；合并明确勾选参与分支、结果节点和归属。

文件树和目录树分别显示，父节点在下、子节点在上。绿色标出本步新增节点，蓝框标出所选节点；图形使用实际父边，等粗正交箭头在转弯处倒圆角。列表同时显示当前所引用的分支、节点及正文，计数可观察“只登记不复制”和元数据复用正文的区别。

此模型用于理解逻辑，并非第二套存储核心：只维护内存中的文件身份、正文、公开文件分支和固定目录组成。备份页面演示 P01/B01/C01 的前缀映射，不复制磁盘内容；目录合并不提供操作。授权、冲突令牌、持久化、保护证明和真实 SMB 以生产实现及其验收材料为准。命名和 128 字节备注/标签校验直接复用项目契约。演示正文限制 4096 字节，节点约 300 个，撤销保留最近 80 步。

## 目录

| 位置 | 职责 |
| --- | --- |
| `src/model.rs` | 无浏览器依赖的确定性模型、原子操作和不变量检查 |
| `src/scenarios.rs` | 8 个分步脚本，与自由页共用命令 |
| `src/graph.rs` | 拓扑布局、向上箭头、安全转义与节点卡片 |
| `src/view.rs` | 过程播放、操作表单、撤销重做、目录组成和树 |
| `src/entry.rs`、`src/bin/build.rs` | 两个 HTML 入口与 Rust 构建工具 |
| `src/bin/serve.rs` | 独立只读 HTTP 演示服务 |
| `styles.css` | 仅演示布局；尺寸和轮廓引用共享样式 |
| `src/bin/reference.rs`、`reference/`、`examples.md` | 原 27 个参考实例、SVG/PNG、静态查看页及图片索引 |
| `tests/` | 真实浏览器操作验证、截图和结果 |
| `dist/` | 独立 HTTP 静态发行包，包含 WASM、生成绑定、字体、样式和页面 |

## 构建与打开

在项目根目录执行，依赖已缓存时可完全离线构建：

```sh
cargo test --locked --offline --manifest-path demos/branch-system/Cargo.toml --lib --target-dir target/web-native
cargo run --locked --offline --manifest-path demos/branch-system/Cargo.toml --bin zxs-demo-build --target-dir target/web-native
```

使用 Rust 工具链与 wasm-bindgen 0.2.129；业务、交互和图形全部为 Rust，JavaScript 仅有公共生成加载器和 wasm-bindgen 生成绑定。构建前后核对输入 SHA-256，`dist/build-manifest.json` 记录输入和发行资源。浏览器须通过 HTTP 打开 WASM 页面；`reference/index.html` 是可直接打开的无脚本静态页面。`ZXS_WEB_BINDGEN` 可指定匹配版本的 CLI 路径。

独立启动只读演示服务（默认 `127.0.0.1:7401`）：

```sh
cargo run --locked --offline --manifest-path demos/branch-system/Cargo.toml --bin zxs-demo-serve --target-dir target/web-native
```

将 `dist/` 作为静态站点根目录后，`/` 为过程演示，`/playground.html` 为自由操作。已有本项目 Web 发行目录时，可以发布到其 `/demo/` 子目录：

```sh
cargo run --locked --offline --manifest-path demos/branch-system/Cargo.toml --bin zxs-demo-build --target-dir target/web-native -- --publish-web
```

该命令只替换 `web/dist/demo/`，不重启存储服务或修改业务数据。重新构建主 Web 会生成全新发行目录，应随后再次发布演示。文档图片可单独重建：

```sh
cargo run --locked --offline --manifest-path demos/branch-system/Cargo.toml --bin zxs-demo-reference --target-dir target/web-native
```

浏览器验证命令及结果见 [测试说明](tests/README.md)。此前演示截图、静态图检查代码与结果一并归档在 `tests/history/`；当前检查位于 `tests/browser.mjs` 和 `tests/reference.mjs`。
