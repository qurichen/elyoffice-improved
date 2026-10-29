# LOCAL.R23.DELIMITER / VALIDATE / REVIEW — 独立 R1

结论：**PASS（仅上述三项的本轮 CSV 实现、所列验证与保护范围）**。当前候选未发现剩余可操作缺陷。首次原生恢复失败仍为 FAIL，原因尚未确定；**LOCAL.R23.REMAIN 必须保持开放**，本结论不表示恢复/日志问题已修复或长期稳定性已通过。

这是第23轮首次正式审查；此前均为同一 R1 的预核。审查仅阅读实现、冻结测试、原始运行结果和制品，并做独立只读文件解析及散列核对；未构建、运行应用/测试或修改实现。唯一写入为本报告。

## 实现与测试契约

原实现直接计首个物理字节行，引用字段内标点、引用内换行、UTF-8 BOM和多字节编码均可能改变默认分隔符。当前 Pending 先决定编码，再用既有严格 decode 解码；推断单次遍历首个逻辑记录，在候选字段边界及其前导空白后识别引用，跳过引用内部标点/换行，并处理成对双引号。非引用字段中间的字面引号不改变记录边界。TSV/TAB扩展名、候选优先顺序、空文件/单字段默认逗号保持；解码失败仍由既有扫描标记 broken，不能无声替换字符后导入。

预核发现的第一版逐候选调用 split 比列数，会将 Name,"a;b;c" 等非首字段的引用内部标点当作另一候选分隔符；该版未交付。当前改为引用区间外计数，并新增独立冻结补充用例，原两份冻结测试正文未变。检查了空白、转义、引用内CR/LF、未引用字面引号、制表引用和后续字段路径，未发现本范围的新错误。推断扫描是线性遍历，没有逐候选构造整行字段；完整解码及读取仍在后台，不据此宣称超大文件流式处理或运行性能已有量化提升。

既有预览 split、整文件列扫描和 is_code 文本保护未改。两个真实 Book/LOK 用例先校验分隔符、预览三列和列类型，再实际导入并读六格，另存XLSX、同步移除旧tab后重新打开文件，再读六格；不是只重复读取原Entity或仅搜索文件中的残留字符串。

## 冻结修前证据

三份当前测试及 .work/round23/frozen 归档散列一致：

- text/delimiter_tests.rs：`011CC8AB6B642BBF3C0FA649C3B3417E1A5D2C2893A3534148467E4736E90B0E`
- text/delimiter_fields_tests.rs：`2547709D9261D845BB0355DF43737DAAE764F6175DD2AC259A65937F57E55944`
- tests/output/delimiter.rs：`62C72D03450006545FB3C6B0F3567D828AC3938DAF24C3D1DD96D93362A8436B`

text-before.rs 等于原项目基线；两份修前构建源码去除新增测试模块入口后逐字等于它。build-before.jsonl 和 build-before-fields.jsonl 均实际 fresh=false、构建成功，原始stderr分别记5分54秒、1分02秒。修前程序分别保存为 SHA-256 `0214EDE859D3167ACDAF5515FEC4C2D8194156D9338FC0B67AFC9E85A58CC8D5`、`F35F0A526E60D016C364ACB27032F917DE8024E74BB10A6B0A17E586608A5973`；副本与运行身份相符。

三份原始stdout逐名核对：原纯测试1过8败、Book/LOK用例0过2败、补充字段测试0过3败，总计14个测试1过13败。各进程完整结束、test/root退出101，无SKIP、guard或forced cleanup。两个Book/LOK用例修前停在默认分隔符断言，后续导入/保存/重开尚未执行；补充第三项在引用制表数据断言失败。失败断言之后的分支不算已验证，不能把修前结果表述为已观察到保存文件损坏。

## 修后结果与实际数据

独立解析原始stdout中的逐名状态，包括Office子进程标记跨行，对照最终 artifacts.json：**五组401个不同测试通过**，含12项新增纯测试与2项新增Book/LOK测试，不重复叠加选测或第22轮623项结果。

| 组 | 通过 | 实际耗时 |
| --- | ---: | ---: |
| Office | 33 | 1.21秒 |
| Files | 130 | 7.37秒 |
| Doc | 64 | 221.66秒 |
| Table | 82 | 201.81秒 |
| Show | 92 | 294.36秒 |

五组均completed、test/root退出0、0失败，无SKIP、guard或forced cleanup，worker峰值不超过1。Files准确两名既有ignored子进程辅助入口 invalid_profile_child/profile_identity_child，由父测试显式调用，不额外计数；其他组0忽略。提交余量最低13342MiB、物理余量最低8306MiB，原2GiB/1GiB守卫保留。

独立用Python标准库读取 import-evidence 中两个实际输出，未调用办公引擎：UTF-8与UTF-16LE输入字节均精确等于fixture，标准CSV读出两行三列。两份XLSX精确含A1:C2六格，全部为shared-string，无公式或额外格；A1“姓名,a,b”、A2“行😀,c,d”、B1/B2“00123”/“00456”、C1/C2两组20位整数完整。保存文件散列为 `6485EE2D61888E3E1E290A956B403C3FC66C57641F356BB7C8A2BF538FE67C9D`、`FBB71789240B859A81F8F0AD9A9E02B7EDA92DEC00DFB126612013FDC5D9FCBC`，与 import-content.json 相符。文本类型与完整数位均有文件证据，不仅是显示文字碰巧相同。

## 构建、原生结果与未关闭问题

Table测试程序实际重编译48.13秒；优化构建1分40秒成功，仅Table实际 fresh=false，Doc/Show为 fresh=true，复用第22轮同散列程序。Table优化版 SHA-256为 `ABE6AC7D4C62F4C9A90C561812318ECA6B1531BBF1A9FEAF223E26C36A5D318A`；worker继续复用第17轮 `561EEBBDA94FF1AB17EA06DAC945C739A95708AF62444A0ED2C05695B117B58D`。四个运行副本、五个测试副本及当前编译输出散列相符；未冒称三应用或内核均重新编译。

首次 native-recovery-4a21b7df 结果散列 `506CE82C9BF0D7B908204C35BCCE2CDF580ADB7EF82AB9FA540447AFC09FE6D8` 保持。它已生成完整恢复点并注入所属Job终止，首次退出1；重启在restore阶段触发“Owned log was unexpectedly truncated”，unexpected_cleanup=true、重启进程被清理退出1，故仍为 **FAIL**。两Job最终空、原输入与已提交恢复点保持、两份stderr无ERROR，这些事实不把失败变为通过。现有约8KB日志及append/8MiB裁剪实现不足以确定瞬间截短、读取竞态或其他根因；本轮CSV改动也不是恢复/日志修复。REMAIN开放是准确处置。

同一程序、同一冻结脚本仅在新隔离profile复验一次，native-recovery-d3765df4 完整通过：完整点后故障注入退出1/Job空，真实恢复未保存副本、首图块、原生另存，正常退出0/Job空，原输入不变，无意外清理。脚本仍为 `B7AB7930C566D4B4D57F5C9054F743A05ED5C8E2D19D7ADFFD7657224694B049`，未通过改脚本/阈值生成成功。

独立ZIP/XML读取该成功流程的 committed-point.bin 与 recovered-copy.xlsx，A1完整为“Round20 recovery 中文 😀”，B1自身公式“1+2”、本格缓存值3；散列分别为 `2043C0C2496A94BDB6FC5C2A7DBAA248CA0A5B5D094EDA2BF4078D72F3DA76DF`、`A3FBEA52526D2F0B05096B5CC9892DAE8912CB03BE287751747E6ACEAE32B8EE`，与清单一致。已亲自查看 restored.png：活动Untitled 1带未保存标记、B1公式栏与3可见；A1列宽截显，完整Unicode靠文件读取确认，提示仍说明光标/撤销历史未恢复。

Doc fc7ae6f0、Table38278f65、Show4cdb41d9 三个严格关闭各一轮通过，当前程序身份、原件散列、opened后首图块、退出0、Job空均相符；原始stderr独立扫描为0 ERROR、0无效句柄，无guard或forced cleanup。成功恢复及关闭证据不覆盖首次失败，不足以宣称恢复长期稳定；系统IME、原生加密、多屏多DPI等也未由此验收。

## 源码、制品及范围保护

3884个原文件完整保留，仅text.rs、tests/output.rs、README.md及tasks/TASKS.md四个允许项变化；当前四个新项目文件为三份冻结测试及本轮验证文档。五份本轮C/E源码对和继承第22轮十一份源码对均匹配清单；本轮五份Rust文件为412/489/68/27/61行，均不超过500。整个vendor、Cargo依赖、worker/协议及72个LibreOffice补丁保持基线，没有扩大到类型转换引擎或组件修改。

54份历史冻结制品散列实算不变；历史记录中指向共享编译输出的来源路径不冒充永久冻结副本。原120项暂存、HEAD及index散列 `35A149422418AA5EB4725BD66E21CB43EAA9572AF4F1F1C1E9E5A4EBBE780FE3` 均保持。最终清单 reviewed=false 时的五组逐名结果、源码、九份当前程序、修前/失败原生结果及独立内容报告均与实际文件对应。

docs/windows-validation-round23.md 已充分区分CSV测试窗口/实际引擎流程、普通原生恢复/关闭、首次未归因FAIL及单次复验PASS。推断仍是需要用户检查预览的启发式；严格禁止类型转换、转换差异、超大CSV流式处理/查询、系统输入法与长期试用等没有写成已完成。可归档本轮 DELIMITER/VALIDATE/REVIEW 三项；**LOCAL.R23.REMAIN 继续未勾选，不因本报告PASS而关闭**。
