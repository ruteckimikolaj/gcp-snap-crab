mod backup;
mod restore;

use anyhow::Result;

use crate::gcp::GcpClientTrait;
use crate::state::create_backup_flow::CreateBackupFlow;
use crate::state::restore_flow::RestoreFlow;
use crate::types::{
    AppState, Backup, InputMode, OperationMode, SqlInstance,
};
use crate::validation::{validate_backup_name, validate_instance_name, validate_project_id};

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
    pub filter_query: String,
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
            filter_query: String::new(),
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

    pub async fn select_project(&mut self, project: String) -> Result<()> {
        if !self.remembered_projects.contains(&project) {
            self.remembered_projects.push(project.clone());
        }
        self.filter_query.clear();
        match self.state {
            AppState::SelectingSourceProject => {
                self.restore_flow.source_project = Some(project.clone());
                self.state = AppState::SelectingSourceInstance;
            }
            AppState::SelectingProjectForBackup => {
                self.create_backup_flow.project = Some(project.clone());
                self.state = AppState::SelectingInstanceForBackup;
            }
            AppState::SelectingTargetProject => {
                self.restore_flow.target_project = Some(project.clone());
                self.state = AppState::SelectingTargetInstance;
            }
            _ => return Ok(()),
        }
        self.load_instances(&project).await?;
        Ok(())
    }

    pub fn filtered_instances(&self) -> Vec<&SqlInstance> {
        let instances = match self.operation_mode {
            Some(OperationMode::Restore) => &self.restore_flow.instances,
            Some(OperationMode::CreateBackup) => &self.create_backup_flow.instances,
            None => &self.restore_flow.instances,
        };
        if self.filter_query.is_empty() {
            instances.iter().collect()
        } else {
            let q = self.filter_query.to_lowercase();
            instances.iter().filter(|i| i.name.to_lowercase().contains(&q)).collect()
        }
    }

    pub fn filtered_backups(&self) -> Vec<&Backup> {
        if self.filter_query.is_empty() {
            self.restore_flow.backups.iter().collect()
        } else {
            let q = self.filter_query.to_lowercase();
            self.restore_flow.backups.iter().filter(|b| {
                b.id.to_lowercase().contains(&q)
                    || b.start_time
                        .map(|t| t.format("%Y-%m-%d").to_string())
                        .unwrap_or_default()
                        .contains(&q)
            }).collect()
        }
    }

    pub fn move_selection_up(&mut self) {
        match self.state {
            AppState::SelectingOperation => {
                if self.selected_operation_index > 0 {
                    self.selected_operation_index -= 1;
                }
            }
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
            AppState::SelectingSourceInstance | AppState::SelectingTargetInstance => {
                let max = self.filtered_instances().len().saturating_sub(1);
                if self.restore_flow.selected_instance_index < max {
                    self.restore_flow.selected_instance_index += 1;
                }
            }
            AppState::SelectingInstanceForBackup => {
                let max = self.filtered_instances().len().saturating_sub(1);
                if self.create_backup_flow.selected_instance_index < max {
                    self.create_backup_flow.selected_instance_index += 1;
                }
            }
            AppState::SelectingBackup => {
                let max = self.filtered_backups().len().saturating_sub(1);
                if self.restore_flow.selected_backup_index < max {
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
                self.load_projects().await?;
            }
            AppState::SelectingSourceInstance => {
                let name = self
                    .filtered_instances()
                    .get(self.restore_flow.selected_instance_index)
                    .map(|i| i.name.clone());
                if let Some(instance_name) = name {
                    self.filter_query.clear();
                    self.restore_flow.selected_instance_index = 0;
                    self.restore_flow.source_instance = Some(instance_name.clone());
                    if let Some(project) = &self.restore_flow.source_project.clone() {
                        self.state = AppState::SelectingBackup;
                        self.load_backups(project, &instance_name).await?;
                    }
                }
            }
            AppState::SelectingInstanceForBackup => {
                let name = self
                    .filtered_instances()
                    .get(self.create_backup_flow.selected_instance_index)
                    .map(|i| i.name.clone());
                if let Some(instance_name) = name {
                    self.filter_query.clear();
                    self.create_backup_flow.selected_instance_index = 0;
                    self.create_backup_flow.instance = Some(instance_name);
                    self.state = AppState::EnteringBackupName;
                    self.start_manual_input("backup_name");
                }
            }
            AppState::SelectingBackup => {
                let backup_id = self
                    .filtered_backups()
                    .get(self.restore_flow.selected_backup_index)
                    .map(|b| b.id.clone());
                if let Some(id) = backup_id {
                    self.filter_query.clear();
                    self.restore_flow.selected_backup_index = 0;
                    self.restore_flow.selected_backup = Some(id);
                    self.state = AppState::SelectingTargetProject;
                }
            }
            AppState::SelectingTargetProject => {
                self.start_manual_input("target_project");
            }
            AppState::SelectingTargetInstance => {
                let name = self
                    .filtered_instances()
                    .get(self.restore_flow.selected_instance_index)
                    .map(|i| i.name.clone());
                if let Some(instance_name) = name {
                    self.filter_query.clear();
                    self.restore_flow.selected_instance_index = 0;
                    self.restore_flow.target_instance = Some(instance_name);
                    self.create_restore_config();
                    self.state = AppState::ConfirmRestore;
                }
            }
            AppState::ConfirmCreateBackup => {
                self.perform_create_backup().await?;
            }
            AppState::ConfirmRestore => {
                self.perform_restore().await?;
            }
            _ => {}
        }
        Ok(())
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

    pub fn cancel_manual_input(&mut self) {
        self.manual_input_active = false;
        self.manual_input_buffer.clear();
        self.input_mode = InputMode::Normal;
    }

    pub async fn finish_manual_input(&mut self) -> Result<()> {
        let input_value = self.manual_input_buffer.trim().to_string();
        if !input_value.is_empty() {
            match self.manual_input_type.as_str() {
                "source_project" => {
                    if let Err(e) = validate_project_id(&input_value) {
                        self.error = Some(format!("Invalid project ID: {}. Press ESC to clear.", e));
                        return Ok(());
                    }
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
                            self.state = AppState::SelectingInstanceForBackup;
                        }
                        None => {}
                    }
                    self.load_instances(&input_value).await?;
                }
                "target_project" => {
                    if let Err(e) = validate_project_id(&input_value) {
                        self.error = Some(format!("Invalid project ID: {}. Press ESC to clear.", e));
                        return Ok(());
                    }
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
                    if let Err(e) = validate_instance_name(&input_value) {
                        self.error = Some(format!("Invalid instance name: {}. Press ESC to clear.", e));
                        return Ok(());
                    }
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
                    if let Err(e) = validate_backup_name(&input_value) {
                        self.error = Some(format!("Invalid backup name: {}. Press ESC to clear.", e));
                        return Ok(());
                    }
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

    pub(self) async fn poll_operation(&mut self, project: &str, operation_id: &str) -> Option<String> {
        match self.gcp_client.get_operation_status(project, operation_id).await {
            Ok(op) => Some(op.status),
            Err(e) => {
                self.error = Some(format!("Failed to check operation status: {}", e));
                None
            }
        }
    }
}
