# 第15轮：Windows 长资料目录启动修复

2026-10-24，接续上游整合后的71补丁版本。没有迁移真实资料、使用隐蔽浅路径
回退、修改系统长路径开关、提交/发布或替换已安装版本。

## 根因

Windows SAL 的 `osl_openFileDirectory` 已保证目录末尾有反斜杠，但
`OpenDirectory` 的 `endsWith` 判断取反，随后又加一个反斜杠和 `*.*`。
普通 Win32 路径会规范化双分隔符，`\\?\` 长路径形式则不做同样处理。
资料目录自身未到260字符时，它的内部子目录也可能提前跨过转换边界。

原生调试捕获到目录打开的 `InteractiveAugmentedIOException`、包装后的运行时
异常及异常终止，不再把 `0xc0000409` 推断为已证实的 CRT 参数错误。
同一底层探针中，258/300字符目录转换为长路径后：单分隔符原生读取成功，
双分隔符报 Windows 123，旧 SAL 枚举报2；新库相同 SAL 枚举恢复为0。

`0072-windows-directory-mask-separator.patch` 只改正这一处条件，已有分隔符时
追加 `*.*`，没有时追加 `\*.*`。影响范围是捆绑内核的 Windows SAL 普通
文件系统目录枚举，不仅限于 LOK；不把它描述成 Windows 系统级修复。
旧71个补丁保持不变。实际重编译该C++文件并链接新的 `sal3.dll`，构建exit0。

## 验证与证据

- 修前新71版矩阵：80/180字符正常，240/300字符及长profile+长TEMP失败；
  短profile+长Windows TEMP/TMP正常。9组均无强制清理，失败不计通过。
- 修后九组矩阵全部完成启动、打开合成文档、绘制、保存与退出，exit0，
  无强制清理。Worker会将内核TMPDIR设为profile/tmp，不能据此声称所有
  内核临时目录路由或任意长TEMP都已验收。
- 新增1项Windows回归覆盖180/240/300/520字符及中文、空格、百分号、emoji
  的320字符资料路径；均为UTF-16单位。逐个启动，在指定深目录保存后关闭、
  重开该文件，读出 `Hello 世界` 并请求图块，正常退出；没有转移指定目录。
- 底层探针的四个对照目录修后SAL均返回0；原生双分隔符负对照仍报123，
  证明不是系统设置变动让错误输入也被接受。
- 三应用在新库上串行完整回归：Doc64、Table54、Show92全过；连同新增
  长资料路径1项、旧高缩放图块3项，共214项通过。没有跳过、资源守卫中止
  或强制清理；三应用与这两组专项的worker峰值均为1。
- 优化版Table/Show的原生默认后缀、取消后继续保存、拒绝覆盖、指定格式
  保存以及新profile指定文件重开全部通过；原输入保留，首开/重开exit0，
  所属Job均空。不是系统输入法验收，也不是本轮Doc原生恢复测试。
- 独立R1源码复核和最终R2运行/制品复核均通过；记录见
  `tasks/reviews/local-round15-path-r1.md`、`local-round15-path-r2.md`。

本机证据根目录为 `E:/ElyOfficeBuild/qa-round15`。修前矩阵
`after-upstream71-f19c30bc/results.json`、修后 `after72-791b1ef0/results.json`、
`sal-before.log` / `sal-after72.log`；新用例日志在
`ely-trial-round15-long-20261024-145111-43050351`。C++构建为
`E:/ElyOfficeBuild/logs/kernel-build-20261024-144605.log`。
完整应用结果在 `ely-trial-round15-ely-*`，图块在
`ely-trial-round15-tiles-20261024-150329-203a60dd`；原生为
`native-offices-b8b57089/results.json`。`artifacts72.json` 记录新旧sal3.dll、
配套worker/应用/测试及源码散列，并确认旧71补丁、120项暂存及index未变。

早期直接调试没有匹配应用的unipoll参数，发生一次超时强制清理；该记录
仅用于诊断，绝不作为回归通过。调试器自身完成也不等于目标worker成功。

## 边界

这是本机合成文件和有限路径长度的验证，不是32767单位、超长单个路径组件、
UNC网络目录、权限受限位置、重解析点、真实系统输入法、全DPI或长期稳定认证。
成功请求图块不等于复杂文档视觉保真。根 AGPL、上游声明、原暂存状态保持。
