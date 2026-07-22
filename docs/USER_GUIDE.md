# 用户手册

## 系统要求

| 系统 | 维护器产物 | Vigils 本地安装包 | 额外要求 |
|---|---|---|---|
| Windows 10/11 x64 | NSIS `.exe` | `.exe`、`.msi` | Microsoft Edge WebView2 Runtime |
| macOS 14+ Apple Silicon | `.dmg` | `.dmg` | 首次运行可能需要在“隐私与安全性”中确认 |
| 主流 Linux x86_64 | `.deb`、`.AppImage` | `.AppImage`、`.deb`、`.rpm` | WebKitGTK；系统包安装需要 PolicyKit/`pkexec` |

在线更新需要能够访问配置的 HTTPS 发布地址。使用系统包安装或卸载 Vigils 时，当前用户还必须具有对应的提权权限。

## 安装维护器

从 [GitHub Releases](https://github.com/lvusyy/vigils-maintenance/releases/tag/v0.2.0) 下载当前系统对应的 `0.2.0` 产物，并按平台安装：

- Windows：运行名称以 `_x64-setup.exe` 结尾的 NSIS 安装器；维护器按当前用户安装。
- macOS：打开 `.dmg`，将 Vigils Maintenance 拖入 Applications 后运行。
- Debian/Ubuntu：使用系统软件安装器打开 `.deb`。
- 其他 Linux：为 `.AppImage` 添加执行权限后直接运行，例如 `chmod +x <file>.AppImage`。

先使用 Release 中的 `SHA256SUMS.txt` 核对下载文件。当前产物未配置 Windows Authenticode 或 Apple Developer ID 签名，请勿在无法确认来源与 SHA-256 时绕过系统安全警告。

## 维护概览

应用启动后会根据当前平台检测 Vigils：

- Windows：精确匹配 Vigils 注册表产品记录和标准安装目录；
- macOS：检查 `~/Applications` 和 `/Applications` 下的 `Vigils.app` 或 `Vigil Desktop.app`；
- Linux：查询名为 `vigils` 的 deb/rpm 包，并检查 `~/.local/bin`、`~/Applications`、`/usr/local/bin`、`/usr/bin` 和 `/opt/Vigils`；
- 所有平台：查找 Vigils GUI、`vigil-hub` 和 executable path 与已检测文件精确一致的运行进程。

“启动 Vigils”“打开目录”“修复接入”和“卸载”按钮会根据检测结果与安装方式启用。检测到应用不代表一定支持自动卸载；界面会明确显示卸载是否可用。

## 本地安装

1. 打开“本地安装包”。
2. 单击“选择安装包”，选择当前平台支持的格式。
3. 核对文件名、大小和 SHA-256。
4. 根据需要选择“后台安装”和“完成后启动 Vigils”。非 Windows 平台会忽略不适用的静默安装选项。
5. 单击安装按钮并查看“执行记录”。

各平台的安装行为：

- Windows：直接运行 `.exe`，或通过 `msiexec.exe` 安装 `.msi`。
- macOS：只读挂载 `.dmg`，将其中的 Vigils app bundle 原子替换到 `~/Applications/Vigils.app`，最后卸载磁盘映像。
- Linux AppImage：原子复制到 `~/.local/bin/Vigils.AppImage`、添加执行权限，并写入当前用户 desktop entry。
- Linux deb/rpm：通过 `pkexec` 调用 `apt-get install` 或 `rpm -U --replacepkgs`。

更新已有安装时，维护器会先调用 `vigil-hub daemon stop`，再停止 executable path 精确匹配的 Vigils 进程。更新不会撤销 agent 接入配置。

本地包来自用户明确选择的文件。维护器计算并显示 SHA-256，但不会替用户判断该文件的发布者是否可信。

## 在线更新

1. 在首页单击“检查更新”。
2. 查看版本号、发布日期和发布说明。
3. 单击安装更新。

在线更新必须同时满足：

- 清单地址、安装包地址及重定向后的最终地址均使用 HTTPS；
- 清单不超过 1 MiB，安装包不超过 1 GiB；
- 安装包扩展名与当前平台匹配；
- 安装包通过内置公钥的 minisign 验签；
- 提供 SHA-256 时，哈希也必须匹配。

平台允许的在线安装包扩展名与本地安装一致。可通过右上角设置按钮更换更新清单地址；模板支持 `{{target}}`、`{{arch}}`、`{{current_version}}` 和 `{version}` 占位符。自定义发布源仍必须使用与应用内置公钥匹配的签名。

## 修复 agent 接入

“修复接入”会执行：

```text
vigil-hub setup --all
```

该操作用于重新注册 Vigils 支持的 AI agent 接入配置。结果会记录在“执行记录”页面。

## 卸载 Vigils

1. 在首页单击“卸载”。
2. 选择是否删除本地账本、模型和设置。
3. 确认卸载。

维护器会先停止 daemon、还原 agent 接入并终止 executable path 精确匹配的 Vigils 进程，然后按安装方式卸载：

- Windows：只运行精确匹配的注册表产品记录提供的卸载命令。
- macOS：只允许移除 `~/Applications` 或 `/Applications` 下已识别的 Vigils app bundle。
- Linux AppImage：只允许移除 `~/.local/bin/Vigils.AppImage` 或 `~/Applications/Vigils.AppImage`，并清理 desktop entry。
- Linux deb/rpm：通过 `pkexec` 调用 `apt-get remove vigils` 或 `rpm -e vigils`。

如果 agent 接入配置还原失败，默认中止卸载。只有明确勾选“接入配置还原失败时仍继续卸载”才会忽略该失败。

删除用户数据不可恢复。数据目录由操作系统决定，通常为 Windows 的 `%LOCALAPPDATA%\Vigil`、macOS 的 `~/Library/Application Support/Vigil` 或 Linux 的 `${XDG_DATA_HOME:-~/.local/share}/Vigil`。

## 故障排查

### 未检测到 Vigils

确认 Vigils 使用标准产品标识、包名或安装位置。自定义位置可能只能被检测为可启动应用，无法安全自动卸载；请使用原安装包或系统包管理器处理。

### Linux 系统包安装失败

确认系统提供 PolicyKit/`pkexec`，并且对应包管理器可用。deb 安装需要 `apt-get`，rpm 安装需要 `rpm`。

### 无法检查更新

检查更新源是否为 HTTPS、网络是否可达、模板占位符是否正确，以及清单中的安装包格式是否与当前平台匹配。HTTP 地址会被直接拒绝。

### minisign 验签失败

不要强行运行下载文件。确认发布清单使用了与应用内置公钥匹配的私钥签名，且安装包在签名后没有被替换。

### 卸载被中止

查看执行记录中的 `restore-integrations`。优先修复 `vigil-hub` 后重新卸载；只有确认接受残留 agent 配置时才使用强制选项。
