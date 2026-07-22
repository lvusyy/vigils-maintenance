use crate::model::Installation;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(not(windows))]
use std::process::Command;
use sysinfo::{Process, System};

const PROCESS_FILE_NAMES: &[&str] = &[
    "gui",
    "Vigils",
    "vigils",
    "vigil-desktop-gui",
    "vigil-hub",
    "vigil-native-host",
    "gui.exe",
    "Vigils.exe",
    "vigil-desktop-gui.exe",
    "vigil-hub.exe",
    "vigil-native-host.exe",
    "Vigils.AppImage",
];

pub fn current_platform() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unsupported"
    }
}

pub fn installer_filter_name() -> &'static str {
    if cfg!(windows) {
        "Windows installer"
    } else if cfg!(target_os = "macos") {
        "macOS disk image"
    } else if cfg!(target_os = "linux") {
        "Linux package"
    } else {
        "Vigils package"
    }
}

pub fn supported_package_extensions() -> &'static [&'static str] {
    if cfg!(windows) {
        &["exe", "msi"]
    } else if cfg!(target_os = "macos") {
        &["dmg"]
    } else if cfg!(target_os = "linux") {
        &["AppImage", "deb", "rpm"]
    } else {
        &[]
    }
}

pub fn supported_package_description() -> String {
    supported_package_extensions()
        .iter()
        .map(|extension| format!(".{extension}"))
        .collect::<Vec<_>>()
        .join(" / ")
}

pub fn package_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
}

pub fn is_supported_package_extension(extension: &str) -> bool {
    supported_package_extensions()
        .iter()
        .any(|supported| supported.eq_ignore_ascii_case(extension))
}

pub fn is_supported_package(path: &Path) -> bool {
    package_extension(path).is_some_and(|extension| is_supported_package_extension(&extension))
}

pub fn detect_installation() -> Installation {
    let mut installation = platform_installation().unwrap_or_default();
    installation.platform = current_platform().into();
    installation.supported_packages = supported_package_extensions()
        .iter()
        .map(|extension| format!(".{extension}"))
        .collect();

    let data_dir = dirs::data_local_dir().map(|path| path.join("Vigil"));
    installation.data_location = data_dir.as_ref().map(|path| path.display().to_string());

    let mut roots = known_install_roots();
    if let Some(path) = installation.install_location.as_deref() {
        roots.insert(0, PathBuf::from(path));
    }

    if installation.app_executable.is_none() {
        installation.app_executable =
            find_app_executable(&roots).map(|path| path.display().to_string());
    }
    if installation.install_location.is_none() {
        installation.install_location = installation
            .app_executable
            .as_deref()
            .and_then(install_location_from_executable)
            .map(|path| path.display().to_string());
    }
    if installation.install_type.is_none() {
        installation.install_type = infer_install_type(&installation);
    }

    if installation.hub_executable.is_none() {
        let mut hub_roots = roots;
        if let Some(data) = data_dir {
            hub_roots.push(data.join("bin"));
        }
        add_user_binary_roots(&mut hub_roots);
        installation.hub_executable =
            find_hub_executable(&hub_roots).map(|path| path.display().to_string());
    }

    installation.uninstall_supported = uninstall_supported(&installation);
    installation.running_processes = running_vigil_processes(&installation);
    installation.installed = installation.uninstall_command.is_some()
        || installation.app_executable.is_some()
        || installation.hub_executable.is_some();
    installation
}

#[cfg(windows)]
fn known_install_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(local) = dirs::data_local_dir() {
        roots.push(local.join("Vigils"));
        roots.push(local.join("Vigil Desktop"));
        roots.push(local.join("ai.vigils.desktop"));
        roots.push(local.join("Programs").join("Vigils"));
    }
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        roots.push(PathBuf::from(program_files).join("Vigils"));
    }
    roots
}

#[cfg(target_os = "macos")]
fn known_install_roots() -> Vec<PathBuf> {
    mac_app_bundles()
        .into_iter()
        .map(|bundle| bundle.join("Contents").join("MacOS"))
        .collect()
}

#[cfg(target_os = "linux")]
fn known_install_roots() -> Vec<PathBuf> {
    let mut roots = vec![
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/opt/Vigils"),
    ];
    if let Some(home) = dirs::home_dir() {
        roots.insert(0, home.join(".local").join("bin"));
        roots.insert(1, home.join("Applications"));
    }
    roots
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn known_install_roots() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(not(windows))]
fn add_user_binary_roots(roots: &mut Vec<PathBuf>) {
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".local").join("bin"));
        roots.push(PathBuf::from("/usr/local/bin"));
        roots.push(PathBuf::from("/usr/bin"));
    }
}

#[cfg(windows)]
fn add_user_binary_roots(_roots: &mut Vec<PathBuf>) {}

fn app_file_names() -> &'static [&'static str] {
    if cfg!(windows) {
        &["gui.exe", "Vigils.exe", "vigil-desktop-gui.exe"]
    } else if cfg!(target_os = "macos") {
        &["gui", "Vigils", "vigil-desktop-gui"]
    } else {
        &["Vigils.AppImage", "vigils", "gui", "vigil-desktop-gui"]
    }
}

fn hub_file_names() -> &'static [&'static str] {
    if cfg!(windows) {
        &["vigil-hub.exe"]
    } else {
        &["vigil-hub"]
    }
}

fn find_app_executable(roots: &[PathBuf]) -> Option<PathBuf> {
    find_file(roots, app_file_names())
}

fn find_hub_executable(roots: &[PathBuf]) -> Option<PathBuf> {
    find_file(roots, hub_file_names())
}

fn find_file(roots: &[PathBuf], names: &[&str]) -> Option<PathBuf> {
    for root in roots {
        for name in names {
            let candidate = root.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn install_location_from_executable(executable: &str) -> Option<PathBuf> {
    let path = Path::new(executable);
    #[cfg(target_os = "macos")]
    if let Some(bundle) = path.ancestors().find(|ancestor| {
        ancestor
            .extension()
            .is_some_and(|value| value.to_string_lossy().eq_ignore_ascii_case("app"))
    }) {
        return Some(bundle.to_path_buf());
    }
    path.parent().map(Path::to_path_buf)
}

fn infer_install_type(installation: &Installation) -> Option<String> {
    if cfg!(windows) {
        Some("windows-portable".into())
    } else if cfg!(target_os = "macos") {
        installation
            .install_location
            .as_deref()
            .filter(|path| path.to_ascii_lowercase().ends_with(".app"))
            .map(|_| "macos-app".into())
    } else if cfg!(target_os = "linux") {
        installation
            .app_executable
            .as_deref()
            .and_then(|path| Path::new(path).extension())
            .and_then(|extension| extension.to_str())
            .filter(|extension| extension.eq_ignore_ascii_case("appimage"))
            .map(|_| "linux-appimage".into())
    } else {
        None
    }
}

fn uninstall_supported(installation: &Installation) -> bool {
    match installation.install_type.as_deref() {
        Some("windows-registry") => installation.uninstall_command.is_some(),
        Some("macos-app" | "linux-appimage" | "linux-deb" | "linux-rpm") => true,
        _ => false,
    }
}

pub fn running_vigil_processes(installation: &Installation) -> Vec<String> {
    let system = System::new_all();
    matching_vigil_processes(&system, installation)
        .into_iter()
        .map(process_label)
        .collect()
}

pub fn stop_vigil_processes(installation: &Installation) -> Vec<(String, bool, String)> {
    let system = System::new_all();
    matching_vigil_processes(&system, installation)
        .into_iter()
        .map(|process| {
            let name = process_label(process);
            let stopped = process.kill();
            let detail = if stopped {
                "terminated exact executable path"
            } else {
                "failed to terminate exact executable path"
            };
            (name, stopped, detail.into())
        })
        .collect()
}

fn matching_vigil_processes<'a>(
    system: &'a System,
    installation: &Installation,
) -> Vec<&'a Process> {
    let trusted_paths = trusted_process_paths(installation);
    if trusted_paths.is_empty() {
        return Vec::new();
    }

    system
        .processes()
        .values()
        .filter(|process| {
            process
                .exe()
                .is_some_and(|path| executable_matches(path, &trusted_paths))
        })
        .collect()
}

fn process_label(process: &Process) -> String {
    format!("{} ({})", process.name().to_string_lossy(), process.pid())
}

fn trusted_process_paths(installation: &Installation) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for executable in [
        installation.app_executable.as_deref(),
        installation.hub_executable.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        push_existing_path(&mut paths, Path::new(executable));
    }

    let mut roots = Vec::new();
    if let Some(location) = installation.install_location.as_deref() {
        roots.push(PathBuf::from(location));
    }
    if let Some(data) = installation.data_location.as_deref() {
        roots.push(PathBuf::from(data).join("bin"));
    }
    for root in roots {
        for file_name in PROCESS_FILE_NAMES {
            push_existing_path(&mut paths, &root.join(file_name));
        }
    }
    paths
}

fn push_existing_path(paths: &mut Vec<PathBuf>, candidate: &Path) {
    let Ok(canonical) = fs::canonicalize(candidate) else {
        return;
    };
    if canonical.is_file() && !paths.iter().any(|path| paths_equal(path, &canonical)) {
        paths.push(canonical);
    }
}

fn executable_matches(candidate: &Path, trusted_paths: &[PathBuf]) -> bool {
    let Ok(candidate) = fs::canonicalize(candidate) else {
        return false;
    };
    trusted_paths
        .iter()
        .any(|trusted| paths_equal(trusted, &candidate))
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}

#[cfg(windows)]
pub fn split_windows_command_line(input: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut quoted = false;

    while chars.peek().is_some() {
        while !quoted
            && chars
                .peek()
                .is_some_and(|character| character.is_whitespace())
        {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        current.clear();
        let mut backslashes = 0usize;
        for character in chars.by_ref() {
            match character {
                '\\' => backslashes += 1,
                '"' => {
                    current.extend(std::iter::repeat('\\').take(backslashes / 2));
                    if backslashes % 2 == 0 {
                        quoted = !quoted;
                    } else {
                        current.push('"');
                    }
                    backslashes = 0;
                }
                character if character.is_whitespace() && !quoted => {
                    current.extend(std::iter::repeat('\\').take(backslashes));
                    backslashes = 0;
                    break;
                }
                character => {
                    current.extend(std::iter::repeat('\\').take(backslashes));
                    backslashes = 0;
                    current.push(character);
                }
            }
        }
        current.extend(std::iter::repeat('\\').take(backslashes));
        if !current.is_empty() {
            args.push(current.clone());
        }
    }
    if quoted {
        return Err("uninstall command contains an unmatched quote".into());
    }
    if args.is_empty() {
        return Err("uninstall command is empty".into());
    }
    Ok(args)
}

#[cfg(windows)]
fn platform_installation() -> Option<Installation> {
    use winreg::enums::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    };
    use winreg::RegKey;

    let roots = [
        RegKey::predef(HKEY_CURRENT_USER),
        RegKey::predef(HKEY_LOCAL_MACHINE),
    ];
    let views = [KEY_READ | KEY_WOW64_64KEY, KEY_READ | KEY_WOW64_32KEY];
    for root in roots {
        for view in views {
            let Ok(uninstall) = root.open_subkey_with_flags(
                "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
                view,
            ) else {
                continue;
            };
            for key_name in uninstall.enum_keys().filter_map(Result::ok) {
                let Ok(key) = uninstall.open_subkey_with_flags(&key_name, view) else {
                    continue;
                };
                let display_name: String = key.get_value("DisplayName").unwrap_or_default();
                if !is_vigils_registry_entry(&key_name, &display_name) {
                    continue;
                }
                let install_location: String = key.get_value("InstallLocation").unwrap_or_default();
                let uninstall_command: String = key
                    .get_value("QuietUninstallString")
                    .or_else(|_| key.get_value("UninstallString"))
                    .unwrap_or_default();
                return Some(Installation {
                    installed: true,
                    install_type: Some("windows-registry".into()),
                    version: nonempty(key.get_value("DisplayVersion").unwrap_or_default()),
                    display_name: nonempty(display_name),
                    install_location: nonempty(install_location),
                    uninstall_command: nonempty(uninstall_command),
                    ..Installation::default()
                });
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn platform_installation() -> Option<Installation> {
    for bundle in mac_app_bundles() {
        if !bundle.is_dir() {
            continue;
        }
        let executable = find_file(
            &[bundle.join("Contents").join("MacOS")],
            &["gui", "Vigils", "vigil-desktop-gui"],
        );
        let version = mac_bundle_version(&bundle);
        return Some(Installation {
            installed: true,
            install_type: Some("macos-app".into()),
            version,
            display_name: Some("Vigils".into()),
            install_location: Some(bundle.display().to_string()),
            app_executable: executable.map(|path| path.display().to_string()),
            ..Installation::default()
        });
    }
    None
}

#[cfg(target_os = "macos")]
fn mac_app_bundles() -> Vec<PathBuf> {
    let mut bundles = vec![
        PathBuf::from("/Applications/Vigils.app"),
        PathBuf::from("/Applications/Vigil Desktop.app"),
    ];
    if let Some(home) = dirs::home_dir() {
        bundles.insert(0, home.join("Applications").join("Vigils.app"));
        bundles.insert(1, home.join("Applications").join("Vigil Desktop.app"));
    }
    bundles
}

#[cfg(target_os = "macos")]
fn mac_bundle_version(bundle: &Path) -> Option<String> {
    let info = bundle.join("Contents").join("Info.plist");
    let output = Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", "Print :CFBundleShortVersionString"])
        .arg(info)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    nonempty(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(target_os = "linux")]
fn platform_installation() -> Option<Installation> {
    if let Some(version) = query_package_version("dpkg-query", &["-W", "-f=${Version}", "vigils"]) {
        return Some(Installation {
            installed: true,
            install_type: Some("linux-deb".into()),
            version: Some(normalize_linux_package_version(&version)),
            display_name: Some("Vigils".into()),
            ..Installation::default()
        });
    }
    if let Some(version) = query_package_version("rpm", &["-q", "--qf", "%{VERSION}", "vigils"]) {
        return Some(Installation {
            installed: true,
            install_type: Some("linux-rpm".into()),
            version: Some(normalize_linux_package_version(&version)),
            display_name: Some("Vigils".into()),
            ..Installation::default()
        });
    }
    None
}

#[cfg(target_os = "linux")]
fn query_package_version(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    nonempty(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(target_os = "linux")]
fn normalize_linux_package_version(version: &str) -> String {
    version
        .rsplit_once(':')
        .map_or(version, |(_, value)| value)
        .split('-')
        .next()
        .unwrap_or(version)
        .to_string()
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn platform_installation() -> Option<Installation> {
    None
}

fn nonempty(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(windows)]
fn is_vigils_registry_entry(key_name: &str, display_name: &str) -> bool {
    key_name.eq_ignore_ascii_case("ai.vigils.desktop")
        || display_name.trim().eq_ignore_ascii_case("Vigils")
        || display_name.trim().eq_ignore_ascii_case("Vigil Desktop")
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn validated_removable_install_path(installation: &Installation) -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    let candidate = installation
        .install_location
        .as_deref()
        .or(installation.app_executable.as_deref())
        .ok_or_else(|| "安装记录缺少可移除路径".to_string())?;
    #[cfg(target_os = "linux")]
    let candidate = installation
        .app_executable
        .as_deref()
        .or(installation.install_location.as_deref())
        .ok_or_else(|| "安装记录缺少可移除路径".to_string())?;
    let canonical =
        fs::canonicalize(candidate).map_err(|error| format!("无法验证安装路径：{error}"))?;
    let allowed = removable_install_paths();
    if allowed.iter().any(|path| {
        fs::canonicalize(path).is_ok_and(|allowed_path| paths_equal(&allowed_path, &canonical))
    }) {
        Ok(canonical)
    } else {
        Err(format!(
            "拒绝移除非标准 Vigils 路径：{}",
            canonical.display()
        ))
    }
}

#[cfg(target_os = "macos")]
fn removable_install_paths() -> Vec<PathBuf> {
    mac_app_bundles()
}

#[cfg(target_os = "linux")]
fn removable_install_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".local").join("bin").join("Vigils.AppImage"));
        paths.push(home.join("Applications").join("Vigils.AppImage"));
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_extension_check_is_case_insensitive() {
        let extension = supported_package_extensions().first().unwrap();
        assert!(is_supported_package_extension(extension));
        assert!(is_supported_package_extension(
            &extension.to_ascii_uppercase()
        ));
        assert!(!is_supported_package_extension("zip"));
    }

    #[cfg(windows)]
    #[test]
    fn parses_quoted_uninstall_command() {
        let parsed = split_windows_command_line(
            r#""C:\Program Files\Vigils\uninstall.exe" /S "value with spaces""#,
        )
        .unwrap();
        assert_eq!(parsed[0], r#"C:\Program Files\Vigils\uninstall.exe"#);
        assert_eq!(parsed[1], "/S");
        assert_eq!(parsed[2], "value with spaces");
    }

    #[cfg(windows)]
    #[test]
    fn rejects_unmatched_quote() {
        assert!(split_windows_command_line(r#""C:\broken.exe"#).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn registry_match_accepts_only_known_product_identity() {
        assert!(is_vigils_registry_entry("ai.vigils.desktop", "Anything"));
        assert!(is_vigils_registry_entry("random-key", "Vigils"));
        assert!(is_vigils_registry_entry("random-key", "Vigil Desktop"));
        assert!(!is_vigils_registry_entry(
            "unrelated-vigil-tool",
            "Vigil Monitor"
        ));
        assert!(!is_vigils_registry_entry(
            "ai.vigils.maintenance",
            "Vigils Maintenance"
        ));
    }

    #[test]
    fn executable_match_requires_the_same_file_path() {
        let trusted_dir = tempfile::tempdir().unwrap();
        let unrelated_dir = tempfile::tempdir().unwrap();
        let executable_name = if cfg!(windows) { "gui.exe" } else { "gui" };
        let trusted = trusted_dir.path().join(executable_name);
        let unrelated = unrelated_dir.path().join(executable_name);
        fs::write(&trusted, b"trusted").unwrap();
        fs::write(&unrelated, b"unrelated").unwrap();

        let trusted_paths = vec![fs::canonicalize(&trusted).unwrap()];
        assert!(executable_matches(&trusted, &trusted_paths));
        assert!(!executable_matches(&unrelated, &trusted_paths));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn normalizes_debian_package_version() {
        assert_eq!(normalize_linux_package_version("1:0.1.7-2"), "0.1.7");
    }
}
