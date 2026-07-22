# 安全策略

## 支持版本

当前维护版本：`0.2.x`。

## 报告漏洞

请不要在公开 Issue 中披露尚未修复的漏洞。优先使用 GitHub 仓库的 Security Advisory 私密报告功能；如果该功能不可用，请联系仓库所有者并仅提供复现所需的最小信息。

报告应包含：

- 受影响版本；
- 复现步骤和预期影响；
- 涉及的文件、命令或更新清单；
- 可行的缓解建议；
- 是否已在公开渠道披露。

不要在报告中提交真实私钥、访问 token 或用户数据。

## 安全边界

- 在线更新只接受 HTTPS，并强制使用内置公钥完成 minisign 验签。
- SHA-256 是额外完整性校验，不能替代发布者签名。
- 更新清单限制为 1 MiB，安装包限制为 1 GiB。
- 仅运行用户明确选择且格式与当前平台匹配的本地安装包，或通过在线验签的安装包。
- Windows 注册表产品识别仅接受指定 key 或精确 DisplayName。
- macOS 自动卸载只允许已识别的标准 Vigils app bundle 路径。
- Linux AppImage 自动卸载只允许已识别的当前用户标准安装路径；deb/rpm 通过系统包管理器处理。
- 进程停止要求 canonical executable path 与已检测 Vigils 文件精确匹配。
- 卸载前还原 AI agent 接入配置；还原失败时默认中止。

## 2026-07-22 审计摘要

quick tier 审计覆盖 OWASP Top 10 和 Rust 依赖供应链。审计发现并修复：

- 宽松注册表子串匹配可能选择无关产品；
- 按 `gui.exe` 映像名终止可能影响无关进程；
- `rfd` 默认 Linux backend 引入的 `RUSTSEC-2026-0194` 和 `RUSTSEC-2026-0195`；
- 在线更新允许仅使用同源清单提供的 SHA-256。

整改后验证：

```text
cargo test --all-targets                       11 passed, 0 failed
cargo clippy --all-targets -- -D warnings      passed
cargo audit                                    0 vulnerabilities
```

Cargo 仍会报告 Tauri 跨平台 lockfile 中 GTK 3 和 `unic-*` 的上游维护或 unsound 警告。GTK 3 依赖用于 Linux 目标，应随 Tauri 版本升级持续跟踪。

## 发布签名状态

`v0.2.0` 产物由 GitHub Release 提供 SHA-256，但尚未配置 Windows Authenticode 或 Apple Developer ID/notarization，适合功能验证和受控交付。面向不受控环境分发前，应完成对应平台代码签名；用户应仅从本仓库下载并核对 `SHA256SUMS.txt`。
