use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame, Terminal,
};
use std::time::{Duration, Instant};

use crate::app::App;
use crate::types::{AppState, InputMode, OperationMode};

// Clean color palette for better visibility and modern look
const BASE_FG: Color = Color::Rgb(216, 222, 233); // Main text
const BASE_BG: Color = Color::Rgb(46, 52, 64); // Background
const ACCENT_COLOR: Color = Color::Rgb(136, 192, 208); // Primary accent
const SUCCESS_COLOR: Color = Color::Rgb(163, 190, 140); // Success/green
const WARNING_COLOR: Color = Color::Rgb(235, 203, 139); // Warning/yellow
const HIGHLIGHT_BG: Color = Color::Rgb(59, 66, 82); // Selection background
const BORDER_COLOR: Color = Color::Rgb(76, 86, 106); // Inactive borders
const INPUT_TEXT: Color = Color::Rgb(235, 203, 139); // Input text - bright and visible

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

fn render_error_popup(f: &mut Frame, app: &mut App) {
    if let Some(error_msg) = &app.error {
        let popup_area = centered_rect(60, 25, f.area());
        f.render_widget(Clear, popup_area); //this clears the background

        let error_text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "❌ ERROR ❌",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(error_msg.as_str()),
        ];

        let block = Block::default()
            .title("Error")
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .style(Style::default().fg(Color::Red));

        let paragraph = Paragraph::new(error_text)
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, popup_area);
    }
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let title = if app.dry_run_mode {
        " GCP SQL Backup Tool - DRY RUN MODE "
    } else {
        " GCP SQL Backup Tool "
    };

    let subtitle = match app.state {
        AppState::SelectingOperation => "Welcome - Choose an operation to start",
        AppState::CheckingPrerequisites => "Checking Prerequisites...",
        AppState::SelectingSourceProject => "Step 1/5: Select Source Project",
        AppState::SelectingSourceInstance => "Step 2/5: Select Source Instance",
        AppState::SelectingBackup => "Step 3/5: Select Backup",
        AppState::SelectingTargetProject => "Step 4/5: Select Target Project",
        AppState::SelectingTargetInstance => "Step 5/5: Select Target Instance",
        AppState::ConfirmRestore => "Step 6: Confirm Restoration",
        AppState::PerformingRestore => "Monitoring Restore Progress...",
        AppState::SelectingProjectForBackup => "Step 1/4: Select Project for Backup",
        AppState::SelectingInstanceForBackup => "Step 2/4: Select Instance for Backup",
        AppState::EnteringBackupName => "Step 3/4: Enter Backup Name",
        AppState::ConfirmCreateBackup => "Step 4: Confirm Backup Creation",
        AppState::PerformingCreateBackup => "Monitoring Backup Creation...",
        AppState::Error(_) => "Error Occurred",
        AppState::Quitting => "",
    };

    let header_block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(if app.dry_run_mode {
            Style::default().fg(WARNING_COLOR).bg(BASE_BG)
        } else {
            Style::default().fg(BASE_FG).bg(BASE_BG)
        });

    let header_content = Paragraph::new(subtitle)
        .style(Style::default().fg(ACCENT_COLOR))
        .alignment(Alignment::Center)
        .block(header_block);

    f.render_widget(header_content, area);
}

fn render_content(f: &mut Frame, area: Rect, app: &mut App) {
    match &app.state {
        AppState::SelectingOperation => render_operation_selection(f, area, app),
        AppState::CheckingPrerequisites => render_loading(f, area, "Checking prerequisites..."),
        AppState::SelectingSourceProject
        | AppState::SelectingSourceInstance
        | AppState::SelectingBackup
        | AppState::SelectingTargetProject
        | AppState::SelectingTargetInstance
        | AppState::ConfirmRestore
        | AppState::PerformingRestore => render_two_section_layout(f, area, app),
        AppState::SelectingProjectForBackup
        | AppState::SelectingInstanceForBackup
        | AppState::EnteringBackupName
        | AppState::ConfirmCreateBackup
        | AppState::PerformingCreateBackup => render_create_backup_layout(f, area, app),
        AppState::Error(msg) => render_error(f, area, msg),
        AppState::Quitting => {}
    }
}

fn render_create_backup_layout(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    render_backup_project_selection(f, top_chunks[0], app);
    render_backup_instance_selection(f, top_chunks[1], app);
    render_backup_name_input(f, bottom_chunks[0], app);
    render_backup_status(f, bottom_chunks[1], app);
}

fn render_backup_project_selection(f: &mut Frame, area: Rect, app: &mut App) {
    let hint = matches!(app.state, AppState::SelectingProjectForBackup)
        .then_some("→ Press Enter to select...");
    render_step_box(f, area, "Project to Backup", app.create_backup_flow.project.as_deref(), hint);
}

fn render_backup_instance_selection(f: &mut Frame, area: Rect, app: &mut App) {
    if matches!(app.state, AppState::SelectingInstanceForBackup)
        && !app.create_backup_flow.instances.is_empty()
        && app.create_backup_flow.instance.is_none()
    {
        render_instance_list(f, area, app, "Instance to Backup");
    } else {
        let inst_hint = matches!(app.state, AppState::SelectingInstanceForBackup).then(|| {
            if app.loading { "→ Loading instances..." }
            else if app.create_backup_flow.instances.is_empty() { "→ No instances found" }
            else { "→ Select instance..." }
        });
        render_step_box(f, area, "Instance to Backup", app.create_backup_flow.instance.as_deref(), inst_hint);
    }
}

fn render_backup_name_input(f: &mut Frame, area: Rect, app: &mut App) {
    let hint = matches!(app.state, AppState::EnteringBackupName)
        .then_some("→ Press Enter to name backup...");
    let name_value = app.create_backup_flow.config.as_ref().map(|c| c.name.as_str());
    render_step_box(f, area, "Backup Name", name_value, hint);
}

fn render_backup_status(f: &mut Frame, area: Rect, app: &mut App) {
    let status_content = if let Some(_operation_id) = &app.create_backup_flow.operation_id {
        match app.create_backup_flow.status.as_deref() {
            Some("DONE") => "✅ Backup created successfully!",
            Some("RUNNING") => "🔄 Backup in progress...",
            Some("PENDING") => "⏳ Backup is pending...",
            Some("FAILED") | Some("ERROR") => "❌ Backup failed!",
            _ => "📊 Checking backup status...",
        }
    } else if app.create_backup_flow.config.is_some() {
        "✅ Ready to create backup!\nPress Enter to confirm."
    } else {
        "Complete previous steps."
    };

    let status_style = if let Some(_) = &app.create_backup_flow.operation_id {
        match app.create_backup_flow.status.as_deref() {
            Some("DONE") => Style::default().fg(SUCCESS_COLOR),
            Some("RUNNING") => Style::default().fg(WARNING_COLOR),
            Some("PENDING") => Style::default().fg(ACCENT_COLOR),
            Some("FAILED") | Some("ERROR") => Style::default().fg(Color::Red),
            _ => Style::default().fg(WARNING_COLOR),
        }
    } else if app.create_backup_flow.config.is_some() {
        Style::default().fg(SUCCESS_COLOR)
    } else {
        Style::default().fg(BORDER_COLOR)
    };

    f.render_widget(
        Paragraph::new(status_content)
            .block(
                Block::default()
                    .title("Backup Status")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(status_style),
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_two_section_layout(f: &mut Frame, area: Rect, app: &mut App) {
    // Create 2-section horizontal layout like example app
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Source section
            Constraint::Percentage(50), // Target section
        ])
        .split(area);

    render_source_section(f, main_chunks[0], app);
    render_target_section(f, main_chunks[1], app);
}

fn render_step_box(f: &mut Frame, area: Rect, title: &str, value: Option<&str>, active_hint: Option<&str>) {
    let style = if active_hint.is_some() && value.is_none() {
        Style::default().fg(ACCENT_COLOR)
    } else if value.is_some() {
        Style::default().fg(SUCCESS_COLOR)
    } else {
        Style::default().fg(BORDER_COLOR)
    };
    let content = if let Some(v) = value {
        format!("✓ {}", v)
    } else if let Some(hint) = active_hint {
        hint.to_string()
    } else {
        "Pending...".to_string()
    };
    f.render_widget(
        Paragraph::new(content)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(style),
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_source_section(f: &mut Frame, area: Rect, app: &mut App) {
    let instance_is_list = matches!(app.state, AppState::SelectingSourceInstance)
        && !app.restore_flow.instances.is_empty()
        && app.restore_flow.source_instance.is_none();
    let backup_is_list = matches!(app.state, AppState::SelectingBackup)
        && !app.restore_flow.backups.is_empty()
        && app.restore_flow.selected_backup.is_none();
    let constraints: &[Constraint] = if instance_is_list {
        &[Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)]
    } else if backup_is_list {
        &[Constraint::Length(3), Constraint::Length(3), Constraint::Min(0)]
    } else {
        &[Constraint::Length(3), Constraint::Length(3), Constraint::Length(3)]
    };
    let source_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    // Source Project
    let proj_hint = matches!(app.state, AppState::SelectingSourceProject)
        .then_some("→ Press Enter to select...");
    render_step_box(f, source_chunks[0], "Source Project", app.restore_flow.source_project.as_deref(), proj_hint);

    // Source Instance
    if matches!(app.state, AppState::SelectingSourceInstance)
        && !app.restore_flow.instances.is_empty()
        && app.restore_flow.source_instance.is_none()
    {
        render_instance_list(f, source_chunks[1], app, "Source Instance");
    } else {
        let inst_hint = matches!(app.state, AppState::SelectingSourceInstance).then(|| {
            if app.loading { "→ Loading instances..." }
            else if app.restore_flow.instances.is_empty() { "→ No instances found" }
            else { "→ Select instance..." }
        });
        render_step_box(f, source_chunks[1], "Source Instance", app.restore_flow.source_instance.as_deref(), inst_hint);
    }

    // Source Backup
    if matches!(app.state, AppState::SelectingBackup)
        && !app.restore_flow.backups.is_empty()
        && app.restore_flow.selected_backup.is_none()
    {
        render_backup_list(f, source_chunks[2], app);
    } else {
        let backup_hint = matches!(app.state, AppState::SelectingBackup).then(|| {
            if app.loading { "→ Loading backups...".to_string() }
            else if app.restore_flow.backups.is_empty() { "→ No backups found".to_string() }
            else { format!("→ Choose from {} backups", app.restore_flow.backups.len()) }
        });
        render_step_box(f, source_chunks[2], "Source Backup", app.restore_flow.selected_backup.as_deref(), backup_hint.as_deref());
    }
}

fn render_instance_list(f: &mut Frame, area: Rect, app: &mut App, title: &str) {
    let is_filtering = app.input_mode == InputMode::Filtering;
    let list_area = area;

    let selected_index = match app.operation_mode {
        Some(OperationMode::Restore) => app.restore_flow.selected_instance_index,
        Some(OperationMode::CreateBackup) => app.create_backup_flow.selected_instance_index,
        None => 0,
    };
    let instances = app.filtered_instances();
    let total = instances.len();

    let items: Vec<ListItem> = instances
        .iter()
        .enumerate()
        .map(|(i, instance)| {
            let style = if i == selected_index {
                Style::default()
                    .fg(ACCENT_COLOR)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(BASE_FG)
            };
            ListItem::new(format!("  {}", instance.name)).style(style)
        })
        .collect();

    let block_title = if is_filtering {
        format!("{} — {} match(es)", title, total)
    } else {
        format!("{} — [/] to search", title)
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(block_title)
                .style(Style::default().fg(ACCENT_COLOR)),
        )
        .highlight_style(
            Style::default()
                .bg(HIGHLIGHT_BG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("► ");

    let mut state = ListState::default();
    state.select(Some(selected_index));
    f.render_stateful_widget(list, list_area, &mut state);

    let mut scrollbar_state = ScrollbarState::new(total).position(selected_index);
    f.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        list_area,
        &mut scrollbar_state,
    );
}

fn render_backup_list(f: &mut Frame, area: Rect, app: &mut App) {
    let is_filtering = app.input_mode == InputMode::Filtering;
    let list_area = area;

    let selected_index = app.restore_flow.selected_backup_index;
    let backups = app.filtered_backups();
    let total = backups.len();

    let items: Vec<ListItem> = backups
        .iter()
        .enumerate()
        .map(|(i, backup)| {
            let style = if i == selected_index {
                Style::default()
                    .fg(ACCENT_COLOR)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(BASE_FG)
            };
            let date_str = backup
                .start_time
                .map(|t| t.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            ListItem::new(format!("  {} | {}", date_str, backup.id)).style(style)
        })
        .collect();

    let block_title = if is_filtering {
        format!("Source Backup — {} match(es)", total)
    } else {
        "Source Backup — [/] to search".to_string()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(block_title)
                .style(Style::default().fg(ACCENT_COLOR)),
        )
        .highlight_style(
            Style::default()
                .bg(HIGHLIGHT_BG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("► ");

    let mut state = ListState::default();
    state.select(Some(selected_index));
    f.render_stateful_widget(list, list_area, &mut state);

    let mut scrollbar_state = ScrollbarState::new(total).position(selected_index);
    f.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        list_area,
        &mut scrollbar_state,
    );
}

fn render_target_section(f: &mut Frame, area: Rect, app: &mut App) {
    let instance_is_list = matches!(app.state, AppState::SelectingTargetInstance)
        && !app.restore_flow.instances.is_empty()
        && app.restore_flow.target_instance.is_none();
    let constraints: &[Constraint] = if instance_is_list {
        &[Constraint::Length(3), Constraint::Min(0), Constraint::Length(5)]
    } else {
        &[Constraint::Length(3), Constraint::Length(3), Constraint::Min(0)]
    };
    let target_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    // Target Project
    let proj_hint = matches!(app.state, AppState::SelectingTargetProject)
        .then_some("→ Press Enter to select...");
    render_step_box(f, target_chunks[0], "Target Project", app.restore_flow.target_project.as_deref(), proj_hint);

    // Target Instance
    if matches!(app.state, AppState::SelectingTargetInstance)
        && !app.restore_flow.instances.is_empty()
        && app.restore_flow.target_instance.is_none()
    {
        render_instance_list(f, target_chunks[1], app, "Target Instance");
    } else {
        let inst_hint = matches!(app.state, AppState::SelectingTargetInstance).then(|| {
            if app.loading { "→ Loading instances..." }
            else if app.restore_flow.instances.is_empty() { "→ No instances found" }
            else { "→ Select instance..." }
        });
        render_step_box(f, target_chunks[1], "Target Instance", app.restore_flow.target_instance.as_deref(), inst_hint);
    }

    // Status/Info section - Now shows restore progress with actual status
    let status_content = if let Some(_operation_id) = &app.restore_flow.operation_id {
        match app.restore_flow.status.as_deref() {
            Some("DONE") => "✅ Restore completed successfully!\nBackup has been applied.",
            Some("RUNNING") => {
                "🔄 Restore in progress...\nPlease wait, this may take several minutes."
            }
            Some("PENDING") => "⏳ Restore is pending...\nOperation is queued for execution.",
            Some("FAILED") | Some("ERROR") => "❌ Restore failed!\nCheck logs for details.",
            _ => "📊 Checking restore status...\nMonitoring progress...",
        }
    } else if app.restore_flow.target_instance.is_some()
        && app.restore_flow.selected_backup.is_some()
    {
        "✅ Ready to restore!\nPress Enter to confirm."
    } else {
        "Complete source\nselection first."
    };

    let status_style = if let Some(_) = &app.restore_flow.operation_id {
        match app.restore_flow.status.as_deref() {
            Some("DONE") => Style::default().fg(SUCCESS_COLOR),
            Some("RUNNING") => Style::default().fg(WARNING_COLOR),
            Some("PENDING") => Style::default().fg(ACCENT_COLOR),
            Some("FAILED") | Some("ERROR") => Style::default().fg(Color::Red),
            _ => Style::default().fg(WARNING_COLOR),
        }
    } else if app.restore_flow.target_instance.is_some()
        && app.restore_flow.selected_backup.is_some()
    {
        Style::default().fg(SUCCESS_COLOR)
    } else {
        Style::default().fg(BORDER_COLOR)
    };

    f.render_widget(
        Paragraph::new(status_content)
            .block(
                Block::default()
                    .title("Restore Status")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(status_style),
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        target_chunks[2],
    );
}

fn render_loading(f: &mut Frame, area: Rect, message: &str) {
    let loading_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "⏳ Loading...",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(message),
        Line::from(""),
        Line::from("Please wait..."),
    ];

    let loading = Paragraph::new(loading_text)
        .block(Block::default().borders(Borders::ALL).title("Loading"))
        .alignment(Alignment::Center);

    f.render_widget(loading, area);
}

fn render_search_bar(f: &mut Frame, area: Rect, query: &str) {
    let text = format!(" {}█", query);
    f.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(" SEARCH MODE — type to filter · ESC cancel · ENTER confirm · / exit ")
                    .style(Style::default().fg(WARNING_COLOR)),
            )
            .style(Style::default().fg(INPUT_TEXT)),
        area,
    );
}

fn render_error(f: &mut Frame, area: Rect, error_msg: &str) {
    let error_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "❌ ERROR",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(error_msg),
        Line::from(""),
        Line::from(Span::styled(
            "Press 'q' to exit",
            Style::default().fg(Color::Yellow),
        )),
    ];

    let error = Paragraph::new(error_text)
        .block(Block::default().borders(Borders::ALL).title("Error"))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(error, area);
}

fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    if app.input_mode == InputMode::Filtering {
        render_search_bar(f, area, &app.filter_query);
        return;
    }

    let help_text = if app.manual_input_active {
        " [Enter] Confirm | [Esc] Cancel "
    } else {
        match app.state {
            AppState::SelectingOperation => {
                " [↑/↓] Navigate | [Enter] Select | [h] Help | [q] Quit "
            }
            _ => {
                if app.restore_flow.operation_id.is_some()
                    || app.create_backup_flow.operation_id.is_some()
                {
                    " [↑/↓] Navigate | [Enter] Select | [Esc] Back | [r] Refresh | [n] New | [h] Help | [q] Quit "
                } else {
                    " [↑/↓] Navigate | [Enter] Select | [Esc] Back | [r] Refresh | [/] Search | [h] Help | [q] Quit "
                }
            }
        }
    };

    f.render_widget(
        Paragraph::new(help_text)
            .block(
                Block::default()
                    .title("Controls")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(Style::default().fg(BORDER_COLOR)),
            )
            .alignment(Alignment::Center)
            .style(Style::default().fg(BASE_FG)),
        area,
    );
}

fn render_restore_warning_popup(f: &mut Frame, app: &App) {
    if let Some(config) = &app.restore_flow.config {
        let popup_area = centered_rect(85, 60, f.area());
        f.render_widget(Clear, popup_area);

        let warning_block = Block::default()
            .title("⚠️  CRITICAL WARNING - BACKUP RESTORATION  ⚠️")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .style(
                Style::default().fg(Color::White).bg(Color::Rgb(139, 0, 0)), // Dark red background
            );

        f.render_widget(warning_block, popup_area);

        let inner_area = Rect {
            x: popup_area.x + 2,
            y: popup_area.y + 2,
            width: popup_area.width.saturating_sub(4),
            height: popup_area.height.saturating_sub(4),
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(8),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(inner_area);

        let header_text = vec![Line::from(Span::styled(
            "🚨 IRREVERSIBLE DATABASE RESTORATION 🚨",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
        ))];
        f.render_widget(
            Paragraph::new(header_text)
                .alignment(Alignment::Center)
                .style(Style::default().bg(Color::Rgb(139, 0, 0))),
            chunks[0],
        );

        let source_text = format!("{} → {}", config.source_project, config.source_instance);
        let target_text = format!("{} → {}", config.target_project, config.target_instance);

        let config_text = vec![
            Line::from(Span::styled(
                "Restoration Configuration:",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "📂 Source: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(&source_text, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(
                    "💾 Backup: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(&config.backup_id, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(
                    "🎯 Target: ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(&target_text, Style::default().fg(Color::White)),
            ]),
        ];
        f.render_widget(
            Paragraph::new(config_text)
                .alignment(Alignment::Left)
                .style(Style::default().bg(Color::Rgb(139, 0, 0)))
                .wrap(Wrap { trim: true }),
            chunks[1],
        );

        let danger_text = vec![Line::from(Span::styled(
            "⚠️  THIS WILL COMPLETELY REPLACE THE TARGET DATABASE  ⚠️",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::SLOW_BLINK),
        ))];
        f.render_widget(
            Paragraph::new(danger_text)
                .alignment(Alignment::Center)
                .style(Style::default().bg(Color::Rgb(139, 0, 0))),
            chunks[2],
        );

        let instructions_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("• All existing data in ", Style::default().fg(Color::White)),
                Span::styled(
                    &config.target_instance,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " will be PERMANENTLY LOST",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(Span::styled(
                "• This operation cannot be undone or reversed",
                Style::default().fg(Color::White),
            )),
            Line::from(Span::styled(
                "• The restoration process may take several minutes",
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "[Enter] ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "PROCEED WITH RESTORATION  ",
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    "[Esc] ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::styled("CANCEL AND GO BACK", Style::default().fg(Color::White)),
            ]),
        ];
        f.render_widget(
            Paragraph::new(instructions_text)
                .alignment(Alignment::Center)
                .style(Style::default().bg(Color::Rgb(139, 0, 0)))
                .wrap(Wrap { trim: true }),
            chunks[3],
        );
    }
}

fn render_create_backup_warning_popup(f: &mut Frame, app: &App) {
    if let Some(config) = &app.create_backup_flow.config {
        let popup_area = centered_rect(85, 60, f.area());
        f.render_widget(Clear, popup_area);

        let warning_block = Block::default()
            .title("✅  Confirm Backup Creation  ✅")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .style(Style::default().fg(Color::White).bg(Color::DarkGray));

        f.render_widget(warning_block, popup_area);

        let inner_area = Rect {
            x: popup_area.x + 2,
            y: popup_area.y + 2,
            width: popup_area.width.saturating_sub(4),
            height: popup_area.height.saturating_sub(4),
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(8),
                Constraint::Min(0),
            ])
            .split(inner_area);

        let header_text = vec![Line::from(Span::styled(
            "Please confirm the details below",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ))];
        f.render_widget(
            Paragraph::new(header_text)
                .alignment(Alignment::Center)
                .style(Style::default().bg(Color::DarkGray)),
            chunks[0],
        );

        let config_text = vec![
            Line::from(Span::styled(
                "Backup Configuration:",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "📂 Project: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(&config.project, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(
                    "💾 Instance: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(&config.instance, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(
                    "📝 Name: ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(&config.name, Style::default().fg(Color::White)),
            ]),
        ];
        f.render_widget(
            Paragraph::new(config_text)
                .alignment(Alignment::Left)
                .style(Style::default().bg(Color::DarkGray))
                .wrap(Wrap { trim: true }),
            chunks[1],
        );

        let instructions_text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "This will create a new backup for the specified instance.",
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "[Enter] ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("CREATE BACKUP  ", Style::default().fg(Color::White)),
                Span::styled(
                    "[Esc] ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::styled("CANCEL", Style::default().fg(Color::White)),
            ]),
        ];
        f.render_widget(
            Paragraph::new(instructions_text)
                .alignment(Alignment::Center)
                .style(Style::default().bg(Color::DarkGray))
                .wrap(Wrap { trim: true }),
            chunks[2],
        );
    }
}

fn render_help_popup(f: &mut Frame, _app: &App) {
    let popup_area = centered_rect(80, 70, f.area());
    f.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(Span::styled(
            "🔧 HELP - GCP SQL Backup Tool",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "General:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  --dry-run                 Simulate operations without executing"),
        Line::from(""),
        Line::from(Span::styled(
            "Navigation:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  ↑/↓       Navigate through lists"),
        Line::from("  Enter     Select item or confirm action"),
        Line::from("  Esc       Go back to previous step"),
        Line::from(""),
        Line::from(Span::styled(
            "Commands:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  M         Manual input for projects/instances"),
        Line::from("  R         Refresh current list or operation status"),
        Line::from("  N         Start a new operation"),
        Line::from("  H         Toggle this help screen"),
        Line::from("  Q         Quit application"),
        Line::from(""),
        Line::from(Span::styled(
            "Press H or Esc to close this help",
            Style::default().fg(Color::Yellow),
        )),
    ];

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .wrap(Wrap { trim: true })
        .style(Style::default().bg(Color::Black));

    f.render_widget(help, popup_area);
}

fn render_operation_selection(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .title("Choose an Operation")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(ACCENT_COLOR));

    let items = vec![
        ListItem::new("Restore a backup"),
        ListItem::new("Create a new backup"),
    ];

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(HIGHLIGHT_BG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("► ");

    let mut state = ListState::default();
    state.select(Some(app.selected_operation_index));

    f.render_stateful_widget(list, area, &mut state);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_manual_input_popup(f: &mut Frame, app: &App) {
    let area = f.area();
    let min_width = 50;
    let max_width = 80;
    let width = if area.width < min_width + 10 {
        area.width.saturating_sub(4)
    } else {
        (area.width * 60 / 100).min(max_width).max(min_width)
    };

    let height = 9;

    let popup_area = Rect {
        x: (area.width.saturating_sub(width)) / 2,
        y: (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };

    let title = match app.manual_input_type.as_str() {
        "source_project" => "Enter Source Project ID",
        "target_project" => "Enter Target Project ID",
        "backup_name" => "Enter a Name for the Backup",
        _ => "Enter Input",
    };

    f.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(popup_area);

    let input = Paragraph::new(app.manual_input_buffer.as_str())
        .style(Style::default().fg(INPUT_TEXT))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(title)
                .title_alignment(Alignment::Center)
                .style(Style::default().fg(ACCENT_COLOR).bg(BASE_BG)),
        );
    f.render_widget(input, chunks[0]);

    f.set_cursor_position((
        chunks[0].x + app.manual_input_buffer.len() as u16 + 1,
        chunks[0].y + 1,
    ));

    if !app.remembered_projects.is_empty() && app.manual_input_type.contains("project") {
        let content = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Recent projects:",
                Style::default().fg(BORDER_COLOR),
            )),
            Line::from(Span::styled(
                app.remembered_projects.join(", "),
                Style::default().fg(ACCENT_COLOR),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "[Enter] Confirm | [Esc] Cancel",
                Style::default().fg(WARNING_COLOR),
            )),
        ];

        let help = Paragraph::new(content)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(help, chunks[1]);
    } else {
        let help = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "[Enter] Confirm | [Esc] Cancel",
                Style::default().fg(WARNING_COLOR),
            )),
        ])
        .alignment(Alignment::Center);
        f.render_widget(help, chunks[1]);
    }
}
