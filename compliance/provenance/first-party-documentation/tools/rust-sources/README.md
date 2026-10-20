# Rust 源码自主维护

正式入口仅为 Web 与 SMB，运维 CLI 属于内部工具。范围定义在 [workspaces.json](workspaces.json)，统一来源清单为 [THIRD_PARTY_RUST_SOURCES.md](../../docs/THIRD_PARTY_RUST_SOURCES.md)，精确内容回执为 [rust-sources.json](../../vendor/rust-sources.json)。

```sh
python3 tools/rust-sources/manage.py verify --metadata
python3 -m unittest discover -s tools/rust-sources -p 'test_*.py'
```

`verify` 拒绝源码、manifest、锁文件、features/编译角色和文档漂移；不更新已登记哈希。`--metadata` 使用空 Cargo home，核对全部登记目标的本地源码路径与 normal/build 闭包，不代表跨平台实际构建或设备验收。

有意修改源码后，保留原始 archive 身份与版权，更新对应 fork 补丁/PROVENANCE，完成业务回归，再显式运行 `manage.py refresh` 并审查来源文档与 JSON 的差异。registry 保留区 `vendor/rust` 不接受未登记修改；需要定制时建立独立 fork/内部组件并添加来源映射。构建辅助代码与运行时职责分别记录。

`import_sources.py --output /tmp/zxs-restored-rust` 从本机 registry 归档缓存重建登记的原始 registry 源码，拒绝非空目标和异常归档。仅显式 `--fetch` 才允许按固定 URL 获取归档，下载后仍须匹配原始 SHA-256；它不覆盖本地 fork 或应用源码。详见 `import_sources.py --help`。Git 来源依赖当前为零；新增时须补齐精确来源替换与回执，不得依赖本机 checkout。

`collect_notices.py` 是人工维护辅助器，按发布记录的 Git commit 查找缺失许可文件；日常构建不运行、不联网。已有补充文件与其来源也受统一哈希检查。上游没有提供的材料应明确记录待复核，不能生成虚构版权通知。

`audit.py` 供安全门禁使用，扫描真实消费者锁文件及本地 fork 的原始身份，不忽略公告。源码可重现、扫描无已知发现及完整人工安全/许可审查属于不同证据。
