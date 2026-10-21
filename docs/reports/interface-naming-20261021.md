# 界面基础组件功能命名

本批只在助手自己的云电脑实施，接续首批基础组件改名。

| 原活动组件 | 当前组件 | 原始版本 |
| --- | --- | --- |
| leptos_dom | dom-helpers | leptos_dom 0.8.8 |
| tachys | view-renderer | tachys 0.2.19 |
| reactive_graph | reactive-state | reactive_graph 0.2.16 |
| any_spawner | task-dispatch | any_spawner 0.3.0 |
| either_of | view-alternatives | either_of 0.1.9 |
| oco_ref | shared-value | oco_ref 0.2.1 |

## 内部一致性

六个包、五个已有测试根同步迁移。渲染器内部 reactive_graph 模块目录改为 reactive_state，oco.rs 改为 shared_value.rs，十个内部文件通过显式双射清单保持数量与来源；不使用旧路径转接壳。web-interface 重导出、view-macros 生成路径、特性转发、CI 探针和活跃文档示例同步修改。原 Oco 类型改为 SharedValue，无兼容别名。通用功能性 API、序列化数据及浏览器协议不改名。

来源/许可证/版权、原始文档链接及历史证明保留原身份。普通当前源码更新与不可变历史改名记录分离；丢失或覆盖文件不能被 refresh 当作改名接受。

## 同批修复

- tracing_props! 不再生成依赖调用方恰好导入某个框架包的硬编码路径，改用定义方 $crate 路径和隐藏 tracing 导出。只有显式 trace-component-props 特性启用 tracing，不改变默认依赖特性。
- 新探针通过别名调用宏两个分支，在不依赖 web-interface 的消费者中验证；探针是本项目新代码，存于 test/ci/fixtures，不伪装为上游材料。
- SharedValue 原保留的序列化测试缺少开发依赖，补充已有内部 serde_json 的测试专用边，未扩大运行时依赖。

## 已验证与边界

- 正式界面的原生、WASM 编译通过。
- view-macros 13 与 web-interface 46 个单元测试通过。
- shared-value 17 个单元和 13 个文档测试通过；view-alternatives 三个单元和三个文档测试通过。
- 独立宏探针原生运行及 WASM 检查通过。
- 全局维护代码格式、Clippy、宿主全部目标诊断、release WASM 与静态门禁通过。
- CSR 探针及追踪宏原生/WASM 检查通过；28 组已移除 API 和 13 组已移除特性负向门禁通过。
- 从当前源码重建绑定工具后，完整生产打包通过，生成正式 WASM、压缩文件和构建清单。真实浏览器仍由对应提交的独立 CI 验收；不能用编译或打包成功代替浏览器运行结果。

完整规则见[命名规范](../NAMING.md)。
