# 第18轮独立审查 R1

结论：FAIL。存在一项需要修复的交互回归，不能将当前候选标为验收完成。

审查日期：2026-10-25。依据 `agents.md`、`tasks/TASKS.md` 的审查规则，
范围为本轮表格数据对话框、共用批次入口、必要调用与测试证据。审查者只读
源码及既有产物，没有修改实现、运行竞争测试或重建程序；本报告是唯一写入。

## [P2] 列菜单在触发按压释放前冻结来源修订，正常完成点击后无法执行

位置：`apps/ely-table/src/data/column.rs:23–39` 的 `ask_column`，尤其是第29行
在筛选条目读取后立即保存 `worker.now(doc)`，以及确认时的严格批次入口。

内核的筛选按钮在 `MouseButtonDown` 分支发出菜单请求：
`gridwin.cxx:2129/2136` → `gridwin2.cxx:145` → `LaunchAutoFilterMenu`。
LOK在 `gridwin.cxx:977` 直接返回客户端回调，跳过桌面弹窗路径的鼠标释放。
客户端 `LokView` 的按压状态仍为真；弹出对话框后，真实成对的鼠标释放仍会
经过 capture 阶段的 `on_mouse_up_out` → `on_mouse_up` 发出 `POST_MOUSE`。

如果菜单读取已经完成，随后这个正常释放会推进worker修订，使刚保存的Doc
过期。用户没有另选范围或工作表，菜单排序/筛选却会被严格检查拒绝。这里
不能通过忽略所有鼠标输入或取消修订校验解决，否则会重新放开误改目标。
应在触发按压和排队输入明确收尾后读取菜单，同时保持过期目标保护。

## 已复现的运行证据

新增正向控制：
`tests::data_context::a_filter_arrow_press_and_its_release_keep_the_menu_usable`。
该测试使用实际渲染的LokView画布，在筛选箭头发送MouseDown，等待内核菜单
request及已排队输入落地，再发送MouseUpOut；另有断言确认成对释放确实进入
视图队列，随后通过菜单执行原数据块排序。它没有直接调用 `ask_column`
来绕过触发按压，也没有取消严格状态检查。

运行目录：
`E:/ElyOfficeBuild/qa-round18/ely-trial-round18-gesture-before-ely-table-20261025-160024-cb5c7e00`。
结果为0过1败，4.70秒，exit/root exit 101，completed，峰值worker1；没有
SKIP、ignored、资源保护中止或强制清理。失败是实际数据断言：A2:A4仍为
`3/1/2`，而正常完成的原范围排序应为 `1/2/3`。

该程序已保留为 `table-gesture-before.exe`，独立重算SHA-256与结果记录相同：
`F98BF39427B181C487277D7560010067AAEA4F027988FCCE53B014F70C0E67E9`。
`tests-v3-source.json` 与 `test-source-v3` 中的构建前测试归档散列一致。
现有直接调用菜单的正向控制没有覆盖这一down/read/up交互顺序。

## 其他范围与后续复审

其余已检查的四类对话框原Doc传递、`run_on`/`run_all_in` 共用转发、列菜单
有序批次及模块拆分，未发现另一项明确源码缺陷。v2将空D2的读取修正为明确
选中后的 `SELECTED.filled==0`，保留原内容断言，缺失及解析错误显式失败，
不属于放宽测试；v1异常和同v2修前/修后记录继续保留。

当前结论不等于先前全部运行证据被否定，也不以先前通过项覆盖上述回归。
修复后须保留本报告及失败程序，运行相同v3正向控制和过期目标反向控制，
再核对最终源码、运行程序、原文件/补丁/index保护及新的完整验收记录。
下一次正式审查为R2；本轮未出具PASS或允许勾选任务。
