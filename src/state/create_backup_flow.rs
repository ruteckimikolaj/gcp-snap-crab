use crate::types::{CreateBackupConfig, SqlInstance};

#[derive(Default)]
pub struct CreateBackupFlow {
    pub project: Option<String>,
    pub instance: Option<String>,
    pub config: Option<CreateBackupConfig>,
    pub operation_id: Option<String>,
    pub status: Option<String>,
    pub instances: Vec<SqlInstance>,
    pub selected_instance_index: usize,
}

impl CreateBackupFlow {
    pub fn new() -> Self {
        Self::default()
    }
}