use crate::types::{RestoreConfig, SqlInstance, Backup};

#[derive(Default)]
pub struct RestoreFlow {
    pub source_project: Option<String>,
    pub source_instance: Option<String>,
    pub target_project: Option<String>,
    pub target_instance: Option<String>,
    pub selected_backup: Option<String>,
    pub config: Option<RestoreConfig>,
    pub operation_id: Option<String>,
    pub status: Option<String>,
    pub instances: Vec<SqlInstance>,
    pub backups: Vec<Backup>,
    pub selected_instance_index: usize,
    pub selected_backup_index: usize,
}

impl RestoreFlow {
    pub fn new() -> Self {
        Self::default()
    }
}