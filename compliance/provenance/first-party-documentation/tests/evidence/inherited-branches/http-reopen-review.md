# HTTP 恢复夹具的锁释放时序

全工作区第一次验收中，`file_notes_are_authorized_bounded_persistent_and_download_uses_current_container_name` 在丢弃旧 Router 后立即 `Engine::open`，偶发返回 `WRITER_ALREADY_ACTIVE`。原始失败保留在 `native-workspace-initial-lock-race.log`；同次运行其余 11 项 HTTP 场景通过。

代码核查定位到测试夹具退出的竞态：`Telemetry::start` 的后台线程每 5 秒将 Weak 引用临时升级为强引用，再执行采样。若 Router 恰好在一次采样期间被丢弃，采样持有的后端引用仍让 Engine 保留排他 OS 锁。线程完成当前采样后才释放这个引用；`drop(router)` 不会等待该线程。Engine 的读租约仅持有独立的读计数表，不持有 Engine 的写锁。

只修改了 HTTP 测试夹具：新增 `reopen_after_router_drop`，并将同文件 4 个 Router 丢弃后的恢复入口统一使用它。夹具每 10 毫秒检查一次实际 `Engine::open`，只对 `WRITER_ALREADY_ACTIVE` 等待，最长 2 秒；其他错误立即失败，超过截止时间仍有写者也硬失败。没有修改生产采样器、存储排他锁、分支逻辑或测试断言。

验证结果：

- 原失败用例单独重跑通过，见 `http-notes-reopen.log`。
- HTTP 全套 12 项通过，见 `http-engine-reopen.log`。

以上均使用测试临时目录，不读取或修改现有服务数据。最终全工作区验收由主流程另行记录；本记录不将一次重跑等同于无限次稳定性证明。
