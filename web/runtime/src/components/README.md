# zxs 公共界面组件

这些模块由既有共享 Rust/WASM Web 组件实际移入并按项目职责拆分，保留原登录卡片、页面壳、顶栏插槽、通知、原生对话框与语义表单结构。原组件中的固定管理员认证和账户接口已移除；这里不请求业务 API，也不导入应用的状态或 DTO。

| 模块 | 职责 |
| --- | --- |
| `mod.rs` | 公共导出、身份视图、错误反馈及不实现 `Debug` 的凭据输入类型 |
| `state.rs` | 加载、空状态和含安全请求标识的错误展示 |
| `dialog.rs` | 原生 modal、Esc、初始焦点、Tab 边界与关闭后的焦点恢复 |
| `notification.rs` | 文档事件通知、数量和文本长度限制、自动消失 |
| `select.rs` | 受控单选菜单、原设计弹层、键盘选择、焦点和无障碍语义 |
| `header.rs` | 顶栏 Portal 插槽、主题和语言切换 |
| `shell.rs` | 匿名/恢复/认证状态、原登录卡容器、页面壳、账户与退出入口 |
| `login.rs` | 用户名/密码语义表单、本地输入校验、请求中与错误状态 |
| `account.rs` | 只读用户名和当前/新/确认密码表单，不改变身份或权限 |

应用通过 `Signal` 传入会话、请求中和错误状态，通过 `Callback` 执行登录、退出、恢复会话、页面切换及修改密码。组件仅提交输入值；真正请求、鉴权、凭据存储、成功结果确认和会话失效由 `web/src` 与原生 Rust 客户端负责。应用传入的会话使用相等性 `Memo`，避免文件进度变化重新挂载壳和未提交表单。

`AppShell` 的可选连接选项位于认证卡之外，不改变六行登录布局。普通用户与管理员共用壳和账户表单；是否允许业务操作由应用与服务端权限共同决定。对应样式使用项目内 `web/public/styles`，所有自有组件、类、事件和 DOM 标记统一使用 zxs 命名。

`Select` 接受 `label: String`、`options: Signal<Vec<SelectOption>>`、`value: Signal<String>` 和 `on_change: Callback<String>`；`disabled: Signal<bool>` 和 `placeholder: String` 可选。选项由 `SelectOption::new(value, label)` 创建，`disabled` 字段控制是否可选，空字符串也是有效值。菜单使用浏览器顶层 popover，避免在带轮廓的文件面板中被裁切；Rust 管理方向键、Home/End、输入跳转、Enter/空格确认、Escape/Tab 和外点关闭。焦点保持在 combobox，`aria-activedescendant` 指向当前选项。公共样式文件不再单独存在而没有实际控件。

登录卡的标签、输入和按钮统一为 `1rem`、常规字重，不再按卡片高度放大字号。六行使用内容和最小 44px 控件驱动高度，窄屏与放大文字时取消固定比例。
