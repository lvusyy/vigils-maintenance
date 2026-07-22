# 用户手册

## 系统要求

- Windows 10 或 Windows 11 x64。
- 当前 Windows 用户具有安装目标程序所需的权限。
- 在线更新需要可访问配置的 HTTPS 发布地址。
- Tauri 窗口依赖 Microsoft Edge WebView2 Runtime；Windows 10/11 通常已预装。

## 安装维护器

从 [GitHub Releases](https://github.com/lvusyy/vigils-maintenance/releases/tag/v0.1.0) 下载安装器并运行：

```text
Vigils.Maintenance_0.1.0_x64-setup.exe
```

维护器按当前用户安装，不要求为其他 Windows 用户部署。也可以直接运行 `dist\uvigils.exe`。

当前 `0.1.0` 产物未进行 Windows Authenticode 签名。在受控环境外运行时，Windows 可能显示未知发布者或 SmartScreen 警告；公开分发版本应由发布方完成代码签名，用户不应在无法确认文件来源和 SHA-256 时绕过警告。

发布前可在 PowerShell 中核对安装器：

```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath '.\Vigils.Maintenance_0.1.0_x64-setup.exe'
```

期望值见[交付记录](RELEASE_0.1.0.md)。

## 维护概览

应用启动后自动检测：

- 注册表中的 Vigils 产品记录；
- 安装位置和 Vigils GUI；
- `vigil-hub.exe`；
- executable path 与已检测文件一致的运行进程；
- `%LOCALAPPDATA%\Vigil` 用户数据目录。

“启动 Vigils”“打开目录”“修复接入”和“卸载”按钮会根据检测结果启用。

## 本地安装

1. 打开“本地安装包”。
2. 单击“选择安装包”，选择 `.exe` 或 `.msi`。
3. 核对文件名、大小和 SHA-256。
4. 根据需要选择“后台安装”和“完成后启动 Vigils”。
5. 单击安装按钮并查看“执行记录”。

更新已有安装时，维护器会先调用 `vigil-hub daemon stop`，再停止 executable path 精确匹配的 Vigils 进程。更新不会撤销 agent 接入配置。

本地包来自用户明确选择的文件。维护器计算并显示 SHA-256，但不会替用户判断该文件的发布者是否可信。

## 在线更新

1. 在首页单击“检查更新”。
2. 查看版本号、发布日期和发布说明。
3. 单击安装更新。

在线更新必须同时满足：

- 清单地址和安装包地址使用 HTTPS；
- 重定向后的地址仍使用 HTTPS；
- 清单不超过 1 MiB，安装包不超过 1 GiB；
- 安装包扩展名为 `.exe` 或 `.msi`；
- 安装包通过内置公钥的 minisign 验签；
- 提供 SHA-256 时，哈希也必须匹配。

可通过右上角设置按钮更换更新清单地址。模板支持 `{{target}}`、`{{arch}}`、`{{current_version}}` 和 `{version}` 占位符。自定义发布源仍必须使用与应用内置公钥匹配的签名。

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

维护器按以下顺序执行：

1. `vigil-hub daemon stop`
2. `vigil-hub setup --all --uninstall`
3. 停止 executable path 精确匹配的 Vigils 进程
4. 运行 Vigils 注册的卸载程序
5. 按选择决定是否删除 `%LOCALAPPDATA%\Vigil`

如果 agent 接入配置还原失败，默认中止卸载。只有明确勾选“接入配置还原失败时仍继续卸载”才会忽略该失败。

删除用户数据不可恢复。保留该选项未勾选时，账本、模型和设置不会删除。

## 故障排查

### 未检测到 Vigils

维护器仅接受注册表 key `ai.vigils.desktop`，或 DisplayName 精确为 `Vigils`、`Vigil Desktop` 的产品。也会检查标准本地安装目录。如果使用了自定义目录且没有正确注册，请先用原安装包修复。

### 无法检查更新

检查更新源是否为 HTTPS、网络是否可达，以及模板占位符是否正确。HTTP 地址会被直接拒绝。

### minisign 验签失败

不要强行运行下载文件。确认发布清单使用了与应用内置公钥匹配的私钥签名，且安装包在签名后没有被替换。

### 卸载被中止

查看执行记录中的 `restore-integrations`。优先修复 `vigil-hub` 后重新卸载；只有确认接受残留 agent 配置时才使用强制选项。
