use crate::{
    error::{InstallorError, InstallorResult},
    models::*,
};
use std::{collections::HashMap, str::FromStr};
use url::Url;

pub fn spreadsheet_id(shared_url: &str) -> InstallorResult<String> {
    let parsed = Url::parse(shared_url)
        .map_err(|_| InstallorError::InvalidConfig("Paste a valid Google Sheets URL".into()))?;
    if !parsed.host_str().is_some_and(|h| h.ends_with("google.com")) {
        return Err(InstallorError::InvalidConfig(
            "Only Google Sheets links are supported".into(),
        ));
    }
    let parts: Vec<_> = parsed
        .path_segments()
        .map(|s| s.collect())
        .unwrap_or_default();
    if let Some(pos) = parts.iter().position(|p| *p == "d")
        && let Some(id) = parts
            .get(pos + 1)
            .filter(|id| !id.is_empty() && **id != "e")
    {
        return Ok((*id).to_string());
    }
    if let Some(id) = parsed
        .query_pairs()
        .find(|(k, _)| k == "id")
        .map(|(_, v)| v.into_owned())
    {
        return Ok(id);
    }
    Err(InstallorError::InvalidConfig("Could not find a spreadsheet ID in this link. Use the URL from Google Sheets → Share → Copy link.".into()))
}

pub fn export_url(shared_url: &str, tab: &str) -> InstallorResult<String> {
    let id = spreadsheet_id(shared_url)?;
    let mut url = Url::parse(&format!(
        "https://docs.google.com/spreadsheets/d/{id}/gviz/tq"
    ))
    .unwrap();
    url.query_pairs_mut()
        .append_pair("tqx", "out:csv")
        .append_pair("sheet", tab);
    Ok(url.into())
}

async fn fetch_tab(
    client: &reqwest::Client,
    shared_url: &str,
    tab: &str,
) -> InstallorResult<String> {
    let response = client
        .get(export_url(shared_url, tab)?)
        .send()
        .await?
        .error_for_status()?;
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body = response.text().await?;
    if content_type.contains("text/html") || body.trim_start().starts_with("<!DOCTYPE html") {
        return Err(InstallorError::InvalidConfig(format!(
            "The {tab} tab is not publicly readable. In Google Sheets choose Share → General access → Anyone with the link → Viewer."
        )));
    }
    Ok(body)
}

fn rows(csv_text: &str) -> InstallorResult<Vec<HashMap<String, String>>> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(csv_text.as_bytes());
    let headers = reader.headers()?.clone();
    let mut out = Vec::new();
    for record in reader.records() {
        let record = record?;
        out.push(
            headers
                .iter()
                .zip(record.iter())
                .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
                .collect(),
        );
    }
    Ok(out)
}

fn value(row: &HashMap<String, String>, key: &str) -> String {
    row.get(key).cloned().unwrap_or_default()
}

pub fn parse_catalog(csv_text: &str) -> InstallorResult<Vec<CatalogEntry>> {
    let now = now_rfc3339();
    let parsed = rows(csv_text)?;
    if !parsed.is_empty() && !parsed[0].contains_key("name") {
        return Err(InstallorError::InvalidConfig(
            "Catalog tab must contain a 'name' header".into(),
        ));
    }
    Ok(parsed
        .into_iter()
        .filter(|r| !value(r, "name").is_empty())
        .map(|r| {
            let name = value(&r, "name");
            let mut id = value(&r, "id");
            if id.is_empty() {
                id = format!(
                    "sheet-{}",
                    name.to_ascii_lowercase()
                        .chars()
                        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                        .collect::<String>()
                        .trim_matches('-')
                );
            }
            let created = value(&r, "created_at");
            let updated = value(&r, "updated_at");
            CatalogEntry {
                id,
                name,
                publisher: value(&r, "publisher"),
                category: value(&r, "category"),
                platforms: value(&r, "platforms"),
                source_type: SourceType::from_str(&value(&r, "source_type")).unwrap_or_default(),
                source_id: value(&r, "source_id"),
                download_url: value(&r, "download_url"),
                installer_type: InstallerType::from_str(&value(&r, "installer_type"))
                    .unwrap_or_default(),
                silent_args: value(&r, "silent_args"),
                checksum_sha256: value(&r, "checksum_sha256"),
                tags: value(&r, "tags"),
                notes: value(&r, "notes"),
                created_at: if created.is_empty() {
                    now.clone()
                } else {
                    created
                },
                updated_at: if updated.is_empty() {
                    now.clone()
                } else {
                    updated
                },
            }
        })
        .collect())
}

pub fn parse_installations(csv_text: &str) -> InstallorResult<Vec<InstallationEntry>> {
    let now = now_rfc3339();
    Ok(rows(csv_text)?
        .into_iter()
        .filter(|r| !value(r, "software_id").is_empty())
        .map(|r| {
            let software_id = value(&r, "software_id");
            let machine_id = value(&r, "machine_id");
            let mut id = value(&r, "id");
            if id.is_empty() {
                id = format!("{software_id}-{machine_id}");
            }
            let updated = value(&r, "updated_at");
            InstallationEntry {
                id,
                software_id,
                machine_id,
                machine_label: value(&r, "machine_label"),
                installed: matches!(
                    value(&r, "installed").to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "installed"
                ),
                installed_version: value(&r, "installed_version"),
                installed_path: value(&r, "installed_path"),
                installer_file_path: value(&r, "installer_file_path"),
                last_verified_at: value(&r, "last_verified_at"),
                updated_at: if updated.is_empty() {
                    now.clone()
                } else {
                    updated
                },
            }
        })
        .collect())
}

pub async fn import(client: &reqwest::Client, shared_url: &str) -> InstallorResult<CatalogBundle> {
    let catalog = parse_catalog(&fetch_tab(client, shared_url, "Catalog").await?)?;
    let installations = match fetch_tab(client, shared_url, "Installations").await {
        Ok(v) => parse_installations(&v)?,
        Err(_) => Vec::new(),
    };
    Ok(CatalogBundle {
        catalog,
        installations,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extracts_regular_share_url() {
        assert_eq!(
            spreadsheet_id("https://docs.google.com/spreadsheets/d/abc_123/edit?usp=sharing")
                .unwrap(),
            "abc_123"
        );
    }
    #[test]
    fn parses_minimal_catalog() {
        let r = parse_catalog("name,publisher,source_type\nOBS Studio,OBS,custom\n").unwrap();
        assert_eq!(r[0].id, "sheet-obs-studio");
    }
}
