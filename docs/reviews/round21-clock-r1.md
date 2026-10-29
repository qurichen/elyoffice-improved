# 第21轮独立审查 R1

结论：PASS，范围仅为本轮恢复存储变更。未发现该范围内遗留待修缺陷。首轮原生输入abort保持FAIL且未修复；本结论不代表全部原生尝试通过或整体输入稳定。

审查日期：2026-10-27。这是第21轮首轮正式独立审查，之前的源码和失败现场预核属于同一R1。依据为 `agents.md`、`tasks/TASKS.md` 与本轮范围；审查者没有构建、运行竞争测试、改实现或测试、安装或发布。本报告为唯一写入。

## 源码与冻结测试

- `write_once` 从磁盘序号选择、发布到prune、更新成功状态和清discard标记，均持有同目录FileLock；maintain和discard复用该锁，忙时明确返回WouldBlock，退出作用域释放锁。未把这些协作写入口的互斥扩大为所有读取、恢复或外部删除目录的事务保证。
- 新序号取请求时间与磁盘已有完整点最高序号之后的较大值；重新建立Recovery对象也从磁盘恢复顺序。逐一排除bin、enc、cursor、history占位，symlink_metadata也能发现目录和悬空链接占位。checked_add和现有时间表示上限失败会报错，不回绕。
- 内容及明文cursor复用现有临时写入、同步和persist_noclobber发布，旧完整点不会被覆盖；按数量/容量删最低逻辑序号，因此回拨后的新点不再排到旧点之前而被先删。沿用文件与索引格式，没有增加依赖、内核或worker协议。
- `RecoveryPoint.at` 在时钟不前进或发生占位时可成为逻辑排序时刻；last_ok仍为调用者提供的wall time。文档明确没有另存精确采集时刻，没有更改Windows时间，也不将逻辑序号称为精确审计时间。
- 四份最终源码C/E及清单散列一致，行数分别为Recovery422、clock46、原测试根文件422、新clock测试135，均不超过500行。两个测试文件与首次冻结归档逐字节一致：`8D434F80441276143EDD75D9C12D4317046F8EEA72F1818828E090B27BC756FC`、`39C44CE601C2D766CC7CF28845E39AFCEEF70CCC6FA5944B8518EEC7CA5F7C7B`。

修前程序 `before-bin/ely_files.exe` 的SHA为 `12FA31D7364E6BD5E872599CF2DDD7F58F63730C4C41B437AF7EF2DDEA1476EB`；编译事件确认fresh=false，原日志为1分19秒。实际原始输出为0过8败、0ignored，completed、exit/root101，无SKIP、guard或强制回收。前五项验证同时间覆盖、回拨排序/保留及孤立metadata问题，后三项验证新增协作锁契约，不能冒称此前已有锁协议遭破坏。修前在首个失败断言后未运行的后续参数分支没有计通过。

## 本次实际回归与构建

证据根目录：`E:/ElyOfficeBuild/qa-round21`。最终清单 `artifacts.json` 在本次读取时reviewed=false，待主任务归档正式报告。

独立读取五组结果、原始输出和测试程序，按每条test起始到最终状态解析，包括Office多行RECOVERY_BOUNDARY输出；逐名与清单一致，按程序/名称去重为387项。

| 组 | 通过数 | 程序报告耗时 |
| --- | ---: | ---: |
| Office全库 | 33 | 0.99秒 |
| Files全库 | 130 | 6.72秒 |
| Doc应用 | 64 | 210.72秒 |
| Table应用 | 68 | 176.74秒 |
| Show应用 | 92 | 269.08秒 |

五组均completed、exit/root0、0失败，无SKIP、guard中止或强制回收，峰worker不超过1；最低可用提交14952MiB、物理7603MiB，原2GiB/1GiB保护保持。Files仅准确两个旧ignored辅助入口 `key::scope::tests::invalid_profile_child`、`key::scope::tests::profile_identity_child`，由原有父测试显式include-ignored调用，不额外计入387。其它组0ignored。八项新增已包含在Files130，Office的十个真实进程终止子场景也不另加；未冒称重跑全部LOK视图或整个工作区。

修后测试构建原日志2分57秒，优化构建3分34秒；三应用均实际fresh=false、build-finished成功。accepted-bin冻结程序SHA为Doc `71D8F6BF98835A99CC11B47FA16FBB3427D9A60386FE4DC484D2671E7C41ED5C`、Table `36C29C3AFDA48402A60576AB7FEFFA99C74368FF129A57DCF94EE50D86ED2A03`、Show `E1AB33FA2348EC57AEF579900AA0762191474844F7E520091DE71DB114AB120B`；五份测试副本与实际运行身份一致。兼容worker复用第17轮，未冒称新编译C++内核或worker。

## 原生成功证据与未修失败

- 同一冻结恢复脚本的第二次当前程序运行 `native/native-recovery-18f88066` 完成：独立完整点读取后注入所属Job终止，first exit1；同profile重开后正常exit0，两阶段Job空、无意外清理、无ERROR、峰worker1。原始日志明确恢复到work/doc-1后打开Untitled 1并绘首tile，再原生另存副本；原输入仍与模板字节相同。程序、脚本与辅助文件身份吻合。
- 独立解读committed-point.bin及recovered-copy.xlsx，两者A1完整内容为 `Round20 recovery 中文 😀`，B1公式 `1+2`、B1本格缓存值3；文件散列与native-content.json和原生记录一致。实际查看restored.png：活动未保存副本、B1公式和值可见，A1截显不能代替完整内容验证，提示明确光标/撤销历史没有恢复。
- 第20轮最终程序的 `native-r20-control/native-recovery-734ba4e8` 同脚本正常对照通过，身份为其历史冻结程序；这不能证明第20轮已经复现下述输入abort，也不能证明输入缺陷已消失。
- 当前Doc/Table/Show各一轮严格关闭，分别为3581ad8c、3d374b64、3ad154dd；独立扫描原始stderr均无ERROR/0x80070578，正常exit0、Job空、原输入不变、无guard/forced，峰worker1，程序和原冻结检查器身份一致。

必须保留的失败是 `native/native-recovery-dae7f052`：stage=edit、injected_crash=false、unexpected_cleanup=true、退出-1073740791，实际位于预定故障注入之前。failed-native.json所记结果散列与现场一致，同程序/同脚本的后续成功没有覆盖它。

现场先记录Minimize，后在固定组件de961eb的edit.rs:52以offset11切长度0字符串而abort。只读源码支持旧layout与新text/selection失配的候选路径；现有证据不足以确定全部时序、Minimize发起者或归因到存储修改，也不足以断言旧程序已复现同一崩溃。README、任务及验证文档均明确该输入崩溃未修、列为后续优先项，未将本轮结项包装成整体原生稳定验收。

## 保护与交付边界

独立重算2096份原文件无缺失，仅README、TASKS、Recovery实现和恢复测试根模块四项变化；归档报告前新增clock模块、clock专测及验证文档三份。原index散列、120项暂存、72内核补丁保持。31份历史冻结制品、worker/SAL及第20轮manifest按原散列全部吻合；六个共享debug/deps路径属于会更新的编译输出，其历史冻结副本仍在，没有用当前工作输出冒充旧程序。四份优化程序和五份测试副本当前散列也全部匹配。

本次使用传入时间值验证存储排序，原生恢复使用当前真实时间；未实施系统时钟回拨实验。固定测试密钥密文不等于原生加密Office或个人密钥库验收，普通内容/cursor仍为分文件发布；没有承诺所有元数据事务化、任意中断恢复、完整点之间全部输入、系统IME、多DPI/多屏或长期稳定。本轮未发布、安装或替换日常版本；未修输入abort必须随收尾保留。
