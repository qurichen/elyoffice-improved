# 第19轮：Windows窗口销毁与刷新列表同步

2026-10-26。按路线图阶段1，处理第17/18轮保留的关闭后 `0x80070578`
诊断。范围是已定位的窗口刷新登记竞态；使用合成文件、隔离资料目录与
所属Job，不操作真实文档、安装版、系统配置或剪贴板，不自动发布。

## 根因证据

第18轮优化版Table保留为 `qa-round19/before-bin/ely-table.exe`，SHA-256：
`B9BC69CE4B4B82FB5F49F0C6BBB0264049235CC59FDA4E6C840760F79399EE9D`。
同一72补丁内核和兼容worker运行原生保存/重开，内容与退出通过，但退出
日志仍出现无效窗口句柄。证据根目录为 `E:/ElyOfficeBuild/qa-round19`。

| 修前证据 | 结果 |
| --- | --- |
| `before-native/native-offices-296ae248` | 首轮无错误，重开退出有1条0x80070578；两阶段exit0、Job空，保存/取消/拒绝覆盖/内容/原件保护均通过 |
| 同组 `lifetime-before.json` | 新严格日志检查exit1、passed=false；内容通过不能覆盖退出日志失败 |
| `before-native/close-probe-b32ea7b1` | 原程序同冻结退出探针，3轮中前2轮通过，第3轮出现错误；每轮exit0、原件不变、Job空，无资源中止或强制清理 |
| `before-native/close-probe-c2a6eb0c` | 最终v2探针在同一原程序上首轮复现，exit1；原生exit0、Job空、原件不变，无资源中止或强制清理 |
| `diagnostic2-native/close-probe-517ddc87` | 带源文件位置的诊断程序首轮复现，定位到platform.rs:507的RedrawWindow；exit0、原件不变、峰worker1，无资源中止或强制清理 |

最初怀疑Direct Manipulation清理，E盘临时诊断给Stop、Abandon、Deactivate、
隐藏/销毁窗口、拖放注销和广播调用加标识；错误没有命中这些位置，未据此
修改手势对象。第二版只在诊断日志显示已有的 `file/line` 元数据，实际定位：

```text
ERROR 无效的窗口句柄。 (0x80070578)
source vendor\gpui-pre-windows\src\platform.rs:507
```

`begin_vsync_thread` 持有 `raw_window_handles` 的读锁，遍历并调用
`RedrawWindow(..., RDW_INVALIDATE)`。原 `WM_DESTROY` 只异步PostMessage通知
平台移除窗口，消息处理之前HWND已经失效，刷新线程仍能读到旧登记项。
这是本次运行证据与源码顺序共同支持的根因，不将所有同错误码都归为此原因。

## 生命周期修复

- `events.rs::handle_destroy_msg` 在关闭回调之前，同UI线程同步发送私有
  注销消息；平台沿用已有写锁移除登记，等待正在调用重绘的读者完成。
  同步注销返回后窗口销毁继续，之后的刷新遍历不再包含该HWND。
- 平台处理完注销消息返回明确ACK，发送方未获ACK仍记录错误；原有
  `RedrawWindow().ok().log_err()` 保留。没有按错误码过滤、增加IsWindow
  先验检查或关闭日志，也没有改变正常输入、保存和worker协议。
- 生产改动仅为两个既有vendor文件。它们原本超过500行，本轮保留第三方
  文件组织，仅作最小生命周期修改；新增自有脚本均小于500行。

原生语义依据：[WM_DESTROY](https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-destroy)、
[SendMessage](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendmessage)、
[RedrawWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-redrawwindow)。
同步消息调用在同线程直接处理；当前刷新路径只有RDW_INVALIDATE，不要求
RDW_UPDATENOW/ERASENOW的同步绘制。这里的顺序论证仅限所核对的调用与锁。

所有E盘临时诊断已在正式构建前恢复：Direct Manipulation、窗口、事件、平台
及日志格式共5份文件；随后只同步两份真实修复源码，记录于 `staged.json`。
C盘生产日志与手势代码始终未加入临时诊断。

## 回归方法与冻结

严格日志脚本和退出探针v1在实际修前程序上已执行，散列在首次修复构建前
归档。复制归档时生产修复文本已编辑，不把时间顺序写成“早于所有生产编辑”。
退出探针v2只将PowerShell自动变量名 `$profile` 改为 `$caseProfile`，断言
不变；另在正式修后运行前归档，并在同一原程序上实际复现失败。v1保留，
机械还原变量名可核对两版除改名外字节一致，不用旧版散列冒充当前脚本。

- `windows-close-lifetime.ps1` 核对原生套件的程序、worker、辅助文件散列，
  内容/退出/Job字段及两阶段全部ERROR和无效句柄；修前明确exit1。
- `windows-close-probe.ps1` 等实际打开指定文件并绘出首tile，再请求正常
  关闭；每轮独立profile、所属Job、单worker，2GiB提交/1GiB物理余量守卫。
  同时核对原件散列、进程退出、Job回收及错误日志；失败和中止不当作通过。
- 当前两份脚本散列分别为 `6A8F827D17C0BCB6FE8306D6E304EDD418BEADF9596BFD932B35835BAD6632E0`
  与 `8533F9271935B66D935B2B8D1DB381A824BA31E70C8D81B56775493C17473928`；
  退出探针v1为 `CC80CD7B9387678D4E4D4DEDF7138F7F6D7B79D232556FB8CD6927674740B1D3`。

## 不计为通过的运行

- `diagnostic-native/native-offices-e86b8f44` 启动早于诊断构建结束，不能证明
  执行了指定源码；排除其源码对应和诊断结论，保留结果不删除。
- `diagnostic2-native/native-offices-78ee99ca` 在显式另存阶段超时，所属Job
  强制清理，completed=false；不计原生验收通过，也不能单凭此认定产品根因。
- `diagnostic2-native/native-offices-49de97b2` 的两阶段内容/退出通过且无错误，
  仍是未修复源码的诊断版；间歇性错误单次未出现不能证明已经修好。

## 最终验收

正式优化构建成功，14分47秒，三个应用均实际重新编译；只复用原有兼容
worker。正式程序复制到 `accepted-bin` 后再执行验收，没有运行中覆盖程序。

| 正式制品 | SHA-256 |
| --- | --- |
| `ely-doc.exe` | `05FC0DD9B77F3131CB68A6F190BCD6E0E6142C249CD0ACCF35723B390A919EFD` |
| `ely-table.exe` | `83EDDA40C291364147C5E085CCE0F4EF7743688CC54ABF5184230C497739FA5A` |
| `ely-show.exe` | `0C13034E7FE29AB1D14F6039DFE3EAD7B3C4F182AEC4F2E2CA04CB12701BE008` |
| 复用第17轮 `ely-lok-worker.exe` | `561EEBBDA94FF1AB17EA06DAC945C739A95708AF62444A0ED2C05695B117B58D` |

| 正式真实窗口退出 | 循环数 | 证据目录 |
| --- | --- | --- |
| Table指定ODS | 6 | `accepted-probes/close-probe-eb56ca4b` |
| Doc合成DOCX副本 | 2 | `accepted-probes/close-probe-07eeb512` |
| Show合成PPTX副本 | 2 | `accepted-probes/close-probe-c7d6513b` |

共10轮全部通过，无ERROR或0x80070578；每轮正常exit0、Job空、原件不变、
峰worker1，无资源中止或强制清理，既有内存余量守卫未放宽。10轮是窗口
循环，不重复当作10项Rust应用回归；当次实际DPI为168，非多DPI验收。

Table原生保存重开 `accepted-native/native-offices-a060fa7b`、Show原生
保存重开 `accepted-native/native-offices-b7820b7e` 均完成。默认格式与内容、
取消后普通保存保留修改、拒绝覆盖、指定ODS/ODP、原件保留和指定文件重开
均通过；两应用各两阶段exit0、Job空，无强制清理。严格日志检查分别为
`lifetime-after-table.json`、`lifetime-after-show.json`，四阶段均通过。

当前源码的测试版构建成功，6分59秒，随后六组回归实际完成：

| 回归组 | 通过数 | 测试程序报告耗时 | 证据目录尾标 |
| --- | --- | --- | --- |
| Doc `tests::` | 64 | 190.94秒 | `023011-f058ebd0` |
| Table `tests::` | 68 | 163.98秒 | `023325-75b93f88` |
| Show `tests::` | 92 | 248.56秒 | `023612-38ffbd5a` |
| LOK视图资源守卫 | 4 | 10.64秒 | `024024-37573194` |
| 边框拖出视图后释放 | 1 | 4.53秒 | `024039-46d034bb` |
| 公式函数点击不向网格发鼠标输入 | 1 | 4.66秒 | `024045-174908b5` |

合计230项不同测试，均为0失败、0忽略；每组正常退出，无跳过、资源中止
或强制清理，峰worker不超过1。应用组运行全部 `tests::`，三个LOK组按所列
过滤器选择，不冒称运行整个工作区。真实窗口循环和原生保存重开另计，
不与230相加。完整目录、过滤器、测试名、程序散列见 `artifacts.json`。

最终独立R1复核通过，报告逐字归档为
[`local-round19-close-r1.md`](reviews/round19-close-r1.md)。
任务已按实际结果完成；未复用诊断版充当正式修复版，也未把较早失败的
对话框运行改写成通过。

## 保护与范围

最终清单 `E:/ElyOfficeBuild/qa-round19/artifacts.json` 重算14份制品、2份
C/E源码对和全部起始文件；2083份起始文件中只改README、TASKS及两份vendor
源码，新增两份检查脚本、本验证记录和独立审查报告。原120项暂存及index
散列保持，原72内核补丁不变。
LibreOffice内核、SAL、worker及IPC无本轮生产修改，不需重新构建C++内核。
优化版三应用与实际使用的兼容worker分开记录，不冒称本轮重新编译worker。

通过有限真实窗口循环不等于所有窗口/句柄竞态、全部硬件手势、多DPI/多屏、
系统输入法或长期稳定认证。本轮230项已在当前源码构建的测试程序上实际
运行；第18轮的230项保留为历史基线，不将历轮重叠测试相加。
