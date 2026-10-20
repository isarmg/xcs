# 分支系统图形实例

**直接查看：[过程演示](index.html) · [自由操作](playground.html) · [27 个静态图示](reference/index.html)。**

**最新补充：**[24 合并复用](#example-24) · [25 合并选历史节点](#example-25) · [26 标签继承](#example-26) · [27 备份前缀](#example-27) · [详细规则](../../docs/design/file-merge-tags-backup.md)。

**此前补充：**[23 改名与文件路径节点](#example-23)。

**此前新增：**[17 跨目录移交](#example-17) · [18 移交后跟踪](#example-18) · [19 批量移动](#example-19) · [20 批量复制](#example-20) · [21 批量删除](#example-21) · [22 批量重命名](#example-22)。

父节点在下、子节点在上。绿色为本次新增节点，灰色为已有或复用节点，紫色为明确选择，蓝色胶囊为分支引用标签。

规则：[三个已确认边界](../../docs/design/branch-system-logic.md#s21) · [移动与跟踪移交](../../docs/design/branch-system-logic.md#s23) · [批量分支操作](../../docs/design/branch-system-logic.md#s24)。设计图示不代表实现或验收结果。

<a id="example-1"></a>
## 01．登记文件分支

![登记文件分支：两个名称，共用一个父节点](reference/01-file-register.png)

还没有新内容节点；两个蓝色标签都直接指向 F1。空占位文件为 0。 [查看 SVG 原图](reference/01-file-register.svg)

<a id="example-2"></a>
## 02．文件首次写入

![文件首次写入：在父节点上方产生新节点](reference/02-file-first-write.png)

审阅稿第一次保存“预算 120”，才产生真实文件节点 F2；main 留在 F1。 [查看 SVG 原图](reference/02-file-first-write.svg)

<a id="example-3"></a>
## 03．登记目录分支

![登记目录分支：直接共用父目录子树](reference/03-directory-register.png)

此时没有新增文件分支、文件内容节点或自有子树；资料中的后代也直接复用。 [查看 SVG 原图](reference/03-directory-register.svg)

<a id="example-4"></a>
## 04．目录内只写一文件

![只写报告：同时产生文件节点和目录子树](reference/04-directory-first-write.png)

本次只为报告创建公共文件分支。数据 G1、资料 S1 继续复用，目录分支仍叫方案甲。 [查看 SVG 原图](reference/04-directory-first-write.svg)

<a id="example-5"></a>
## 05．再首次修改数据

![报告已修改过，数据现在才第一次写入](reference/05-next-file-write.png)

目录追加 T3，数据第一次产生 G2；报告继续复用 F3，不增加新的命名目录分支。 [查看 SVG 原图](reference/05-next-file-write.svg)

<a id="example-6"></a>
## 06．切换已有文件分支

![切换到已有文件分支：只新增目录子树](reference/06-switch-file.png)

方案乙第一次操作是选中审阅稿 F8。这是目录写入，完成首次分化，但没有新增文件正文。 [查看 SVG 原图](reference/06-switch-file.svg)

<a id="example-7"></a>
## 07．明确指定后的写入

![明确选中审阅稿后，继续写入审阅稿](reference/07-explicit-write.png)

P3：用户已经指定已有分支，后续保存推进这个分支，不再私自另建文件分支。 [查看 SVG 原图](reference/07-explicit-write.svg)

<a id="example-8"></a>
## 08．切换子文件夹

![切换子文件夹：父目录直接引用目标子树](reference/08-switch-folder.png)

资料选择发布版 S5；说明、图片、附件仍可各用不同文件分支，不做同名批量切换。 [查看 SVG 原图](reference/08-switch-folder.svg)

<a id="example-9"></a>
## 09．深层文件修改

![修改深层说明：沿路径生成必要的新子树](reference/09-nested-write.png)

每列都是同一对象的父子演进。报告 F1、图片 I1、模板 K1 都复用，只有说明产生新正文。 [查看 SVG 原图](reference/09-nested-write.svg)

<a id="example-10"></a>
## 10．完整目录恢复

![新增、删除：目录保存完整组成](reference/10-namespace.png)

P1：切回 main 恢复报告和数据；选择方案甲显示报告和日志，两个分支不互相混入文件。 [查看 SVG 原图](reference/10-namespace.svg)

<a id="example-11"></a>
## 11．选中分支重命名

![只在选中分支改名：文件身份和正文不变](reference/11-rename-move.png)

在方案甲将报告改名为客户报告，未选 main 仍保留原名称。文件身份 A、节点 F1 和正文继续复用。 [查看 SVG 原图](reference/11-rename-move.svg)

<a id="example-12"></a>
## 12．切换 SMB 默认入口

![SMB 默认主分支：用户可以选择方案甲](reference/12-smb-default.png)

蓝色标记表示默认入口。切换入口不创建正文或子树；后续新打开的操作使用选定视图。 [查看 SVG 原图](reference/12-smb-default.svg)

<a id="example-13"></a>
## 13．真正的空文件版本

![真正提交空文件：0 字节也是真实新版本](reference/13-empty-file.png)

新建分支时没有空占位文件。这里的 F2 来自用户实际提交“清空文件”，因此是完整的真实版本。 [查看 SVG 原图](reference/13-empty-file.svg)

<a id="example-14"></a>
## 14．分支名与备注

![改分支名、改备注：标签变，节点不增加](reference/14-label-metadata.png)

P2：公共文件分支自动命名后可改名。分支 ID 不变；标签和备注编辑只记元数据历史。 [查看 SVG 原图](reference/14-label-metadata.svg)

<a id="example-15"></a>
## 15．冻结引用与最新 HEAD

![冻结 F1 不追赶 F9，也不把公共 HEAD 倒退](reference/15-frozen-head.png)

仅修改数据，报告仍读冻结的 F1。报告公共 main 之前已到 F9，保持 F9，不因兄弟文件保存而倒退。 [查看 SVG 原图](reference/15-frozen-head.svg)

<a id="example-16"></a>
## 16．完整分叉历史

![真正分叉与连续子树：父在下，子在上](reference/16-branch-history.png)

方案甲、方案乙由用户显式创建。每个分支继续修改时追加子树，名称与分支 ID 保持不变。 [查看 SVG 原图](reference/16-branch-history.svg)

<a id="example-17"></a>
## 17．跨目录移交

![移动 a.m → b.l：两侧各记一个子树](reference/17-move-between-branches.png)

文件 f 实体迁到 b；a.m 记录移出并停止跟踪，b.l 记录加入并接管。a.n、b.p 保持原样。 [查看 SVG 原图](reference/17-move-between-branches.svg)

<a id="example-18"></a>
## 18．移动后继续写入

![移动后继续写：只由 b.l 跟踪更新](reference/18-tracking-after-move.png)

在 b.l 保存新内容，产生 F2 与 L2；a.m 不再跟踪，未选 a.n 仍从旧逻辑路径读取 F1。 [查看 SVG 原图](reference/18-tracking-after-move.svg)

<a id="example-19"></a>
## 19．批量移动与配对

![批量移动：源、目标逐项配对](reference/19-batch-move.png)

映射 a.m@F1 → b.l、a.n@F2 → b.p；a.r、b.q 未选不变。文件 f 的实体只迁到 b 一次。 [查看 SVG 原图](reference/19-batch-move.svg)

<a id="example-20"></a>
## 20．批量复制

![批量复制：源不变，选中目标加入独立副本](reference/20-batch-copy.png)

读取 a.m 的 f@F1，复制到 b 的 l、p 分支；只产生一个实体副本 c@C1。原文件 f 与未选 b.q 不变。 [查看 SVG 原图](reference/20-batch-copy.svg)

<a id="example-21"></a>
## 21．批量删除

![批量删除：只移除选中分支，保留其他引用](reference/21-batch-delete.png)

勾选 a.m、a.n 移除报告，两者停止跟踪。未选 a.r 仍能读取 F1，因此实际文件和所需版本不能一并清除。 [查看 SVG 原图](reference/21-batch-delete.svg)

<a id="example-22"></a>
## 22．批量重命名

![批量重命名：选中分支换名，未选仍保留旧名](reference/22-batch-rename.png)

在 a.m、a.n 改成客户报告.docx；a.r 仍叫报告.docx。文件身份 f、F1/F2 正文和所有旧子树保持不变。 [查看 SVG 原图](reference/22-batch-rename.svg)

<a id="example-23"></a>
## 23．改名与文件路径节点

![文件夹改名：目录和每个文件都增加子节点](reference/23-rename-path-nodes.png)

项目改名为客户项目：目录增加 D1，每个文件增加路径节点；正文仍是 F1、G1，两份正文没有复制。 [查看 SVG 原图](reference/23-rename-path-nodes.svg)

<a id="example-24"></a>
## 24．合并复用节点

![文件合并：复用所选分支头](reference/24-file-merge-reuse.png)

选择 A、B，采用 B 的当前 F3；结果归属 A，不新增正文或文件子节点。 [查看 SVG 原图](reference/24-file-merge-reuse.svg)

<a id="example-25"></a>
## 25．合并选历史节点

![文件合并：选任意历史节点作为结果](reference/25-file-merge-chosen.png)

选择 A、B，但采用该文件其他节点 F0；生成元数据节点 M，复用 F0 的正文与标签。 [查看 SVG 原图](reference/25-file-merge-chosen.svg)

<a id="example-26"></a>
## 26．版本标签继承

![标签修改：继承、子节点与历史编辑](reference/26-version-tags.png)

标签属于具体文件节点；修改标签产生元数据子节点，正文及原节点标签保持不变。 [查看 SVG 原图](reference/26-version-tags.svg)

<a id="example-27"></a>
## 27．备份路径前缀

![三份保存：每个备份盘独立逻辑前缀](reference/27-backup-prefix.png)

前缀由用户指定；主盘移动或改名不会把真实主盘路径写进备份前缀。 [查看 SVG 原图](reference/27-backup-prefix.svg)

