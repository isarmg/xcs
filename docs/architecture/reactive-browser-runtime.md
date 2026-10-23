<a id="stable-client-side-reactive-runtime"></a>

# 稳定的客户端响应式运行时

当前浏览器技术栈分为三类职责：

- `reactive-state` 管理响应式依赖跟踪、owner/child 生命周期、清理、signal、记忆化和 effect；没有服务端/客户端共享的 hydration 上下文，也没有热重载函数指针注册表。
- `view-renderer` 管理 DOM 构造、模板克隆、事件绑定、增量重建和卸载。其 `hydration` 游标模块仍有用途：遍历客户端克隆模板，而非服务端渲染的 HTML。
- `dom-helpers` 提供浏览器辅助能力；`src/interface` 组合应用组件与生命周期行为。应用中的浏览器定时器使用回调 API，而非可选的 timer futures 适配器。

工作区固定稳定版 Rust，支持 `.get()`、`.set()` 和稳定闭包转换；不提供 nightly signal 调用简写、服务端 hydration 或 subsecond 热重载模式。响应式图/owner、DOM 渲染和应用组件继续分开，各自具有独立的行为与测试契约。

CSR 范围门禁通过隔离 manifest，在启用及禁用 effect 两种情况下运行保留的 14 项响应式单元测试；typed-builder 仅供测试使用。门禁还检查真实模板探针的原生及 WASM 目标，并拒绝已退役的 owner API 和特性请求。界面集成测试覆盖清理及 parent/weak-owner 生命周期。`test/ci/test_reactive_csr_scope.py` 的静态回归防止退役模式、编译器探测和多余直接依赖请求重新进入。

上游身份、原始 manifest、许可证及源码收据继续保留在 `compliance/provenance` 与 `compliance/rust-sources.json`，不得通过改名暗示这些实现由本项目原创。
