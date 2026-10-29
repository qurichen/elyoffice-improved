# LOCAL.R22.INPUT / VALIDATE / REVIEW — 独立 R1

结论：**PASS**。当前候选中未发现剩余、可操作的本轮缺陷。结论限于确定性的 TextInput 文本/旧布局失配修复、所列回归与 Windows 原生流程；不代表已经定位历史 Minimize 发起者、复演第21轮现场的全部时序，或完成整体长期稳定性验收。

本报告是第22轮首次正式审查。此前同一 R1 的静态预核发现全局拖动监听遗漏 Option 消费者，候选已修；两个 Markdown 消费者也已接入无命中处理。汇总器的 should-panic 标签解析已修，原测试断言未改。审查仅阅读源码、冻结测试、原始日志和制品，并做只读散列/结构检查；没有构建、运行测试或修改实现。

## 实现核查

Layout 在实际排版时记录原文与 masked 状态；current_layout 同时核对二者。offset_for_point 只有在当前文本与该布局匹配时才返回字节位置，IME 字符命中再对同一当前文本换算 UTF-16。清空、缩短或 Unicode 替换后不再把旧字节位置送入新文本切片，也没有以截断旧偏移或默认位置遮盖失效。

左键、右键、普通拖动和全局拖动都拒绝无命中；无当前布局时垂直定位保留当前 caret。Markdown 跨字段位置查询用 ? 传播无命中，显式指针的 point_open 在失效时返回；无指针的既有键盘调用仍取字段末尾。七个生产文件的相应调用路径已核对，无遗漏的旧返回类型消费者。

旧布局的 bounds 路径仍保留，避免把预编辑后的下一帧前候选定位整体清空。既有 forms::tests::stale::an_ime_finds_its_composition_before_the_next_layout 实际通过；这不等于真实系统 IME 已验收。

## 冻结与修前证据

两份当前测试源码及 .work/round22/frozen 归档散列一致：

- tests/mod.rs：`9FAAB258D3A257E2869935599A425F46F937456102E746B376E018F1FA6F033F`
- tests/relayout.rs：`A836F7DDF909A3703068D22282875AF13B68D1940D6563795A3D51F6E12E93C5`

build-before-v4.jsonl 的组件测试程序实际 fresh=false，构建成功，stderr 记录10分17秒。runs-before.json 关联的原始 stdout 为 **1通过、7失败、0忽略**，test/root 退出101，正常结束，无 SKIP、资源守卫中止或强制清理。保留的 before-bin/ely_gpui_component.exe 实算 SHA-256 为 `C334CE29925FE5F2FEBAAA98186E56F2462888027A55D29B143BC06676E0FB59`。

七个负例分别涉及外部清空、旧 ASCII 偏移落入新 Unicode、键入替换、预编辑替换，以及清空后的左键、右键和三击。重绘后 Unicode 命中正控制修前已通过。失败断言之后的按键不算已执行；没有把这些测试说成原生 Minimize 时序复演。

## 修后回归与构建

独立解析十组原始 stdout，兼容 test 名称后的 ` - should panic` 和 Office 子进程标记跨行，逐名对照 artifacts.json：**623个不同测试通过**。组件236个名称还逐一等于修前程序列出的固定选择清单，没有重复叠加八项新增测试或父测试里的子场景。

| 组 | 通过 | 实际耗时 |
| --- | ---: | ---: |
| forms | 145 | 1.98秒 |
| Markdown blocks | 45 | 4.07秒 |
| Markdown point | 25 | 0.25秒 |
| Markdown secondary | 10 | 0.15秒 |
| Markdown presses | 11 | 0.09秒 |
| Office | 33 | 1.32秒 |
| Files | 130 | 7.76秒 |
| Doc | 64 | 245.91秒 |
| Table | 68 | 210.00秒 |
| Show | 92 | 322.76秒 |

全部组 completed、test/root退出0、无失败、SKIP、guard 或 forced cleanup，worker峰值不超过1。Files 仅有准确两名原有 ignored 辅助入口 invalid_profile_child/profile_identity_child，由父测试显式调用，不计作额外通过；其余组0忽略。forms 三个既有预期 panic 控制仍保留在145项内。提交余量最低13925MiB、物理余量最低8292MiB，未放宽原2GiB/1GiB保护。

修后测试构建16分35秒；组件程序含2155项可列测试，但本轮仅运行上述236项组件选择。默认编译的上游示例没有当作已运行或视觉验收。优化构建18分51秒成功，三应用均实际 fresh=false；四份运行程序、六份冻结测试程序及对应编译输出的散列相符。

- Doc：`509DB5C6904B54B45B48B41E7E93B1BAE2D1BBBB256F9028C02DA95E05DE89C2`
- Table：`A8E595E417297EC76363EB1C32BAD7F3A29EDAE875C0F396B28055FD8B3C3773`
- Show：`42076220F3B30156125CC5A8A611BA58AEA59FB7509059302588C3C09848B34E`
- 复用第17轮 worker：`561EEBBDA94FF1AB17EA06DAC945C739A95708AF62444A0ED2C05695B117B58D`

## 原生证据

native/native-recovery-ce0423a8 使用同一冻结原生恢复脚本（SHA-256 `B7AB7930C566D4B4D57F5C9054F743A05ED5C8E2D19D7ADFFD7657224694B049`）。结果记录先核对完整恢复点，再终止所属 Job，首次退出1且 Job空；重启读取 Crashed、实际恢复副本、opened Untitled 1 为 tab1 后首图块，再原生另存，正常退出0且 Job空。当前 Table/worker 身份匹配，原输入逐字节不变，无意外清理。first/reopen 原始 stderr 独立扫描均0 ERROR、0x80070578为0。

独立读取实际 ZIP/XML，而非仅信汇总：committed-point.bin 与 recovered-copy.xlsx 的完整 A1 均为 `Round20 recovery 中文 😀`，B1 自身公式为 `1+2`、本格缓存值为3；两份文件散列分别为 `94C5E689BD6F2734CB89F45E1670A58004A7F236E7C10CC00EC8732EF9E85A96`、`42317BC2DED220CA9AE5052DEE88C03BCAE2A2695221D72ADE8D5596AB46DDA6`，与清单一致。已亲自查看 restored.png：活动 Untitled 1 有未保存标记，B1公式栏与格值可见；A1列宽截显，完整 Unicode 由文件读取确认。提示明确光标和撤销历史未恢复。

严格关闭 Doc066a6b81、Table5ef027e7、Show06f0fc36 各一轮通过；原始日志有 opened/首图块，程序散列匹配当前优化版，退出0、Job空、原输入不变，无 ERROR、无效句柄、guard 或 forced cleanup。探针 SHA-256 仍为 `8533F9271935B66D935B2B8D1DB381A824BA31E70C8D81B56775493C17473928`，未弱化日志判据。

第21轮 native-recovery-dae7f052 原失败结果散列仍为 `43E48738F352E2328B2888DCDB8A89EDBD0F274286786EB77C4FDCB7C001BA3B`：edit阶段、尚未故障注入、退出-1073740791、意外清理。它继续是历史 FAIL，本轮当前正常流程没有将其覆盖。确定性修复覆盖观察到的旧布局/新文本越界机制；历史 Minimize 来源和完整原生现场因果仍未证明。

## 来源、范围与保护

1779份上游跟踪文件均存在且 Git 可见，固定提交 de961eb164c1c88797ea35f0860aba9d18c95e4f 的 MIT 与资产许可保留。与独立 vendor 基线比较，准确只有七个生产文件和 tests/mod.rs 八个旧文件改变；新增 relayout.rs、来源说明及296行 portable patch。另将八份原文件与固定源码缓存逐字节核对，再以内存重建差异，确认 patch 与当前源码完全相符，SHA-256 `0062E49F1D07411B5CDE23FAA8A4E69A64F1355D5F130464407866BAFE4AF68D`。未修改缓存/reference。

根 Git pin 保持，经原来源 patch 接入 vendor，组件作为工作区测试成员。独立解析新旧 Cargo.lock，934个包的集合/版本完全相同；只有组件来源删除及准确七条已声明 dev 依赖边加入，其他包记录相同。重建的原 lock 散列 `D1367000B08DA12A1BB4C79FE1CD6664AB035D855E6748004175D23735F654D2` 等于本轮基线，当前 lock 为 `F9994B9D3438920E37E0FAC99119DFD46E964225FF301D2E07AE85E3EB95BE4B`。

2100原项目文件无缺失，仅 agents.md、Cargo.toml、Cargo.lock、README.md、tasks/TASKS.md 五项允许变化；当前1783个新项目文件为1779份上游导入加四份本轮新增。11份 C/E 源码对均匹配清单；全部修改/新增 Rust 文件不超过500行。42份历史制品实算原散列不变；原120项暂存、index散列 `35A149422418AA5EB4725BD66E21CB43EAA9572AF4F1F1C1E9E5A4EBBE780FE3` 与72个 LibreOffice 补丁保持。没有新 C++、worker、IPC或依赖版本修改。

以上与 E:/ElyOfficeBuild/qa-round22/artifacts.json（审查时 reviewed=false）、原始结果及 docs/windows-validation-round22.md 对应。边界说明充分：合成 Unicode 输入不是系统 IME，原生流程为明文、DPI168，未验收全部组件/全工作区、多屏多DPI、原生加密、长期使用或完整文档排版。三项 R22 任务在出报告前仍未勾选。本轮可按上述范围归档收尾。
