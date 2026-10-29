# LOCAL.R25.TEXTMODE / VALIDATE / REVIEW — 独立 R1

结论：**PASS（第25轮可选文本保留工作流及所列验证范围）**。当前候选未发现剩余可操作缺陷。该功能是显式选择的“全部按文本保留”，不是完整的类型转换差异预览、流式CSV处理或任意文档无损认证；历史第23轮归因项继续开放。

这是本轮首次正式审查，之前为同一R1预核。审查阅读实现、冻结测试、原始结果、制品及截图，独立做散列和CSV/ZIP/XML检查；未构建、运行应用/测试或修改实现。唯一写入为本报告。

## 实现核查

每份Pending的text_only默认false。import_options克隆原how，仅在开启时把完整扫描得到的全部列覆盖为Text，再用于实际导入；不改写原有普通列选择。开关自身不rescan，关闭后原Skip/Date等选择可恢复。改变编码、分隔符或引号仍走既有整文件扫描，模式标志不清除，覆盖最大列宽，包含8行预览之外后来出现的新列。

导入仍要求扫描完成且解码未broken；asked及Arc来源检查继续拒绝旧扫描结果。界面显示有效列类型，文本模式/扫描期间禁用逐列转换；列回调再次检查这两个状态，并用get_mut避免旧列宽索引越界。没有以模式开关绕过编码、引用语法或扫描门禁。中英文开关及说明键完整，诊断只记录模式布尔值，不记录字段正文。

转换通过现有Import列选项表达，未新增worker协议、过滤器格式、Cargo依赖或内核补丁。关闭模式仍采用原普通导入逻辑；保留的是CSV解析后的字段文字，不把引号和分隔符语法字符当成额外内容。

## 冻结与修前证据

五项UI/LOK测试通过既有Book、实际对话框渲染和鼠标事件接口点击Switch标签，不直接调用新模式API。当前源与有效v2冻结副本SHA-256均为 `48E4EECD50D34D0040AF4504290B53629786A90EFE0E65BFBE0788D96E9434AC`，修前/后正文相同。

canonical修前的text.rs、dialogs.rs、i18n/output.rs与原项目基线逐字节相符；测试注册模块仅增加入口，其余旧测试不变。build-byte-before.jsonl实际fresh=false、构建43.38秒成功；保存的程序SHA-256为 `FA72DC8995976B08F17838623B3CB5225567E04580A00AD76CFCF289E33AC8CC`。

原始stdout/stderr逐名核对为 **1过4败、0忽略，12.07秒**；test/root退出101，正常结束，无SKIP、guard或forced cleanup。四项均在旧UI找不到文本保留Switch时失败；普通数值导入、保存和重开正例通过。这是功能不存在的对照，不能说成修前已经产生四份损坏输出，缺失控件之后的步骤没有执行。

最初v1编译setup错误使用了不存在的WindowHandle.window_handle；v2机械替换为已有window.into后与v1逐字对应，没有更改断言，原源/错误日志排除保留。较早仅LF/CRLF不同的provisional 1过4败仍保留，但不作主证或重复计数。

## 修后回归与独立文件证据

独立解析实际stdout的逐名ok状态，并对照artifacts.json：Table完整87项通过，170.81秒；双语目录选择5项通过，0.02秒；共 **92个不同测试**。五项新增UI/LOK测试已包含Table87内。两组均completed、test/root退出0、0忽略，无SKIP、guard或forced cleanup；worker峰值不超过1。提交余量最低30664MiB、物理余量最低12456MiB，原2GiB/1GiB保护未放宽。

新增用例覆盖两编码文本模式导入→保存→关闭→实际重开，模式开关不破坏储存的手动列选择，重扫后模式保留及第11行新增列，以及普通数值导入正例。测试单元格helper会trim两端，因此不把其断言当作精确空白证据。

审查另用独立Python读取实际五份输入及XLSX，不调用办公引擎：

- UTF-8、UTF-16LE和原生UTF-8三份文本输出精确A1:I1九格，全部为shared-string，无公式或额外格。00123、20位整数、科学写法1.234567890123456789E+19、日期样2026-10-28、长小数0.12345678901234567890、字面=1+2、-0、中文emoji均逐格保持；I1精确为“  padded  ”，两端各两个空格。
- 修前和修后的普通模式均精确三格：A1数值7、B1数值1.5、C1文本=1+2，无公式；独立XML类型为n/n/s，证明普通数值行为未变。
- 所有输入字节逐份等于相应UTF-8或带BOM的UTF-16LE预期，标准CSV读出与fixture一致，文件散列与text-content.json相符。

三份文本输出SHA-256依次为 `A164A9419D448C51DE8AB7FCBE10625CCF5108EBFC340CB6D5B7B863CD00C61C`、`EBB6A30B0824D2E0419980463F6EF420EF0C4112BE9C880599A4DC9597643459`、`3E05FF4B4784040EB25FE4BF216F9AD869849A54F60BE599295A8ADF02205264`。空白完整性证据来自保存文件的实际字符串，未把trim后的显示文本扩大为重开内存中空白的额外验证。

## 构建与原生界面

测试实际重编译Table53.28秒；优化构建目标为Table，13分02秒、fresh=false，包含相关Rust依赖重编。最终Table SHA-256为 `D85C696350F0F76ED4230E1C1FFECFD07FDD25445935837267C1BD64DB653DC3`，worker仍复用第17轮 `561EEBBDA94FF1AB17EA06DAC945C739A95708AF62444A0ED2C05695B117B58D`。两个测试程序及两个运行程序冻结副本/编译来源散列相符；未声称Doc/Show应用、worker或C++内核重建。

新增151行原生脚本与执行前冻结副本一致，SHA-256 `2E9F2ACDB7A07420B72E07107F40727EC026729CB903F38D6A531F8947337A9F`。它从所属Job.Root取得窗口，只向该HWND发送点击消息，SaveAs对话框限定所属PID；坐标在专属截图范围内检查，并按实测client offset12,0转换。专属GUID目录容纳输入/输出，原件散列、资源、日志、正常退出及Job空检查保留，没有向产品加入测试钩子。

native-text-import-cb289d98实际为168DPI英文UI：预览后点击Switch，日志记录开关on；再点击Open，记录text only true，正确source.csv opened之后首图块；原生SaveAs生成保存文件，再退出0、Job空，无ERROR、无效句柄、guard或强制清理，原CSV散列不变。原始stderr、坐标记录、harness/程序身份及独立九格文件证据相符。

已亲自查看preview.png和text-mode.png：初始开关关闭，普通模式列类型可选；开启后Switch状态变化、说明可见，可见列均显示禁用的Text。截图只显示横向视口的一部分列，全部九格类型及文字由实际文件读取证明。这是单次真实UTF-8原生流程；UTF-16LE来自渲染鼠标事件与真实LOK往返测试，不能混称UTF-16原生窗口验收，也不包含一次新的原生重开流程。

## 保护与范围

3894个原文件无缺失，仅text.rs、dialogs.rs、tests/output.rs、i18n/output.rs、README和TASKS六项允许变化；当前三个新项目文件为UI测试模块、原生脚本和验证文档。六份当前C/E源对及二十份未重叠继承源对均与清单一致；本轮源码/脚本423/183/490/156/175/151行，均不超过500。94份历史制品散列不变；HEAD、原120项暂存、index散列 `35A149422418AA5EB4725BD66E21CB43EAA9572AF4F1F1C1E9E5A4EBBE780FE3` 和72个内核补丁保持。Cargo、vendor及worker源码未变。

最终artifacts.json在reviewed=false时关联的两组逐名测试、原字节修前对照、当前程序、原生结果和五份文件与实际证据相符。docs/windows-validation-round25.md明确区分精确空白与trim读法、普通数值与文本类型、原生UTF-8与测试上下文UTF-16LE，并保留类型转换差异预览、风险提示、流式处理等后续范围；历史R23归因项不因本报告关闭。

可按本轮TEXTMODE/VALIDATE/REVIEW三项归档收尾；不外推系统IME、多DPI、长期稳定性或任意复杂文档无损往返。
