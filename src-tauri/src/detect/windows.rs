use crate::models::{DetectedEntry, SourceType};
#[cfg(target_os = "windows")]
use crate::{detect::SoftwareDetector, error::InstallorResult};

#[cfg(target_os = "windows")]
pub struct WindowsDetector;

#[cfg(target_os = "windows")]
impl SoftwareDetector for WindowsDetector {
    fn detect(&self) -> InstallorResult<Vec<DetectedEntry>> {
        use winreg::{
            RegKey,
            enums::{
                HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
            },
        };
        let roots = [
            (HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_64KEY),
            (HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_32KEY),
            (HKEY_CURRENT_USER, KEY_READ),
        ];
        let path = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
        let mut found = Vec::new();
        for (root, flags) in roots {
            let Ok(uninstall) = RegKey::predef(root).open_subkey_with_flags(path, flags) else {
                continue;
            };
            for key_name in uninstall.enum_keys().flatten() {
                let Ok(key) = uninstall.open_subkey_with_flags(&key_name, flags) else {
                    continue;
                };
                let name: String = key.get_value("DisplayName").unwrap_or_default();
                let system_component: u32 = key.get_value("SystemComponent").unwrap_or(0);
                let parent: String = key.get_value("ParentKeyName").unwrap_or_default();
                if name.trim().is_empty() || system_component == 1 || !parent.is_empty() {
                    continue;
                }
                found.push(DetectedEntry {
                    name,
                    version: key.get_value("DisplayVersion").unwrap_or_default(),
                    publisher: key.get_value("Publisher").unwrap_or_default(),
                    installed_path: key.get_value("InstallLocation").unwrap_or_default(),
                    source_type: SourceType::Custom,
                    source_id: String::new(),
                });
            }
        }
        found.sort_by_key(|a| a.name.to_lowercase());
        found.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name) && a.version == b.version);
        Ok(found)
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn parse_fixture(input: &str) -> Vec<DetectedEntry> {
    input
        .lines()
        .filter_map(|line| {
            let p: Vec<_> = line.split('\t').collect();
            if p.first().is_none_or(|v| v.trim().is_empty())
                || p.get(4) == Some(&"1")
                || p.get(5).is_some_and(|v| !v.is_empty())
            {
                return None;
            }
            Some(DetectedEntry {
                name: p[0].into(),
                version: p.get(1).unwrap_or(&"").to_string(),
                publisher: p.get(2).unwrap_or(&"").to_string(),
                installed_path: p.get(3).unwrap_or(&"").to_string(),
                source_type: SourceType::Custom,
                source_id: String::new(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filters_components() {
        assert_eq!(
            parse_fixture("Visible\t1\tPub\tC:\\\t0\t\nHidden\t1\tPub\t\t1\t").len(),
            1
        );
    }
}

#[cfg(all(test, target_os = "windows"))]
mod host_tests {
    use super::*;
    use crate::detect::SoftwareDetector;
    #[test]
    fn detects_real_windows_software() {
        assert!(!WindowsDetector.detect().unwrap().is_empty());
    }
}
