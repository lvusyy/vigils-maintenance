use crate::model::{OperationResult, OperationStep, PackageInfo};
use crate::platform::{
    detect_installation, is_supported_package, split_windows_command_line, stop_vigil_processes,
};
use crate::release::sha256_file;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::thread;
use std::time::Duration;

pub fn inspect_package(path: &Path) -> Result<PackageInfo, String> {
    let canonical = fs::canonicalize(path).map_err(|error| format!("无法访问安装包：{error}"))?;
    if !canonical.is_file() || !is_supported_package(&canonical) {
        return Err("请选择 .exe 或 .msi Windows 安装包".into());
    }
    let metadata = canonical
        .metadata()
        .map_err(|error| format!("读取安装包信息失败：{error}"))?;
    let file_name = canonical
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "安装包文件名不是有效 UTF-8".to_string())?;
    let package_type = canonical
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_uppercase();
    Ok(PackageInfo {
        path: canonical.display().to_string(),
        file_name: file_name.to_string(),
        size_bytes: metadata.len(),
        sha256: sha256_file(&canonical)?,
        package_type,
    })
}

pub fn install_package(
    path: &Path,
    expected_sha256: Option<&str>,
    silent: bool,
    launch_after: bool,
) -> Result<OperationResult, String> {
    let package = inspect_package(path)?;
    if let Some(expected) = expected_sha256
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if !package.sha256.eq_ignore_ascii_case(expected) {
            return Err(format!(
                "安装包 SHA-256 不匹配：expected={expected} actual={}",
                package.sha256
            ));
        }
    }
    let mut steps = vec![step(
        "verify-package",
        true,
        format!("SHA-256 {}", package.sha256),
    )];
    let current = detect_installation();
    if current.installed {
        stop_runtime(&current, &mut steps, false);
    }

    let status = run_installer(Path::new(&package.path), silent)?;
    let ok = status.success();
    steps.push(step(
        "run-installer",
        ok,
        status
            .code()
            .map(|code| format!("exit code {code}"))
            .unwrap_or_else(|| "process terminated".into()),
    ));
    if !ok {
        return Ok(OperationResult {
            ok: false,
            message: "安装程序返回失败状态".into(),
            steps,
            restart_required: false,
        });
    }

    thread::sleep(Duration::from_millis(800));
    let installed = detect_installation();
    let detected = installed.installed;
    steps.push(step(
        "verify-installation",
        detected,
        installed
            .version
            .as_deref()
            .map(|version| format!("detected version {version}"))
            .unwrap_or_else(|| "installer finished; registration is not visible yet".into()),
    ));
    if launch_after {
        if let Some(executable) = installed.app_executable.as_deref() {
            let launch = Command::new(executable).spawn();
            steps.push(step(
                "launch-app",
                launch.is_ok(),
                launch
                    .map(|_| executable.to_string())
                    .unwrap_or_else(|error| error.to_string()),
            ));
        }
    }
    Ok(OperationResult {
        ok: true,
        message: if detected {
            "Vigils 安装完成"
        } else {
            "安装程序已完成"
        }
        .into(),
        steps,
        restart_required: false,
    })
}

pub fn uninstall(purge_data: bool, force: bool) -> Result<OperationResult, String> {
    let installation = detect_installation();
    if !installation.installed {
        return Err("未检测到 Vigils 安装".into());
    }
    let mut steps = Vec::new();
    stop_runtime(&installation, &mut steps, true);
    let integration_failed = steps
        .iter()
        .any(|item| item.name == "restore-integrations" && !item.ok);
    if integration_failed && !force {
        return Ok(OperationResult {
            ok: false,
            message: "未能完整还原 AI agent 接入配置；确认后可使用强制卸载".into(),
            steps,
            restart_required: false,
        });
    }

    let command_line = installation
        .uninstall_command
        .as_deref()
        .ok_or_else(|| "安装记录缺少卸载命令，请使用原安装包修复后重试".to_string())?;
    let args = split_windows_command_line(command_line)?;
    let executable = args.first().ok_or_else(|| "卸载命令为空".to_string())?;
    let status = Command::new(executable)
        .args(&args[1..])
        .status()
        .map_err(|error| format!("启动卸载程序失败：{error}"))?;
    steps.push(step(
        "run-uninstaller",
        status.success(),
        status
            .code()
            .map(|code| format!("exit code {code}"))
            .unwrap_or_else(|| "process terminated".into()),
    ));
    if !status.success() {
        return Ok(OperationResult {
            ok: false,
            message: "卸载程序返回失败状态".into(),
            steps,
            restart_required: false,
        });
    }

    if purge_data {
        let data_path = dirs::data_local_dir().map(|root| root.join("Vigil"));
        match data_path {
            Some(path) if path.is_dir() => match fs::remove_dir_all(&path) {
                Ok(()) => steps.push(step("purge-data", true, path.display().to_string())),
                Err(error) => steps.push(step("purge-data", false, error.to_string())),
            },
            Some(path) => steps.push(step(
                "purge-data",
                true,
                format!("{} does not exist", path.display()),
            )),
            None => steps.push(step(
                "purge-data",
                false,
                "local data directory unavailable",
            )),
        }
    } else {
        steps.push(step("preserve-data", true, "保留账本、模型和用户设置"));
    }
    let required = steps
        .iter()
        .all(|item| item.ok || item.name == "stop-process");
    Ok(OperationResult {
        ok: required,
        message: if required {
            "Vigils 已卸载"
        } else {
            "Vigils 已卸载，但部分清理步骤需要人工检查"
        }
        .into(),
        steps,
        restart_required: false,
    })
}

pub fn repair_integrations() -> Result<OperationResult, String> {
    let installation = detect_installation();
    let hub = installation
        .hub_executable
        .as_deref()
        .ok_or_else(|| "未找到 vigil-hub.exe，无法修复接入配置".to_string())?;
    let output = Command::new(hub)
        .args(["setup", "--all"])
        .output()
        .map_err(|error| format!("启动 vigil-hub 修复失败：{error}"))?;
    let detail = combined_output(&output.stdout, &output.stderr);
    Ok(OperationResult {
        ok: output.status.success(),
        message: if output.status.success() {
            "AI agent 接入配置已修复"
        } else {
            "接入配置修复失败"
        }
        .into(),
        steps: vec![step("repair-integrations", output.status.success(), detail)],
        restart_required: false,
    })
}

pub fn launch_app() -> Result<(), String> {
    let installation = detect_installation();
    let executable = installation
        .app_executable
        .ok_or_else(|| "未找到 Vigils GUI 可执行文件".to_string())?;
    Command::new(&executable)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("启动 Vigils 失败：{error}"))
}

pub fn open_install_location() -> Result<(), String> {
    let installation = detect_installation();
    let location = installation
        .install_location
        .or_else(|| {
            installation.app_executable.and_then(|value| {
                PathBuf::from(value)
                    .parent()
                    .map(Path::to_path_buf)
                    .map(|p| p.display().to_string())
            })
        })
        .ok_or_else(|| "未找到安装目录".to_string())?;
    Command::new("explorer.exe")
        .arg(location)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("打开安装目录失败：{error}"))
}

fn run_installer(path: &Path, silent: bool) -> Result<ExitStatus, String> {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("exe") => {
            let mut command = Command::new(path);
            if silent {
                command.arg("/S");
            }
            command
                .status()
                .map_err(|error| format!("启动 NSIS 安装包失败：{error}"))
        }
        Some("msi") => {
            let mut command = Command::new("msiexec.exe");
            command.arg("/i").arg(path);
            if silent {
                command.args(["/passive", "/norestart"]);
            }
            command
                .status()
                .map_err(|error| format!("启动 MSI 安装包失败：{error}"))
        }
        _ => Err("不支持的安装包类型".into()),
    }
}

fn stop_runtime(
    installation: &crate::model::Installation,
    steps: &mut Vec<OperationStep>,
    restore_integrations: bool,
) {
    if let Some(hub) = installation.hub_executable.as_deref() {
        match Command::new(hub).args(["daemon", "stop"]).output() {
            Ok(output) => steps.push(step(
                "stop-daemon",
                output.status.success(),
                combined_output(&output.stdout, &output.stderr),
            )),
            Err(error) => steps.push(step("stop-daemon", false, error.to_string())),
        }
        if restore_integrations {
            match Command::new(hub)
                .args(["setup", "--all", "--uninstall"])
                .output()
            {
                Ok(output) => steps.push(step(
                    "restore-integrations",
                    output.status.success(),
                    combined_output(&output.stdout, &output.stderr),
                )),
                Err(error) => steps.push(step("restore-integrations", false, error.to_string())),
            }
        }
    } else {
        steps.push(step("stop-daemon", true, "vigil-hub not present"));
        if restore_integrations {
            steps.push(step("restore-integrations", true, "vigil-hub not present"));
        }
    }
    for (name, ok, detail) in stop_vigil_processes(installation) {
        steps.push(step("stop-process", ok, format!("{name}: {detail}")));
    }
}

fn step(name: impl Into<String>, ok: bool, detail: impl Into<String>) -> OperationStep {
    OperationStep {
        name: name.into(),
        ok,
        detail: detail.into(),
    }
}

fn combined_output(stdout: &[u8], stderr: &[u8]) -> String {
    let stdout = String::from_utf8_lossy(stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    match (stdout.is_empty(), stderr.is_empty()) {
        (false, false) => format!("{stdout}\n{stderr}"),
        (false, true) => stdout,
        (true, false) => stderr,
        (true, true) => "completed".into(),
    }
}
