use crate::{
    detect::SoftwareDetector,
    error::{InstallorError, InstallorResult},
    models::{DetectedEntry, SourceType},
};

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub struct MacOsDetector;

impl SoftwareDetector for MacOsDetector {
    fn detect(&self) -> InstallorResult<Vec<DetectedEntry>> {
        #[cfg(target_os = "macos")]
        {
            let out = std::process::Command::new("system_profiler")
                .args(["SPApplicationsDataType", "-json"])
                .output()?;
            if !out.status.success() {
                return Err(InstallorError::Operation(
                    String::from_utf8_lossy(&out.stderr).into_owned(),
                ));
            }
            return parse_system_profiler(&String::from_utf8_lossy(&out.stdout));
        }
        #[cfg(not(target_os = "macos"))]
        Ok(Vec::new())
    }
}

#[cfg_attr(not(any(test, target_os = "macos")), allow(dead_code))]
pub fn parse_system_profiler(input: &str) -> InstallorResult<Vec<DetectedEntry>> {
    let root: serde_json::Value = serde_json::from_str(input)?;
    let apps = root
        .get("SPApplicationsDataType")
        .and_then(|v| v.as_array())
        .ok_or_else(|| InstallorError::Operation("missing SPApplicationsDataType array".into()))?;
    Ok(apps
        .iter()
        .filter_map(|a| {
            Some(DetectedEntry {
                name: a.get("_name")?.as_str()?.into(),
                version: a
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .into(),
                publisher: a
                    .get("obtained_from")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .into(),
                installed_path: a
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .into(),
                source_type: SourceType::Custom,
                source_id: String::new(),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_fixture() {
        let v=parse_system_profiler(r#"{"SPApplicationsDataType":[{"_name":"Pages","version":"14","path":"/Applications/Pages.app"}]}"#).unwrap();
        assert_eq!(v[0].name, "Pages");
    }
}
