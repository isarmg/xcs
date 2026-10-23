<a id="synchronization"></a>

# 同步原语

`synchronization` 在同一个包中维护通用锁契约、线程停驻和具体锁原语。它面向本项目的特性依赖图进行组织，而不是保留上游 crate 边界。三个来源仍分别记录在来源目录和不可变来源文档中。

<a id="capability-boundaries"></a>

## 能力边界

- 禁用默认特性时，仅提供 `api`、不依赖外部库的守卫机制以及 `scopeguard`。此配置为 `no_std`，不会启用 libc、线程本地停驻或具体锁。
- `arc_lock` 增加依赖分配的自持有守卫；`atomic_usize` 增加通用可重入锁。两者均不要求 std。
- `parking` 启用 std、底层等待/唤醒队列、Unix 上的 libc、cfg-if 和 smallvec。底层 once-cell 消费者不必启用具体锁。
- `locks` 增加 Mutex、FairMutex、RwLock、ReentrantMutex、Condvar 和 Once，统一使用同一停驻队列。这是默认配置。
- `serialization`、`send_guard` 和 `hardware-lock-elision` 必须显式启用。除非启用 `send_guard`，否则守卫不实现 Send。序列化保持可选，并使用项目中唯一的序列化类型身份。

面向调用方的运行时停驻开关为 `parking`，once_cell 的可选后端开关也为 `parking`。`futures-intrusive` 只在 std 配置下显式启用锁；spin 的通用锁 API 保持 no_std。不再保留兼容门面 crate。

<a id="safety-and-validation"></a>

## 安全与验证

底层 acquire/release 顺序、公平交接、超时回调和停驻对象地址稳定的生命周期契约保持不变。缺失依赖、无法使用的实验性死锁检测器及其空操作钩子已移除，而不是继续暴露一个缺少 backtrace 和图提供方的特性。

固定版本的 parking-core 来源是本次整合的真实主来源；导入的锁 API 和具体锁文件不会被归入它的历史投影。历史主来源的每个文件均保留，并可通过可逆映射核验。为满足这一精确证明边界，历史 Unix/通用/原子 WASM 停驻实现仍在各自目标 cfg 下保留；支持的原生目标为 Linux amd64，普通浏览器 WASM 经过检查，但这不表示支持原子 WASM 运行时。

维护门禁检查原生/WASM 能力依赖图、缺失 API 的反向编译检查、默认非 Send 与显式启用 Send 的守卫、竞争与公平交接、映射/自持有守卫、递归锁、降级、条件谓词、超时、无中毒语义及真实 JSON 序列化。原始锁/核心单元测试仍可执行。原先依赖 rand 的读写压力用例已改为明确命名的确定性混合调度；这不是随机调度，也不是 Loom/Miri 覆盖。issue-129 测试使用启动握手，不再假定 100ms 内完成调度。原始 bincode 专用可选测试和已停用检测器的测试仍保留来源记录；不会模拟缺失的 bincode 或检测器提供方。
