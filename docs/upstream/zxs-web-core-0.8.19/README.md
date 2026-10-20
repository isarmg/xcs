# zxs-web-core

本目录是项目维护的 Rust CSR 运行时。 对外统一通过 zxs-web 使用；只启用 CSR，业务认证、传输和平台接入由项目独立实现。

第三方基础来自 Leptos 0.8.19，以下保留原始 MIT 通知和全文。外部 reactive_graph、tachys 等 Cargo 包保留实际包名。

ZXS 的 `ForSnapshot` 在 Leptos 的逐行 owner 结构与 Tachys 的 keyed DOM 调和器之间整合逐键响应式快照。传输任务使用稳定操作编号作为 key：状态变化只发布该行的新数据；相同快照不通知该行消费者，展开的操作详情与焦点保留。移除 key 后重新出现会创建新行；每个快照的重复 key 在任何数据发布前被拒绝。这是本地新增运行时能力，原始 `For` 保留原语义。自有回归覆盖值更新、重排、相同值通知抑制、移除重建和重复 key；真实浏览器回归另外验证一次实际上传从 `uploading` 到 `published` 的 DOM、详情展开和焦点保留。

```text
MIT License

Copyright (c) 2022 Greg Johnston

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
