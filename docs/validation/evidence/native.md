# 原生工作区验证

当前完整工作区已扩展至 172 项通过，含文件容器与目录批量分支，见[新版文件管理验收](file-manager.md)。下列 154 项及公共界面计数保留为早期基线。

日期：2026-10-18。Linux，Rust 1.99.0。测试使用临时合成数据和目录模拟盘；Unix socket、文件描述符与文件所有权测试在具备相应系统能力的环境执行。

```sh
cargo test --workspace --locked --offline --target-dir target/native
cargo test --locked --offline --manifest-path tools/contract-check/Cargo.toml --target-dir target/native
cargo test --locked --offline --manifest-path integrations/smb/Cargo.toml -p smb-protocol --target-dir target/native
cargo run --locked --offline --manifest-path tools/unicode-build/Cargo.toml -- --check
cargo run --locked --offline --manifest-path tools/web-build/Cargo.toml -- check --product-root .
```

原生工作区基线测试共 **154 项通过，0 项失败**：客户端 2、契约 5、核心 40、命名 3、平台 66、服务端 32、HTTP/真实后端集成 6。服务端计数包含 9 项真实 Unix socket/核心 SMB RPC 用例。CLI 可执行入口没有单元用例，实际命令贯通记录另见 [API/CLI 证据](api.md)。

公共 Web 界面接入后，额外增加 2 项账号单元测试和 2 项真实后端 HTTP 测试。当前服务端 **34 + 8 = 42 项通过**，记录在 [本次服务端输出](../../../tests/evidence/native/server-ui-tests.txt)：覆盖修改当前账号密码、权限范围保持、错误不清会话、全部旧会话撤销、严格请求字段、CSRF/同源、最大长度 JSON 转义与三成员凭据恢复。默认隔离环境最初拒绝一项现有 Unix socket 用例，原始 [隔离环境输出](../../../tests/evidence/native/server-ui-tests-sandbox.txt) 保留该 EPERM；允许本机 socket 后全套通过。没有修改该用例以绕过检查。

原始 [工作区输出](../../../tests/evidence/native/workspace-tests.txt) 与 [契约工具输出](../../../tests/evidence/native/contract-tests.txt) 保留实际测试名、计数和退出结果。契约工具本身 3 项测试通过；固定向量检查与工具单元测试分别计数。

[最终契约与文档校验](../../../tests/evidence/native/contract-validation.txt) 记录固定向量和本地链接检查；它只验证文档契约，不能代替存储、SDK 或设备验收。

独立 [SMB 协议测试](../../../tests/evidence/native/smb-protocol-tests.txt) 3 项通过，覆盖受限帧、同主机文件描述符转交和超长输入拒绝。真实 SDK 模块及客户端互操作不计入这 3 项。

固定 Unicode 三份输入 SHA-256 和输出逐字节重现检查通过；WASM 目标检查通过。Web/App、实际 HTTP/CLI 和 Samba SDK 分别在相应证据及 README 记录，不能从此工作区测试推断其运行结果。

不包含真实 Btrfs 断电、盘位更换、安全卸载、吞吐量 SLO 或六平台真机验收。
