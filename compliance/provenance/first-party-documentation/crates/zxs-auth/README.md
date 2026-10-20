# ZXS 认证基础

此 crate 是管理员用户名、Argon2id 密码、会话令牌、CSRF 和浏览器 Origin 校验算法的唯一实现，不依赖 HTTP 框架、Linux 或数据库。算法与参数保持现有契约；服务端与 CLI 直接依赖 `zxs-auth`，平台不再保留认证兼容导出。

账户状态、会话到期、请求限流和持久化由服务端管理，文件权限及存储提交由核心管理。本 crate 不授予存储权限。

验证：`cargo test --locked --offline -p zxs-auth`。
