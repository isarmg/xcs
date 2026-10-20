# ZXS Web 绑定生成器

从 wasm-bindgen-cli 0.2.129 的正式绑定命令提取，只接受 ZXS 的 `--target web --no-typescript --out-name NAME --out-dir DIR INPUT` 契约，继续使用同版 `wasm-bindgen-cli-support` 解析和生成绑定。原始发布包及逐文件映射见 [PROVENANCE.json](PROVENANCE.json)，原始 MIT / Apache 许可随源码保留。

删除 test-runner、wasm2es6js、HTTP 测试服务器、网络下载及其依赖；该正式构建工具没有 rouille、multipart、ureq 或 TLS 依赖。输入验证、WASM schema 检查及生成错误仍由原有支持库处理。Web 绑定参数与原 CLI 对应调用保持一致，包括默认 Wasm URL 和 reference-types 行为。

```sh
cargo test --locked --offline --manifest-path tools/zxs-bindgen/Cargo.toml
bash .github/scripts/rust-build-tool.sh wasm-bindgen
```

这是 ZXS 自维护的编译辅助代码；版本字符串保持 `wasm-bindgen 0.2.129`，以匹配 Web crate 的 schema 版本。它不是上游三个通用命令的完整替代品。
