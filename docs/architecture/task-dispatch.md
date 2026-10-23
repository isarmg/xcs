<a id="task-dispatch-boundary"></a>

# 任务调度边界

`task-dispatch` 负责进程级执行器选择。支持浏览器事件循环、Tokio 和原生测试使用的 futures 执行器。初始化具有原子性，只允许一个调用者成功；重复初始化失败不会替换已生效的后端。

## 选择执行入口

- 应用已依赖 Tokio，且需要任务返回值、取消句柄或阻塞任务：直接使用 `tokio::spawn`、`tokio::task::spawn_blocking` 等运行时 API。
- 公共响应式代码只需发出任务，不想绑定后端：使用 `task_dispatch::Executor`。`spawn` 要求 `Send + 'static`，`spawn_local` 只要求 `'static`；两者的 future 都必须返回 `()`，且均不返回 join handle。
- 浏览器入口选择 `wasm-bindgen` 特性并调用 `Executor::init_wasm_bindgen()`；[界面挂载入口](../../src/interface/src/mount.rs) 已执行此初始化，不要再为组件单独选择后端。
- 原生 Tokio 消费者选择 `tokio` 特性；测试选择 `futures-executor` 特性。启用 feature 只提供初始化函数，不会自动初始化或创建 Tokio 运行时。

以下是独立原生程序的局部任务示例，需要 `task-dispatch/tokio` 及 `tokio` 的 `rt`、`sync` 特性。`LocalSet` 解决 `!Send` 执行上下文，oneshot 明确等待任务结果，而不是用一次 tick 猜测任务是否完成：

```rust
use std::rc::Rc;
use task_dispatch::Executor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Executor::init_tokio()?;
    let runtime = tokio::runtime::Builder::new_current_thread().build()?;
    runtime.block_on(tokio::task::LocalSet::new().run_until(async {
        let value = Rc::new(String::from("ready"));
        let (send, receive) = tokio::sync::oneshot::channel();
        Executor::spawn_local(async move {
            let _ = send.send(value.as_str().to_owned());
        });
        assert_eq!(receive.await.expect("task completed"), "ready");
    }));
    Ok(())
}
```

`futures-executor` 后端的局部任务不会自己前进：测试应在创建任务的同一线程调用 `Executor::poll_local()`；它运行到当前任务全部 pending 为止，不是持续事件循环。对需要计时器或网络驱动的 future，仅选择 futures 后端不够。

## 初始化与生命周期约束

所有构建配置下，初始化前调用 `spawn` 或 `spawn_local` 都会 panic。任务不得因为 release 模式或日志特性而静默消失。浏览器将 Send 和局部 future 都排入微任务执行器。Tokio 调用者必须进入其运行时，局部任务另需 `LocalSet`。futures 后端共享一个惰性线程池，并为每个调用线程维护局部池；递归轮询不会二次借用同一池。初始化前或使用事件驱动后端时，`poll_local` 按设计不执行操作。

未使用的通用自定义执行器 API，以及不支持的 glib / async-executor 适配器已移除。线程局部执行器尤其不得通过进程全局函数指针发布。新后端必须有真实消费者和明确执行契约，不能只提供兼容空壳。

维护的执行测试位于 `test/task-dispatch`。基础异步门禁在 debug / release 下检查初始化和跨线程行为，并检查 WASM 编译。Web 绑定门禁还在 Chromium 中执行浏览器调度契约。`test/foundation/task-dispatch` 中原始上游测试保留来源归属；已删除适配器的测试不是活动契约。原始源码身份、版本与许可证继续由合规清单和来源材料登记。

## Tokio 运行时与适配器

`tokio` 同时维护运行时和 `tokio::adapters`，`tokio-macros` 保持独立宿主过程宏。`rt`、`sync`、`time` 与 `io-util` 是核心能力；可选 `adapters-*` 分别提供 codec、兼容 I/O、流与阻塞桥接、任务、定时器和键控任务。`full` 选择运行时，`adapters-full` 选择适配器集合，两者不能互相替代。

常用适配入口与对应特性：

- `tokio::adapters::io::ReaderStream` / `StreamReader`：`adapters-io`，用于异步字节流与 I/O 互转。
- `tokio::adapters::io::SyncIoBridge`：`adapters-io-util`，用于同步 API 桥接；不要直接阻塞异步工作线程。
- `tokio::adapters::sync::CancellationToken`：`adapters`，合作式取消；触发 token 不等于外部操作已回滚。
- `tokio::adapters::task::TaskTracker`：`adapters-rt`；`JoinMap` 另需 `adapters-join-map`。
- `tokio::adapters::time::DelayQueue`：`adapters-time`，需要活动的 Tokio 时间驱动。

依赖选择以[当前 manifest](../../src/foundation/concurrency/tokio/Cargo.toml)为准。使用网络/时间时，除 Cargo 特性外，还需通过运行时 builder 启用相应驱动；`#[tokio::main]` 需要 `macros`，默认多线程形式还需要 `rt-multi-thread`。

最小运行时不依赖 futures、bytes 或 hash-table。普通同步适配器不依赖 bytes，键控任务才选择 hash-table。浏览器 WASM 只检查支持的单线程能力，不启用原生网络、进程或信号。内部原生传输适配器不代表 Web 提供上传、下载或预览。

`.github/scripts/check-runtime-features.py` 与 `check-runtime-join-map.py` 检查独立特性、ReaderStream 缓冲/EOF/错误融合、信号量公平性/取消/许可释放、发送预留/关闭，以及键控任务生命周期。缺少真实测试依赖的上游材料不计作执行。

## 原始文档

- [any_spawner 0.3.0](../upstream/any_spawner-0.3.0/README.md)：全局执行器和任务接口的原始设计；当前仅支持本页列出的三个后端。
- [Tokio 1.53.3](../upstream/tokio-1.53.3/README.md) 与 [tokio-util 0.7.20](../upstream/tokio-util-0.7.20/README.md)：运行时及适配器来源；本仓库适配器位于 `tokio::adapters`，不是独立 `tokio_util` 依赖。
