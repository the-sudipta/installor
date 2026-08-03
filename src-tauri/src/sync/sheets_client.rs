use crate::{
    error::{InstallorError, InstallorResult},
    models::*,
};
use serde::de::DeserializeOwned;

#[derive(Clone)]
pub struct SheetsClient {
    pub base_url: String,
    pub token: String,
    pub http: reqwest::Client,
}

impl SheetsClient {
    pub fn new(base_url: String, token: String) -> Self {
        Self {
            base_url,
            token,
            http: reqwest::Client::new(),
        }
    }
    async fn get_<T: DeserializeOwned>(&self, action: &str) -> InstallorResult<T> {
        let response = self
            .http
            .get(&self.base_url)
            .query(&[("action", action), ("token", self.token.as_str())])
            .send()
            .await?
            .error_for_status()?;
        let value: serde_json::Value = response.json().await?;
        if let Some(e) = value.get("error").and_then(|v| v.as_str()) {
            return Err(InstallorError::Operation(format!("Apps Script: {e}")));
        }
        Ok(serde_json::from_value(value)?)
    }
    pub async fn ping(&self) -> InstallorResult<PingResponse> {
        self.get_("ping").await
    }
    pub async fn list(&self) -> InstallorResult<CatalogBundle> {
        self.get_("list").await
    }
    async fn post_(&self, action: &str, extra: serde_json::Value) -> InstallorResult<()> {
        let mut body = extra;
        body["action"] = action.into();
        body["token"] = self.token.clone().into();
        let value: serde_json::Value = self
            .http
            .post(&self.base_url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if let Some(e) = value.get("error").and_then(|v| v.as_str()) {
            return Err(InstallorError::Operation(format!("Apps Script: {e}")));
        }
        Ok(())
    }
    pub async fn upsert_catalog(&self, row: &CatalogEntry) -> InstallorResult<()> {
        self.post_("upsert_catalog", serde_json::json!({"row":row}))
            .await
    }
    pub async fn upsert_installation(&self, row: &InstallationEntry) -> InstallorResult<()> {
        self.post_("upsert_installation", serde_json::json!({"row":row}))
            .await
    }
    pub async fn delete_catalog(&self, id: &str) -> InstallorResult<()> {
        self.post_("delete_catalog", serde_json::json!({"id":id}))
            .await
    }
    pub async fn delete_installation(&self, id: &str) -> InstallorResult<()> {
        self.post_("delete_installation", serde_json::json!({"id":id}))
            .await
    }
}
