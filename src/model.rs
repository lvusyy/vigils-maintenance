use serde::{Deserialize, Serialize};

pub const DEFAULT_UPDATE_ENDPOINT: &str =
    "https://vigils.oocup.de/desktop-updates/{{target}}-{{arch}}/{{current_version}}.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Installation {
    pub platform: String,
    pub installed: bool,
    pub install_type: Option<String>,
    pub uninstall_supported: bool,
    pub supported_packages: Vec<String>,
    pub version: Option<String>,
    pub display_name: Option<String>,
    pub install_location: Option<String>,
    pub app_executable: Option<String>,
    pub hub_executable: Option<String>,
    pub uninstall_command: Option<String>,
    pub running_processes: Vec<String>,
    pub data_location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub version: String,
    pub notes: String,
    pub pub_date: Option<String>,
    pub url: String,
    pub signature_present: bool,
    pub sha256: Option<String>,
    pub manifest_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateManifest {
    pub version: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub pub_date: Option<String>,
    pub url: String,
    #[serde(default)]
    pub signature: String,
    #[serde(default)]
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageInfo {
    pub path: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub package_type: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationStep {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationResult {
    pub ok: bool,
    pub message: String,
    pub steps: Vec<OperationStep>,
    pub restart_required: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub phase: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percent: Option<u8>,
}
