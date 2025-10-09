use anyhow::Result;

use crate::gcp::GcpClientTrait;
use crate::state::create_backup_flow::CreateBackupFlow;
use crate::state::restore_flow::RestoreFlow;
use crate::types::{
    AppState, Backup, CreateBackupConfig, InputMode, Operation, OperationMode, RestoreConfig,
    RestoreRequest, RestoreBackupContext, SqlInstance,
};

pub struct App {
    pub operation_mode: Option<OperationMode>,
    pub state: AppState,
    pub dry_run_mode: bool,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub gcp_client: Box<dyn GcpClientTrait>,
    pub authenticated_user: Option<String>,
    pub remembered_projects: Vec<String>,
    pub remembered_instances: Vec<String>,
    pub selected_operation_index: usize,
    pub loading: bool,
    pub show_help: bool,
    pub manual_input_active: bool,
    pub manual_input_buffer: String,
    pub manual_input_type: String,
    pub restore_flow: RestoreFlow,
    pub create_backup_flow: CreateBackupFlow,
    pub error: Option<String>,
}

impl App {
    pub fn new(gcp_client: Box<dyn GcpClientTrait>, dry_run_mode: bool) -> Self {
        Self {
            operation_mode: None,
            state: AppState::SelectingOperation,
            dry_run_mode,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            gcp_client,
            authenticated_user: None,
            remembered_projects: Vec::new(),
            remembered_instances: Vec::new(),
            selected_operation_index: 0,
            loading: false,
            show_help: false,
            manual_input_active: false,
            manual_input_buffer: String::new(),
            manual_input_type: String::new(),
            restore_flow: RestoreFlow::new(),
            create_backup_flow: CreateBackupFlow::new(),
            error: None,
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        self.state = AppState::CheckingPrerequisites;
        self.loading = true;
        self.error = None;

        match self.gcp_client.check_prerequisites().await {
            Ok(user) => {
                self.authenticated_user = Some(user);
                self.loading = false;
                self.state = AppState::SelectingOperation;
            }
            Err(e) => {
                self.loading = false;
                self.state = AppState::Error(e.to_string());
            }
        }

        Ok(())
    }

    pub async fn load_projects(&mut self) -> Result<()> {
        self.loading = false;
        self.start_manual_input("source_project");
        Ok(())
    }

    pub async fn load_instances(&mut self, project_id: &str) -> Result<()> {
        self.loading = true;
        self.error = None;
        match self.gcp_client.list_sql_instances(project_id).await {
            Ok(instances) => {
                match self.operation_mode {
                    Some(OperationMode::Restore) => {
                        self.restore_flow.instances = instances;
                        self.restore_flow.selected_instance_index = 0;
                    }
                    Some(OperationMode::CreateBackup) => {
                        self.create_backup_flow.instances = instances;
                        self.create_backup_flow.selected_instance_index = 0;
                    }
                    None => {}
                }
                self.loading = false;
            }
            Err(e) => {
                self.loading = false;
                self.error = Some(format!(
                    "Failed to load instances: {}. Press ESC to clear.",
                    e
                ));
            }
        }
        Ok(())
    }

    pub async fn load_backups(&mut self, project_id: &str, instance_id: &str) -> Result<()> {
        self.loading = true;
        self.error = None;
        match self.gcp_client.list_backups(project_id, instance_id).await {
            Ok(backups) => {
                self.restore_flow.backups = backups;
                self.restore_flow.selected_backup_index = 0;
                self.loading = false;
            }
            Err(e) => {
                self.loading = false;
                self.error = Some(format!(
                    "Failed to load backups: {}. Press ESC to clear.",
                    e
                ));
            }
        }
        Ok(())
    }

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

    pub async fn check_restore_status(&mut self) -> Result<()> {
        if let (Some(operation_id), Some(config)) = (
            &self.restore_flow.operation_id.clone(),
            &self.restore_flow.config.clone(),
        ) {
            if self.dry_run_mode {
                self.restore_flow.status = Some("DONE".to_string());
                return Ok(());
            }

            match self
                .gcp_client
                .get_operation_status(&config.target_project, operation_id)
                .await
            {
                Ok(operation) => {
                    self.restore_flow.status = Some(operation.status.clone());
                }
                Err(e) => {
                    self.error = Some(format!("Failed to check restore status: {}", e));
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

            match self
                .gcp_client
                .get_operation_status(&config.project, operation_id)
                .await
            {
                Ok(operation) => {
                    self.create_backup_flow.status = Some(operation.status.clone());
                }
                Err(e) => {
                    self.error = Some(format!("Failed to check backup status: {}", e));
                }
            }
        }
        Ok(())
    }

    pub fn move_selection_up(&mut self) {
        match self.state {
            AppState::SelectingOperation => {
                if self.selected_operation_index > 0 {
                    self.selected_operation_index -= 1;
                }
            }
            AppState::SelectingSourceProject
            | AppState::SelectingTargetProject
            | AppState::SelectingProjectForBackup => {}
            AppState::SelectingSourceInstance | AppState::SelectingTargetInstance => {
                if self.restore_flow.selected_instance_index > 0 {
                    self.restore_flow.selected_instance_index -= 1;
                }
            }
            AppState::SelectingInstanceForBackup => {
                if self.create_backup_flow.selected_instance_index > 0 {
                    self.create_backup_flow.selected_instance_index -= 1;
                }
            }
            AppState::SelectingBackup => {
                if self.restore_flow.selected_backup_index > 0 {
                    self.restore_flow.selected_backup_index -= 1;
                }
            }
            _ => {}
        }
    }

    pub fn move_selection_down(&mut self) {
        match self.state {
            AppState::SelectingOperation => {
                if self.selected_operation_index < 1 {
                    self.selected_operation_index += 1;
                }
            }
            AppState::SelectingSourceProject
            | AppState::SelectingTargetProject
            | AppState::SelectingProjectForBackup => {}
            AppState::SelectingSourceInstance | AppState::SelectingTargetInstance => {
                if self.restore_flow.selected_instance_index
                    < self.restore_flow.instances.len().saturating_sub(1)
                {
                    self.restore_flow.selected_instance_index += 1;
                }
            }
            AppState::SelectingInstanceForBackup => {
                if self.create_backup_flow.selected_instance_index
                    < self.create_backup_flow.instances.len().saturating_sub(1)
                {
                    self.create_backup_flow.selected_instance_index += 1;
                }
            }
            AppState::SelectingBackup => {
                if self.restore_flow.selected_backup_index
                    < self.restore_flow.backups.len().saturating_sub(1)
                {
                    self.restore_flow.selected_backup_index += 1;
                }
            }
            _ => {}
        }
    }

    pub async fn select_current_item(&mut self) -> Result<()> {
        match self.state {
            AppState::SelectingOperation => {
                let selected_mode = if self.selected_operation_index == 0 {
                    OperationMode::Restore
                } else {
                    OperationMode::CreateBackup
                };
                self.operation_mode = Some(selected_mode);
                match selected_mode {
                    OperationMode::Restore => self.state = AppState::SelectingSourceProject,
                    OperationMode::CreateBackup => self.state = AppState::SelectingProjectForBackup,
                }
                self.load_projects().await?;
            }
            AppState::SelectingSourceProject | AppState::SelectingProjectForBackup => {
                self.start_manual_input("source_project");
            }
            AppState::SelectingSourceInstance => {
                if let Some(instance) = self
                    .restore_flow
                    .instances
                    .get(self.restore_flow.selected_instance_index)
                    .cloned()
                {
                    self.restore_flow.source_instance = Some(instance.name.clone());
                    if let Some(project) = &self.restore_flow.source_project.clone() {
                        self.state = AppState::SelectingBackup;
                        self.load_backups(project, &instance.name).await?;
                    }
                }
            }
            AppState::SelectingInstanceForBackup => {
                if let Some(instance) = self
                    .create_backup_flow
                    .instances
                    .get(self.create_backup_flow.selected_instance_index)
                    .cloned()
                {
                    self.create_backup_flow.instance = Some(instance.name.clone());
                    self.state = AppState::EnteringBackupName;
                    self.start_manual_input("backup_name");
                }
            }
            AppState::SelectingBackup => {
                if let Some(backup) = self
                    .restore_flow
                    .backups
                    .get(self.restore_flow.selected_backup_index)
                    .cloned()
                {
                    self.restore_flow.selected_backup = Some(backup.id.clone());
                    self.state = AppState::SelectingTargetProject;
                }
            }
            AppState::SelectingTargetProject => {
                self.start_manual_input("target_project");
            }
            AppState::SelectingTargetInstance => {
                if let Some(instance) = self
                    .restore_flow
                    .instances
                    .get(self.restore_flow.selected_instance_index)
                    .cloned()
                {
                    self.restore_flow.target_instance = Some(instance.name.clone());
                    self.create_restore_config();
                    self.state = AppState::ConfirmRestore;
                }
            }
            AppState::ConfirmCreateBackup => {
                self.perform_create_backup().await?;
            }
            _ => {}
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

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn start_manual_input(&mut self, input_type: &str) {
        self.manual_input_active = true;
        self.manual_input_type = input_type.to_string();
        self.manual_input_buffer.clear();
        self.input_mode = InputMode::Editing;
    }

    pub async fn finish_manual_input(&mut self) -> Result<()> {
        let input_value = self.manual_input_buffer.trim().to_string();
        if !input_value.is_empty() {
            match self.manual_input_type.as_str() {
                "source_project" => {
                    if !self.remembered_projects.contains(&input_value) {
                        self.remembered_projects.push(input_value.clone());
                    }
                    self.manual_input_active = false;
                    self.input_mode = InputMode::Normal;
                    match self.operation_mode {
                        Some(OperationMode::Restore) => {
                            self.restore_flow.source_project = Some(input_value.clone());
                            self.state = AppState::SelectingSourceInstance;
                        }
                        Some(OperationMode::CreateBackup) => {
                            self.create_backup_flow.project = Some(input_value.clone());
                            self.state = AppState::SelectingInstanceForBackup
                        }
                        None => {}
                    }
                    self.load_instances(&input_value).await?;
                }
                "target_project" => {
                    if !self.remembered_projects.contains(&input_value) {
                        self.remembered_projects.push(input_value.clone());
                    }
                    self.restore_flow.target_project = Some(input_value.clone());
                    self.manual_input_active = false;
                    self.input_mode = InputMode::Normal;
                    self.state = AppState::SelectingTargetInstance;
                    self.load_instances(&input_value).await?;
                }
                "instance" => {
                    if !self.remembered_instances.contains(&input_value) {
                        self.remembered_instances.push(input_value.clone());
                    }
                    let instance = SqlInstance {
                        name: input_value.clone(),
                        database_version: "Manual".to_string(),
                        region: "Manual".to_string(),
                        tier: "Manual".to_string(),
                    };
                    match self.operation_mode {
                        Some(OperationMode::Restore) => {
                            self.restore_flow.instances.push(instance);
                            self.restore_flow.selected_instance_index =
                                self.restore_flow.instances.len() - 1;
                        }
                        Some(OperationMode::CreateBackup) => {
                            self.create_backup_flow.instances.push(instance);
                            self.create_backup_flow.selected_instance_index =
                                self.create_backup_flow.instances.len() - 1;
                        }
                        None => {}
                    }
                }
                "backup" => {
                    let backup = Backup {
                        id: input_value.clone(),
                        start_time: None,
                        backup_type: "Manual".to_string(),
                        status: "Manual".to_string(),
                    };
                    self.restore_flow.backups.push(backup);
                    self.restore_flow.selected_backup_index = self.restore_flow.backups.len() - 1;
                }
                "backup_name" => {
                    self.manual_input_active = false;
                    self.input_mode = InputMode::Normal;
                    self.create_backup_config(input_value);
                    self.state = AppState::ConfirmCreateBackup;
                }
                _ => {}
            }
        } else {
            self.manual_input_active = false;
            self.input_mode = InputMode::Normal;
        }
        Ok(())
    }

    pub fn cancel_manual_input(&mut self) {
        self.manual_input_active = false;
        self.manual_input_buffer.clear();
        self.input_mode = InputMode::Normal;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gcp::MockGcpClientTrait;
    use crate::types::{AppState, InputMode, OperationMode, SqlInstance};
    use anyhow::anyhow;

    #[test]
    fn test_app_initialization() {
        let mock_gcp_client = MockGcpClientTrait::new();
        let app = App::new(Box::new(mock_gcp_client), false);

        assert_eq!(app.state, AppState::SelectingOperation);
        assert!(!app.dry_run_mode);
        assert!(app.authenticated_user.is_none());
        assert!(app.restore_flow.instances.is_empty());
        assert!(app.restore_flow.backups.is_empty());
    }

    #[tokio::test]
    async fn test_initialize_success() {
        let mut mock_gcp_client = MockGcpClientTrait::new();
        mock_gcp_client
            .expect_check_prerequisites()
            .times(1)
            .returning(|| Ok("test-user@google.com".to_string()));

        let mut app = App::new(Box::new(mock_gcp_client), false);
        app.initialize().await.unwrap();

        assert_eq!(app.state, AppState::SelectingOperation);
        assert_eq!(
            app.authenticated_user,
            Some("test-user@google.com".to_string())
        );
        assert!(!app.loading);
    }

    #[tokio::test]
    async fn test_initialize_failure() {
        let mut mock_gcp_client = MockGcpClientTrait::new();
        mock_gcp_client
            .expect_check_prerequisites()
            .times(1)
            .returning(|| Err(anyhow!("gcloud not found")));

        let mut app = App::new(Box::new(mock_gcp_client), false);
        app.initialize().await.unwrap();

        assert_eq!(app.state, AppState::Error("gcloud not found".to_string()));
        assert!(app.authenticated_user.is_none());
        assert!(!app.loading);
    }

    #[tokio::test]
    async fn test_select_operation_restore() {
        let mock_gcp_client = MockGcpClientTrait::new();
        let mut app = App::new(Box::new(mock_gcp_client), false);
        app.selected_operation_index = 0; // Restore

        app.select_current_item().await.unwrap();

        assert_eq!(app.state, AppState::SelectingSourceProject);
        assert_eq!(app.operation_mode, Some(OperationMode::Restore));
        assert!(app.manual_input_active);
        assert_eq!(app.manual_input_type, "source_project");
    }

    #[tokio::test]
    async fn test_select_operation_create_backup() {
        let mock_gcp_client = MockGcpClientTrait::new();
        let mut app = App::new(Box::new(mock_gcp_client), false);
        app.selected_operation_index = 1; // Create Backup

        app.select_current_item().await.unwrap();

        assert_eq!(app.state, AppState::SelectingProjectForBackup);
        assert_eq!(app.operation_mode, Some(OperationMode::CreateBackup));
        assert!(app.manual_input_active);
        assert_eq!(app.manual_input_type, "source_project");
    }

    #[tokio::test]
    async fn test_finish_manual_input_source_project() {
        let project_id = "test-project".to_string();
        let instances = vec![SqlInstance {
            name: "instance-1".to_string(),
            database_version: "v1".to_string(),
            region: "region-1".to_string(),
            tier: "db-n1-standard-1".to_string(),
        }];

        let mut mock_gcp_client = MockGcpClientTrait::new();
        mock_gcp_client
            .expect_list_sql_instances()
            .withf(move |p| p == project_id)
            .times(1)
            .returning({
                let instances = instances.clone();
                move |_| Ok(instances.clone())
            });

        let mut app = App::new(Box::new(mock_gcp_client), false);
        app.operation_mode = Some(OperationMode::Restore);
        app.manual_input_type = "source_project".to_string();
        app.manual_input_buffer = "test-project".to_string();

        app.finish_manual_input().await.unwrap();

        assert_eq!(app.state, AppState::SelectingSourceInstance);
        assert_eq!(
            app.restore_flow.source_project,
            Some("test-project".to_string())
        );
        assert!(!app.manual_input_active);
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.restore_flow.instances.len(), 1);
        assert_eq!(app.restore_flow.instances[0].name, "instance-1");
    }

    #[test]
    fn test_navigation_instance_selection() {
        let mock_gcp_client = MockGcpClientTrait::new();
        let mut app = App::new(Box::new(mock_gcp_client), false);
        app.state = AppState::SelectingInstanceForBackup;
        app.operation_mode = Some(OperationMode::CreateBackup);
        app.create_backup_flow.instances = vec![
            SqlInstance {
                name: "instance-1".to_string(),
                database_version: "".to_string(),
                region: "".to_string(),
                tier: "".to_string(),
            },
            SqlInstance {
                name: "instance-2".to_string(),
                database_version: "".to_string(),
                region: "".to_string(),
                tier: "".to_string(),
            },
            SqlInstance {
                name: "instance-3".to_string(),
                database_version: "".to_string(),
                region: "".to_string(),
                tier: "".to_string(),
            },
        ];
        app.create_backup_flow.selected_instance_index = 1;

        // Move down
        app.move_selection_down();
        assert_eq!(app.create_backup_flow.selected_instance_index, 2);

        // Move down at the end
        app.move_selection_down();
        assert_eq!(app.create_backup_flow.selected_instance_index, 2);

        // Move up
        app.move_selection_up();
        assert_eq!(app.create_backup_flow.selected_instance_index, 1);

        // Move up
        app.move_selection_up();
        assert_eq!(app.create_backup_flow.selected_instance_index, 0);

        // Move up at the start
        app.move_selection_up();
        assert_eq!(app.create_backup_flow.selected_instance_index, 0);
    }
}
