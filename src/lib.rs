mod model;
mod operations;
mod platform;
mod release;

use model::{Installation, OperationResult, PackageInfo, UpdateInfo};
use std::path::PathBuf;
use tauri::AppHandle;

#[tauri::command]
fn get_status() -> Installation {
    platform::detect_installation()
}

#[tauri::command]
fn choose_installer() -> Option<String> {
    rfd::FileDialog::new()
        .add_filter(
            platform::installer_filter_name(),
            platform::supported_package_extensions(),
        )
        .set_title("选择 Vigils 安装包")
        .pick_file()
        .map(|path| path.display().to_string())
}

#[tauri::command]
async fn inspect_installer(path: String) -> Result<PackageInfo, String> {
    tauri::async_runtime::spawn_blocking(move || operations::inspect_package(&PathBuf::from(path)))
        .await
        .map_err(|error| format!("检查安装包任务失败：{error}"))?
}

#[tauri::command]
async fn check_update(endpoint: Option<String>) -> Result<UpdateInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let status = platform::detect_installation();
        let current = status.version.as_deref().unwrap_or("0.0.1");
        release::fetch_update(endpoint.as_deref(), current).map(|(info, _)| info)
    })
    .await
    .map_err(|error| format!("检查更新任务失败：{error}"))?
}

#[tauri::command]
async fn install_local(
    path: String,
    expected_sha256: Option<String>,
    silent: bool,
    launch_after: bool,
) -> Result<OperationResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        operations::install_package(
            &PathBuf::from(path),
            expected_sha256.as_deref(),
            silent,
            launch_after,
        )
    })
    .await
    .map_err(|error| format!("安装任务失败：{error}"))?
}

#[tauri::command]
async fn update_now(
    app: AppHandle,
    endpoint: Option<String>,
    silent: bool,
    launch_after: bool,
) -> Result<OperationResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let status = platform::detect_installation();
        let current = status.version.as_deref().unwrap_or("0.0.1");
        let (info, manifest) = release::fetch_update(endpoint.as_deref(), current)?;
        if !info.available {
            return Err("当前已是最新版本".into());
        }
        let (package, verified_hash) = release::download_verified(&app, &manifest)?;
        operations::install_package(&package, Some(&verified_hash), silent, launch_after)
    })
    .await
    .map_err(|error| format!("更新任务失败：{error}"))?
}

#[tauri::command]
async fn uninstall_vigils(purge_data: bool, force: bool) -> Result<OperationResult, String> {
    tauri::async_runtime::spawn_blocking(move || operations::uninstall(purge_data, force))
        .await
        .map_err(|error| format!("卸载任务失败：{error}"))?
}

#[tauri::command]
async fn repair_integrations() -> Result<OperationResult, String> {
    tauri::async_runtime::spawn_blocking(operations::repair_integrations)
        .await
        .map_err(|error| format!("修复任务失败：{error}"))?
}

#[tauri::command]
fn launch_vigils() -> Result<(), String> {
    operations::launch_app()
}

#[tauri::command]
fn open_install_location() -> Result<(), String> {
    operations::open_install_location()
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_status,
            choose_installer,
            inspect_installer,
            check_update,
            install_local,
            update_now,
            uninstall_vigils,
            repair_integrations,
            launch_vigils,
            open_install_location,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Vigils Maintenance");
}
