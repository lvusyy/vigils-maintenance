use crate::model::{ProgressEvent, UpdateInfo, UpdateManifest, DEFAULT_UPDATE_ENDPOINT};
use crate::platform::{is_supported_package_extension, supported_package_description};
use base64::Engine as _;
use semver::Version;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

const UPDATE_PUBKEY: &str = "RWTF6nynfSRLW//J/K4inS8RdovCJ+MhwtfG5xUJ4sJK/silUB9E8D3c";
const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
const MAX_INSTALLER_BYTES: u64 = 1024 * 1024 * 1024;

pub fn resolve_manifest_url(
    template: Option<&str>,
    current_version: &str,
) -> Result<String, String> {
    let template = template
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_UPDATE_ENDPOINT);
    if !template.starts_with("https://") {
        return Err("更新清单必须使用 HTTPS".into());
    }
    Ok(template
        .replace("{{target}}", update_target())
        .replace("{{arch}}", update_arch())
        .replace("{{current_version}}", current_version)
        .replace("{version}", current_version))
}

pub fn fetch_update(
    template: Option<&str>,
    current_version: &str,
) -> Result<(UpdateInfo, UpdateManifest), String> {
    let manifest_url = resolve_manifest_url(template, current_version)?;
    let response = ureq::get(&manifest_url)
        .timeout(Duration::from_secs(20))
        .call()
        .map_err(|error| format!("无法获取更新清单：{error}"))?;
    ensure_https(response.get_url(), "更新清单重定向")?;
    if response
        .header("Content-Length")
        .and_then(|value| value.parse::<usize>().ok())
        .is_some_and(|length| length > MAX_MANIFEST_BYTES)
    {
        return Err("更新清单超过 1 MiB 限制".into());
    }
    let mut body = Vec::new();
    response
        .into_reader()
        .take((MAX_MANIFEST_BYTES + 1) as u64)
        .read_to_end(&mut body)
        .map_err(|error| format!("读取更新清单失败：{error}"))?;
    if body.len() > MAX_MANIFEST_BYTES {
        return Err("更新清单超过 1 MiB 限制".into());
    }
    let manifest: UpdateManifest =
        serde_json::from_slice(&body).map_err(|error| format!("更新清单格式无效：{error}"))?;
    validate_download_url(&manifest.url)?;
    validate_update_authenticity(&manifest.signature, manifest.sha256.as_deref())?;
    let available = compare_versions(&manifest.version, current_version)?;
    let info = UpdateInfo {
        available,
        current_version: current_version.to_string(),
        version: manifest.version.clone(),
        notes: manifest.notes.clone(),
        pub_date: manifest.pub_date.clone(),
        url: manifest.url.clone(),
        signature_present: !manifest.signature.trim().is_empty(),
        sha256: manifest.sha256.clone(),
        manifest_url,
    };
    Ok((info, manifest))
}

pub fn download_verified(
    app: &AppHandle,
    manifest: &UpdateManifest,
) -> Result<(PathBuf, String), String> {
    validate_download_url(&manifest.url)?;
    let response = ureq::get(&manifest.url)
        .timeout(Duration::from_secs(180))
        .call()
        .map_err(|error| format!("下载安装包失败：{error}"))?;
    ensure_https(response.get_url(), "安装包重定向")?;
    let total = response
        .header("Content-Length")
        .and_then(|value| value.parse::<u64>().ok());
    if total.is_some_and(|size| size > MAX_INSTALLER_BYTES) {
        return Err("安装包超过 1 GiB 限制".into());
    }

    let download_dir = dirs::data_local_dir()
        .ok_or_else(|| "无法定位本地应用数据目录".to_string())?
        .join("Uvigils")
        .join("downloads");
    fs::create_dir_all(&download_dir).map_err(|error| format!("创建下载目录失败：{error}"))?;
    let extension = download_extension(&manifest.url)?;
    let safe_version: String = manifest
        .version
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
        .collect();
    if safe_version.is_empty() {
        return Err("更新版本号无法用于本地文件名".into());
    }
    let destination = download_dir.join(format!("Vigils-{safe_version}-setup.{extension}"));
    let partial = destination.with_extension(format!("{extension}.part"));
    let mut output =
        File::create(&partial).map_err(|error| format!("创建临时下载文件失败：{error}"))?;
    let mut reader = response.into_reader();
    let mut hasher = Sha256::new();
    let mut downloaded = 0u64;
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let read = reader.read(&mut buffer).map_err(|error| {
            let _ = fs::remove_file(&partial);
            format!("读取安装包失败：{error}")
        })?;
        if read == 0 {
            break;
        }
        downloaded = downloaded.saturating_add(read as u64);
        if downloaded > MAX_INSTALLER_BYTES {
            let _ = fs::remove_file(&partial);
            return Err("安装包超过 1 GiB 限制".into());
        }
        output.write_all(&buffer[..read]).map_err(|error| {
            let _ = fs::remove_file(&partial);
            format!("写入安装包失败：{error}")
        })?;
        hasher.update(&buffer[..read]);
        let percent = total
            .filter(|value| *value > 0)
            .map(|value| ((downloaded.saturating_mul(100) / value).min(100)) as u8);
        let _ = app.emit(
            "maintenance-progress",
            ProgressEvent {
                phase: "download".into(),
                downloaded,
                total,
                percent,
            },
        );
    }
    output
        .flush()
        .map_err(|error| format!("刷新安装包失败：{error}"))?;
    drop(output);

    let actual_hash = hex::encode(hasher.finalize());
    if let Some(expected) = manifest.sha256.as_deref() {
        if !actual_hash.eq_ignore_ascii_case(expected.trim()) {
            let _ = fs::remove_file(&partial);
            return Err(format!(
                "安装包 SHA-256 不匹配：expected={} actual={actual_hash}",
                expected.trim()
            ));
        }
    }
    let bytes = fs::read(&partial).map_err(|error| format!("读取安装包用于验签失败：{error}"))?;
    if let Err(error) = verify_minisign(&bytes, &manifest.signature, UPDATE_PUBKEY) {
        let _ = fs::remove_file(&partial);
        return Err(error);
    }
    if destination.exists() {
        fs::remove_file(&destination).map_err(|error| format!("替换旧安装包失败：{error}"))?;
    }
    fs::rename(&partial, &destination).map_err(|error| format!("提交安装包失败：{error}"))?;
    let _ = app.emit(
        "maintenance-progress",
        ProgressEvent {
            phase: "verified".into(),
            downloaded,
            total,
            percent: Some(100),
        },
    );
    Ok((destination, actual_hash))
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| format!("打开文件失败：{error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("读取文件失败：{error}"))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn verify_minisign(data: &[u8], wrapped_signature: &str, public_key: &str) -> Result<(), String> {
    let signature_bytes = base64::engine::general_purpose::STANDARD
        .decode(wrapped_signature.trim())
        .map_err(|error| format!("minisign 签名不是有效 Base64：{error}"))?;
    let signature_text = String::from_utf8(signature_bytes)
        .map_err(|error| format!("minisign 签名不是 UTF-8：{error}"))?;
    let key = minisign_verify::PublicKey::from_base64(public_key)
        .map_err(|error| format!("内置 minisign 公钥无效：{error}"))?;
    let signature = minisign_verify::Signature::decode(&signature_text)
        .map_err(|error| format!("minisign 签名格式无效：{error}"))?;
    key.verify(data, &signature, false)
        .map_err(|error| format!("安装包 minisign 验签失败：{error}"))
}

fn validate_download_url(url: &str) -> Result<(), String> {
    ensure_https(url, "安装包 URL")?;
    download_extension(url).map(|_| ())
}

fn download_extension(url: &str) -> Result<String, String> {
    let extension = Path::new(url)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| "安装包 URL 缺少文件扩展名".to_string())?;
    if is_supported_package_extension(&extension) {
        Ok(extension)
    } else {
        Err(format!(
            "当前系统的在线更新只允许 {} 安装包",
            supported_package_description()
        ))
    }
}

fn update_target() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    }
}

fn update_arch() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "x86") {
        "i686"
    } else {
        "x86_64"
    }
}

fn ensure_https(url: &str, label: &str) -> Result<(), String> {
    if url.starts_with("https://") {
        Ok(())
    } else {
        Err(format!("{label} 必须使用 HTTPS"))
    }
}

fn validate_sha256(hash: &str) -> Result<(), String> {
    let hash = hash.trim();
    if hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err("更新清单的 SHA-256 必须是 64 位十六进制字符串".into())
    }
}

fn validate_update_authenticity(signature: &str, sha256: Option<&str>) -> Result<(), String> {
    if signature.trim().is_empty() {
        return Err("更新清单缺少 minisign 签名，已拒绝无法验证发布者的更新".into());
    }
    if let Some(hash) = sha256 {
        validate_sha256(hash)?;
    }
    Ok(())
}

fn compare_versions(remote: &str, current: &str) -> Result<bool, String> {
    let remote = parse_version(remote)?;
    let current = parse_version(current).unwrap_or_else(|_| Version::new(0, 0, 0));
    Ok(remote > current)
}

fn parse_version(input: &str) -> Result<Version, String> {
    Version::parse(input.trim().trim_start_matches(['v', 'V']))
        .map_err(|error| format!("版本号 {input:?} 不是有效 SemVer：{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_tauri_endpoint_placeholders() {
        let url = resolve_manifest_url(None, "0.1.7").unwrap();
        assert!(url.starts_with("https://"));
        assert!(url.contains(&format!("{}-{}/0.1.7.json", update_target(), update_arch())));
    }

    #[test]
    fn rejects_non_https_endpoint() {
        assert!(resolve_manifest_url(Some("http://example.test/latest.json"), "1.0.0").is_err());
    }

    #[test]
    fn online_update_requires_minisign_signature() {
        assert!(validate_update_authenticity(
            "",
            Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        )
        .is_err());
        assert!(validate_update_authenticity("signed", None).is_ok());
    }

    #[test]
    fn update_package_must_match_current_platform() {
        let supported = crate::platform::supported_package_extensions()
            .first()
            .unwrap();
        assert!(download_extension(&format!("https://example.test/Vigils.{supported}")).is_ok());
        assert!(download_extension("https://example.test/Vigils.zip").is_err());
    }

    #[test]
    fn semver_comparison_is_numeric() {
        assert!(compare_versions("0.11.1", "0.1.7").unwrap());
        assert!(!compare_versions("0.1.7", "0.1.7").unwrap());
        assert!(!compare_versions("0.1.6", "0.1.7").unwrap());
    }

    #[test]
    fn hashes_files() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        fs::write(temp.path(), b"abc").unwrap();
        assert_eq!(
            sha256_file(temp.path()).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
