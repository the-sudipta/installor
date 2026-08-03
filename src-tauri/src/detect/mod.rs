pub mod linux;
pub mod macos;
pub mod windows;

use crate::{error::InstallorResult, models::DetectedEntry};

pub trait SoftwareDetector {
    fn detect(&self) -> InstallorResult<Vec<DetectedEntry>>;
}

pub fn detect_installed_software() -> InstallorResult<Vec<DetectedEntry>> {
    #[cfg(target_os = "windows")]
    {
        return windows::WindowsDetector.detect();
    }
    #[cfg(target_os = "macos")]
    {
        return macos::MacOsDetector.detect();
    }
    #[cfg(target_os = "linux")]
    {
        return linux::LinuxDetector.detect();
    }
    #[allow(unreachable_code)]
    Ok(Vec::new())
}
