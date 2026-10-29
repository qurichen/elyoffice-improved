# 第19轮独立审查 R1

结论：PASS。已定位的Windows窗口关闭后刷新登记竞态，其源码顺序、修前复现与修后证据一致；本轮限定范围内未发现遗留待修缺陷。

审查日期：2026-10-26。这是 LOCAL.R19.CLOSE 的首轮正式独立审查，之前只做同轮预读与证据预核。依据为 `agents.md`、`tasks/TASKS.md` 和本轮范围；审查者没有修改生产或测试源码，没有构建、竞争运行测试、安装或发布。本报告为审查者唯一写入。

## 源码与根因

- 原 `WM_DESTROY` 在回调后异步投递平台注销消息；VSync仍可能在登记移除前持有读锁并对已失效HWND执行 `RedrawWindow(..., RDW_INVALIDATE)`。诊断2的实际错误明确附带 `vendor/gpui-pre-windows/src/platform.rs:507`，不是仅凭错误码推断Direct Manipulation清理。
- 当前 `events.rs::handle_destroy_msg` 在拖放注销和关闭回调之前，同UI线程同步发送既有私有注销消息。接收端沿用既有写锁移除登记，返回之后才给ACK；已持读锁的刷新调用先完成，后续遍历不再包含该HWND。写锁在返回前释放，关闭回调不在该写锁内执行。
- 消息校验号和目标HWND保持；发送方未获ACK仍记录ERROR。原 `RedrawWindow().ok().log_err()` 保留，没有添加IsWindow先验过滤、屏蔽错误码或关闭日志。生产修复没有改正常输入、保存、worker协议、依赖或内核。
- 两份C/E源码及清单逐字节对应：events SHA-256 `0A0E48B3FCBE9EEBA04F2F49EFD672E7E5A1A8592AD3B23A960D300E4123A255`，platform SHA-256 `E2BA7DBE93EAEB52ED9FA677E2152E4710D72FEC940023509540FF0150307BE6`。预读后最终源码未再变化。
- 两份vendor文件原本均超过500行，本轮只改销毁函数的注销时序及平台ACK返回值，保留第三方组织，没有借此新增超长自有模块或作无关拆分。新增自有退出探针109行、严格日志检查41行，均小于500行。

## 修前与检查脚本

证据根目录：`E:/ElyOfficeBuild/qa-round19`。最终预审清单为 `artifacts.json`，本次读取时 `reviewed=false`，正式报告尚待主任务归档。

- 第18轮原Table制品仍为 `B9BC69CE4B4B82FB5F49F0C6BBB0264049235CC59FDA4E6C840760F79399EE9D`。v1探针真实三轮为2过1败；同一最终v2在 `before-native/close-probe-c2a6eb0c` 首轮复现无效句柄并判失败，各原生进程exit0、Job空、原件不变且无guard/forced。内容或退出成功没有掩盖日志失败。
- v1及v2源码均保留归档。独立在内存中将v2的 `$caseProfile` 还原成 `$profile`，完整文本SHA精确恢复v1的 `CC80CD7B9387678D4E4D4DEDF7138F7F6D7B79D232556FB8CD6927674740B1D3`；改名未放宽断言。
- 当前v2 SHA为 `8533F9271935B66D935B2B8D1DB381A824BA31E70C8D81B56775493C17473928`；严格日志检查为 `6A8F827D17C0BCB6FE8306D6E304EDD418BEADF9596BFD932B35835BAD6632E0`，现文件与v2冻结归档一致。后者的修前 `lifetime-before.json` 明确失败。
- 文档如实区分“首次修复构建前冻结”与“早于所有生产编辑”，并保留诊断构建未完成即启动的e86b8f44排除记录、78ee99ca超时强制清理失败和未修诊断版偶然无错误的记录，未改写成正式通过。

## 本轮实际运行证据

独立读取六组实际 `results.json`、stdout及stderr，逐名提取成功项，按测试程序与名称去重为230，与最终清单逐名及程序散列一致：

| 组 | 不同项 | 程序报告耗时 | 目录尾标 |
| --- | --- | --- | --- |
| Doc `tests::` | 64 | 190.94秒 | `023011-f058ebd0` |
| Table `tests::` | 68 | 163.98秒 | `023325-75b93f88` |
| Show `tests::` | 92 | 248.56秒 | `023612-38ffbd5a` |
| LOK guards | 4 | 10.64秒 | `024024-37573194` |
| 边框拖出后释放 | 1 | 4.53秒 | `024039-46d034bb` |
| 函数点击不向表格发鼠标输入 | 1 | 4.66秒 | `024045-174908b5` |

六组均completed、exit/root exit0、0失败、0忽略，无SKIP、guard中止或forced cleanup，峰worker均1。三个应用组0 filtered；三个LOK组仅为所列过滤器。最低可用提交25706MiB、物理9260MiB，既有资源守卫保持。没有复用第18轮历史通过替代本次运行，也没有重复计入窗口循环。

- 优化构建原日志为14分47秒，JSON明确三个应用fresh=false及build-finished success；测试版构建原日志6分59秒。固定accepted-bin的Doc/Table/Show SHA分别为 `05FC0DD9B77F3131CB68A6F190BCD6E0E6142C249CD0ACCF35723B390A919EFD`、`83EDDA40C291364147C5E085CCE0F4EF7743688CC54ABF5184230C497739FA5A`、`0C13034E7FE29AB1D14F6039DFE3EAD7B3C4F182AEC4F2E2CA04CB12701BE008`。
- 正式退出探针Table6轮、Doc2轮、Show2轮，共10轮逐条completed/passed、正常exit0、Job空、原件不变、峰worker1、无guard/forced。独立扫描每轮原始stderr，均无ERROR或0x80070578，程序/输入/检查脚本身份与清单相符。实际DPI为168。
- Table `accepted-native/native-offices-a060fa7b` 与Show `accepted-native/native-offices-b7820b7e` 的保存、取消后普通保存保留内容、拒绝覆盖、指定ODS/ODP和指定文件重开字段均通过；四阶段正常退出、Job空、无强制清理。独立扫描原始stderr均无ERROR/无效句柄，日志散列和四份辅助文件身份与严格检查记录一致，重开日志明确打开explicit.ods/explicit.odp。
- 独立解读Table的ODS得到A1 `Round13 中文表格 😀`、A2 `Cancel keeps this draft 中文`，B1公式 `of:=1+2` 及本格缓存值3；Show最终PPTX/ODP均为3页，两应用guard与输入字节相同。没有只凭完成标志或其他单元格/全文件字符串认定内容通过。

## 原文件与制品保护

独立重算清单14份制品均匹配，包含正式与修前/诊断程序、四份当前测试程序、worker及SAL。复用release worker仍为 `561EEBBDA94FF1AB17EA06DAC945C739A95708AF62444A0ED2C05695B117B58D`；debug worker、SAL和72内核补丁保持。未把复用worker写成本轮重编译。

独立重算2083份起始文件，无缺失，仅README、TASKS和两份获准vendor源码变化；新入库范围为验证文档和两份脚本。原index SHA及120项暂存保持。E盘临时Direct Manipulation、window及logging诊断已恢复到E盘原版，当前events/platform仅对应两份真实修复；C盘这些非目标源码未变化。

文档和任务状态与实际证据吻合。PASS范围仅为本次已定位竞态及列出的回归；10轮窗口与四阶段原生通过不构成所有句柄错误、全部硬件手势、多DPI/多屏、系统IME、完整视觉兼容或长期稳定认证。本轮未发布、安装或替换日常版本。
