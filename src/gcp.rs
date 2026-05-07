use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::process::Command as AsyncCommand;
use tokio::sync::Mutex;

use crate::types::{
    Backup, CreateBackupConfig, GcpApiResponse, Operation, RestoreRequest, SqlInstance,
};

#[mockall::automock]
#[async_trait]
pub trait GcpClientTrait: Send + Sync {
    async fn check_prerequisites(&self) -> Result<String>;
    async fn list_sql_instances(&self, project_id: &str) -> Result<Vec<SqlInstance>>;
    async fn list_backups(&self, project_id: &str, instance_id: &str) -> Result<Vec<Backup>>;
    async fn get_operation_status(&self, project_id: &str, operation_id: &str) -> Result<Operation>;
    async fn restore_backup(
        &self,
        restore_request: &RestoreRequest,
        target_project: &str,
        target_instance: &str,
    ) -> Result<String>;
    async fn create_backup(&self, backup_config: &CreateBackupConfig) -> Result<String>;
}

pub struct GcpClient {
    client: Client,
    token_cache: Mutex<Option<(String, Instant)>>,
}

impl GcpClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            token_cache: Mutex::new(None),
        }
    }

    async fn get_access_token(&self) -> Result<String> {
        const TOKEN_TTL: Duration = Duration::from_secs(55 * 60);
        let mut cache = self.token_cache.lock().await;
        if let Some((token, fetched_at)) = &*cache {
            if fetched_at.elapsed() < TOKEN_TTL {
                return Ok(token.clone());
            }
        }

        let output = AsyncCommand::new("gcloud")
            .args(&["auth", "print-access-token"])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!("Failed to get access token"));
        }

        let token = String::from_utf8(output.stdout)?.trim().to_string();
        *cache = Some((token.clone(), Instant::now()));
        Ok(token)
    }
}

#[async_trait]
impl GcpClientTrait for GcpClient {
    async fn check_prerequisites(&self) -> Result<String> {
        // Check if gcloud is installed
        let output = AsyncCommand::new("which")
            .arg("gcloud")
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!("gcloud CLI is not installed"));
        }

        // Check authentication
        let output = AsyncCommand::new("gcloud")
            .args(&["auth", "list", "--filter=status:ACTIVE", "--format=value(account)"])
            .output()
            .await?;

        if !output.status.success() || output.stdout.is_empty() {
            return Err(anyhow!("Not authenticated with gcloud"));
        }

        let account = String::from_utf8(output.stdout)?.trim().to_string();
        Ok(account)
    }

    async fn list_sql_instances(&self, project_id: &str) -> Result<Vec<SqlInstance>> {
        let output = AsyncCommand::new("gcloud")
            .args(&[
                "sql",
                "instances",
                "list",
                &format!("--project={}", project_id),
                "--format=json",
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to list SQL instances: {}", stderr.trim()));
        }

        let json: Value = serde_json::from_str(&String::from_utf8(output.stdout)?)?;
        let instances = json
            .as_array()
            .ok_or_else(|| anyhow!("Unexpected response format for SQL instances"))?
            .iter()
            .map(|item| SqlInstance {
                name: item["name"].as_str().unwrap_or("").to_string(),
                database_version: item["databaseVersion"].as_str().unwrap_or("").to_string(),
                region: item["region"].as_str().unwrap_or("").to_string(),
                tier: item["settings"]["tier"].as_str().unwrap_or("").to_string(),
            })
            .collect();

        Ok(instances)
    }

    async fn list_backups(&self, project_id: &str, instance_id: &str) -> Result<Vec<Backup>> {
        let output = AsyncCommand::new("gcloud")
            .args(&[
                "sql",
                "backups",
                "list",
                &format!("--instance={}", instance_id),
                &format!("--project={}", project_id),
                "--format=json",
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to list backups: {}", stderr.trim()));
        }

        let json: Value = serde_json::from_str(&String::from_utf8(output.stdout)?)?;
        let backups = json
            .as_array()
            .ok_or_else(|| anyhow!("Unexpected response format for backups"))?
            .iter()
            .map(|item| {
                let start_time = item["startTime"]
                    .as_str()
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok());
                let id = item["id"]
                    .as_u64()
                    .map(|n| n.to_string())
                    .or_else(|| item["id"].as_str().map(|s| s.to_string()))
                    .unwrap_or_default();
                Backup {
                    id,
                    start_time,
                    backup_type: item["type"].as_str().unwrap_or("").to_string(),
                    status: item["status"].as_str().unwrap_or("").to_string(),
                }
            })
            .collect();

        Ok(backups)
    }

    async fn get_operation_status(
        &self,
        project_id: &str,
        operation_id: &str,
    ) -> Result<Operation> {
        let token = self.get_access_token().await?;
        let url = format!(
            "https://sqladmin.googleapis.com/v1/projects/{}/operations/{}",
            project_id, operation_id
        );

        let response = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to get operation status: {}",
                response.status()
            ));
        }

        let api_response: GcpApiResponse = response.json().await?;

        Ok(Operation {
            id: operation_id.to_string(),
            operation_type: api_response
                .operation_type
                .unwrap_or_else(|| "Unknown".to_string()),
            status: api_response.status.unwrap_or_else(|| "Unknown".to_string()),
            target_id: api_response
                .target_id
                .unwrap_or_else(|| "Unknown".to_string()),
            start_time: api_response.start_time.and_then(|s| s.parse().ok()),
            end_time: api_response.end_time.and_then(|s| s.parse().ok()),
            error_message: api_response.error.map(|e| e.message),
        })
    }

    async fn restore_backup(
        &self,
        restore_request: &RestoreRequest,
        target_project: &str,
        target_instance: &str,
    ) -> Result<String> {
        let token = self.get_access_token().await?;
        let url = format!(
            "https://sqladmin.googleapis.com/v1/projects/{}/instances/{}/restoreBackup",
            target_project, target_instance
        );

        let response = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(restore_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Restore operation failed: {}", error_text));
        }

        let result: Value = response.json().await?;

        if let Some(name) = result.get("name").and_then(|n| n.as_str()) {
            // Extract operation ID from the full operation name
            let operation_id = name.split('/').last().unwrap_or(name);
            Ok(operation_id.to_string())
        } else {
            Err(anyhow!("No operation ID returned from restore request"))
        }
    }

    async fn create_backup(&self, backup_config: &CreateBackupConfig) -> Result<String> {
        let token = self.get_access_token().await?;
        let url = format!(
            "https://sqladmin.googleapis.com/v1/projects/{}/instances/{}/backupRuns",
            backup_config.project, backup_config.instance
        );

        let request_body = serde_json::json!({
            "description": &backup_config.name
        });

        let response = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Create backup operation failed: {}", error_text));
        }

        let result: Value = response.json().await?;

        if let Some(name) = result.get("name").and_then(|n| n.as_str()) {
            let operation_id = name.split('/').last().unwrap_or(name);
            Ok(operation_id.to_string())
        } else {
            Err(anyhow!(
                "No operation ID returned from create backup request"
            ))
        }
    }
}
