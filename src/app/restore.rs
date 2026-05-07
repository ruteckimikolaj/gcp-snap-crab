use anyhow::Result;

use crate::types::{AppState, RestoreBackupContext, RestoreConfig, RestoreRequest};
use super::App;

impl App {
    pub async fn perform_restore(&mut self) -> Result<()> {
        if let Some(config) = self.restore_flow.config.clone() {
            self.loading = true;
            self.state = AppState::PerformingRestore;

            let restore_request = RestoreRequest {
                restore_backup_context: RestoreBackupContext {
                    backup_run_id: config.backup_id.clone(),
                    project: config.source_project.clone(),
                    instance_id: config.source_instance.clone(),
                },
            };

            if self.dry_run_mode {
                let mock_operation_id =
                    format!("dry-run-operation-{}", chrono::Utc::now().timestamp());
                self.restore_flow.operation_id = Some(mock_operation_id);
                self.restore_flow.status = Some("DONE".to_string());
                self.loading = false;
                self.state = AppState::SelectingTargetInstance;
            } else {
                match self
                    .gcp_client
                    .restore_backup(
                        &restore_request,
                        &config.target_project,
                        &config.target_instance,
                    )
                    .await
                {
                    Ok(operation_id) => {
                        self.restore_flow.operation_id = Some(operation_id.clone());
                        self.restore_flow.status = Some("RUNNING".to_string());
                        self.loading = false;
                        self.state = AppState::SelectingTargetInstance;
                    }
                    Err(e) => {
                        self.loading = false;
                        self.error = Some(format!("Restore failed: {}. Press ESC to clear.", e));
                        self.state = AppState::ConfirmRestore;
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn check_restore_status(&mut self) -> Result<()> {
        if let (Some(operation_id), Some(config)) = (
            &self.restore_flow.operation_id.clone(),
            &self.restore_flow.config.clone(),
        ) {
            if self.dry_run_mode {
                self.restore_flow.status = Some("DONE".to_string());
                return Ok(());
            }
            if let Some(status) = self.poll_operation(&config.target_project, operation_id).await {
                self.restore_flow.status = Some(status);
            }
        }
        Ok(())
    }

    pub fn create_restore_config(&mut self) {
        if let (
            Some(backup_id),
            Some(source_project),
            Some(source_instance),
            Some(target_project),
            Some(target_instance),
        ) = (
            self.restore_flow.selected_backup.as_ref(),
            self.restore_flow.source_project.as_ref(),
            self.restore_flow.source_instance.as_ref(),
            self.restore_flow.target_project.as_ref(),
            self.restore_flow.target_instance.as_ref(),
        ) {
            self.restore_flow.config = Some(RestoreConfig {
                backup_id: backup_id.clone(),
                source_project: source_project.clone(),
                source_instance: source_instance.clone(),
                target_project: target_project.clone(),
                target_instance: target_instance.clone(),
            });
        }
    }
}
