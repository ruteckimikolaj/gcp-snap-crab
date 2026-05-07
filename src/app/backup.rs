use anyhow::Result;

use crate::types::{AppState, CreateBackupConfig};
use super::App;

impl App {
    pub async fn perform_create_backup(&mut self) -> Result<()> {
        if let Some(config) = &self.create_backup_flow.config {
            self.loading = true;
            self.state = AppState::PerformingCreateBackup;

            if self.dry_run_mode {
                let mock_operation_id =
                    format!("dry-run-backup-op-{}", chrono::Utc::now().timestamp());
                self.create_backup_flow.operation_id = Some(mock_operation_id);
                self.create_backup_flow.status = Some("DONE".to_string());
                self.loading = false;
                self.state = AppState::PerformingCreateBackup;
            } else {
                match self.gcp_client.create_backup(config).await {
                    Ok(operation_id) => {
                        self.create_backup_flow.operation_id = Some(operation_id);
                        self.create_backup_flow.status = Some("RUNNING".to_string());
                        self.loading = false;
                        self.state = AppState::PerformingCreateBackup;
                    }
                    Err(e) => {
                        self.loading = false;
                        self.error =
                            Some(format!("Create backup failed: {}. Press ESC to clear.", e));
                        self.state = AppState::ConfirmCreateBackup;
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn check_backup_status(&mut self) -> Result<()> {
        if let (Some(operation_id), Some(config)) = (
            &self.create_backup_flow.operation_id.clone(),
            &self.create_backup_flow.config.clone(),
        ) {
            if self.dry_run_mode {
                self.create_backup_flow.status = Some("DONE".to_string());
                return Ok(());
            }
            if let Some(status) = self.poll_operation(&config.project, operation_id).await {
                self.create_backup_flow.status = Some(status);
            }
        }
        Ok(())
    }

    pub fn create_backup_config(&mut self, backup_name: String) {
        if let (Some(project), Some(instance)) = (
            self.create_backup_flow.project.as_ref(),
            self.create_backup_flow.instance.as_ref(),
        ) {
            self.create_backup_flow.config = Some(CreateBackupConfig {
                project: project.clone(),
                instance: instance.clone(),
                name: backup_name.clone(),
                description: backup_name,
            });
        }
    }
}
