# Vigils Maintenance 0.2.0 交付记录

交付日期：2026-07-22

目标平台：Windows x64、Linux x86_64、macOS Apple Silicon

## 版本内容

- 将维护流程从 Windows 扩展到 Windows、macOS 和 Linux。
- Windows 保留注册表精确产品匹配、NSIS/MSI 安装和 executable path 精确进程终止。
- macOS 支持检测标准 app bundle、从 DMG 原子安装到当前用户 Applications 目录及安全卸载。
- Linux 支持检测和维护 deb、rpm、AppImage 安装；AppImage 按当前用户安装并创建 desktop entry。
- 在线更新按当前平台限制安装包格式，继续强制 HTTPS、大小限制、SHA-256 和 minisign 验签。
- 新增三平台 CI 和自动构建 GitHub Release 的 workflow。

## 发布资产

GitHub Release：[v0.2.0](https://github.com/lvusyy/vigils-maintenance/releases/tag/v0.2.0)

Release 包含 Windows NSIS、Linux deb/AppImage、macOS DMG 和 `SHA256SUMS.txt`。文件名、大小与哈希以 Release 页面及校验文件为准；发布完成后的复核结果会补充到本记录。

## 验证结果

- Windows、Linux、macOS 的 `cargo fmt`、11 项测试和 Clippy 均通过 GitHub Actions。
- Windows 编译机的 11 项测试和 Clippy `-D warnings` 通过。
- `cargo audit` 未发现 vulnerability；存在来自 Tauri/GTK 3 和 `unic-*` 依赖链的维护或 unsound 警告，已记录并持续跟踪。
- Windows、Linux 和 macOS 原生 bundle 由对应系统的 GitHub-hosted runner 构建。

## 签名状态

本版本未配置 Windows Authenticode 或 Apple Developer ID/notarization。公开 Release 用于功能验证和受控交付；用户应仅从本仓库下载并核对 `SHA256SUMS.txt`。面向不受控环境分发前需要完成平台代码签名。
