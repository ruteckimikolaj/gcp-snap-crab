use anyhow::Result;

use crate::gcp::GcpClient;
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
    pub gcp_client: GcpClient,
    pub authenticated_user: Option<String>,
    pub sql_instances: Vec<SqlInstance>,
    pub backups: Vec<Backup>,
    pub remembered_projects: Vec<String>,
    pub remembered_instances: Vec<String>,
    pub selected_operation_index: usize,
    pub selected_project_index: usize,
    pub selected_instance_index: usize,
    pub selected_backup_index: usize,
    pub source_project: Option<String>,
    pub source_instance: Option<String>,
    pub target_project: Option<String>,
    pub target_instance: Option<String>,
    pub selected_backup: Option<String>,
    pub restore_config: Option<RestoreConfig>,
    pub restore_result: Option<String>,
    pub restore_status: Option<String>,
    pub create_backup_config: Option<CreateBackupConfig>,
    pub backup_operation_id: Option<String>,
    pub backup_operation_status: Option<String>,
    pub loading: bool,
    pub show_help: bool,
    pub manual_input_active: bool,
    pub manual_input_buffer: String,
    pub manual_input_type: String, // "project", "instance", "backup", "operation", "backup_name"
}

impl App {
    pub async fn new(dry_run_mode: bool) -> Result<Self> {
        let gcp_client = GcpClient::new();

        Ok(Self {
            operation_mode: None,
            state: AppState::SelectingOperation,
            dry_run_mode,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            gcp_client,
            authenticated_user: None,
            sql_instances: Vec::new(),
            backups: Vec::new(),
            remembered_projects: Vec::new(),
            remembered_instances: Vec::new(),
            selected_operation_index: 0,
            selected_project_index: 0,
            selected_instance_index: 0,
            selected_backup_index: 0,
            source_project: None,
            source_instance: None,
            target_project: None,
            target_instance: None,
            selected_backup: None,
            restore_config: None,
            restore_result: None,
            restore_status: None,
            create_backup_config: None,
            backup_operation_id: None,
            backup_operation_status: None,
            loading: false,
            show_help: false,
            manual_input_active: false,
            manual_input_buffer: String::new(),
            manual_input_type: String::new(),
        })
    }

    pub async fn initialize(&mut self) -> Result<()> {
        self.state = AppState::CheckingPrerequisites;
        self.loading = true;

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
        match self.gcp_client.list_sql_instances(project_id).await {
            Ok(instances) => {
                self.sql_instances = instances;
                self.selected_instance_index = 0;
                self.loading = false;
            }
            Err(e) => {
                self.loading = false;
                self.state = AppState::Error(format!("Failed to load instances: {}", e));
            }
        }
        Ok(())
    }

    pub async fn load_backups(&mut self, project_id: &str, instance_id: &str) -> Result<()> {
        self.loading = true;
        match self.gcp_client.list_backups(project_id, instance_id).await {
            Ok(backups) => {
                self.backups = backups;
                self.selected_backup_index = 0;
                self.loading = false;
            }
            Err(e) => {
                self.loading = false;
                self.state = AppState::Error(format!("Failed to load backups: {}", e));
            }
        }
        Ok(())
    }

    pub async fn perform_restore(&mut self) -> Result<()> {
        if let Some(config) = &self.restore_config {
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
                self.restore_result = Some(mock_operation_id.clone());
                self.loading = false;
                self.state = AppState::SelectingTargetInstance;

                let mock_operation = Operation {
                    id: mock_operation_id,
                    operation_type: "RESTORE_BACKUP".to_string(),
                    status: "DONE".to_string(),
                    target_id: config.target_instance.clone(),
                    start_time: Some(chrono::Utc::now()),
                    end_time: Some(chrono::Utc::now()),
                    error_message: None,
                };
                self.restore_result = Some(mock_operation.id.clone());
                self.restore_status = Some("DONE".to_string());
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
                        self.restore_result = Some(operation_id.clone());
                        self.restore_status = Some("RUNNING".to_string());
                        self.loading = false;
                        self.state = AppState::SelectingTargetInstance;
                    }
                    Err(e) => {
                        self.loading = false;
                        self.state = AppState::Error(format!("Restore failed: {}", e));
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn perform_create_backup(&mut self) -> Result<()> {
        if let Some(config) = &self.create_backup_config {
            self.loading = true;
            self.state = AppState::PerformingCreateBackup;

            if self.dry_run_mode {
                let mock_operation_id =
                    format!("dry-run-backup-op-{}", chrono::Utc::now().timestamp());
                self.backup_operation_id = Some(mock_operation_id.clone());
                self.backup_operation_status = Some("DONE".to_string());
                self.loading = false;
                self.state = AppState::PerformingCreateBackup;
            } else {
                match self.gcp_client.create_backup(config).await {
                    Ok(operation_id) => {
                        self.backup_operation_id = Some(operation_id.clone());
                        self.backup_operation_status = Some("RUNNING".to_string());
                        self.loading = false;
                        self.state = AppState::PerformingCreateBackup;
                    }
                    Err(e) => {
                        self.loading = false;
                        self.state = AppState::Error(format!("Create backup failed: {}", e));
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn check_restore_status(&mut self) -> Result<()> {
        if let (Some(operation_id), Some(config)) = (&self.restore_result, &self.restore_config) {
            if self.dry_run_mode {
                self.restore_status = Some("DONE".to_string());
                return Ok(());
            }

            match self
                .gcp_client
                .get_operation_status(&config.target_project, operation_id)
                .await
            {
                Ok(operation) => {
                    self.restore_status = Some(operation.status.clone());
                }
                Err(e) => {
                    eprintln!("Failed to check restore status: {}", e);
                }
            }
        }
        Ok(())
    }

    pub async fn check_backup_status(&mut self) -> Result<()> {
        if let (Some(operation_id), Some(config)) =
            (&self.backup_operation_id, &self.create_backup_config)
        {
            if self.dry_run_mode {
                self.backup_operation_status = Some("DONE".to_string());
                return Ok(());
            }

            match self
                .gcp_client
                .get_operation_status(&config.project, operation_id)
                .await
            {
                Ok(operation) => {
                    self.backup_operation_status = Some(operation.status.clone());
                }
                Err(e) => {
                    eprintln!("Failed to check backup status: {}", e);
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
            AppState::SelectingSourceInstance
            | AppState::SelectingTargetInstance
            | AppState::SelectingInstanceForBackup => {
                if self.selected_instance_index > 0 {
                    self.selected_instance_index -= 1;
                }
            }
            AppState::SelectingBackup => {
                if self.selected_backup_index > 0 {
                    self.selected_backup_index -= 1;
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
            AppState::SelectingSourceInstance
            | AppState::SelectingTargetInstance
            | AppState::SelectingInstanceForBackup => {
                if self.selected_instance_index < self.sql_instances.len().saturating_sub(1) {
                    self.selected_instance_index += 1;
                }
            }
            AppState::SelectingBackup => {
                if self.selected_backup_index < self.backups.len().saturating_sub(1) {
                    self.selected_backup_index += 1;
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
                if let Some(instance) = self.sql_instances.get(self.selected_instance_index).cloned()
                {
                    self.source_instance = Some(instance.name.clone());
                    if let Some(project) = &self.source_project.clone() {
                        self.state = AppState::SelectingBackup;
                        self.load_backups(project, &instance.name).await?;
                    }
                }
            }
            AppState::SelectingInstanceForBackup => {
                if let Some(instance) = self.sql_instances.get(self.selected_instance_index).cloned()
                {
                    self.source_instance = Some(instance.name.clone());
                    self.state = AppState::EnteringBackupName;
                    self.start_manual_input("backup_name");
                }
            }
            AppState::SelectingBackup => {
                if let Some(backup) = self.backups.get(self.selected_backup_index).cloned() {
                    self.selected_backup = Some(backup.id.clone());
                    self.state = AppState::SelectingTargetProject;
                    self.selected_project_index = 0;
                }
            }
            AppState::SelectingTargetProject => {
                self.start_manual_input("target_project");
            }
            AppState::SelectingTargetInstance => {
                if let Some(instance) = self.sql_instances.get(self.selected_instance_index).cloned()
                {
                    self.target_instance = Some(instance.name.clone());
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
            &self.selected_backup,
            &self.source_project,
            &self.source_instance,
            &self.target_project,
            &self.target_instance,
        ) {
            self.restore_config = Some(RestoreConfig {
                backup_id: backup_id.clone(),
                source_project: source_project.clone(),
                source_instance: source_instance.clone(),
                target_project: target_project.clone(),
                target_instance: target_instance.clone(),
            });
        }
    }

    pub fn create_backup_config(&mut self, backup_name: String) {
        if let (Some(project), Some(instance)) = (&self.source_project, &self.source_instance) {
            self.create_backup_config = Some(CreateBackupConfig {
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
                    self.source_project = Some(input_value.clone());
                    self.manual_input_active = false;
                    self.input_mode = InputMode::Normal;
                    match self.operation_mode {
                        Some(OperationMode::Restore) => self.state = AppState::SelectingSourceInstance,
                        Some(OperationMode::CreateBackup) => {
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
                    self.target_project = Some(input_value.clone());
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
                    self.sql_instances.push(instance);
                    self.selected_instance_index = self.sql_instances.len() - 1;
                }
                "backup" => {
                    let backup = Backup {
                        id: input_value.clone(),
                        start_time: None,
                        backup_type: "Manual".to_string(),
                        status: "Manual".to_string(),
                    };
                    self.backups.push(backup);
                    self.selected_backup_index = self.backups.len() - 1;
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
