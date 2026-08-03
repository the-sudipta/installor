use crate::{
    error::{InstallorError, InstallorResult},
    models::*,
};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

fn filename(entry: &CatalogEntry) -> String {
    let from_url = url::Url::parse(&entry.download_url)
        .ok()
        .and_then(|u| u.path_segments()?.next_back().map(str::to_string))
        .filter(|s| !s.is_empty());
    let raw = from_url.unwrap_or_else(|| format!("{}-installer", entry.name));
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "._-".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub async fn download(
    client: &reqwest::Client,
    app: &AppHandle,
    entry: &CatalogEntry,
    folder: &Path,
) -> InstallorResult<DownloadOutcome> {
    download_with_progress(client, entry, folder, |progress| {
        let _ = app.emit("download-progress", progress);
    })
    .await
}

async fn download_with_progress(
    client: &reqwest::Client,
    entry: &CatalogEntry,
    folder: &Path,
    mut on_progress: impl FnMut(DownloadProgress),
) -> InstallorResult<DownloadOutcome> {
    if entry.download_url.trim().is_empty() {
        return Err(InstallorError::InvalidConfig(format!(
            "{} has no download URL",
            entry.name
        )));
    }
    tokio::fs::create_dir_all(folder).await?;
    let path: PathBuf = folder.join(filename(entry));
    let response = client
        .get(&entry.download_url)
        .send()
        .await?
        .error_for_status()?;
    let total = response.content_length();
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(&path).await?;
    let mut downloaded = 0;
    let mut hasher = Sha256::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;
        on_progress(DownloadProgress {
            software_id: entry.id.clone(),
            downloaded,
            total,
        });
    }
    file.flush().await?;
    let actual = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let checksum_ok = entry.checksum_sha256.trim().is_empty()
        || actual.eq_ignore_ascii_case(entry.checksum_sha256.trim());
    Ok(DownloadOutcome {
        software_id: entry.id.clone(),
        path: path.to_string_lossy().into_owned(),
        checksum_ok,
        error: if checksum_ok {
            None
        } else {
            Some(format!(
                "SHA-256 mismatch (expected {}, got {actual})",
                entry.checksum_sha256
            ))
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    #[tokio::test]
    async fn streams_file_reports_progress_and_checks_sha256() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/tiny.txt"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello"))
            .mount(&server)
            .await;
        let entry = CatalogEntry {
            id: "tiny".into(),
            name: "Tiny".into(),
            download_url: format!("{}/tiny.txt", server.uri()),
            checksum_sha256: "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
                .into(),
            ..Default::default()
        };
        let folder = tempfile::tempdir().unwrap();
        let mut progress_events = 0;
        let result = download_with_progress(&reqwest::Client::new(), &entry, folder.path(), |_| {
            progress_events += 1
        })
        .await
        .unwrap();
        assert!(result.checksum_ok);
        assert_eq!(std::fs::read_to_string(result.path).unwrap(), "hello");
        assert!(progress_events > 0);
    }

    #[tokio::test]
    async fn flags_bad_checksum() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello"))
            .mount(&server)
            .await;
        let entry = CatalogEntry {
            id: "bad".into(),
            name: "Bad".into(),
            download_url: server.uri(),
            checksum_sha256: "0".repeat(64),
            ..Default::default()
        };
        let folder = tempfile::tempdir().unwrap();
        let result = download_with_progress(&reqwest::Client::new(), &entry, folder.path(), |_| {})
            .await
            .unwrap();
        assert!(!result.checksum_ok);
        assert!(result.error.unwrap().contains("mismatch"));
    }
}
