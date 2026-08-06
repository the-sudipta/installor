use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Winget,
    Brew,
    Apt,
    Dnf,
    Pacman,
    Flatpak,
    #[default]
    Custom,
}

impl fmt::Display for SourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Winget => "winget",
                Self::Brew => "brew",
                Self::Apt => "apt",
                Self::Dnf => "dnf",
                Self::Pacman => "pacman",
                Self::Flatpak => "flatpak",
                Self::Custom => "custom",
            }
        )
    }
}

impl FromStr for SourceType {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "winget" => Ok(Self::Winget),
            "brew" => Ok(Self::Brew),
            "apt" | "dpkg" => Ok(Self::Apt),
            "dnf" | "rpm" => Ok(Self::Dnf),
            "pacman" => Ok(Self::Pacman),
            "flatpak" => Ok(Self::Flatpak),
            "custom" | "" => Ok(Self::Custom),
            other => Err(format!("unknown source type: {other}")),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InstallerType {
    Msi,
    Inno,
    Nsis,
    Pkg,
    Dmg,
    Deb,
    Rpm,
    #[serde(rename = "appimage")]
    AppImage,
    #[default]
    Other,
}

impl fmt::Display for InstallerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Msi => "msi",
                Self::Inno => "inno",
                Self::Nsis => "nsis",
                Self::Pkg => "pkg",
                Self::Dmg => "dmg",
                Self::Deb => "deb",
                Self::Rpm => "rpm",
                Self::AppImage => "appimage",
                Self::Other => "other",
            }
        )
    }
}

impl FromStr for InstallerType {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "msi" => Ok(Self::Msi),
            "inno" => Ok(Self::Inno),
            "nsis" => Ok(Self::Nsis),
            "pkg" => Ok(Self::Pkg),
            "dmg" => Ok(Self::Dmg),
            "deb" => Ok(Self::Deb),
            "rpm" => Ok(Self::Rpm),
            "appimage" | "app_image" => Ok(Self::AppImage),
            "other" | "" => Ok(Self::Other),
            other => Err(format!("unknown installer type: {other}")),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub category: String,
    pub platforms: String,
    pub source_type: SourceType,
    pub source_id: String,
    pub download_url: String,
    pub installer_type: InstallerType,
    pub silent_args: String,
    pub checksum_sha256: String,
    pub tags: String,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct InstallationEntry {
    pub id: String,
    pub software_id: String,
    pub machine_id: String,
    pub machine_label: String,
    pub installed: bool,
    pub installed_version: String,
    pub installed_path: String,
    pub installer_file_path: String,
    pub last_verified_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct DetectedEntry {
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub installed_path: String,
    pub source_type: SourceType,
    pub source_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AppSettings {
    pub public_sheet_url: String,
    pub apps_script_url: String,
    pub download_folder: String,
    pub machine_label: String,
    pub recursive_installer_search: bool,
    pub has_shared_secret: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct DeleteMarker {
    pub id: String,
    pub deleted_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CatalogBundle {
    pub catalog: Vec<CatalogEntry>,
    pub installations: Vec<InstallationEntry>,
    #[serde(default)]
    pub deleted_catalog: Vec<DeleteMarker>,
    #[serde(default)]
    pub deleted_installations: Vec<DeleteMarker>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SyncReport {
    pub pulled_catalog: usize,
    pub pulled_installations: usize,
    pub removed_catalog: usize,
    pub removed_installations: usize,
    pub pushed_catalog: usize,
    pub pushed_installations: usize,
    pub pushed_deletions: usize,
    pub pending_after: usize,
    pub last_synced_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SyncResult {
    pub bundle: CatalogBundle,
    pub report: SyncReport,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct BulkAddResult {
    pub bundle: CatalogBundle,
    pub catalog_added: usize,
    pub installations_recorded: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AppsScriptSetup {
    pub script: String,
    pub secret_created: bool,
    pub source_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SyncState {
    pub configured: bool,
    pub pending_changes: usize,
    pub last_synced_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PingResponse {
    pub ok: bool,
    #[serde(default)]
    pub sheet_name: String,
    #[serde(default)]
    pub catalog_count: usize,
    #[serde(default)]
    pub installations_count: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DownloadProgress {
    pub software_id: String,
    pub downloaded: u64,
    pub total: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DownloadOutcome {
    pub software_id: String,
    pub path: String,
    pub checksum_ok: bool,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct InstallOutcome {
    pub success: bool,
    pub launched: bool,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstallRun {
    pub software_id: String,
    pub outcome: Option<InstallOutcome>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileSuggestion {
    pub software_id: String,
    pub path: String,
    pub score: f64,
}

pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}
