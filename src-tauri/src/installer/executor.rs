use crate::{
    error::{InstallorError, InstallorResult},
    installer::plan::InstallPlan,
    models::InstallOutcome,
};

pub async fn execute(plan: InstallPlan) -> InstallorResult<InstallOutcome> {
    match plan {
        InstallPlan::Shell { cmd, args } => {
            let out = tokio::process::Command::new(&cmd)
                .args(&args)
                .output()
                .await?;
            Ok(InstallOutcome {
                success: out.status.success(),
                launched: false,
                stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            })
        }
        InstallPlan::MakeExecutableAndLaunch { path } => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
            }
            tokio::process::Command::new(&path).spawn()?;
            Ok(InstallOutcome {
                success: true,
                launched: true,
                ..Default::default()
            })
        }
        InstallPlan::LaunchInteractive { path } => {
            open::that(path)?;
            Ok(InstallOutcome {
                success: true,
                launched: true,
                ..Default::default()
            })
        }
        InstallPlan::MountAndCopy {
            dmg_path,
            target_dir,
        } => install_dmg(&dmg_path, target_dir.as_deref()).await,
    }
}

#[cfg(target_os = "macos")]
async fn install_dmg(path: &str, target: Option<&str>) -> InstallorResult<InstallOutcome> {
    let mount = tempfile::tempdir()?;
    let attach = tokio::process::Command::new("hdiutil")
        .args([
            "attach",
            path,
            "-mountpoint",
            mount.path().to_str().unwrap(),
            "-nobrowse",
        ])
        .output()
        .await?;
    if !attach.status.success() {
        return Ok(InstallOutcome {
            success: false,
            stderr: String::from_utf8_lossy(&attach.stderr).into_owned(),
            ..Default::default()
        });
    }
    let app = walkdir::WalkDir::new(mount.path())
        .max_depth(2)
        .into_iter()
        .filter_map(Result::ok)
        .find(|e| e.path().extension().is_some_and(|x| x == "app"))
        .ok_or_else(|| InstallorError::Operation("DMG contains no .app bundle".into()))?;
    let dest = target.unwrap_or("/Applications");
    let copy = tokio::process::Command::new("ditto")
        .args([
            app.path().to_str().unwrap(),
            &format!("{dest}/{}", app.file_name().to_string_lossy()),
        ])
        .output()
        .await?;
    let _ = tokio::process::Command::new("hdiutil")
        .args(["detach", mount.path().to_str().unwrap()])
        .output()
        .await;
    Ok(InstallOutcome {
        success: copy.status.success(),
        stdout: String::from_utf8_lossy(&copy.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&copy.stderr).into_owned(),
        launched: false,
    })
}
#[cfg(not(target_os = "macos"))]
async fn install_dmg(_path: &str, _target: Option<&str>) -> InstallorResult<InstallOutcome> {
    Err(InstallorError::Unsupported(
        "DMG installation is available only on macOS".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn executes_argv_without_a_shell() {
        #[cfg(windows)]
        let plan = InstallPlan::Shell {
            cmd: "cmd".into(),
            args: vec!["/C".into(), "echo installor".into()],
        };
        #[cfg(not(windows))]
        let plan = InstallPlan::Shell {
            cmd: "printf".into(),
            args: vec!["installor".into()],
        };
        let result = execute(plan).await.unwrap();
        assert!(result.success);
        assert!(result.stdout.contains("installor"));
    }
}
