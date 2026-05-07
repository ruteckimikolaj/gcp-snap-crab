mod popups;
mod render;
mod widgets;

use popups::{
    render_create_backup_warning_popup, render_error_popup, render_help_popup,
    render_manual_input_popup, render_restore_warning_popup,
};
use render::{
    render_content, render_footer, render_header,
};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{backend::Backend, style::Color, Frame, Terminal};
use std::time::{Duration, Instant};

use crate::app::App;
use crate::types::{AppState, InputMode};

pub(super) const BASE_FG: Color = Color::Rgb(216, 222, 233);
pub(super) const BASE_BG: Color = Color::Rgb(46, 52, 64);
pub(super) const ACCENT_COLOR: Color = Color::Rgb(136, 192, 208);
pub(super) const SUCCESS_COLOR: Color = Color::Rgb(163, 190, 140);
pub(super) const WARNING_COLOR: Color = Color::Rgb(235, 203, 139);
pub(super) const HIGHLIGHT_BG: Color = Color::Rgb(59, 66, 82);
pub(super) const BORDER_COLOR: Color = Color::Rgb(76, 86, 106);
pub(super) const INPUT_TEXT: Color = Color::Rgb(235, 203, 139);

pub async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()>
where
    <B as Backend>::Error: Send + Sync + 'static,
{
    app.initialize().await?;
    let mut last_tick = Instant::now();
    let mut last_status_check = Instant::now();
    let tick_rate = Duration::from_millis(250);
    let status_check_interval = Duration::from_secs(5);

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.input_mode {
                        InputMode::Normal => {
                            if let Err(e) =
                                handle_normal_input(&mut app, key.code, key.modifiers).await
                            {
                                app.state = AppState::Error(e.to_string());
                            }
                        }
                        InputMode::Editing => {
                            handle_edit_input(&mut app, key.code).await?;
                        }
                        InputMode::Filtering => {
                            handle_filter_input(&mut app, key.code).await?;
                        }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }

        if last_status_check.elapsed() >= status_check_interval {
            if app.restore_flow.operation_id.is_some() {
                let _ = app.check_restore_status().await;
            }
            if app.create_backup_flow.operation_id.is_some() {
                let _ = app.check_backup_status().await;
            }
            last_status_check = Instant::now();
        }

        if matches!(app.state, AppState::Quitting) {
            break;
        }
        if matches!(app.state, AppState::Error(_)) && !app.show_help {
            break;
        }
    }

    Ok(())
}

pub async fn handle_normal_input(
    app: &mut App,
    key: KeyCode,
    _modifiers: KeyModifiers,
) -> Result<()> {
    match key {
        KeyCode::Char('q') => {
            app.state = AppState::Quitting;
        }
        KeyCode::Esc => {
            if app.error.is_some() {
                app.error = None;
            } else if app.show_help {
                app.toggle_help();
            } else if app.manual_input_active {
                app.cancel_manual_input();
            } else {
                match app.state {
                    AppState::ConfirmRestore => {
                        app.restore_flow.target_instance = None;
                        app.restore_flow.selected_instance_index = 0;
                        app.state = AppState::SelectingTargetInstance;
                    }
                    AppState::ConfirmCreateBackup => {
                        app.create_backup_flow.config = None;
                        app.state = AppState::EnteringBackupName;
                    }
                    AppState::SelectingSourceInstance => {
                        app.restore_flow.source_project = None;
                        app.restore_flow.instances.clear();
                        app.restore_flow.selected_instance_index = 0;
                        app.state = AppState::SelectingSourceProject;
                    }
                    AppState::SelectingBackup => {
                        app.restore_flow.source_instance = None;
                        app.restore_flow.backups.clear();
                        app.restore_flow.selected_backup_index = 0;
                        app.state = AppState::SelectingSourceInstance;
                    }
                    AppState::SelectingTargetProject => {
                        app.restore_flow.selected_backup = None;
                        app.state = AppState::SelectingBackup;
                    }
                    AppState::SelectingTargetInstance => {
                        app.restore_flow.target_project = None;
                        app.restore_flow.instances.clear();
                        app.restore_flow.selected_instance_index = 0;
                        app.state = AppState::SelectingTargetProject;
                    }
                    AppState::PerformingRestore => {
                        app.state = AppState::SelectingTargetInstance;
                    }
                    AppState::SelectingInstanceForBackup => {
                        app.create_backup_flow.project = None;
                        app.create_backup_flow.instances.clear();
                        app.create_backup_flow.selected_instance_index = 0;
                        app.state = AppState::SelectingProjectForBackup;
                    }
                    AppState::EnteringBackupName => {
                        app.create_backup_flow.instance = None;
                        app.state = AppState::SelectingInstanceForBackup;
                    }
                    AppState::PerformingCreateBackup => {
                        app.state = AppState::ConfirmCreateBackup;
                    }
                    _ => {
                        app.state = AppState::SelectingOperation;
                    }
                }
            }
        }
        KeyCode::Char('/') => {
            if app.input_mode != InputMode::Filtering {
                match app.state {
                    AppState::SelectingSourceInstance
                    | AppState::SelectingTargetInstance
                    | AppState::SelectingInstanceForBackup
                    | AppState::SelectingBackup => {
                        app.input_mode = InputMode::Filtering;
                    }
                    _ => {}
                }
            }
        }
        KeyCode::Char('h') => app.toggle_help(),
        KeyCode::Up => app.move_selection_up(),
        KeyCode::Down => app.move_selection_down(),
        KeyCode::Enter => app.select_current_item().await?,
        KeyCode::Char('m') => match app.state {
            AppState::SelectingSourceProject
            | AppState::SelectingTargetProject
            | AppState::SelectingProjectForBackup => {
                app.start_manual_input("source_project");
            }
            AppState::SelectingSourceInstance
            | AppState::SelectingTargetInstance
            | AppState::SelectingInstanceForBackup => {
                app.start_manual_input("instance");
            }
            AppState::SelectingBackup => {
                app.start_manual_input("backup");
            }
            AppState::EnteringBackupName => {
                app.start_manual_input("backup_name");
            }
            _ => {}
        },
        KeyCode::Char('r') => {
            match app.state {
                AppState::SelectingSourceInstance | AppState::SelectingTargetInstance => {
                    if let Some(project) = &app.restore_flow.source_project.clone() {
                        app.load_instances(project).await?;
                    }
                }
                AppState::SelectingInstanceForBackup => {
                    if let Some(project) = &app.create_backup_flow.project.clone() {
                        app.load_instances(project).await?;
                    }
                }
                AppState::SelectingBackup => {
                    if let (Some(project), Some(instance)) = (
                        &app.restore_flow.source_project.clone(),
                        &app.restore_flow.source_instance.clone(),
                    ) {
                        app.load_backups(project, instance).await?;
                    }
                }
                _ => {}
            }
            if app.restore_flow.operation_id.is_some() {
                app.check_restore_status().await?;
            }
            if app.create_backup_flow.operation_id.is_some() {
                app.check_backup_status().await?;
            }
        }
        KeyCode::Char('n') => {
            app.state = AppState::SelectingOperation;
            app.operation_mode = None;
            app.restore_flow = crate::state::restore_flow::RestoreFlow::new();
            app.create_backup_flow = crate::state::create_backup_flow::CreateBackupFlow::new();
        }
        KeyCode::Char('y') => {
            let text = if let Some(op_id) = &app.restore_flow.operation_id {
                Some(op_id.clone())
            } else if let Some(op_id) = &app.create_backup_flow.operation_id {
                Some(op_id.clone())
            } else if matches!(app.state, AppState::SelectingBackup) {
                app.filtered_backups()
                    .get(app.restore_flow.selected_backup_index)
                    .map(|b| b.id.clone())
            } else {
                app.restore_flow.selected_backup.clone()
            };
            if let Some(text) = text {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    if clipboard.set_text(text.clone()).is_ok() {
                        app.yank_notification = Some((text, std::time::Instant::now()));
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

pub async fn handle_edit_input(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Enter => {
            if app.manual_input_active {
                app.finish_manual_input().await?;
            }
        }
        KeyCode::Esc => {
            if app.manual_input_active {
                app.cancel_manual_input();
            } else {
                app.input_mode = InputMode::Normal;
                app.input_buffer.clear();
            }
        }
        KeyCode::Char(c) => {
            if app.manual_input_active {
                app.manual_input_buffer.push(c);
            } else {
                app.input_buffer.push(c);
            }
        }
        KeyCode::Backspace => {
            if app.manual_input_active {
                app.manual_input_buffer.pop();
            } else {
                app.input_buffer.pop();
            }
        }
        _ => {}
    }
    Ok(())
}

pub async fn handle_filter_input(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('/') => {
            app.input_mode = InputMode::Normal;
            if matches!(key, KeyCode::Esc) {
                app.filter_query.clear();
                app.restore_flow.selected_instance_index = 0;
                app.create_backup_flow.selected_instance_index = 0;
                app.restore_flow.selected_backup_index = 0;
            }
        }
        KeyCode::Char(c) => {
            app.filter_query.push(c);
            app.restore_flow.selected_instance_index = 0;
            app.create_backup_flow.selected_instance_index = 0;
            app.restore_flow.selected_backup_index = 0;
        }
        KeyCode::Backspace => {
            app.filter_query.pop();
            app.restore_flow.selected_instance_index = 0;
            app.create_backup_flow.selected_instance_index = 0;
            app.restore_flow.selected_backup_index = 0;
        }
        _ => {}
    }
    Ok(())
}

fn ui(f: &mut Frame, app: &mut App) {
    use ratatui::layout::{Constraint, Direction, Layout};

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    render_header(f, main_chunks[0], app);
    render_content(f, main_chunks[1], app);
    render_footer(f, main_chunks[2], app);

    if app.show_help {
        render_help_popup(f, app);
    }
    if app.manual_input_active {
        render_manual_input_popup(f, app);
    }
    if matches!(app.state, AppState::ConfirmRestore) {
        render_restore_warning_popup(f, app);
    }
    if matches!(app.state, AppState::ConfirmCreateBackup) {
        render_create_backup_warning_popup(f, app);
    }
    if app.error.is_some() {
        render_error_popup(f, app);
    }
}
