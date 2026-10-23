<a id="async-notification-capability"></a>

# 异步通知能力

`async-notification` 提供通知队列及可选异步锁。

<a id="boundaries"></a>

## 边界

- `event` 模块提供事件、监听器与通知原语；`listener!` 构造栈上监听器，无需启用 `locks`。
- `locks` 提供屏障、互斥锁、一次性单元、读写锁、信号量、自持有守卫及命名 future；轮询/阻塞策略为私有实现。
- 默认 `std` 提供标准库队列及仅原生可用的停驻，不默认启用锁。
- 无 `std` 时使用原子/自旋队列；WASM 不链接 `parking`，不暴露原生阻塞等待。
- SQLite 仅使用标准库事件能力；reactive-state 显式启用 `locks` 及自持有 RwLock 守卫。

取消须按队列契约转移通知，调用方不能依赖信号量严格 FIFO：`try_acquire` 可能插队。互斥锁的最终公平性也不是 FIFO 保证。

<a id="verification"></a>

## 检查

安装维护中的 Rust 工具链及原生/WASM 目标后，运行 `python3 .github/scripts/check-async-notification.py`。门禁检查 std/locks 组合、能力反向编译、原生停驻、消费者特性隔离、竞争/取消、SQLite 关闭及 reactive async-derived 行为。辅助执行器只在记录到真实唤醒后重新轮询，超时不能掩盖通知丢失。

内联测试及可运行的外部测试由门禁执行。依赖不可用开发提供方的测试只保留源码，不计为通过；基准未启用。

<a id="provenance-and-source-recovery"></a>

## 来源

来源为 event-listener 5.4.2、event-listener-strategy 0.5.4 和 async-lock 3.4.2。许可证、精确路径和可逆测试映射保存在合规凭据中；原始生产源码、清单及 event 基准位于 `compliance/provenance/async-notification`，外部测试位于 `test/foundation/async-notification/<original-package>`。
