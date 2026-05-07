use std::time::Instant;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{
        Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame,
};

use crate::app::App;
use crate::types::{AppState, InputMode};
use super::{ACCENT_COLOR, BASE_BG, BASE_FG, BORDER_COLOR, SUCCESS_COLOR, WARNING_COLOR};

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

fn spinner_frame(started_at: Option<Instant>) -> &'static str {
    let ms = started_at.map(|t| t.elapsed().as_millis()).unwrap_or(0);
    SPINNER[(ms / 100) as usize % SPINNER.len()]
}

fn format_elapsed(started_at: Option<Instant>) -> String {
    let Some(t) = started_at else { return String::new() };
    let secs = t.elapsed().as_secs();
    if secs < 60 {
        format!("({}s)", secs)
    } else {
        format!("({}m {}s)", secs / 60, secs % 60)
    }
}
use super::widgets::{
    render_backup_list, render_error, render_instance_list, render_loading, render_search_bar,
    render_step_box,
};

pub(super) fn render_header(f: &mut Frame, area: Rect, app: &App) {
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

pub(super) fn render_content(f: &mut Frame, area: Rect, app: &mut App) {
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

pub(super) fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    if app.input_mode == InputMode::Filtering {
        render_search_bar(f, area, &app.filter_query);
        return;
    }

    if let Some((ref text, ref since)) = app.yank_notification {
        if since.elapsed().as_secs() < 3 {
            let truncated = if text.len() > 40 { format!("{}…", &text[..40]) } else { text.clone() };
            f.render_widget(
                Paragraph::new(format!("✓ Copied: {}", truncated))
                    .block(
                        Block::default()
                            .title("Clipboard")
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .style(Style::default().fg(SUCCESS_COLOR)),
                    )
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(SUCCESS_COLOR)),
                area,
            );
            return;
        }
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
                    " [↑/↓] Navigate | [Enter] Select | [Esc] Back | [r] Refresh | [y] Copy ID | [n] New | [h] Help | [q] Quit "
                } else {
                    " [↑/↓] Navigate | [Enter] Select | [Esc] Back | [r] Refresh | [/] Search | [y] Copy | [h] Help | [q] Quit "
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
            if app.loading {
                "→ Loading instances..."
            } else if app.create_backup_flow.instances.is_empty() {
                "→ No instances found"
            } else {
                "→ Select instance..."
            }
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
        let elapsed = format_elapsed(app.create_backup_flow.operation_started_at);
        match app.create_backup_flow.status.as_deref() {
            Some("DONE") => "✅ Backup created successfully!".to_string(),
            Some("RUNNING") => format!(
                "{} Backup in progress... {}",
                spinner_frame(app.create_backup_flow.operation_started_at),
                elapsed
            ),
            Some("PENDING") => format!("⏳ Backup is pending... {}", elapsed),
            Some("FAILED") | Some("ERROR") => "❌ Backup failed!".to_string(),
            _ => format!("{} Checking backup status... {}", spinner_frame(app.create_backup_flow.operation_started_at), elapsed),
        }
    } else if app.create_backup_flow.config.is_some() {
        "✅ Ready to create backup!\nPress Enter to confirm.".to_string()
    } else {
        "Complete previous steps.".to_string()
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
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    render_source_section(f, main_chunks[0], app);
    render_target_section(f, main_chunks[1], app);
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

    let proj_hint = matches!(app.state, AppState::SelectingSourceProject)
        .then_some("→ Press Enter to select...");
    render_step_box(f, source_chunks[0], "Source Project", app.restore_flow.source_project.as_deref(), proj_hint);

    if matches!(app.state, AppState::SelectingSourceInstance)
        && !app.restore_flow.instances.is_empty()
        && app.restore_flow.source_instance.is_none()
    {
        render_instance_list(f, source_chunks[1], app, "Source Instance");
    } else {
        let inst_hint = matches!(app.state, AppState::SelectingSourceInstance).then(|| {
            if app.loading {
                "→ Loading instances..."
            } else if app.restore_flow.instances.is_empty() {
                "→ No instances found"
            } else {
                "→ Select instance..."
            }
        });
        render_step_box(f, source_chunks[1], "Source Instance", app.restore_flow.source_instance.as_deref(), inst_hint);
    }

    if matches!(app.state, AppState::SelectingBackup)
        && !app.restore_flow.backups.is_empty()
        && app.restore_flow.selected_backup.is_none()
    {
        render_backup_list(f, source_chunks[2], app);
    } else {
        let backup_hint = matches!(app.state, AppState::SelectingBackup).then(|| {
            if app.loading {
                "→ Loading backups...".to_string()
            } else if app.restore_flow.backups.is_empty() {
                "→ No backups found".to_string()
            } else {
                format!("→ Choose from {} backups", app.restore_flow.backups.len())
            }
        });
        render_step_box(f, source_chunks[2], "Source Backup", app.restore_flow.selected_backup.as_deref(), backup_hint.as_deref());
    }
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

    let proj_hint = matches!(app.state, AppState::SelectingTargetProject)
        .then_some("→ Press Enter to select...");
    render_step_box(f, target_chunks[0], "Target Project", app.restore_flow.target_project.as_deref(), proj_hint);

    if matches!(app.state, AppState::SelectingTargetInstance)
        && !app.restore_flow.instances.is_empty()
        && app.restore_flow.target_instance.is_none()
    {
        render_instance_list(f, target_chunks[1], app, "Target Instance");
    } else {
        let inst_hint = matches!(app.state, AppState::SelectingTargetInstance).then(|| {
            if app.loading {
                "→ Loading instances..."
            } else if app.restore_flow.instances.is_empty() {
                "→ No instances found"
            } else {
                "→ Select instance..."
            }
        });
        render_step_box(f, target_chunks[1], "Target Instance", app.restore_flow.target_instance.as_deref(), inst_hint);
    }

    let status_content = if let Some(_operation_id) = &app.restore_flow.operation_id {
        let elapsed = format_elapsed(app.restore_flow.operation_started_at);
        match app.restore_flow.status.as_deref() {
            Some("DONE") => "✅ Restore completed successfully!\nBackup has been applied.".to_string(),
            Some("RUNNING") => format!(
                "{} Restore in progress... {}\nPlease wait, this may take several minutes.",
                spinner_frame(app.restore_flow.operation_started_at),
                elapsed
            ),
            Some("PENDING") => format!("⏳ Restore is pending... {}\nOperation is queued for execution.", elapsed),
            Some("FAILED") | Some("ERROR") => "❌ Restore failed!\nCheck logs for details.".to_string(),
            _ => format!("{} Checking restore status... {}\nMonitoring progress...", spinner_frame(app.restore_flow.operation_started_at), elapsed),
        }
    } else if app.restore_flow.target_instance.is_some()
        && app.restore_flow.selected_backup.is_some()
    {
        "✅ Ready to restore!\nPress Enter to confirm.".to_string()
    } else {
        "Complete source\nselection first.".to_string()
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
                .bg(super::HIGHLIGHT_BG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("► ");

    let mut state = ListState::default();
    state.select(Some(app.selected_operation_index));

    f.render_stateful_widget(list, area, &mut state);
}
