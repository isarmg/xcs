# XCS Unicode 表生成器

Rust 工具从 `crates/xcs-naming/data` 读取三个固定 Unicode 16.0.0 输入，先核对已登记 SHA-256，再生成字母/标记/数字类别、默认忽略字符和完整 C/F case folding 表。它不访问网络，不使用宿主 Unicode 属性或区域设置。

```sh
cargo run --locked --offline --manifest-path tools/unicode-build/Cargo.toml -- --check
cargo run --locked --offline --manifest-path tools/unicode-build/Cargo.toml -- --write
```

默认只检查逐字节重现，`--write` 显式替换生成文件。更新 Unicode 版本属于命名策略迁移，不能只改变哈希后静默覆盖。输入来源、许可及摘要见 [命名 README](../../crates/xcs-naming/README.md)。
