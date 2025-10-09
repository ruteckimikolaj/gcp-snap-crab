use gcp_snap_crab::app::App;
use gcp_snap_crab::gcp::MockGcpClientTrait;
use gcp_snap_crab::types::{AppState, InputMode, OperationMode, SqlInstance};
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