use crate::{
    error::InstallorResult,
    models::{CatalogEntry, FileSuggestion},
};
use std::path::Path;

fn normalized(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphabetic() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .filter(|p| !p.chars().all(|c| c.is_ascii_digit()))
        .collect::<Vec<_>>()
        .join(" ")
}
fn installer(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        matches!(
            e.to_ascii_lowercase().as_str(),
            "exe" | "msi" | "pkg" | "dmg" | "deb" | "rpm" | "appimage"
        )
    })
}
pub fn find(
    entries: &[CatalogEntry],
    folder: &Path,
    recursive: bool,
) -> InstallorResult<Vec<FileSuggestion>> {
    let walker = walkdir::WalkDir::new(folder).max_depth(if recursive { usize::MAX } else { 1 });
    let files: Vec<_> = walker
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file() && installer(e.path()))
        .map(|e| e.into_path())
        .collect();
    let mut out = Vec::new();
    for entry in entries
        .iter()
        .filter(|e| matches!(e.source_type, crate::models::SourceType::Custom))
    {
        let target = normalized(&entry.name);
        let mut matches: Vec<_> = files
            .iter()
            .filter_map(|p| {
                let score =
                    strsim::jaro_winkler(&target, &normalized(&p.file_stem()?.to_string_lossy()));
                (score >= 0.6).then(|| FileSuggestion {
                    software_id: entry.id.clone(),
                    path: p.to_string_lossy().into_owned(),
                    score,
                })
            })
            .collect();
        matches.sort_by(|a, b| b.score.total_cmp(&a.score));
        matches.truncate(3);
        out.extend(matches);
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn obs_ranks_first() {
        let names = [
            "OBS-Studio-30.1-Full-Installer-x64.exe",
            "unrelated-tool.exe",
        ];
        let target = normalized("OBS Studio");
        let mut s: Vec<_> = names
            .iter()
            .map(|n| (*n, strsim::jaro_winkler(&target, &normalized(n))))
            .collect();
        s.sort_by(|a, b| b.1.total_cmp(&a.1));
        assert_eq!(s[0].0, names[0]);
    }
}
