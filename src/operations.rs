use crate::model::{Installation, OperationResult, OperationStep, PackageInfo};
#[cfg(windows)]
use crate::platform::split_windows_command_line;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use crate::platform::validated_removable_install_path;
use crate::platform::{
    detect_installation, is_supported_package, package_extension, stop_vigil_processes,
    supported_package_description,
};
use crate::release::sha256_file;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

struct InstallerOutcome {
    ok: bool,
    detail: String,
    restart_required: bool,
}

pub fn inspect_package(path: &Path) -> Result<PackageInfo, String> {
    let canonical = fs::canonicalize(path).map_err(|error| format!("无法访问安装包：{error}"))?;
    if !canonical.is_file() || !is_supported_package(&canonical) {
        return Err(format!(
            "当前系统只支持 {} 安装包",
            supported_package_description()
        ));
    }
    let metadata = canonical
        .metadata()
        .map_err(|error| format!("读取安装包信息失败：{error}"))?;
    let file_name = canonical
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "安装包文件名不是有效 UTF-8".to_string())?;
    let package_type = package_extension(&canonical)
        .unwrap_or_default()
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

    let outcome = run_installer(Path::new(&package.path), silent)?;
    steps.push(step("run-installer", outcome.ok, outcome.detail));
    if !outcome.ok {
        return Ok(OperationResult {
            ok: false,
            message: "安装程序返回失败状态".into(),
            steps,
            restart_required: outcome.restart_required,
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
            .unwrap_or_else(|| {
                if detected {
                    "installation detected; version unavailable".into()
                } else {
                    "installer finished; installation is not visible yet".into()
                }
            }),
    ));
    if launch_after {
        match launch_detected_app(&installed) {
            Ok(detail) => steps.push(step("launch-app", true, detail)),
            Err(error) => steps.push(step("launch-app", false, error)),
        }
    }
    Ok(OperationResult {
        ok: true,
        message: if detected {
            "Vigils 安装完成"
        } else {
            "安装程序已完成，请刷新状态确认"
        }
        .into(),
        steps,
        restart_required: outcome.restart_required,
    })
}

pub fn uninstall(purge_data: bool, force: bool) -> Result<OperationResult, String> {
    let installation = detect_installation();
    if !installation.installed {
        return Err("未检测到 Vigils 安装".into());
    }
    if !installation.uninstall_supported {
        return Err("当前安装方式无法安全自动卸载，请使用原包管理器处理".into());
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

    let outcome = run_uninstaller(&installation)?;
    steps.push(step("run-uninstaller", outcome.ok, outcome.detail));
    if !outcome.ok {
        return Ok(OperationResult {
            ok: false,
            message: "卸载程序返回失败状态".into(),
            steps,
            restart_required: outcome.restart_required,
        });
    }

    if purge_data {
        purge_user_data(&mut steps);
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
        restart_required: outcome.restart_required,
    })
}

pub fn repair_integrations() -> Result<OperationResult, String> {
    let installation = detect_installation();
    let hub = installation
        .hub_executable
        .as_deref()
        .ok_or_else(|| "未找到 vigil-hub，无法修复接入配置".to_string())?;
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
    launch_detected_app(&detect_installation()).map(|_| ())
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
                    .map(|path| path.display().to_string())
            })
        })
        .ok_or_else(|| "未找到安装目录".to_string())?;
    open_path(Path::new(&location))
}

fn launch_detected_app(installation: &Installation) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    if let Some(bundle) = installation.install_location.as_deref() {
        if bundle.to_ascii_lowercase().ends_with(".app") {
            Command::new("open")
                .arg(bundle)
                .spawn()
                .map_err(|error| format!("启动 Vigils 失败：{error}"))?;
            return Ok(bundle.into());
        }
    }

    let executable = installation
        .app_executable
        .as_deref()
        .ok_or_else(|| "未找到 Vigils GUI 可执行文件".to_string())?;
    Command::new(executable)
        .spawn()
        .map(|_| executable.into())
        .map_err(|error| format!("启动 Vigils 失败：{error}"))
}

#[cfg(windows)]
fn open_path(path: &Path) -> Result<(), String> {
    Command::new("explorer.exe")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("打开安装目录失败：{error}"))
}

#[cfg(target_os = "macos")]
fn open_path(path: &Path) -> Result<(), String> {
    Command::new("open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("打开安装目录失败：{error}"))
}

#[cfg(target_os = "linux")]
fn open_path(path: &Path) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("打开安装目录失败：{error}"))
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn open_path(_path: &Path) -> Result<(), String> {
    Err("当前操作系统不支持打开安装目录".into())
}

#[cfg(windows)]
fn run_installer(path: &Path, silent: bool) -> Result<InstallerOutcome, String> {
    let mut command = match package_extension(path).as_deref() {
        Some("exe") => {
            let mut command = Command::new(path);
            if silent {
                command.arg("/S");
            }
            command
        }
        Some("msi") => {
            let mut command = Command::new("msiexec.exe");
            command.arg("/i").arg(path);
            if silent {
                command.args(["/passive", "/norestart"]);
            }
            command
        }
        _ => return Err("不支持的 Windows 安装包类型".into()),
    };
    let status = command
        .status()
        .map_err(|error| format!("启动 Windows 安装包失败：{error}"))?;
    Ok(status_outcome(status))
}

#[cfg(target_os = "macos")]
fn run_installer(path: &Path, _silent: bool) -> Result<InstallerOutcome, String> {
    if package_extension(path).as_deref() != Some("dmg") {
        return Err("macOS 仅支持 .dmg 安装包".into());
    }
    install_macos_dmg(path)
}

#[cfg(target_os = "linux")]
fn run_installer(path: &Path, _silent: bool) -> Result<InstallerOutcome, String> {
    match package_extension(path).as_deref() {
        Some("appimage") => install_linux_appimage(path),
        Some("deb") => run_privileged_package_command("apt-get", &["install", "-y"], path, "DEB"),
        Some("rpm") => run_privileged_package_command("rpm", &["-U", "--replacepkgs"], path, "RPM"),
        _ => Err("不支持的 Linux 安装包类型".into()),
    }
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn run_installer(_path: &Path, _silent: bool) -> Result<InstallerOutcome, String> {
    Err("当前操作系统不支持安装 Vigils".into())
}

#[cfg(windows)]
fn run_uninstaller(installation: &Installation) -> Result<InstallerOutcome, String> {
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
    Ok(status_outcome(status))
}

#[cfg(target_os = "macos")]
fn run_uninstaller(installation: &Installation) -> Result<InstallerOutcome, String> {
    let bundle = validated_removable_install_path(installation)?;
    fs::remove_dir_all(&bundle).map_err(|error| format!("移除 macOS 应用失败：{error}"))?;
    Ok(InstallerOutcome {
        ok: true,
        detail: format!("removed {}", bundle.display()),
        restart_required: false,
    })
}

#[cfg(target_os = "linux")]
fn run_uninstaller(installation: &Installation) -> Result<InstallerOutcome, String> {
    match installation.install_type.as_deref() {
        Some("linux-appimage") => {
            let appimage = validated_removable_install_path(installation)?;
            fs::remove_file(&appimage).map_err(|error| format!("移除 AppImage 失败：{error}"))?;
            remove_linux_desktop_entry();
            Ok(InstallerOutcome {
                ok: true,
                detail: format!("removed {}", appimage.display()),
                restart_required: false,
            })
        }
        Some("linux-deb") => {
            run_privileged_uninstall_command("apt-get", &["remove", "-y", "vigils"], "DEB")
        }
        Some("linux-rpm") => run_privileged_uninstall_command("rpm", &["-e", "vigils"], "RPM"),
        _ => Err("当前 Linux 安装方式无法安全自动卸载".into()),
    }
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn run_uninstaller(_installation: &Installation) -> Result<InstallerOutcome, String> {
    Err("当前操作系统不支持卸载 Vigils".into())
}

#[cfg(windows)]
fn status_outcome(status: std::process::ExitStatus) -> InstallerOutcome {
    InstallerOutcome {
        ok: status.success(),
        detail: status
            .code()
            .map(|code| format!("exit code {code}"))
            .unwrap_or_else(|| "process terminated".into()),
        restart_required: false,
    }
}

#[cfg(target_os = "macos")]
fn install_macos_dmg(path: &Path) -> Result<InstallerOutcome, String> {
    let attach = Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-readonly"])
        .arg(path)
        .output()
        .map_err(|error| format!("挂载 DMG 失败：{error}"))?;
    if !attach.status.success() {
        return Ok(InstallerOutcome {
            ok: false,
            detail: combined_output(&attach.stdout, &attach.stderr),
            restart_required: false,
        });
    }
    let mount_point = parse_macos_mount_point(&attach.stdout)
        .ok_or_else(|| "DMG 已挂载，但无法确定挂载目录".to_string())?;
    let install_result = copy_macos_app_from_mount(&mount_point);
    let detach = Command::new("hdiutil")
        .arg("detach")
        .arg(&mount_point)
        .output();

    let mut outcome = install_result?;
    if let Ok(detach) = detach {
        if !detach.status.success() {
            outcome.detail.push_str("; warning: failed to detach DMG");
        }
    } else {
        outcome
            .detail
            .push_str("; warning: failed to run hdiutil detach");
    }
    Ok(outcome)
}

#[cfg(target_os = "macos")]
fn parse_macos_mount_point(stdout: &[u8]) -> Option<PathBuf> {
    String::from_utf8_lossy(stdout)
        .lines()
        .rev()
        .filter_map(|line| line.split('\t').next_back())
        .map(str::trim)
        .find(|field| field.starts_with("/Volumes/"))
        .map(PathBuf::from)
}

#[cfg(target_os = "macos")]
fn copy_macos_app_from_mount(mount_point: &Path) -> Result<InstallerOutcome, String> {
    let source = fs::read_dir(mount_point)
        .map_err(|error| format!("读取 DMG 内容失败：{error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| matches!(name, "Vigils.app" | "Vigil Desktop.app"))
        })
        .ok_or_else(|| "DMG 中未找到 Vigils.app".to_string())?;
    let applications = dirs::home_dir()
        .ok_or_else(|| "无法定位用户主目录".to_string())?
        .join("Applications");
    fs::create_dir_all(&applications)
        .map_err(|error| format!("创建用户 Applications 目录失败：{error}"))?;
    let destination = applications.join("Vigils.app");
    let staging = applications.join(".Vigils.app.installing");
    let previous = applications.join(".Vigils.app.previous");
    remove_path_if_present(&staging)?;
    remove_path_if_present(&previous)?;

    let copy = Command::new("ditto")
        .arg(&source)
        .arg(&staging)
        .output()
        .map_err(|error| format!("复制 Vigils.app 失败：{error}"))?;
    if !copy.status.success() {
        remove_path_if_present(&staging)?;
        return Ok(InstallerOutcome {
            ok: false,
            detail: combined_output(&copy.stdout, &copy.stderr),
            restart_required: false,
        });
    }
    if destination.exists() {
        fs::rename(&destination, &previous)
            .map_err(|error| format!("备份旧 Vigils.app 失败：{error}"))?;
    }
    if let Err(error) = fs::rename(&staging, &destination) {
        if previous.exists() {
            let _ = fs::rename(&previous, &destination);
        }
        return Err(format!("提交新 Vigils.app 失败：{error}"));
    }
    remove_path_if_present(&previous)?;
    Ok(InstallerOutcome {
        ok: true,
        detail: format!("installed {}", destination.display()),
        restart_required: false,
    })
}

#[cfg(target_os = "macos")]
fn remove_path_if_present(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("清理 {} 失败：{error}", path.display()))?;
    } else if path.exists() {
        fs::remove_file(path).map_err(|error| format!("清理 {} 失败：{error}", path.display()))?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn install_linux_appimage(path: &Path) -> Result<InstallerOutcome, String> {
    use std::os::unix::fs::PermissionsExt;

    let destination_dir = dirs::home_dir()
        .ok_or_else(|| "无法定位用户主目录".to_string())?
        .join(".local")
        .join("bin");
    fs::create_dir_all(&destination_dir)
        .map_err(|error| format!("创建用户 bin 目录失败：{error}"))?;
    let destination = destination_dir.join("Vigils.AppImage");
    let partial = destination_dir.join(".Vigils.AppImage.part");
    if partial.exists() {
        fs::remove_file(&partial).map_err(|error| format!("清理旧临时文件失败：{error}"))?;
    }
    fs::copy(path, &partial).map_err(|error| format!("复制 AppImage 失败：{error}"))?;
    let mut permissions = fs::metadata(&partial)
        .map_err(|error| format!("读取 AppImage 权限失败：{error}"))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&partial, permissions)
        .map_err(|error| format!("设置 AppImage 可执行权限失败：{error}"))?;
    fs::rename(&partial, &destination).map_err(|error| format!("提交 AppImage 失败：{error}"))?;
    let desktop_detail = write_linux_desktop_entry(&destination)
        .unwrap_or_else(|error| format!("desktop entry warning: {error}"));
    Ok(InstallerOutcome {
        ok: true,
        detail: format!("installed {}; {desktop_detail}", destination.display()),
        restart_required: false,
    })
}

#[cfg(target_os = "linux")]
fn write_linux_desktop_entry(executable: &Path) -> Result<String, String> {
    let applications = dirs::data_local_dir()
        .ok_or_else(|| "无法定位用户应用数据目录".to_string())?
        .join("applications");
    fs::create_dir_all(&applications)
        .map_err(|error| format!("创建 applications 目录失败：{error}"))?;
    let entry = applications.join("ai.vigils.desktop.desktop");
    let executable = executable.display().to_string().replace('"', "\\\"");
    let content = format!(
        "[Desktop Entry]\nType=Application\nName=Vigils\nExec=\"{executable}\"\nTerminal=false\nCategories=Utility;Security;\n"
    );
    fs::write(&entry, content).map_err(|error| format!("写入 desktop entry 失败：{error}"))?;
    Ok(format!("desktop entry {}", entry.display()))
}

#[cfg(target_os = "linux")]
fn remove_linux_desktop_entry() {
    if let Some(data) = dirs::data_local_dir() {
        let _ = fs::remove_file(data.join("applications").join("ai.vigils.desktop.desktop"));
    }
}

#[cfg(target_os = "linux")]
fn run_privileged_package_command(
    program: &str,
    args: &[&str],
    path: &Path,
    package_type: &str,
) -> Result<InstallerOutcome, String> {
    let output = Command::new("pkexec")
        .arg(program)
        .args(args)
        .arg(path)
        .output()
        .map_err(|error| {
            format!("无法启动 {package_type} 系统安装；请确认 PolicyKit/pkexec 可用：{error}")
        })?;
    Ok(InstallerOutcome {
        ok: output.status.success(),
        detail: combined_output(&output.stdout, &output.stderr),
        restart_required: false,
    })
}

#[cfg(target_os = "linux")]
fn run_privileged_uninstall_command(
    program: &str,
    args: &[&str],
    package_type: &str,
) -> Result<InstallerOutcome, String> {
    let output = Command::new("pkexec")
        .arg(program)
        .args(args)
        .output()
        .map_err(|error| {
            format!("无法启动 {package_type} 系统卸载；请确认 PolicyKit/pkexec 可用：{error}")
        })?;
    Ok(InstallerOutcome {
        ok: output.status.success(),
        detail: combined_output(&output.stdout, &output.stderr),
        restart_required: false,
    })
}

fn purge_user_data(steps: &mut Vec<OperationStep>) {
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
}

fn stop_runtime(
    installation: &Installation,
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
