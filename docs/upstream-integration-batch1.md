# 上游更新整合：第一批

本批选择性吸收 ElyOffice 办公核心修复，保留改进版已有的保存安全、恢复、
OOXML 兼容与 Windows 高缩放修复。不是把整个上游 0.1.11 覆盖到本项目。
构建与测试在 E 盘隔离副本完成，验证后已回迁主项目源码；没有安装、提交或发布。
整合始于2026-10-23，运行验收于2026-10-24继续。

Selective upstream backports for the Windows office core, retaining this
fork's safety and compatibility work. This is not a full upstream0.1.11
upgrade. No installed application or published release is replaced.

## 来源与去重

- 上游：[ELY-Open-Source/ElyOffice](https://github.com/ELY-Open-Source/ElyOffice)。
- 核对快照：[d0e6e638](https://github.com/ELY-Open-Source/ElyOffice/commit/d0e6e63834e6781d27436ba0d4255810bb14e443)。
- 本批源文件主要取自 [c564d4b](https://github.com/ELY-Open-Source/ElyOffice/commit/c564d4b85ad2a338e1e234eb078b2b80975fc2ab)，
  另吸收 [7e265bd](https://github.com/ELY-Open-Source/ElyOffice/commit/7e265bdd77be3d35a680b9873dbb5f9436f66c04) 的64位表格坐标修正，
  以及 [024be88](https://github.com/ELY-Open-Source/ElyOffice/commit/024be88fc7cff3b75f716d20282a78352a643b17) 的 Windows 焦点补丁。
- 按共同基线与当前工作区逐文件三方合并，不以旧 HEAD 替代本机十多轮修改。
- 英语词典白名单、保存失败测试注入、表格旧菜单刷新、演示路径测试已有
  等效修复，保留本地写法，不重复实现。测试换行冲突按内容读取与导出格式
  分别处理，不删掉内容断言来让测试通过。
- 保留根 AGPL 许可；引入部分的上游 MIT 声明见
  `LICENSES/elyoffice-upstream-MIT.txt`。不改变第三方引擎许可。

## 吸收内容

| 范围 | 本批改动 |
| --- | --- |
| Windows 粘贴 | 缺少 HTML 剪贴板格式时按“没有此格式”处理，不阻断普通文字读取 |
| 公式补全 | Enter/Tab按输入顺序处理所选候选，等待工作进程时不重新显示旧候选 |
| 输入辅助 | 公式和数据验证提示层阻止鼠标误点下方单元格，选区高亮保留文字可读性 |
| 筛选 | 无明确标题的整列筛选不再触发异步标题询问；覆盖保存后重开场景 |
| 字体 | 根据字体实际字形覆盖中西文，保留不适用于中文的字体选择边界 |
| 表格 | 单行自动行高、边缘拖选滚动、滚动范围与64位坐标修正 |
| 母版 | 母版视图读取真实母版信息，不误用普通幻灯片信息 |
| Windows 焦点 | 隐藏内核窗口的内部焦点检查不再依赖实际系统前台焦点 |
| 更新来源 | 保留现有更新机制，但发布API指向 qurichen/elyoffice-improved，避免原版覆盖本分支 |

原补丁 `0001–0064` 在主树中逐字节保持不变。上游 `0052–0057` 在本分支
追加为 `0065–0070`，解决同编号不同功能的冲突；`0071` 单独追加缺失的
`doc.hxx` 引用，不改写已经构建过的 `0049`。这是71个内核补丁，不是71项
全部独立的新功能。客户端/工作进程通信同时更新，必须使用本批配套制品。

组件仍钉在 `de961eb`，没有全量更新依赖锁文件；锁文件只增加已经锁定的
`clipboard-win` 直接依赖。上游新版滚动条与 `guard_close` 依赖新版组件库，
连同滚动条专用测试一起延后，本批不复制组件库实现来替代。

## 验证

| 验证范围 | 结果 |
| --- | --- |
| C++内核 | 识别64/71前缀后追加7补丁，实际重编译Calc/Writer/Impress/VCL并链接；exit0 |
| Rust制品 | Doc/Table/Show优化版、配套worker、核心和界面测试编译通过；使用现有依赖锁与缓存 |
| 核心专项 | 54项通过：字体、筛选、公式、母版、选区计数、行高、无障碍及旧OOXML兼容 |
| 界面专项 | 最终19项通过：公式、拖选、表格边框、主题、无障碍、自然释放、持有时退出及持锁panic回收 |
| 三应用完整测试 | Doc64、Table54、Show92，共210项通过 |
| 更新来源 | fork发布API守卫1项通过，没有执行网络更新或安装 |
| 合计 | 23组、284项；最终均无SKIP、守卫中止或强制清理；不计入历史93/97项或失败尝试 |
| 原生优化版 | Table/Show实际175%缩放下默认后缀、取消后保存、拒绝覆盖、指定ODS/ODP保存和新profile指定文件重开全部通过 |
| 文件/退出 | 原输入与拒绝覆盖目标保留；首开/重开exit0，所属Job均无残留；未测试真实系统输入法 |
| 源码对应 | 59份候选核对后57份实际回迁，book.rs与tests/data.rs内容相同保留原样；746份其他源码与构建副本一致 |
| 保护项 | 主树原64补丁逐字节不变，120项既有暂存及index散列不变；4份Python检查器与3份二进制fixture对应一致 |
| 独立复核 | R1、R2问题已修；独立最终R3 PASS，见 `tasks/reviews/local-upstream1-r3.md` |

同一内核与未改变的核心/应用制品的已通过记录可保留；每次界面测试代码
修正后均重编并重新运行相关界面组。制品与记录中包含SHA256，不把早期
1秒无重编译、缺Python或强制清理的尝试记作通过。

本机完整证据位于 `E:/ElyOfficeBuild/qa-upstream1`：
`runtime-summary.json` 对应23组实际日志；`artifacts-before.json` 与
`artifacts-after.json` 记录回迁前后源码、检查器、制品与暂存散列。
`native-offices-22392ca5/results.json` 记录原生保存与重开；截图仅检查有限
合成输入，不扩大为全部幻灯片或复杂文档的视觉验收。
内核构建日志为 `E:/ElyOfficeBuild/logs/kernel-build-20261023-233300.log`。

首轮的三组旧兼容测试因找不到Python检查器报错；在任务进程中显式加入
本机已有Python313路径后重新执行，不改系统PATH、不安装依赖、不删断言。
首轮公式界面测试触发了原有单worker数量保护，不计通过：测试后台任务
继续持有worker。测试退出时显式调用正常shutdown并回收子进程，相关用例
加共享退出守卫；新增仍被界面持有时的真实worker退出回归。独立R2又发现
强引用会阻止自然释放，以及中毒mutex会在异常展开时二次panic；最终使用
弱引用、显式回收中毒guard、在展开时保留原始错误，正常清理失败仍显式报错。
对应自然释放和持锁panic两项控制均已补验。没有提高worker
数量、降低内存保护或把强制清理记录当成通过。初次失败日志保留。

## 尚未包含与限制

- 邮件应用、设置重构、新版界面组件/滚动条、打印预设和整套更新器改版未并入。
- `0067` 增加已填单元格计数接口，但本批未更换 `Book::merge` 的界面判断，
  因此不宣称空范围合并的应用层问题已修复。
  后续[第16轮](windows-validation-round16.md)接入该计数并完成应用回归，
  不追计为本批整合时已完成的修复。
- 真实系统剪贴板的写入测试需要明确选择，本批不为了测试替换用户剪贴板。
- Windows 单机的合成文件测试不等于所有复杂文档、打印、所有DPI、输入法、
  macOS/Linux、安装包或长期多日运行均已验证。
- 第15轮的深 profile 启动崩溃仍未完成根因修复；这批整合不把它记为已修复。
  后续第15轮已修复该特定目录枚举问题，见 `docs/windows-validation-round15.md`；
  本文其余状态保留为整合批次结束时的记录，不倒算成上游整合的修复。
- 保留现有工作区改动和暂存状态，不自动提交、推送、打包或替换安装版本。
