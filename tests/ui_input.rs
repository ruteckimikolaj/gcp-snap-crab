use gcp_snap_crab::app::App;
use gcp_snap_crab::gcp::MockGcpClientTrait;
use gcp_snap_crab::types::InputMode;
use gcp_snap_crab::ui::{handle_edit_input, handle_normal_input};
use crossterm::event::{KeyCode, KeyModifiers};

fn create_test_app() -> App {
    let mock_gcp_client = MockGcpClientTrait::new();
    App::new(Box::new(mock_gcp_client), false)
}

#[tokio::test]
async fn test_handle_normal_input_toggle_help() {
    let mut app = create_test_app();
    assert!(!app.show_help);

    handle_normal_input(&mut app, KeyCode::Char('h'), KeyModifiers::NONE)
        .await
        .unwrap();
    assert!(app.show_help);

    handle_normal_input(&mut app, KeyCode::Char('h'), KeyModifiers::NONE)
        .await
        .unwrap();
    assert!(!app.show_help);
}

#[tokio::test]
async fn test_handle_normal_input_escape_from_manual_input() {
    let mut app = create_test_app();
    app.start_manual_input("test");
    assert!(app.manual_input_active);

    handle_normal_input(&mut app, KeyCode::Esc, KeyModifiers::NONE)
        .await
        .unwrap();
    assert!(!app.manual_input_active);
}

#[tokio::test]
async fn test_handle_edit_input_char_and_backspace() {
    let mut app = create_test_app();
    app.start_manual_input("test");

    handle_edit_input(&mut app, KeyCode::Char('a')).await.unwrap();
    assert_eq!(app.manual_input_buffer, "a");

    handle_edit_input(&mut app, KeyCode::Char('b')).await.unwrap();
    assert_eq!(app.manual_input_buffer, "ab");

    handle_edit_input(&mut app, KeyCode::Backspace).await.unwrap();
    assert_eq!(app.manual_input_buffer, "a");

    handle_edit_input(&mut app, KeyCode::Backspace).await.unwrap();
    assert_eq!(app.manual_input_buffer, "");
}

#[tokio::test]
async fn test_handle_edit_input_escape() {
    let mut app = create_test_app();
    app.start_manual_input("test");
    app.manual_input_buffer = "some text".to_string();

    handle_edit_input(&mut app, KeyCode::Esc).await.unwrap();
    assert!(!app.manual_input_active);
    assert_eq!(app.input_mode, InputMode::Normal);
    assert!(app.manual_input_buffer.is_empty());
}