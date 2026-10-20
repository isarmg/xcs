下载同一版本的全部发布文件，放在同一个空目录中：两个 `.tar.gz` 归档、`INSTALL.md`、`BUILD.txt` 和 `SHA256SUMS`。先核对所选标签与提交，再校验全部文件：

```sh
(
set -eu
sha256sum --check SHA256SUMS
VERSION="$(sed -n 's/^version=//p' BUILD.txt)"
test -n "$VERSION"
mkdir "server-$VERSION" "web-$VERSION"
tar -xzf "zxs-$VERSION-x86_64-unknown-linux-gnu.tar.gz" -C "server-$VERSION"
tar -xzf "web-interface-$VERSION.tar.gz" -C "web-$VERSION"
"./server-$VERSION/zxs" --help
"./server-$VERSION/zxs" serve --help
)
```

`mkdir` 必须成功创建两个新目录；已有目录应另选新名称。服务端与 Web 分别解压到独立空目录，避免混合文件或覆盖现有业务数据。两个归档都携带完整的 `docs/`、根说明与 `compliance/`；请从解压目录阅读 `docs/release.md`、`docs/25-operation-manual.md` 和 `docs/components/smb.md`。文档中的开发源码路径用于定位对应提交，开发与来源复核需要另外检出 `BUILD.txt` 指定的仓库提交。

服务端通过 `zxs serve` 提供业务 API 和 Web 静态资源；按运维文档配置数据与认证，并将 `--web-dir` 指向 Web 解压目录的绝对路径。Linux 二进制要求兼容 Ubuntu 24.04 的 GNU libc 及运行库。

SMB 通过独立的 Samba → 本机桥接访问核心。发布包不包含 Samba VFS 二进制；须按照 `docs/components/smb.md`，使用固定的 Samba 4.23.0 SDK 单独构建、核对 ABI 并完成实际客户端验收。
