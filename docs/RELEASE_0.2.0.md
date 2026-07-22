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

| 文件 | 大小 | SHA-256 |
|---|---:|---|
| [`Vigils.Maintenance_0.2.0_x64-setup.exe`](https://github.com/lvusyy/vigils-maintenance/releases/download/v0.2.0/Vigils.Maintenance_0.2.0_x64-setup.exe) | 2,855,927 bytes | `bf91beff2e4768faa33b98090da8daebbd910ee79a3d0b8539f0f3542526bf33` |
| [`Vigils.Maintenance_0.2.0_amd64.deb`](https://github.com/lvusyy/vigils-maintenance/releases/download/v0.2.0/Vigils.Maintenance_0.2.0_amd64.deb) | 4,158,504 bytes | `3050999086d35733e4f80e5dbe55e1acd8bf111a5ff4dd73861585bace301be2` |
| [`Vigils.Maintenance_0.2.0_amd64.AppImage`](https://github.com/lvusyy/vigils-maintenance/releases/download/v0.2.0/Vigils.Maintenance_0.2.0_amd64.AppImage) | 78,617,080 bytes | `f7f1bc8c5a82fe1c69a4ae0f8be20b3991fb066c9cd63ec98f09479f799aa659` |
| [`Vigils.Maintenance_0.2.0_aarch64.dmg`](https://github.com/lvusyy/vigils-maintenance/releases/download/v0.2.0/Vigils.Maintenance_0.2.0_aarch64.dmg) | 3,976,967 bytes | `cdc2f890c44599b2e50ae20577bfa71f7ae4d3842ffbb13c46cff077f5366fee` |

完整校验文件：[SHA256SUMS.txt](https://github.com/lvusyy/vigils-maintenance/releases/download/v0.2.0/SHA256SUMS.txt)

## 验证结果

- Windows、Linux、macOS 的 `cargo fmt`、11 项测试和 Clippy 均通过 GitHub Actions。
- Windows 编译机的 11 项测试和 Clippy `-D warnings` 通过。
- `cargo audit` 未发现 vulnerability；存在来自 Tauri/GTK 3 和 `unic-*` 依赖链的维护或 unsound 警告，已记录并持续跟踪。
- Windows、Linux 和 macOS 原生 bundle 由对应系统的 GitHub-hosted runner 构建。
- Windows NSIS 静默安装退出码为 `0`，注册表 DisplayVersion 为 `0.2.0`；静默卸载退出码为 `0`，卸载注册项已清除。
- 从 GitHub Release 重新下载全部资产后，`sha256sum -c SHA256SUMS.txt` 的 4 项校验均为 `OK`。

## 签名状态

本版本未配置 Windows Authenticode 或 Apple Developer ID/notarization。公开 Release 用于功能验证和受控交付；用户应仅从本仓库下载并核对 `SHA256SUMS.txt`。面向不受控环境分发前需要完成平台代码签名。
