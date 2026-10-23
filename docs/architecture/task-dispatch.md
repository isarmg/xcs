<a id="task-dispatch-boundary"></a>

# 任务调度边界

`task-dispatch` 负责进程级执行器选择。支持浏览器事件循环、Tokio 和原生测试使用的 futures 执行器。初始化具有原子性，只允许一个调用者成功；重复初始化失败不会替换已生效的后端。

所有构建配置下，初始化前调用 `spawn` 或 `spawn_local` 都会 panic。任务不得因为 release 模式或日志特性而静默消失。浏览器将 Send 和局部 future 都排入微任务执行器。Tokio 调用者必须进入其运行时，局部任务另需 `LocalSet`。futures 后端共享一个惰性线程池，并为每个调用线程维护局部池；递归轮询不会二次借用同一池。初始化前或使用事件驱动后端时，`poll_local` 按设计不执行操作。

未使用的通用自定义执行器 API，以及不支持的 glib / async-executor 适配器已移除。线程局部执行器尤其不得通过进程全局函数指针发布。新后端必须有真实消费者和明确执行契约，不能只提供兼容空壳。

维护的执行测试位于 `test/task-dispatch`。基础异步门禁在 debug / release 下检查初始化和跨线程行为，并检查 WASM 编译。Web 绑定门禁还在 Chromium 中执行浏览器调度契约。`test/foundation/task-dispatch` 中原始上游测试保留来源归属；已删除适配器的测试不是活动契约。原始源码身份、版本与许可证继续由合规清单和来源材料登记。
