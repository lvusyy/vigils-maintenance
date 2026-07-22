# Vigils Maintenance

Vigils Maintenance 是面向 Windows、macOS 和 Linux 的独立桌面维护工具，用于安装、更新、修复和卸载 Vigils。项目采用 Rust、Tauri 2 和静态 HTML/CSS/JavaScript，不依赖原 Vigils 源码运行。

## 当前交付

- 版本：`0.2.0`
- Windows x64：NSIS 安装器
- Linux x86_64：deb 和 AppImage
- macOS Apple Silicon：DMG
- 下载：[GitHub Releases](https://github.com/lvusyy/vigils-maintenance/releases/tag/v0.2.0)
- 校验：[0.2.0 交付记录](docs/RELEASE_0.2.0.md)和 Release 中的 `SHA256SUMS.txt`

当前产物未配置 Windows Authenticode 或 Apple Developer ID 签名。操作系统可能显示未知发布者或阻止首次启动；请仅从本仓库 Release 下载并先核对 SHA-256。面向不受控环境分发前，应配置对应平台的代码签名与 notarization（公证）。

## 功能

- 检测 Vigils 安装版本、路径和运行状态。
- 从本地安装包安装 Vigils，并计算 SHA-256。Windows 支持 `.exe`、`.msi`，macOS 支持 `.dmg`，Linux 支持 `.AppImage`、`.deb`、`.rpm`。
- 通过 HTTPS 更新清单检查和安装更新。
- 使用内置公钥强制验证在线安装包的 minisign 签名。
- 更新或卸载前停止 Vigils daemon 和路径精确匹配的 Vigils 进程。
- 卸载前调用 `vigil-hub setup --all --uninstall` 还原 AI agent 接入配置。
- 可选删除当前系统本地数据目录下 `Vigil` 中的账本、模型和设置。
- 启动 Vigils、打开安装目录及修复 agent 接入配置。

## 快速使用

1. 从 [GitHub Releases](https://github.com/lvusyy/vigils-maintenance/releases/tag/v0.2.0) 下载当前系统对应的安装包并运行。
2. 打开 Vigils Maintenance，等待首页完成本机状态检测。
3. 使用“检查更新”安装在线更新，或在“本地安装包”页面选择安装文件。
4. 卸载 Vigils 前确认是否保留本机 Vigils 用户数据。

完整操作和风险说明见[用户手册](docs/USER_GUIDE.md)。

## 开发与发布

- [构建与发布说明](docs/BUILD_AND_RELEASE.md)
- [更新清单规范](docs/UPDATE_MANIFEST.md)
- [0.2.0 交付记录](docs/RELEASE_0.2.0.md)
- [0.1.0 交付记录](docs/RELEASE_0.1.0.md)
- [安全策略与审计摘要](SECURITY.md)

## 项目结构

```text
frontend/       Tauri WebView 用户界面
src/            安装检测、下载验签和维护操作
capabilities/   Tauri 权限边界
icons/          应用与安装包图标
dist/           本地构建产物；Git 忽略并通过 GitHub Releases 分发
docs/           使用、构建和发布文档
```

## 许可

本仓库当前未附带开源许可证。公开可见不代表授予复制、修改或再分发权利。
