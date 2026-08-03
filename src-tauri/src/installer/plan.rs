use crate::{
    error::{InstallorError, InstallorResult},
    models::{CatalogEntry, InstallerType, SourceType},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "kind")]
pub enum InstallPlan {
    Shell {
        cmd: String,
        args: Vec<String>,
    },
    MountAndCopy {
        dmg_path: String,
        target_dir: Option<String>,
    },
    MakeExecutableAndLaunch {
        path: String,
    },
    LaunchInteractive {
        path: String,
    },
}

fn extra(value: &str) -> InstallorResult<Vec<String>> {
    shell_words::split(value)
        .map_err(|e| InstallorError::InvalidConfig(format!("invalid silent arguments: {e}")))
}

pub fn build_install_plan(
    entry: &CatalogEntry,
    installer_path: Option<&str>,
) -> InstallorResult<InstallPlan> {
    let id = &entry.source_id;
    let plan = match entry.source_type {
        SourceType::Winget => InstallPlan::Shell {
            cmd: "winget".into(),
            args: vec![
                "install".into(),
                "--id".into(),
                id.into(),
                "--exact".into(),
                "--accept-package-agreements".into(),
                "--accept-source-agreements".into(),
            ],
        },
        SourceType::Brew => InstallPlan::Shell {
            cmd: "brew".into(),
            args: vec!["install".into(), id.into()],
        },
        SourceType::Apt => InstallPlan::Shell {
            cmd: "sudo".into(),
            args: vec!["apt-get".into(), "install".into(), "-y".into(), id.into()],
        },
        SourceType::Dnf => InstallPlan::Shell {
            cmd: "sudo".into(),
            args: vec!["dnf".into(), "install".into(), "-y".into(), id.into()],
        },
        SourceType::Pacman => InstallPlan::Shell {
            cmd: "sudo".into(),
            args: vec![
                "pacman".into(),
                "-S".into(),
                "--noconfirm".into(),
                id.into(),
            ],
        },
        SourceType::Flatpak => InstallPlan::Shell {
            cmd: "flatpak".into(),
            args: vec!["install".into(), "-y".into(), "flathub".into(), id.into()],
        },
        SourceType::Custom => {
            let path = installer_path.filter(|p| !p.is_empty()).ok_or_else(|| {
                InstallorError::InvalidConfig(format!(
                    "{} needs a downloaded or confirmed installer file",
                    entry.name
                ))
            })?;
            match entry.installer_type {
                InstallerType::Msi => {
                    let mut args =
                        vec!["/i".into(), path.into(), "/qn".into(), "/norestart".into()];
                    args.extend(extra(&entry.silent_args)?);
                    InstallPlan::Shell {
                        cmd: "msiexec".into(),
                        args,
                    }
                }
                InstallerType::Inno => {
                    let mut args = if entry.silent_args.is_empty() {
                        vec![
                            "/VERYSILENT".into(),
                            "/SUPPRESSMSGBOXES".into(),
                            "/NORESTART".into(),
                            "/SP-".into(),
                        ]
                    } else {
                        extra(&entry.silent_args)?
                    };
                    InstallPlan::Shell {
                        cmd: path.into(),
                        args: std::mem::take(&mut args),
                    }
                }
                InstallerType::Nsis => InstallPlan::Shell {
                    cmd: path.into(),
                    args: if entry.silent_args.is_empty() {
                        vec!["/S".into()]
                    } else {
                        extra(&entry.silent_args)?
                    },
                },
                InstallerType::Pkg => {
                    let mut args = vec!["-pkg".into(), path.into(), "-target".into(), "/".into()];
                    args.extend(extra(&entry.silent_args)?);
                    InstallPlan::Shell {
                        cmd: "installer".into(),
                        args,
                    }
                }
                InstallerType::Dmg => InstallPlan::MountAndCopy {
                    dmg_path: path.into(),
                    target_dir: None,
                },
                InstallerType::Deb => InstallPlan::Shell {
                    cmd: "sudo".into(),
                    args: vec!["dpkg".into(), "-i".into(), path.into()],
                },
                InstallerType::Rpm => InstallPlan::Shell {
                    cmd: "sudo".into(),
                    args: vec!["rpm".into(), "-Uvh".into(), path.into()],
                },
                InstallerType::AppImage => {
                    InstallPlan::MakeExecutableAndLaunch { path: path.into() }
                }
                InstallerType::Other => InstallPlan::LaunchInteractive { path: path.into() },
            }
        }
    };
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn e(source: SourceType, kind: InstallerType) -> CatalogEntry {
        CatalogEntry {
            source_type: source,
            installer_type: kind,
            source_id: "pkg.id".into(),
            name: "Test".into(),
            ..Default::default()
        }
    }
    #[test]
    fn exact_native_argv() {
        assert_eq!(
            build_install_plan(&e(SourceType::Winget, InstallerType::Other), None).unwrap(),
            InstallPlan::Shell {
                cmd: "winget".into(),
                args: vec![
                    "install",
                    "--id",
                    "pkg.id",
                    "--exact",
                    "--accept-package-agreements",
                    "--accept-source-agreements"
                ]
                .into_iter()
                .map(String::from)
                .collect()
            }
        );
    }
    #[test]
    fn exact_msi_argv() {
        assert_eq!(
            build_install_plan(&e(SourceType::Custom, InstallerType::Msi), Some("x.msi")).unwrap(),
            InstallPlan::Shell {
                cmd: "msiexec".into(),
                args: vec!["/i", "x.msi", "/qn", "/norestart"]
                    .into_iter()
                    .map(String::from)
                    .collect()
            }
        );
    }
    #[test]
    fn covers_all_custom_types() {
        for kind in [
            InstallerType::Msi,
            InstallerType::Inno,
            InstallerType::Nsis,
            InstallerType::Pkg,
            InstallerType::Dmg,
            InstallerType::Deb,
            InstallerType::Rpm,
            InstallerType::AppImage,
            InstallerType::Other,
        ] {
            assert!(build_install_plan(&e(SourceType::Custom, kind), Some("file")).is_ok());
        }
    }
}
