# xcs Rust Web 构建工具

本工具只构建本项目的 Rust CSR 应用。`build` 执行锁定的离线 Cargo、严格匹配的 wasm-bindgen 0.2.129，再生成静态发行目录；`check` 检查 wasm32，`test` 运行原生 Rust 测试；`icons` 从 Rust 几何代码生成应用 PNG/ICO/ICNS。

```sh
cargo run --locked --offline --manifest-path tools/web-build/Cargo.toml -- build --product-root .
cargo run --locked --offline --manifest-path tools/web-build/Cargo.toml -- icons --product-root .
```

`XCS_WEB_BINDGEN` 指定 CLI，`XCS_WEB_TARGET_DIR` 指定浏览器编译缓存。默认缓存为 `target/web`，输出为 `web/dist`。生产构建不需要 Node、Python、npm 或其他项目目录。

发布有独占锁、输入前后摘要比较、完整资源 SHA-256 清单和确定性 WASM gzip；目录根和子项都拒绝符号链接，仅接收普通文件与目录。FIFO、socket 和设备等特殊文件会在读取前失败，避免输入清单扫描阻塞。普通发布失败会恢复旧目录。构建进程退出后才可处理遗留 `web/.build-lock`；发现 `.dist-backup` 时先检查中断状态。两个目录重命名不构成断电事务。

JavaScript 仅由 wasm-bindgen 生成绑定，并由本工具生成通用模块启动文件；文件管理、认证、摘要和业务交互均来自 Rust。
