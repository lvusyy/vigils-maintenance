use crate::model::Installation;
use std::fs;
use std::path::{Path, PathBuf};
use sysinfo::{Process, System};

const PROCESS_FILE_NAMES: &[&str] = &[
    "gui.exe",
    "Vigils.exe",
    "vigil-desktop-gui.exe",
    "vigil-hub.exe",
    "vigil-native-host.exe",
];

pub fn detect_installation() -> Installation {
    let mut installation = registry_installation().unwrap_or_default();
    let data_dir = dirs::data_local_dir().map(|p| p.join("Vigil"));
    installation.data_location = data_dir.as_ref().map(|p| p.display().to_string());

    let mut roots = Vec::new();
    if let Some(path) = installation.install_location.as_deref() {
        roots.push(PathBuf::from(path));
    }
    roots.extend(known_install_roots());

    if installation.app_executable.is_none() {
        installation.app_executable =
            find_file(&roots, &["gui.exe", "Vigils.exe", "vigil-desktop-gui.exe"])
                .map(|p| p.display().to_string());
    }
    if installation.hub_executable.is_none() {
        let mut hub_roots = roots.clone();
        if let Some(data) = data_dir {
            hub_roots.push(data.join("bin"));
        }
        installation.hub_executable =
            find_file(&hub_roots, &["vigil-hub.exe"]).map(|p| p.display().to_string());
    }

    installation.running_processes = running_vigil_processes(&installation);
    installation.installed = installation.uninstall_command.is_some()
        || installation.app_executable.is_some()
        || installation.hub_executable.is_some();
    installation
}

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

pub fn split_windows_command_line(input: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut quoted = false;

    while chars.peek().is_some() {
        while !quoted && chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        current.clear();
        let mut backslashes = 0usize;
        for ch in chars.by_ref() {
            match ch {
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
                c if c.is_whitespace() && !quoted => {
                    current.extend(std::iter::repeat('\\').take(backslashes));
                    backslashes = 0;
                    break;
                }
                c => {
                    current.extend(std::iter::repeat('\\').take(backslashes));
                    backslashes = 0;
                    current.push(c);
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
fn registry_installation() -> Option<Installation> {
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

#[cfg(not(windows))]
fn registry_installation() -> Option<Installation> {
    None
}

fn nonempty(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn is_vigils_registry_entry(key_name: &str, display_name: &str) -> bool {
    key_name.eq_ignore_ascii_case("ai.vigils.desktop")
        || display_name.trim().eq_ignore_ascii_case("Vigils")
        || display_name.trim().eq_ignore_ascii_case("Vigil Desktop")
}

pub fn is_supported_package(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|v| v.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("exe" | "msi")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn rejects_unmatched_quote() {
        assert!(split_windows_command_line(r#""C:\broken.exe"#).is_err());
    }

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
        let trusted = trusted_dir.path().join("gui.exe");
        let unrelated = unrelated_dir.path().join("gui.exe");
        fs::write(&trusted, b"trusted").unwrap();
        fs::write(&unrelated, b"unrelated").unwrap();

        let trusted_paths = vec![fs::canonicalize(&trusted).unwrap()];
        assert!(executable_matches(&trusted, &trusted_paths));
        assert!(!executable_matches(&unrelated, &trusted_paths));
    }
}
