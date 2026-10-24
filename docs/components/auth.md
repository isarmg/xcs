# ZXS 认证基础

此 crate 是管理员用户名、Argon2id 密码、会话令牌、CSRF 和浏览器 Origin 校验算法的唯一实现，不依赖 HTTP 框架、Linux 或数据库。算法与参数保持现有契约；服务端与离线救援工具直接依赖 `authentication`，平台不提供认证兼容导出。

唯一管理员凭据状态、会话到期、请求限流和持久化由服务端管理；核心不持有用户 ACL，负责对象归属与存储事务约束。本 crate 不处理存储业务约束，也不提供账户集合或角色模型。

验证：`cargo test --locked --offline -p authentication`。
