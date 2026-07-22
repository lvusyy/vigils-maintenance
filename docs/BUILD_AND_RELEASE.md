# 构建与发布说明

## 技术栈

- Rust 2021，最低 Rust `1.80`
- Tauri 2
- 静态 HTML/CSS/JavaScript frontend
- Windows NSIS、Linux deb/AppImage、macOS DMG bundle

## 开发检查

获取源码并运行平台无关检查：

```text
git clone https://github.com/lvusyy/vigils-maintenance.git
cd vigils-maintenance
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo audit
```

`cargo audit` 可能报告 Tauri 跨平台 lockfile 中 GTK 3 和 `unic-*` 的维护警告。发布门禁是不允许已知 vulnerability；维护警告需要记录并随 Tauri 升级持续处理。

## 平台构建

Tauri 原生 bundle 必须在对应操作系统构建，不支持在一台机器上直接交叉生成全部桌面安装包。

| 构建系统 | 主要依赖 | 命令 | 输出 |
|---|---|---|---|
| Windows 10/11 x64 | WebView2、NSIS | `cargo tauri build --bundles nsis` | `target/release/bundle/nsis/*.exe` |
| Ubuntu 24.04 x86_64 | WebKitGTK 4.1、GTK 3、librsvg、patchelf | `cargo tauri build --bundles deb,appimage` | `target/release/bundle/deb/*.deb`、`target/release/bundle/appimage/*.AppImage` |
| macOS 14 Apple Silicon | Xcode Command Line Tools | `cargo tauri build --bundles dmg` | `target/release/bundle/dmg/*.dmg` |

Linux CI 使用以下系统依赖：

```bash
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf
```

Windows NSIS 使用 `currentUser` 模式。macOS 和 Linux 的最终包结构由 Tauri 配置生成，参见 `tauri.conf.json`。

## GitHub Actions

- `Cross-platform CI` 在 `main` push 和 pull request 上运行三平台格式检查、测试与 Clippy。
- `Cross-platform Release` 可手动触发产物验证；手动运行只上传 30 天 artifact，不创建 Release。
- 推送 `v*` tag 时，release workflow 构建三平台 bundle、生成 `SHA256SUMS.txt` 并创建 GitHub Release。

本地构建环境、GitHub runner 或其他内部地址不得写入公开文档与 Release 元数据。

## 版本发布流程

1. 同步修改 `Cargo.toml` 和 `tauri.conf.json` 中的版本号，并更新 `Cargo.lock`。
2. 更新 README、用户手册、更新清单规范和独立版本交付记录。
3. 在 Windows 编译机运行开发检查，并完成 NSIS 安装/卸载回归。
4. 推送到 `main`，等待三平台 `Cross-platform CI` 全部通过。
5. 手动运行一次 `Cross-platform Release`，确认 NSIS、deb、AppImage 和 DMG artifact 均能生成。
6. 配置发布签名：Windows 使用 Authenticode，macOS 使用 Developer ID 与 notarization；Linux 可按发行策略签署仓库元数据或文件。
7. 创建并推送版本 tag；等待 GitHub Release 自动发布全部资产和 `SHA256SUMS.txt`。
8. 下载 Release 资产重新计算大小和 SHA-256，更新交付记录；若记录发生变化，再以文档提交收尾。
9. 使用发布私钥为需要在线分发的 Vigils 安装包生成 minisign 签名，并发布符合[更新清单规范](UPDATE_MANIFEST.md)的 JSON。

## 发布验收清单

- `cargo fmt --all -- --check`、测试和 Clippy 在 Windows、Linux、macOS 全部通过。
- `cargo audit` 无已知 vulnerability。
- NSIS、deb、AppImage 和 DMG 构建成功。
- Windows NSIS 安装/卸载回归通过，注册版本正确。
- macOS DMG 可挂载且包含可运行的 app bundle。
- Linux deb 可读取包元数据，AppImage 具有可执行权限并可启动。
- 面向不受控环境公开分发时，Windows Authenticode 和 macOS Developer ID/notarization 状态有效。
- 更新清单和安装包可通过 HTTPS 获取，在线安装包 minisign 验签成功。
- Release 资产大小和 SHA-256 与 `SHA256SUMS.txt`、交付记录一致。

## 安全边界

- 不要把 minisign 私钥、代码签名私钥或密码提交到仓库；使用受控证书存储或 CI secret。
- `src/release.rs` 中只允许存放 minisign 公钥。
- 不得重新引入按映像名终止 `gui.exe` 的逻辑；进程终止必须保持 canonical executable path 精确匹配。
- Windows 注册表产品识别、macOS app bundle 删除路径和 Linux AppImage 删除路径必须保持严格边界。
- 在线更新不得降级为仅校验同源清单提供的 SHA-256。
