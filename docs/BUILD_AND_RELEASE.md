# 构建与发布说明

## 技术栈

- Rust 2021，最低 Rust `1.80`
- Tauri 2
- 静态 HTML/CSS/JavaScript frontend
- Windows NSIS bundle

## 构建环境

准备 Windows 10/11 x64、Rust `1.80` 或更高版本、Tauri CLI 2、Microsoft Edge WebView2 Runtime 和 NSIS 构建依赖。Rust 工具应已加入 PATH。

获取源码：

```powershell
git clone https://github.com/lvusyy/vigils-maintenance.git
Set-Location vigils-maintenance
```

## 开发检查

```powershell
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo audit
```

`cargo audit` 可能报告 Tauri 跨平台 lockfile 中 GTK 3 和 `unic-*` 的维护警告。发布门禁是不允许已知 vulnerability；维护警告需要记录并随 Tauri 升级持续处理。

## 构建

```powershell
cargo tauri build --bundles nsis
```

输出位置：

```text
target\release\uvigils.exe
target\release\bundle\nsis\Vigils Maintenance_<version>_x64-setup.exe
```

当前配置使用 NSIS `currentUser` 模式，参见 `tauri.conf.json`。

## 版本发布流程

1. 同步修改 `Cargo.toml` 和 `tauri.conf.json` 中的版本号。
2. 更新 `Cargo.lock` 并运行全部开发检查。
3. 为公开分发配置组织的 Windows Authenticode 代码签名证书，并构建 release 和 NSIS 安装器。
4. 用 `Get-AuthenticodeSignature` 确认独立程序和安装器的状态均为 `Valid`。
5. 使用发布私钥为需要在线分发的 Vigils 安装包生成 minisign 签名。
6. 生成符合[更新清单规范](UPDATE_MANIFEST.md)的 JSON。
7. 上传安装包和清单，确认最终 URL 均为 HTTPS。
8. 从干净 Windows 用户环境验证维护器安装、更新和卸载。
9. 计算交付文件 SHA-256，写入独立版本交付记录。
10. 将验证后的文件复制到 `dist/`。

## 发布验收清单

- `cargo fmt --all -- --check` 通过。
- `cargo test --all-targets` 全部通过。
- `cargo clippy --all-targets -- -D warnings` 通过。
- `cargo audit` 无 vulnerability。
- NSIS 构建成功。
- 公开分发时，独立程序和安装器 Authenticode 状态为 `Valid`。
- `uvigils.exe` 启动后不会立即退出。
- NSIS 静默安装返回 0，并写入正确版本。
- NSIS 静默卸载返回 0，卸载注册项被清除。
- 更新清单和安装包可通过 HTTPS 获取。
- 在线安装包 minisign 验签成功。
- `dist/` 文件大小和 SHA-256 与交付记录一致。

## 安全边界

- 不要把 minisign 私钥或密码提交到仓库。
- 不要把 Authenticode 私钥或证书密码提交到仓库；使用受控证书存储或 CI secret。
- `src/release.rs` 中只允许存放公钥。
- 不得重新引入按映像名终止 `gui.exe` 的逻辑。
- 注册表产品识别必须保持精确匹配。
- 在线更新不得降级为仅校验同源清单提供的 SHA-256。
