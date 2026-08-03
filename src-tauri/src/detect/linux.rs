use crate::{
    detect::SoftwareDetector,
    error::InstallorResult,
    models::{DetectedEntry, SourceType},
};
#[cfg(target_os = "linux")]
use std::process::Command;

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub struct LinuxDetector;
impl SoftwareDetector for LinuxDetector {
    fn detect(&self) -> InstallorResult<Vec<DetectedEntry>> {
        #[cfg(target_os = "linux")]
        {
            let specs = [
                (
                    "dpkg-query",
                    vec!["-W", "-f=${Package}\t${Version}\t${Maintainer}\n"],
                    SourceType::Apt,
                    "dpkg",
                ),
                (
                    "rpm",
                    vec!["-qa", "--qf", "%{NAME}\t%{VERSION}\t%{VENDOR}\n"],
                    SourceType::Dnf,
                    "rpm",
                ),
                ("pacman", vec!["-Q"], SourceType::Pacman, "pacman"),
                (
                    "flatpak",
                    vec!["list", "--app", "--columns=application,version,origin"],
                    SourceType::Flatpak,
                    "flatpak",
                ),
                ("snap", vec!["list"], SourceType::Custom, "snap"),
            ];
            let mut all = Vec::new();
            for (bin, args, source, format) in specs {
                let available = Command::new("sh")
                    .args(["-c", &format!("command -v {bin}")])
                    .output()
                    .is_ok_and(|o| o.status.success());
                if !available {
                    continue;
                }
                if let Ok(out) = Command::new(bin).args(args).output() {
                    if out.status.success() {
                        all.extend(parse_packages(
                            &String::from_utf8_lossy(&out.stdout),
                            source,
                            format,
                        ));
                    }
                }
            }
            return Ok(all);
        }
        #[cfg(not(target_os = "linux"))]
        Ok(Vec::new())
    }
}

#[cfg_attr(not(any(test, target_os = "linux")), allow(dead_code))]
pub fn parse_packages(input: &str, source: SourceType, format: &str) -> Vec<DetectedEntry> {
    input
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            if format == "snap" && i == 0 && line.starts_with("Name") {
                return None;
            }
            let p: Vec<&str> = if line.contains('\t') {
                line.split('\t').collect()
            } else {
                line.split_whitespace().collect()
            };
            if p.len() < 2 {
                return None;
            }
            Some(DetectedEntry {
                name: p[0].into(),
                version: p[1].into(),
                publisher: p.get(2).unwrap_or(&"").to_string(),
                installed_path: String::new(),
                source_type: source.clone(),
                source_id: p[0].into(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_dpkg_fixture() {
        let r = parse_packages("curl\t8.1\tDebian\n", SourceType::Apt, "dpkg");
        assert_eq!(r[0].source_id, "curl");
    }
}
