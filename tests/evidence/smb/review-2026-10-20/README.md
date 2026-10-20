# 2026-10-20 SMB FFI 审查验证

## 环境与范围

- Linux x86_64 / Debian 13，`rustc 1.99.0 (b940084d7 2026-09-28)`。
- 官方 Samba 4.23.0 / VFS 51，从 `https://download.samba.org/pub/samba/stable/samba-4.23.0.tar.gz` 下载，tar SHA-256：`b0cb963a63eb1515227f3779a46d3a7f790c1bbfeeb4b31fd43e405eefe7a3af`。固定摘要核对通过，未验证发行签名。
- `source3/include/vfs.h` SHA-256：`854fcd3d14cabc918de6aea755d1af8935ab8d2a4cf5a4d4c17a1e9f06744356`。
- 使用新下载的官方源码完成 configure，编译真实 `smbd/smbd`、`client/smbclient` 和依赖库。之后的最终验证复用了该构建缓存。[SDK 构建确认日志](sdk-build.txt)是最后一次增量复核，不冒充干净构建计时。
- 未安装/启动 Samba，未修改系统服务、用户、网络或权限配置；未执行真实 SMB 客户端往返。本记录不替代 2026-10-18 缺失的历史原始日志。

SDK configure 参数和 Ubuntu CI 依赖清单保存在 [SDK 脚本](../../../../.github/scripts/smb-sdk.sh)和 [CI workflow](../../../../.github/workflows/ci.yml)。本地缺少工具时，将官方 Debian flex/bison/m4 包及官方 CPAN Parse::Yapp 1.21 解压到临时目录使用，没有安装到系统。CI 使用 Ubuntu 官方包。当前机器有 libclang 19，但没有其标准资源头目录，因此额外传入本机已安装 GCC 14 的标准头目录；没有替换或模拟 Samba ABI。

## 最终命令与结果

在配置好以上工具的 PATH、PERL5LIB、BISON_PKGDATADIR、M4 后执行：

```sh
ZXS_SMB_SDK_CACHE=<temporary-sdk-cache> \
ZXS_SMB_SDK_JOBS=4 \
ZXS_SMB_SDK_CLANG_ARGS_JSON='["-isystem/usr/lib/gcc/x86_64-linux-gnu/14/include"]' \
bash .github/scripts/smb-sdk.sh
```

脚本退出 0，[完整最终验证输出](validation.txt)包含：

```sh
cargo clippy --locked --manifest-path integrations/smb/Cargo.toml \
  -p zxs-smb-vfs --all-targets -- -D warnings
cargo test --locked --manifest-path integrations/smb/Cargo.toml -p zxs-smb-vfs
cargo build --locked --manifest-path integrations/smb/Cargo.toml -p zxs-smb-vfs --release
```

- 严格 Clippy 通过。手写代码拒绝未说明的 unsafe 块和 unsafe 函数中的隐式不安全操作；仅 bindgen 生成的 SDK 模块保留明确列出的 C 命名/位域生成代码 lint 例外。
- 7 项测试全部通过，包括输入空值/长度验证、异步复制越界拒绝、取消后不触碰宿主状态、异步等待不阻塞宿主事件循环、路径隔离、失败查询保留指针、锁/保存失败关闭 FD。
- release 模块 `target/native/release/libzxs_smb_vfs.so` SHA-256：`c72636bbdb5aa31b8b217f7b503828447edeb7fa4b85663dcae9f011c8eadce2`。该摘要对应本次平台及 SDK 配置，不承诺跨机器字节一致。
- `ldd` 确认测试进程加载该 SDK 的 `bin/shared/private/libtevent-private-samba.so` 和 `libtalloc-private-samba.so`。测试链接配置还指定真实 SDK 的 smbconf；动态链接器可剔除未使用库，不能将链接选项等同于运行时加载或完整宿主测试。

## 发现并修正的测试夹具并发问题

首次复跑时，一项测试在 `tevent_context_init` 返回空值处失败，尚未执行 VFS 请求。上游 `lib/tevent/tevent.c` 的 `tevent_backend_init` 使用无锁进程全局 `static bool done`，在注册后端链表前即设置 `done = true`。Rust 并行测试的首次上下文创建可看到尚未注册的后端。

修复只在测试辅助函数中用 mutex 串行化 `tevent_context_init`；请求执行、工作线程与取消仍并行。生产模块不创建 tevent context，而使用 Samba 传入的宿主事件循环，该测试修复不更改生产调度策略。

对最终编译测试二进制，在真实 SDK `LD_LIBRARY_PATH` 下执行 30 次独立进程，每次显式 `--test-threads=7`：

```sh
for i in $(seq 1 30); do
  target/native/debug/deps/zxs_smb_vfs-cdc11088d1c62024 --test-threads=7
done
```

[逐次结果](repeated-tests.txt)：30/30 次成功，共 210 项测试通过。二进制后缀是本次 Cargo 输出的实际文件名，其他机器应使用自身 Cargo 返回的测试路径。

日志仅将本地仓库绝对前缀替换为 `<repository>`、SDK 前缀替换为 `<official-sdk>`；未修改测试结果。原始日志中的缓存命中和增量构建信息保留。
