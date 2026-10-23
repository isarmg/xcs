# TLS 不可信证书与来源维护

## 范围与原因

当前 TLS 依赖必须防止 CVE-2026-111943 涉及的不可信证书被接受。不是关闭审计、重标旧代码版本或依赖公告豁免。

- [RUSTSEC-2026-0382](https://github.com/RustSec/advisory-db/blob/main/crates/rustls-webpki/RUSTSEC-2026-0382.md)，公告日期 2026-10-20：已不可信的 emSign 中间证书公钥可能仍被链验证接受。稳定系列修复版本为 0.103.16；0.104 预发布系列从 alpha.8 修复。当前采用 0.103.16。
- [RUSTSEC-2026-0381](https://github.com/RustSec/advisory-db/blob/main/crates/webpki-roots/RUSTSEC-2026-0381.md)，公告日期 2026-10-19：旧根库包含 emSign ECC Root CA - G3。修复版本 1.0.10 移除此根；当前采用 1.0.10。

只删除根不能替代对不可信公钥的验证检查；必须同时保留两类防护。未修改用户系统信任库、部署配置或 TLS 验证策略，未取得或使用事件涉及的真实私钥。

## 精确来源

| 组件 | 官方发布归档 SHA-256 | 官方修复提交 |
| --- | --- | --- |
| rustls-webpki 0.103.16 | `4443b1b6d39d7c663139942cc4558262562d2c84123bb6282ef177009ca8b281` | [`8ecdcc76f56b68fda67a6adf7976f5eed5006a4c`](https://github.com/rustls/webpki/commit/8ecdcc76f56b68fda67a6adf7976f5eed5006a4c)，[PR 546](https://github.com/rustls/webpki/pull/546) |
| webpki-roots 1.0.10 | `1c1c8e20c8a38fc7d65abfe74eb7280e4aa667e8eec8bc84996d2874810748d9` | [`70760478b9d3ea729b22faa0b5324da38a297c1c`](https://github.com/rustls/webpki-roots/commit/70760478b9d3ea729b22faa0b5324da38a297c1c)，[PR 133](https://github.com/rustls/webpki-roots/pull/133) |

归档来自 static.crates.io，SHA-256 与 index.crates.io 的对应版本记录核对。归档内 VCS 身份与上述官方合并提交相同。两个组件的 `src/` 完整采用对应发布归档的原字节，包括八份不可信公钥 DER；路径依赖清单继续采用本项目布局。导入前原始清单、锁文件、校验和、VCS 元数据、registry 版本记录和许可证另存于新的版本目录。

旧版本的 origins 记录、原始文件哈希、许可证和文档不覆盖、不删除。当前来源清单用显式 superseded 收据记录旧活动包到新来源的关系；新版本导入必须校验保留的 `src/` 完整清单和每个文件的哈希。原始身份审计仍覆盖所有活动和合并实现；只有已被实际新代码替换的旧身份进入历史，不作为活动依赖重复扫描。

## 验证边界

离线 TLS 回归位于 `test/client/integration/tls_trust_regression.rs`。使用合成测试证书和官方公开 SPKI 检查正常链、叶证书／中间证书／根中的不可信公钥，以及根库中被移除证书。测试密钥仅属于本地测试证书，不是泄露事件中的密钥。真实 reqwest 客户端通过本地 TLS 服务进行验证，不访问公网服务或系统信任库。

源码门禁验证原始来源不可变、升级历史、离线闭包和当前文件哈希；严格诊断与质量门禁继续检查产品和依赖诊断。RustSec 扫描仍在线读取最新公告，无忽略列表、平台排除或放宽警告。独立上游仓库的完整测试矩阵不等于本项目离线回归，未将二者混称。
