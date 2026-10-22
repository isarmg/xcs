# 03 总体架构与模块边界

[文档目录](README.md) · [上一章：术语、对象与核心不变量](02-concepts-and-invariants.md) · [下一章：Web 与 SMB 访问入口](04-platforms-and-access.md)

<a id="s03"></a>

一张完整总架构图，以及 Web、SMB、核心和持盘边界。

## 本章目录

- [3.1 完整总体架构图](#s03-01)
- [3.2 接入与持盘边界](#s03-02)
- [3.3 技术基线](#s03-03)

<a id="s03-01"></a>

## 3.1 完整总体架构图

```mermaid
flowchart TB
    subgraph WEB_ACCESS[Web 接入路径 / 当前产品页面]
        WEB[浏览器 Web 界面]
        API[HTTPS 文件与管理 API]
        AAUTH[业务身份 / 会话 / 目标分支]
        WEB --> API
        API --> AAUTH
    end

    subgraph SMB_ACCESS[独立 SMB 接入路径 / 主分支基础操作]
        SACCESS[桌面及移动端 SMB 访问]
        SAMBA[Samba SMB 服务]
        SBRIDGE[受控 SMB 业务桥接]
        SAUTH[SMB 身份映射 / 共享主分支绑定 / 基础操作授权]
        SACCESS --> SAMBA
        SAMBA --> SBRIDGE
        SBRIDGE --> SAUTH
    end

    subgraph MANAGEMENT[系统盘管理面]
        PRIMARY[本机账户 / 授权 / 会话主存储]
        HOST[主机诊断 / 受保护初始化与恢复]
    end
    API --> PRIMARY
    API --> HOST

    subgraph C[统一安全与业务边界]
        AUTH[认证 / 授权 / 配额 / 审计]
        NAME[统一名称与路径校验]
        SESS[入口身份 / 会话 / 基础版本 / 幂等]
    end
    AAUTH --> AUTH
    SAUTH --> AUTH
    PRIMARY --> AUTH
    AUTH --> NAME --> SESS
    subgraph DCORE[Storage Core 唯一业务持盘服务]
        NS[主分支及各分支目录快照]
        HIST[版本历史 / 文件分支与节点合并 / 旧关系标记 / 独立树拆分]
        READ[固定版本范围读取 / 健康副本选择]
        WORK[工作区 / 全量封存 / 保存边界]
        TX[单活事务协调器 / 条件发布]
        REP[用户盘位 / 固定来源 / 分级备份与修复]
        GC[历史拓扑保留 / 内容清除 / 安全回收]
        IO[受控 Disk I/O]
    end
    SESS --> NS
    NS --> HIST
    NS --> READ
    NS --> WORK
    HIST --> TX
    WORK --> TX
    TX --> REP --> IO
    READ --> IO
    GC --> IO
    subgraph E[持久化与恢复]
        LOG[三盘应用日志 / Commit / Tree / 检查点]
        IDX[SQLite 可重建索引]
        MAN[每盘清单 / 完整版本 / 提交证明]
        CHECK[整文件哈希 / 分段校验]
        REC[日志重放 / 未知事务恢复 / 索引重建]
    end
    TX --> LOG --> IDX
    IO --> MAN
    IO --> CHECK
    LOG --> REC
    MAN --> REC --> IDX
    subgraph DISKS[用户分配的 n 块独立硬盘]
        P[一级盘 m 块]
        B2[二级盘 c 块 / 指定一级来源]
        B3[三级及后续盘 / 指定上一级来源]
        SP[未分配或备用盘]
    end
    IO --> P
    IO --> B2
    IO --> B3
    LOG --> DISKS
    HELPER[部署者提供的受控挂载流程 / 非内置服务] -.-> DISKS
    MAINT[巡检 / 容量 / 迁移 / 修复] --> REP
    MAINT --> GC
```

Web 经 HTTPS 业务 API 接入，SMB 经 Samba 和服务端桥接接入；两条用户路径分别建立连接，在服务端共用权限校验、存储核心、持久化和分层备份。SMB 的主分支与基础操作限制由服务端执行，Web 私有工作区的分支选择不改变 SMB 目标。

<a id="s03-02"></a>

## 3.2 接入与持盘边界

采用 Samba 承担 SMB 协议服务，通过受控业务桥接将基础文件操作送入 Storage Core。桥接采用 Samba VFS 扩展和受保护本机接口；目标 SDK 与已验证路径见[SMB 桥接说明](components/smb.md)。其他版本及完整应用操作矩阵仍需单独验证。这里需要实现业务对接，但不重写 SMB 协议、不开发客户端虚拟盘，客户端无需本系统自研挂载。[S03][S04]

SMB 共享绑定 `(repository_id, history_tree_id, main_branch_id, share_generation)`，只展示该主分支当前文件。已实现的分支、历史图、清除、拆分、备份和磁盘管理通过获授权 Web/API 访问；不得借可写隐藏目录或特殊文件名触发。

SMB 服务及 API 网关不直接共享版本对象目录。Storage Daemon 仍是业务持盘和授权的最终边界；桥接只能提交受限请求，不能绕过命名、保护份数或历史保存。Samba 本身不能替本系统提供版本事务或跨盘保护保证。

Web 在浏览器中提供认证、文件元数据、历史与管理状态界面，不提供文件上传、下载、正文预览或传输任务页。SMB 入口由 Samba 和服务端业务桥接独立提供；受控 HTTP 内容接口和客户端库仍保留，但不是产品 CLI。`zxs` 直接启动守护进程；`zxs-rescue` 只提供明确的离线恢复、历史包与未初始化配置草稿工具，不提供日常管理命令或远程客户端。

管理 HTTP、登录和主机诊断不等待数据引擎打开。`management-auth.json` 是账户、授权和会话的系统盘主存储；数据盘只接收不含会话的凭据灾备快照，不是离线授权回退。缺失已有主存储时保持恢复要求，不因数据盘可读而自动回滚凭据。详见[管理面边界](management/control-plane.md)。

系统盘不是“仅可丢弃索引”：除独立管理主存储外，池系统目录还保存配置、工作区记录与请求证据；`stage_bytes` 和范围写入会在该目录创建真实正文临时文件，HTTP 暂存也有独立预算与保留规则。不能按“都是缓存”清空系统目录，也不宣称已经将所有正文暂存搬到数据盘。

“三盘元数据”表示用户指定的三个不同物理数据盘兼任控制日志成员，并非另外增加三块硬盘。数据盘仍各自独立，不组成跨盘条带。

<a id="s03-03"></a>

## 3.3 技术基线

| 领域 | 基线 | 说明 |
| --- | --- | --- |
| 服务端 | Rust Storage Daemon | 一个有效写协调器；可并行准备数据，按事务顺序发布 |
| 独立 SMB 接入 | Samba / SMB 3 系列 | 共享主分支基础文件，独立于 Web 接入，服务端统一执行业务规则；不启用 SMB1 |
| SMB 业务桥接 | Samba VFS 扩展到受控核心接口 | 同步执行核心业务操作；完整操作路径按目标 SDK 验证 |
| 本机内部通信 | Unix Domain Socket | 受信服务身份和受限用户代理，不接受任意身份自报 |
| Web / 受控 HTTP 客户端 | HTTPS，优先 HTTP/2 | HTTP/3 为可选扩展，不改变保存语义 |
| 浏览器界面 | 仓库内维护的 Rust/WASM CSR 与响应式运行时 | 文件与高级管理功能均经统一业务 API 访问；浏览器兼容与实际交互单独验收 |
| 每盘文件系统 | 独立 Btrfs | 保持数据校验；恢复环境兼容性独立验收 |
| 内容格式 | 仅 FULL 普通文件 | 每个新写内容版本独立完整；分支创建引用冻结父版本，首次写入才生成自有 FULL |
| 元数据 | 三成员应用日志与检查点 | 两成员决定、三成员保护后正常发布 |
| 索引 | SQLite，可重建 | 不将 SQLite WAL 当成跨盘事务 |
| 校验 | BLAKE3 整文件与分段清单 | 范围验证和全文件验证分别执行 |
| 命名 | PortableName / PortablePath | 固定 Unicode 数据版本，各入口统一校验 |
| 恢复 | 独立只读离线工具 | 不依赖在线索引或其他历史版本打开本盘文件 |

[S03]: https://www.samba.org/samba/docs/current/man-html/smbd.8.html
[S04]: https://www.samba.org/samba/docs/current/man-html/smb.conf.5.html

---

[文档目录](README.md) · [上一章：术语、对象与核心不变量](02-concepts-and-invariants.md) · [下一章：Web 与 SMB 访问入口](04-platforms-and-access.md)
