# 文件管理与目录分支验收

> 历史范围说明：2026-10-20 起产品仅保留 Web 与 SMB；本文 App/Tauri 的路径、命令和结果属于移除前记录，不再作为当前构建或验收要求。

补充更正：初版浏览器脚本只断言监控接口返回 CPU 数值，漏掉了小数被整数解析器拒绝的问题。现已修复并将完整 91 请求流程重新跑通，额外断言页面 CPU 百分比及无错误提示；原问题与最新页面标题调整见[修复记录](../contour-response/README.md)。本页文件业务截图保留移除大标题前的界面，最新菜单页面截图见该修复记录。

2026-10-18，Linux x86_64，Rust 1.99.0，wasm-bindgen 0.2.129，Chromium。这里记录新版文件管理流程；[旧版 Web 记录](../web/README.md)保留早期界面及启动故障证据。

本次原生完整工作区 **172 项通过**：客户端 2、契约 5、核心 48、命名 3、平台 66、服务端 36、真实 HTTP 11、监测 1。Web **48 项通过**，含运行环境 25、组件属性 3、响应式生命周期 2、应用 18。原生桥接 1 项通过；Linux Tauri 使用现有局部 SDK 编译链接，宿主未运行，未制作六平台发行包。

核心及接口回归覆盖：单文件分支只复制/切换该文件；目录递归批量创建独立文件分支、共同分支交集、原子切换；后成员重名冲突、无权限、内容不可用、新增文件缺分支时不部分发布；重启与相同操作重试保持结果；真实容器改名在事务决定前不推进、决定后可重放；内部版本字节和名称保持不变；文件及版本备注授权/长度/持久化；改名后历史版本下载当前名称；新布局校验清单、修复、离线导出、历史包及旧数据读取。

## 执行命令

```sh
cargo test --workspace --locked --offline --target-dir target/native
cargo build --locked --offline --target-dir target/native -p zxs-cli
cargo test --workspace --locked --offline --manifest-path web/Cargo.toml --target-dir target/web
cargo test --locked --offline --manifest-path apps/Cargo.toml --target-dir target/apps
cargo run --locked --offline --manifest-path tools/web-build/Cargo.toml --target-dir target/web-tools -- build --product-root .
cargo run --locked --offline --manifest-path tools/web-build/Cargo.toml --target-dir target/web-tools -- check --product-root .
cargo run --locked --offline --manifest-path tools/contract-check/Cargo.toml --target-dir target/native
```

构建时 `ZXS_WEB_BINDGEN` 指向已安装的 0.2.129 CLI。原生 socket/权限及 localhost 浏览器验证使用具备本机能力的执行环境。运行中的静态目录更新前停止服务，最终资源完成后再重启，使内容与 ETag 清单一致。

[原生测试](native-tests.txt)、[CLI 构建](native-build.txt)、[Web 测试](web-tests.txt)、[Web 构建](web-build.txt)、[原生桥接](app-bridge-tests.txt)、[Linux 链接](linux-host-build.txt)、[最终构建清单](build-manifest.json)。

开发池为私有临时三成员目录 P01 → B01 → C01。浏览器不拦截或伪造接口响应；运行中 7400 预览池保留原数据与密码，仅做资源检查和独立探测会话登录/退出。记录不包含 Cookie、令牌或密码，也不证明真实独立设备断电恢复、吞吐量 SLO 或六平台真机验收。旧正文保持兼容，不声称全盘迁移已完成。

## 最终真实浏览器与资源结果

最终未拦截请求的 Chromium 业务验收 **91 次 API 请求通过**，没有页面异常、意外 HTTP 失败或 CSP 违规。登录用户名、密码和按钮均为 14px/400。真实自定义菜单已验证弹层、键盘切换、Escape、外点关闭和 2px 可见键盘焦点；元数据刷新不再重建已选文件详情。

文件流程覆盖独立硬盘入口、只读副本盘、新建目录与递归子目录、上传自动封存保存、A 文件分支修改/回主版本时 B 不变、文件/版本备注、真实容器改名、内部版本名不变、原版本以当前名称校验下载、转义文本预览。目录创建为三个后代文件分别创建同名分支，再批量切换共同分支，目录外 C 保持原版本。界面在中文/英文往返后所有固定业务文案均为选定语言；320px、390px 页面无横向溢出。

旧验收在网络层取得 CPU 百分比、内存和三盘所在文件系统容量，CPU 页面显示存在前述漏检；历史页实际显示文件操作；会话与主题跨刷新保留，退出后刷新保持未登录。临时数据池重启后再次验证分支选择、共同目录分支、备注和当前名称下载；三盘均只有一个实际 `renamed.txt` 容器，每份含四个普通完整版本文件，链接数为 1，跨盘模拟目录 inode 不相同，缓存与发布状态一致。

运行中 7400 的 **22 份资源**全部匹配清单 SHA-256，WASM magic、application/wasm、gzip 往返、同内容 ETag、304、nosniff 和同源 WASM CSP 通过。原有预览池正常重开，原密码完成隔离登录/退出。最终构建源文件逐项匹配清单，无资源源代码漂移。

[浏览器结果](browser-result.json)、[完整浏览器输出](browser-tests.txt)、[重启与真实容器](persistence-result.json)、[资源结果](asset-result.json)、[响应头](wasm-response-headers.txt)、[源文件身份核对](source-inputs-result.json)、[WASM 检查](wasm-check.txt)、[契约检查](contract-validation.txt)。文档契约校验共 19 条路径、31 组结构/语义、3 组规范编码、2 组校验清单、5 组严格 JSON。

截图：[登录中文](login-zh.png)、[独立磁盘入口](disks-en.png)、[文件与分支](files-en.png)、[390px](files-390.png)、[320px](files-320.png)、[真实状态](status-en.png)、[实际历史](history-en.png)。浏览器诊断期间发现并修复了登录标签缩小、详情刷新关闭菜单及窄屏表格溢出；上述记录只对应全部修复后的最终产物。
