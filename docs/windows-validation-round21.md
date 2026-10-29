# 第21轮：恢复点顺序与时钟回拨

日期：2026-10-27。接续第20轮，以正常工作量完善保存和恢复安全。
本轮只操作隔离合成文件、E盘构建目录和所属测试进程，不改系统时间，
不安装或发布。证据根目录：`E:/ElyOfficeBuild/qa-round21`。

## 问题与修复

原实现用传入系统时间的纳秒值直接命名恢复点，并按该名字排序、清理：

- 相同时间的第二次写入覆盖第一次的完整内容和光标资料，密文同样受影响。
- 时钟回拨后，新编辑排在旧编辑之前；保留数量为1时，新点立即被prune，
  `write_once`却返回成功路径并更新`last_ok`。较宽保留量下，`newest`也会
  恢复较旧内容。重新创建Recovery对象仍受影响，不能只在内存里记计数。
- 同名孤立cursor/history可能被新内容错误认领。

修复继续使用现有`.bin`、`.enc`、`.cursor`、`.history`文件和索引格式：

- 从磁盘已有内容文件名读取最高序号；下一点取请求时间与严格递增下界的较大值，
  并跳过已有内容及任何同名sidecar，包括目录和符号链接占位。
- 检查序号递增溢出和现有时间表示的上限；失败显式报错，不回绕或覆盖。
- 复用现有`restore::publish`的临时写入、同步、禁止覆盖的发布路径。
- 复用`ely_platform::lock::FileLock::try_take`，使同一目录的命名、发布、
  按数量/容量清理、更新成功状态和清discard标记持同一把操作系统锁。
  `maintain`和`discard`也取此锁；忙时返回WouldBlock，解除后可重试。

锁是新增的写/维护互斥保证，只约束这三个协作入口；不把所有读取、恢复、
外部程序改文件或删除整个目录宣称为已具备事务隔离。

时钟不前进时，`RecoveryPoint.at`沿用文件名表示的**逻辑排序时刻**，可能
晚于校正后的真实时间；`last_ok`仍记录调用者传入的实际写入时间。此时记录
警告日志。没有加入单独的真实采集时间字段，也不把逻辑顺序当作精确审计时间。

## 冻结测试与修前实证

先固定测试正文，再编译旧生产实现；没有为测试改造旧生产入口。
修前测试程序实际`fresh=false`，构建1分19秒；完整结果为：

**0通过、8失败、0忽略，测试程序和所属根进程退出码均101。**

`runs-before.json`指向原始stdout/stderr和进程资源记录；所属测试正常结束，
没有SKIP、资源中止或强制回收。修前程序单独存于`before-bin/ely_files.exe`：

`12FA31D7364E6BD5E872599CF2DDD7F58F63730C4C41B437AF7EF2DDEA1476EB`

| 冻结测试 | 修前失败的原因 |
| --- | --- |
| equal_plain_clocks_keep_both_contents_and_their_own_cursors | 两次写入返回同一路径，第一次内容被替换 |
| equal_sealed_clocks_keep_both_complete_bundles | 两次密文写入也覆盖同一路径 |
| a_clock_rollback_never_prunes_the_point_just_reported_saved | 成功返回的点已经被清理 |
| reopening_after_a_clock_rollback_still_restores_the_latest_edit | newest恢复旧内容 |
| orphan_metadata_never_attaches_to_a_new_point | 新内容带上旧cursor |
| a_second_writer_refuses_the_slot_until_unlocked | 原实现不遵守新增协作锁 |
| maintenance_never_prunes_while_a_writer_owns_the_slot | 原实现不遵守新增协作锁 |
| discard_never_removes_points_while_a_writer_owns_the_slot | 原实现不遵守新增协作锁 |

前五项是已有数据/排序问题的实证；后三项是新增互斥契约的回归，不将其
包装成此前已有锁协议遭破坏。回拨两项分别含明文和固定测试密钥密文分支；
修前遇首个断言失败即结束，未运行的后续分支不算通过。

冻结文件在本地`.work/round21/frozen`保存，修复后保持逐字节一致：

| 文件 | SHA-256 |
| --- | --- |
| recovery/tests.rs | `8D434F80441276143EDD75D9C12D4317046F8EEA72F1818828E090B27BC756FC` |
| recovery/tests/clock.rs | `39C44CE601C2D766CC7CF28845E39AFCEEF70CCC6FA5944B8518EEC7CA5F7C7B` |

## 当前源码回归

- 修后测试构建2分57秒，实际重新编译Files、Office及三应用。
- Files全库130通过，另两个既有ignored子进程辅助入口不计入通过数；它们由
  相应父测试显式调用。Office全库33通过。新增8项已包括在Files130内。
- 三应用回归也全部通过；五组共387项不同测试，8项新增用例包含在其中，
  子进程场景和修前测试不重复累加。保留每组的实际退出码和资源记录。

| 测试组 | 通过 | 用时 |
| --- | ---: | ---: |
| ely-office全库 | 33 | 0.99秒 |
| ely-files全库 | 130 | 6.72秒 |
| ElyDoc应用回归 | 64 | 210.72秒 |
| ElyTable应用回归 | 68 | 176.74秒 |
| ElyShow应用回归 | 92 | 269.08秒 |

五组均无失败、SKIP、资源中止或强制回收，所属worker峰值不超过1；
保留2GiB提交余量和1GiB物理内存守卫。Office全库还实际重跑了第20轮的
10个进程终止恢复子场景，不把它们算成本轮新增测试。
本轮没有重复LOK全部视图测试或宣称全工作区覆盖；既有剪贴板相关测试未运行。

## 优化构建与原生验证

三应用实际优化构建完成，用时3分34秒；各应用制品均`fresh=false`，
冻结副本位于`accepted-bin`，五份测试程序另存`test-bin`。
worker复用第17轮，SAL及72内核补丁未变，不记作新的C++或worker构建。

| 程序 | SHA-256 |
| --- | --- |
| ElyDoc | `71D8F6BF98835A99CC11B47FA16FBB3427D9A60386FE4DC484D2671E7C41ED5C` |
| ElyTable | `36C29C3AFDA48402A60576AB7FEFFA99C74368FF129A57DCF94EE50D86ED2A03` |
| ElyShow | `E1AB33FA2348EC57AEF579900AA0762191474844F7E520091DE71DB114AB120B` |
| 复用worker | `561EEBBDA94FF1AB17EA06DAC945C739A95708AF62444A0ED2C05695B117B58D` |

不放宽第20轮原生恢复脚本或关闭探针，原散列均保持不变：

- 当前程序第二次恢复运行`native/native-recovery-18f88066`通过：独立读取
  完整点后才注入所属进程终止，退出1且Job空；同资料目录重启后打开未保存
  副本，再经原生另存为保存，正常关闭退出0且Job空，原件未被覆盖。
  正常阶段无ERROR或意外清理，worker峰值1。
- 另用Python标准库独立解析`committed-point.bin`和`recovered-copy.xlsx`的
  ZIP/XML，两者完整A1为`Round20 recovery 中文 😀`，B1公式为`1+2`、缓存值3。
  固定脚本沿用第20轮测试标记，未靠改名字或减少Unicode字符生成通过。
  `native-content.json`保留内容、散列和校验结果。
- 实际查看`restored.png`：活动标签为未保存的Untitled1，B1公式和值可见；
  A1因列宽截断，完整Unicode另由上述内容读取验证。提示明确光标和撤销历史
  没有恢复；不将截图当作完整文本或系统IME输入验证。
- 第20轮最终程序使用同一脚本的正常对照`native-r20-control/native-recovery-734ba4e8`
  也通过。这是既有恢复流程的阳性对照，不是第20轮已复现下述输入崩溃的证据。
- 当前三应用各一次严格原生关闭通过：Doc为`close/close-probe-3581ad8c`，
  Table为`close/close-probe-3d374b64`，Show为`close/close-probe-3ad154dd`。
  均有首图块、退出0、Job空、原输入不变，无ERROR、无效句柄、资源中止或强制清理。

全部证据整理后，独立R1正式审查通过，结论严格限恢复存储，见下方收尾记录。

## 保留为失败的输入崩溃

当前程序首次恢复运行`native/native-recovery-dae7f052`在`edit`阶段崩溃，
尚未执行预定故障注入（`injected_crash=false`）；退出-1073740791、
`unexpected_cleanup=true`，必须计作失败。日志原样保留，不能用第二次
通过覆盖，也不把这一轮计成“所有原生尝试都通过”。

15:51:35 UTC记录`window controls: Minimize`，15:51:37 UTC共用输入组件
`forms/text/edit.rs:52`用offset11切长度0的字符串而abort。只读调用链预核
发现候选路径：`set_text`改文本和选区、旧layout等待重绘；`offset_for_point`
仍可从旧layout产生超出当前文本的位置，随后End按该位置取行末。
这支持输入/布局失配的调查方向，不等于已定位全部触发时序或Minimize发起者。

本轮输入及依赖源码均未改变，组件仍固定在de961eb；现场未报本轮恢复点
命名或互斥错误。目前证据不足以把崩溃归因到存储修复，也不足以断言第20轮
程序已复现同一崩溃。原生恢复重跑和上轮正常对照只证明相应流程这次可完成，
**输入崩溃仍未修复，列为后续优先项**；本轮结项限于恢复存储改动。

## 范围与保护

基线2096个原文件、原120项暂存/index及72个LibreOffice补丁须保持保护。
原文件只允许本轮修改README、TASKS、Recovery实现及恢复测试根模块。
新增命名/锁模块和专测均独立组织，不增加依赖、C++补丁、worker或IPC协议。

旧轮的冻结程序、worker/SAL和第20轮manifest共31项按原散列保护；六个
`rust-target/debug/deps`路径是Cargo共享编译工作输出，会随重建更新，
其第20轮冻结副本仍保留在`qa-round20/test-final-bin`等目录。最初保护核对
曾将这六个工作输出也当作不可变文件而中止，后明确区分并核对全部历史副本；
没有放宽测试判定或改写第20轮记录。

本轮注入的是传给恢复存储的时间值，未改变Windows时钟；密文测试使用
固定测试密钥，不等于原生加密Office、系统密钥库或会话密钥警告已全流程验收。
原生恢复另以当前真实时间验证，不能充当Windows系统回拨实验。

本轮不新增光标/撤销历史的原生捕获，也未完成真实满盘卷、所有快照/刷新/
替换崩溃边界、系统IME、多屏DPI或长期连续使用验收。完整点之间的输入仍
受恢复间隔限制。普通明文内容与cursor保持既有分文件表示，不宣称本轮已将
所有元数据发布改为一个事务或消除了任意中断的风险。

## 收尾

[正式独立R1](reviews/round21-clock-r1.md)结论为恢复存储范围PASS，
首轮输入abort仍FAIL且未修。报告从本地审查结果逐字节归档，SHA-256为
`E454F5A745B7ECA1944C99898837D0E89E93055BBBAC8C14B62446703C59A79B`。
三个本轮任务在正式审查后勾选；没有将后续输入问题写成完成。

最终`artifacts.json`关联387个不同测试、修前失败、当前成功和失败原生记录、
正常对照、三个关闭探针、四份C/E源码对、测试冻结及历史制品保护。
新增四份项目文件含正式报告；四个原文件按允许范围修改，其余原文件、
暂存和补丁保持。禁用增量编译，构建后的空增量目录已核实并移除。
本轮收尾，不进入下一轮，不安装或上传。

新增存储专测可用`cargo test -p ely-files --lib recovery::tests::clock:: -- --test-threads=1`
复跑；本机五组及原生验证的运行配置、原始日志和制品散列以证据清单为准。
