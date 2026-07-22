# Vigils Maintenance

Vigils Maintenance 是面向 Windows 的独立桌面维护工具，用于安装、更新、修复和卸载 Vigils。项目采用 Rust、Tauri 2 和静态 HTML/CSS/JavaScript，不依赖原 Vigils 源码运行。

## 当前交付

- 版本：`0.1.0`
- 平台：Windows x64
- 安装方式：NSIS，当前用户安装
- 安装器：[`dist/Vigils Maintenance_0.1.0_x64-setup.exe`](dist/Vigils%20Maintenance_0.1.0_x64-setup.exe)
- 独立程序：[`dist/uvigils.exe`](dist/uvigils.exe)

安装器 SHA-256：

```text
4353DDC06938932F214D12F7F0D584EC5B848285796745FC09826FF8093CBBE1
```

独立程序 SHA-256：

```text
E8BC12DF331313F2583BB49BAFD11EB150E46651BB5B8502EC2122094B802C62
```

当前 `0.1.0` 产物尚未进行 Windows Authenticode 代码签名，适合内网、测试或受控交付。公开分发前应使用组织的代码签名证书重新构建并签名，否则 Windows SmartScreen 可能显示未知发布者警告。

## 功能

- 检测 Vigils 安装版本、路径和运行状态。
- 从本地 `.exe` 或 `.msi` 安装包安装 Vigils，并计算 SHA-256。
- 通过 HTTPS 更新清单检查和安装更新。
- 使用内置公钥强制验证在线安装包的 minisign 签名。
- 更新或卸载前停止 Vigils daemon 和路径精确匹配的 Vigils 进程。
- 卸载前调用 `vigil-hub setup --all --uninstall` 还原 AI agent 接入配置。
- 可选删除 `%LOCALAPPDATA%\Vigil` 中的账本、模型和设置。
- 启动 Vigils、打开安装目录及修复 agent 接入配置。

## 快速使用

1. 运行 `dist\Vigils Maintenance_0.1.0_x64-setup.exe` 安装维护器。
2. 打开 Vigils Maintenance，等待首页完成本机状态检测。
3. 使用“检查更新”安装在线更新，或在“本地安装包”页面选择安装文件。
4. 卸载 Vigils 前确认是否保留 `%LOCALAPPDATA%\Vigil` 用户数据。

完整操作和风险说明见[用户手册](docs/USER_GUIDE.md)。

## 开发与发布

- [构建与发布说明](docs/BUILD_AND_RELEASE.md)
- [更新清单规范](docs/UPDATE_MANIFEST.md)
- [0.1.0 交付记录](docs/RELEASE_0.1.0.md)
- [安全策略与审计摘要](SECURITY.md)

## 项目结构

```text
frontend/       Tauri WebView 用户界面
src/            安装检测、下载验签和维护操作
capabilities/   Tauri 权限边界
icons/          Windows 应用图标
dist/           已验证的交付产物
docs/           使用、构建和发布文档
```

## 许可

本仓库当前未附带开源许可证。公开可见不代表授予复制、修改或再分发权利。
