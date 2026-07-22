# Vigils Maintenance 0.1.0 交付记录

交付日期：2026-07-22

目标平台：Windows x64

安装模式：NSIS current user

## 产物

| 文件 | 大小 | SHA-256 |
|---|---:|---|
| `dist/uvigils.exe` | 11,238,400 bytes | `E8BC12DF331313F2583BB49BAFD11EB150E46651BB5B8502EC2122094B802C62` |
| `dist/Vigils Maintenance_0.1.0_x64-setup.exe` | 2,886,735 bytes | `4353DDC06938932F214D12F7F0D584EC5B848285796745FC09826FF8093CBBE1` |

GitHub Release 下载：

- [Vigils.Maintenance_0.1.0_x64-setup.exe](https://github.com/lvusyy/vigils-maintenance/releases/download/v0.1.0/Vigils.Maintenance_0.1.0_x64-setup.exe)
- [uvigils.exe](https://github.com/lvusyy/vigils-maintenance/releases/download/v0.1.0/uvigils.exe)
- [SHA256SUMS.txt](https://github.com/lvusyy/vigils-maintenance/releases/download/v0.1.0/SHA256SUMS.txt)

Authenticode 状态：两个产物均为 `NotSigned`。本版本适合内网、测试或受控交付；公开分发前需要代码签名并重新记录签名后文件的大小和 SHA-256。

## 验证结果

```text
cargo fmt --all -- --check                     passed
cargo test --all-targets                       9 passed, 0 failed
cargo clippy --all-targets -- -D warnings      passed
cargo audit                                    0 vulnerabilities
cargo tauri build --bundles nsis               passed
```

运行和安装回归：

- release 程序在 SSH 非交互 Windows 会话中持续运行 8 秒，未异常退出。
- NSIS 静默安装退出码：`0`。
- 安装注册表 DisplayVersion：`0.1.0`。
- NSIS 静默卸载退出码：`0`。
- 卸载后注册项残留：`false`。

SSH 非交互桌面会话未返回窗口标题，因此本轮不把标题检测列为通过证据。界面布局在此前 `1120 x 760` 截图检查中无横向或纵向溢出。

## 安全整改

- 注册表产品识别改为 key 或 DisplayName 精确匹配。
- 进程停止改为 canonical executable path 精确匹配，不再按 `gui.exe` 映像名批量终止。
- 在线更新强制 minisign 验签，SHA-256 作为额外完整性校验。
- 移除 `rfd` 默认 Linux portal 依赖，解决 `RUSTSEC-2026-0194` 和 `RUSTSEC-2026-0195`。

详细信息见[安全策略与审计摘要](../SECURITY.md)。
