use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct SqlInstance {
    pub name: String,
    pub database_version: String,
    pub region: String,
    pub tier: String,
}

#[derive(Debug, Clone)]
pub struct Backup {
    pub id: String,
    pub start_time: Option<DateTime<Utc>>,
    pub backup_type: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct Operation {
    pub id: String,
    pub operation_type: String,
    pub status: String,
    pub target_id: String,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RestoreRequest {
    #[serde(rename = "restoreBackupContext")]
    pub restore_backup_context: RestoreBackupContext,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RestoreBackupContext {
    #[serde(rename = "backupRunId")]
    pub backup_run_id: String,
    pub project: String,
    #[serde(rename = "instanceId")]
    pub instance_id: String,
}

#[derive(Debug, Deserialize)]
pub struct GcpApiResponse {
    pub name: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "operationType")]
    pub operation_type: Option<String>,
    #[serde(rename = "targetId")]
    pub target_id: Option<String>,
    #[serde(rename = "startTime")]
    pub start_time: Option<String>,
    #[serde(rename = "endTime")]
    pub end_time: Option<String>,
    pub error: Option<GcpError>,
}

#[derive(Debug, Deserialize)]
pub struct GcpError {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Welcome,
    CheckingPrerequisites,
    SelectingSourceProject,
    SelectingSourceInstance,
    SelectingBackup,
    SelectingTargetProject,
    SelectingTargetInstance,
    ConfirmRestore,
    PerformingRestore,
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone)]
pub struct RestoreConfig {
    pub backup_id: String,
    pub source_project: String,
    pub source_instance: String,
    pub target_project: String,
    pub target_instance: String,
}
