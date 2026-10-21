# Experimental source preview / 实验性源码预览

This repository publishes the cumulative work through round 10 of a local
ElyOffice derivative. It is not an official upstream release or an installer.

本仓库发布 ElyOffice 衍生项目截至第10轮的累计改进源码，
并非原作者的官方发行版，也不包含可安装的软件包。

## Origin and licence / 来源与许可

- Upstream: https://git.zacharyzhang.com/ZacharyZhang-NY/ElyOffice
- Baseline: `0d991e1c715859abd1282981fd46989e7e2319c7` (0.1.8).
- Original copyright and AGPL-3.0-or-later are retained.
- Third-party sources and test fixtures retain their respective licences.
- 原作者署名、原许可证及第三方许可说明均予保留。

## Scope / 范围

The work adds 12 engine/build patches to the upstream 51-patch sequence,
plus Windows instance/profile fixes, safer save/backup permissions,
encrypted-recovery isolation and warnings, regression fixtures and evidence.

已完成包含63个补丁的 Windows 内核及三个办公程序构建。
93项选定客户端回归通过；并非完整工作区或全面交互验收。
排版差异、原生输入法、真实加密崩溃恢复及安装回退仍需继续验证。

See `docs/windows-validation-round10.md` for exact evidence and limits.
Earlier records describe the state of their respective rounds.

## Publication hygiene / 发布清理

The starting archive was verified against 1,333 Git blobs:
SHA-256 `636D8CB60C0D942E394310D80E7D8DB5D8DE2B9BB559D9C897DD0E3343BAED5E`.

The public import excludes three inherited Office temporary lock files,
three inherited Python bytecode cache files, and
the inherited `.gitea/workflows/release.yml`, whose runner configuration
contained machine-specific account paths. That upstream workflow is therefore
not present here, despite its mention in earlier historical documentation.
This file and the bilingual README introduction explain publication scope and
project benefits without performance or full-compatibility claims. Application source,
engine patches and test assertions are otherwise unchanged from the archive.

公开快照额外排除了三份上游遗留临时锁文件、三份 Python 字节码缓存，
以及包含个人机器路径的旧发布流程；首页新增中英双语优势与限制说明。
这些清理不改变应用代码；不得运行上游上传脚本来发布本分支。
本机配置、内部记录、凭据、构建缓存及未经检查的 Git 历史不随本次导入发布。

Pattern checks and blob verification are not a comprehensive security audit.
Existing self-signed signing fixtures are test-only, never production credentials.
The PDF fixture is `CN=Ely Test Signer, O=Ely Office`, with the public test
password `ely`. Both fixture hashes match the upstream public pinned commit;
the tests assert this identity. Never trust or reuse its key in production.
No claim of complete Microsoft Office compatibility or production readiness is made.
