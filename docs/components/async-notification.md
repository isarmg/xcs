<a id="async-notification-capability"></a>

# 异步通知能力

`async-notification` 统一维护一个通知队列实现及其上层异步锁。它整合原 `event-listener` 5.4.2、`event-listener-strategy` 0.5.4 和 `async-lock` 3.4.2 源码包，三个活动包合为一个，净减少两个。已停用的包名不保留转发 Cargo 包或依赖别名。

<a id="boundaries"></a>

## 边界

- 公共 `event` 模块：事件、监听器与通知原语；无需 `locks` 即可使用。导出的 `listener!` 宏构造栈上监听器。
- `locks`：屏障、互斥锁、一次性单元、读写锁、信号量、自持有守卫及命名 future。轮询/阻塞策略适配器是私有实现机制。
- 默认 `std`：原有标准库队列实现及仅原生支持的停驻。不启用 `locks`。
- 不启用 `std` 时，仍保留原有原子/自旋队列回退路径；这不构成对生产负载的新推荐。WASM 绝不链接 `parking`，也不暴露原生阻塞等待。
- SQLite 保留原有标准库事件行为，不启用锁。Reactive-state 显式启用 `locks`，保留自持有 RwLock 守卫契约。消费者源码名称已迁移，而非通过别名兼容。

这是实现归属和 API 边界整合，不是经测量证明的加速。队列顺序、取消时的通知转移、锁获取算法、原子顺序、unsafe 实现及原生/WASM 策略均保持不变。通用私有策略包装器未使用的引用访问器已移除；其生成的 `Future::poll` 直接调用相同的非阻塞策略。其余适配器仍支持原生阻塞调用。信号量通知顺序不保证严格按 FIFO 获取许可：`try_acquire` 可能插队。互斥锁的最终公平性也未被强化为 FIFO 保证。

<a id="verification"></a>

## 验证

使用维护中的 Rust 工具链并安装原生/WASM 目标后，运行 `python3 .github/scripts/check-async-notification.py`。此门禁已登记在 foundation-async 套件中，并使用仓库外的临时测试工作区。它检查八种原生/WASM std/locks 组合；锁/阻塞能力边界的反向编译检查；仅原生停驻；SQLite/reactive 的独立特性解析；真实唤醒驱动下的竞争/取消契约；SQLite 关闭/获取取消；以及针对真实 Tokio 提供方的保留 reactive async-derived 测试。辅助执行器在超时或伪唤醒后，没有记录到唤醒就绝不重新轮询，因此超时不能掩盖通知丢失。

三个上游内联测试仍在库中启用。其余原始外部测试文件连同来源信息保留在 `test/foundation/async-notification/<original-package>` 下，原始外部测试文件共十一份，均可恢复。依赖不可用历史开发依赖的上游集成文件属于保留的源码证据，不声称已执行覆盖。仅 intrusive 内联测试夹具将 pin 宏改为标准库等价实现，并为内部 event 命名空间增加限定。

<a id="provenance-and-source-recovery"></a>

## 来源与源码恢复

显式整合凭据保留三个原始身份、许可证、完整输入清单、精确生产路径及有界可逆的测试迁移记录。原始生产字节（包括清单和 event 基准）还以文本归档保存在 `compliance/provenance/async-notification` 下，由交付证明生成器对照已审阅的输入提交进行核验。现有逐来源许可证和来源记录保持不变。原始 event 基准保留，但不作为新基准启用。

集成时必须针对接收方当前目录重新生成计划，并在应用受保护的映射更新和刷新前核验精确输入快照。不得覆盖其他并行变更中无关的目录或外部测试归属更新。
