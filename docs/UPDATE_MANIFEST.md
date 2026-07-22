# 更新清单规范

## 默认地址

```text
https://vigils.oocup.de/desktop-updates/{{target}}-{{arch}}/{{current_version}}.json
```

占位符展开示例：

```text
https://vigils.oocup.de/desktop-updates/windows-x86_64/0.2.0.json
https://vigils.oocup.de/desktop-updates/darwin-aarch64/0.2.0.json
https://vigils.oocup.de/desktop-updates/linux-x86_64/0.2.0.json
```

| 占位符 | 含义 |
|---|---|
| `{{target}}` | `windows`、`darwin` 或 `linux` |
| `{{arch}}` | `x86_64` 或 `aarch64` |
| `{{current_version}}` | 当前安装版本 |
| `{version}` | 当前安装版本的兼容写法 |

服务端应为每个 target/arch 返回对应平台的 Vigils 安装包，不得将维护器自身的安装包当作 Vigils 更新包。

## JSON 格式

```json
{
  "version": "0.2.1",
  "notes": "修复维护流程并改进稳定性。",
  "pub_date": "2026-07-22T10:00:00Z",
  "url": "https://example.invalid/releases/Vigils_0.2.1_x64-setup.exe",
  "signature": "<Base64-encoded UTF-8 minisign signature file>",
  "sha256": "<64 lowercase or uppercase hexadecimal characters>"
}
```

| 字段 | 必需 | 说明 |
|---|---|---|
| `version` | 是 | 有效 SemVer，可带前导 `v`。必须高于当前版本才显示为可更新。 |
| `notes` | 否 | 发布说明。 |
| `pub_date` | 否 | 建议使用 ISO 8601 UTC 时间。 |
| `url` | 是 | HTTPS 安装包 URL，扩展名必须与当前平台匹配。 |
| `signature` | 是 | 安装包 minisign 签名文件的 UTF-8 内容再整体 Base64 编码。 |
| `sha256` | 否 | 安装包 SHA-256；提供后必须匹配。 |

平台扩展名白名单：

| target | 允许扩展名 |
|---|---|
| `windows` | `.exe`、`.msi` |
| `darwin` | `.dmg` |
| `linux` | `.AppImage`、`.deb`、`.rpm` |

扩展名匹配不区分大小写。URL 不应包含 query string 或 fragment，因为当前校验从 URL 路径末尾提取扩展名。

## 生成签名和哈希

以下 PowerShell 示例中的私钥路径仅为占位符，不应放入仓库：

```powershell
$package = '.\Vigils_0.2.1_x64-setup.exe'
minisign -Sm $package -s 'X:\secure\vigils.key'
$signature = [Convert]::ToBase64String(
  [IO.File]::ReadAllBytes("$package.minisig")
)
$sha256 = (Get-FileHash -Algorithm SHA256 $package).Hash.ToLowerInvariant()
```

Linux/macOS 可使用同一个 minisign 流程；只需把 `$package` 替换为对应平台的包路径。将 `$signature` 和 `$sha256` 分别写入 JSON 对应字段。

维护器内置 minisign 公钥：

```text
RWTF6nynfSRLW//J/K4inS8RdovCJ+MhwtfG5xUJ4sJK/silUB9E8D3c
```

发布私钥必须与该公钥配对。若需要轮换密钥，必须先发布包含新公钥的维护器，再用新私钥签署后续在线安装包。

## 服务端要求

- 清单、安装包和所有重定向后的最终地址必须使用 HTTPS。
- 清单响应不得超过 1 MiB，安装包不得超过 1 GiB。
- JSON 应以 UTF-8 返回，建议使用 `Content-Type: application/json`。
- 每个平台清单中的 URL 必须指向该平台可安装的包格式。
- 在安装包和签名上传完成后再发布清单，避免客户端读到不完整版本。

## 验证顺序

维护器会：

1. 展开 target、arch 和当前版本占位符。
2. 校验清单 HTTPS 和大小。
3. 解析 JSON、SemVer、安装包 HTTPS URL、平台扩展名和可选 SHA-256。
4. 下载到系统本地数据目录下 `Uvigils/downloads` 中的 `.part` 文件。
5. 校验大小和 SHA-256。
6. 使用内置公钥进行 minisign 验签。
7. 原子重命名临时文件。
8. 安装前再次计算并比对 SHA-256，降低下载完成后的文件替换风险。
