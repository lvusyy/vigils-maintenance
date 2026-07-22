# 更新清单规范

## 默认地址

```text
https://vigils.oocup.de/desktop-updates/{{target}}-{{arch}}/{{current_version}}.json
```

Windows x64 上的占位符展开结果示例：

```text
https://vigils.oocup.de/desktop-updates/windows-x86_64/0.1.7.json
```

支持的占位符：

| 占位符 | 含义 |
|---|---|
| `{{target}}` | `windows`、`darwin` 或 `linux` |
| `{{arch}}` | `x86_64` 或 `aarch64` |
| `{{current_version}}` | 当前安装版本 |
| `{version}` | 当前安装版本的兼容写法 |

## JSON 格式

```json
{
  "version": "0.1.8",
  "notes": "修复维护流程并改进稳定性。",
  "pub_date": "2026-07-22T10:00:00Z",
  "url": "https://example.invalid/releases/Vigils_0.1.8_x64-setup.exe",
  "signature": "<Base64-encoded UTF-8 minisign signature file>",
  "sha256": "<64 lowercase or uppercase hexadecimal characters>"
}
```

字段要求：

| 字段 | 必需 | 说明 |
|---|---|---|
| `version` | 是 | 有效 SemVer，可带前导 `v`。必须高于当前版本才显示为可更新。 |
| `notes` | 否 | 发布说明。 |
| `pub_date` | 否 | 建议使用 ISO 8601 UTC 时间。 |
| `url` | 是 | HTTPS 安装包 URL，路径必须以 `.exe` 或 `.msi` 结尾。 |
| `signature` | 是 | 安装包 minisign 签名文件的 UTF-8 内容再整体 Base64 编码。 |
| `sha256` | 否 | 安装包 SHA-256；提供后必须匹配。 |

## 生成签名和哈希

以下示例中的私钥路径仅为占位符，不应放入仓库：

```powershell
minisign -Sm '.\Vigils_0.1.8_x64-setup.exe' -s 'X:\secure\vigils.key'
$signature = [Convert]::ToBase64String(
  [IO.File]::ReadAllBytes('.\Vigils_0.1.8_x64-setup.exe.minisig')
)
$sha256 = (Get-FileHash -Algorithm SHA256 '.\Vigils_0.1.8_x64-setup.exe').Hash.ToLowerInvariant()
```

将 `$signature` 和 `$sha256` 分别写入 JSON 对应字段。

维护器内置 minisign 公钥：

```text
RWTF6nynfSRLW//J/K4inS8RdovCJ+MhwtfG5xUJ4sJK/silUB9E8D3c
```

发布私钥必须与该公钥配对。若需要轮换密钥，必须先发布包含新公钥的维护器，再用新私钥签署后续在线安装包。

## 服务端要求

- 清单和安装包必须通过 HTTPS 提供。
- 所有重定向的最终地址也必须是 HTTPS。
- 清单响应不得超过 1 MiB。
- 安装包不得超过 1 GiB。
- 不要对安装包 URL 使用 query string 或 fragment；当前扩展名校验要求 URL 文本以 `.exe` 或 `.msi` 结尾。
- JSON 应以 UTF-8 返回，建议使用 `Content-Type: application/json`。
- 在安装包和签名上传完成后再发布清单，避免客户端读到不完整版本。

## 验证顺序

维护器会：

1. 校验清单 HTTPS 和大小。
2. 解析 JSON、SemVer、安装包 URL 和可选 SHA-256。
3. 下载到 `%LOCALAPPDATA%\Uvigils\downloads` 下的 `.part` 文件。
4. 校验大小和 SHA-256。
5. 使用内置公钥进行 minisign 验签。
6. 原子重命名临时文件。
7. 安装前再次计算并比对 SHA-256，降低下载完成后的文件替换风险。
