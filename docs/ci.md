# 持续集成

[GitHub Actions 工作流](../.github/workflows/ci.yml) 在 push、pull request 及手动触发时检查项目。使用 `rust-toolchain.toml` 指定工具链与唯一根 Cargo.lock，不部署或发布。产品用户入口仅为 Web 与 SMB；没有桌面或移动 App 构建任务。

所有自有 Rust 组件和基础实现由一个根 workspace 管理，命令按包和编译目标选择。统一入口为 [check.sh](../.github/scripts/check.sh)：

| 范围 | 实际检查 |
| --- | --- |
| `rust-sources` | 源码、manifest、全部特性边、锁文件、来源文档和退役记录校验；空 Cargo home 重新解析七个入口／目标闭包，拒绝无闭包使用的活动源码树 |
| `quality` | 自有 workspace 的格式与严格 Clippy；宿主机全部目标及 Web 的 WASM release 编译，所有本地依赖的 Rust warning/error 均失败；基础源码不自动格式化 |
| `security` | 固定 cargo-audit 扫描器查询当前 RustSec；消费者锁和活动／已合并实现的原始身份均扫描，告警或扫描错误失败；已移除来源单列历史记录 |
| `workspace` | 服务端、核心、认证、平台、运维 CLI、HTTP 与 socket 测试；另行串行运行真实大历史包用例 |
| `foundation` | 密码哈希、经真实 storage-client 消费者启用数据的 IDNA 全 Unicode 标量回归，以及数值基础库与位集合／arena／符号反解库的限定 lib 回归；不恢复已退役开发依赖或运行全部上游测试 |
| `web` | Web 应用、公共 runtime 与 CSR 内核/宏的单元、集成及 Rustdoc 测试；过程宏通过仅开发依赖的真实消费者验证文档 |
| `contracts` | 契约工具测试及全仓 schema、配置、文档链接检查 |
| `build-tools` | 发布校验、WASM 构建器、本地绑定工具、Unicode 重现、离线编排与性能报告校验器回归；不执行性能工作负载 |
| `smb-protocol` | 显式选择独立的 smb-protocol 协议包 |
| `smb-sdk` job | 固定哈希的官方 Samba 4.23.0 SDK，真实 VFS Clippy、测试编译和 release 构建均经过零 Rust 警告门禁，再正常执行链接测试 |
| `web-production` | 本地维护的绑定工具、wasm32 release 构建、资源清单和压缩产物 |
| `browser-smoke` | 实际 Web/WASM 对真实服务端的 Chrome 交互；响应式尺寸预览不冒充原生 App 验收 |

安装工具链、WASM target、浏览器/驱动和所需原生库后，在仓库根执行：

```sh
python3 tools/rust-sources/manage.py verify --metadata
bash .github/scripts/check.sh rust-sources
bash .github/scripts/check.sh workspace
bash .github/scripts/check.sh foundation
bash .github/scripts/check.sh web
bash .github/scripts/check.sh smb-protocol
bash .github/scripts/check.sh contracts
bash .github/scripts/check.sh build-tools
bash .github/scripts/check.sh quality
bash .github/scripts/check.sh security
bash .github/scripts/check.sh web-production
bash .github/scripts/check.sh browser-smoke
```

所有业务及构建辅助 Rust 源码通过根工作区的本地依赖提供；正式 Cargo 命令使用 `--locked --offline`。验证器使用临时空 Cargo home 核对登记目标的依赖闭包。工具链和非 Rust SDK 继续独立管理；安全扫描器作为开发辅助工具可以联网更新漏洞信息，其联网不参与正式业务构建。

质量、SDK 和发布编译使用 [Cargo JSON 诊断包装器](../.github/scripts/check-rust-diagnostics.py)，保留命令参数与环境。包装器拒绝所有编译警告、错误及不完整或无效的构建流；其回归同时覆盖解析失败、Cargo 失败和本地依赖警告。质量任务与发布验证任务安装 WASM 标准库后检查目标闭包。

SMB SDK 编译不等于 Samba 模块已加载或跨客户端往返通过；Chrome 检查不等于所有浏览器和移动设备通过；模拟文件故障不等于物理盘掉电验收。依赖源码本地化也不等于漏洞或许可证审查完成，见 [第三方来源清单](THIRD_PARTY_RUST_SOURCES.md)。

工作流仅授予所需权限，checkout 不保留凭据，官方 Actions 固定完整提交 SHA。产品验证使用独立 Cargo home，不缓存 registry/git 源码或测试成功结果。新增组件必须加入根工作区、复用根锁文件、登记中央来源清单并增加对应检查；不得增加子工作区或活动子锁文件。
