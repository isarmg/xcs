# zxs Rust Web 构建工具

本工具只构建本项目的 Rust CSR 界面。`build` 执行锁定的离线 Cargo、严格匹配 0.2.129 schema 的 ZXS Web 绑定生成器，再生成静态发行目录；`check` 检查 wasm32，`test` 运行宿主 Rust 测试。

```sh
ZXS_WEB_BINDGEN="$(bash .github/scripts/rust-build-tool.sh wasm-bindgen)"
export ZXS_WEB_BINDGEN
cargo run --locked --offline -p web-build -- build --product-root .
```

`ZXS_WEB_BINDGEN` 指定 CLI，`ZXS_WEB_TARGET_DIR` 指定浏览器编译缓存。默认缓存为 `target/web`，输出为 `build/web`。CLI 来自自维护的 `tools/web-bindings`，仅保留 Web 绑定命令，删除上游测试服务器、其他命令和网络/TLS 依赖；支持库从 `src/foundation` 离线编译。Python 3 在构建前后验证 `compliance/rust-sources.json` 对应的实际文件，清单与统一来源文档也纳入 Web 构建收据。生产构建不需要 Node、npm 或其他项目目录。

正式 Web 使用 `tools/web-build/src/wasm_release.rs` 中的发布配置：体积优化 `s`、fat LTO、符号剥离。即使调用环境带有原生 release profile 参数，也不会将大型函数名段带入浏览器发行物。

发布有独占锁、输入前后摘要比较、完整资源 SHA-256 清单和确定性 WASM gzip；目录根和子项都拒绝符号链接，仅接收普通文件与目录。FIFO、socket 和设备等特殊文件会在读取前失败，避免输入清单扫描阻塞。普通发布失败会恢复旧目录。构建进程退出后才可处理遗留 `build/.web-build-lock`；发现 `build/.web-backup` 时先检查中断状态。两个目录重命名不构成断电事务。

JavaScript 仅由 wasm-bindgen 生成绑定，并由本工具生成通用模块启动文件；文件管理、认证、摘要和业务交互均来自 Rust。

输入范围由 [`build-inputs.json`](../../tools/web-build/build-inputs.json) 统一列出，构建器和发布验证器使用同一份清单，清单自身也进入输入摘要。构建器拒绝与其编译时版本不同的清单；修改输入定义后必须重建工具和 Web 产物。

`release-package.sh` 打包前重新校验 Web manifest 的身份、完整输出文件集合和 SHA-256，并按源码侧清单重新计算全部输入，不能通过删掉 manifest 中的输入项来绕过。损坏、缺失、额外文件、过期源码、符号链接和特殊文件都会失败；中央法律资料、文档和原生二进制也只接受普通文件。校验后的字节写入独立暂存快照，再从快照生成发布包，避免后续工作目录变化混入归档。此校验不能代替原生编译、浏览器运行或来源审计。
