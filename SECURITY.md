# 安全策略

## 支持版本

当前维护版本：`0.1.x`。

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
- 仅运行用户明确选择的本地 `.exe` / `.msi`，或通过在线验签的安装包。
- 注册表产品识别仅接受指定 key 或精确 DisplayName。
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
cargo test --all-targets                       9 passed, 0 failed
cargo clippy --all-targets -- -D warnings      passed
cargo audit                                    0 vulnerabilities
```

Cargo 仍会报告 Tauri 跨平台 lockfile 中 GTK 3 和 `unic-*` 的上游维护警告。这些依赖不进入 Windows 目标产物，但应随 Tauri 版本升级持续跟踪。

## 发布签名状态

`v0.1.0` 的 Windows 文件具有已记录的 SHA-256，但尚未进行 Authenticode 签名，适合测试、内网或受控交付。公开下载时 Windows SmartScreen 可能显示未知发布者；后续正式发行应配置 Authenticode 代码签名。
