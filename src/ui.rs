use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap, BorderType
    },
    Frame, Terminal,
};
use std::time::{Duration, Instant};

use crate::app::App;
use crate::types::{AppState, InputMode};

// Clean color palette for better visibility and modern look
const BASE_FG: Color = Color::Rgb(216, 222, 233);          // Main text
const BASE_BG: Color = Color::Rgb(46, 52, 64);             // Background
const ACCENT_COLOR: Color = Color::Rgb(136, 192, 208);     // Primary accent
const SUCCESS_COLOR: Color = Color::Rgb(163, 190, 140);    // Success/green
const WARNING_COLOR: Color = Color::Rgb(235, 203, 139);    // Warning/yellow
const HIGHLIGHT_BG: Color = Color::Rgb(59, 66, 82);        // Selection background
const BORDER_COLOR: Color = Color::Rgb(76, 86, 106);       // Inactive borders
const INPUT_TEXT: Color = Color::Rgb(235, 203, 139);       // Input text - bright and visible

pub async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
    app.initialize().await?;
    let mut last_tick = Instant::now();
    let mut last_status_check = Instant::now();
    let tick_rate = Duration::from_millis(250);
    let status_check_interval = Duration::from_secs(5); // Check status every 5 seconds

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
                            if let Err(e) = handle_normal_input(&mut app, key.code, key.modifiers).await {
                                app.state = AppState::Error(e.to_string());
                            }
                        }
                        InputMode::Editing => {
                            handle_edit_input(&mut app, key.code).await?;
                        }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }

        // Periodic status check for ongoing restore operations
        if last_status_check.elapsed() >= status_check_interval {
            if app.restore_result.is_some() {
                // Check status periodically when restore is active
                let _ = app.check_restore_status().await; // Don't break on status check errors
            }
            last_status_check = Instant::now();
        }

        // Check if we should exit
        if matches!(app.state, AppState::Error(_)) && !app.show_help {
            break;
        }
    }

    Ok(())
}

async fn handle_normal_input(app: &mut App, key: KeyCode, modifiers: KeyModifiers) -> Result<()> {
    match key {
        KeyCode::Char('q') => {
            // Q always quits the application
            std::process::exit(0);
        }
        KeyCode::Esc => {
            // ESC goes back to previous step
            if app.show_help {
                app.toggle_help();
            } else if app.manual_input_active {
                app.cancel_manual_input();
            } else if matches!(app.state, AppState::ConfirmRestore) {
                // Cancel restore and go back to target instance selection
                // Clear the target instance selection so no instance appears selected
                app.target_instance = None;
                app.selected_instance_index = 0;
                app.state = AppState::SelectingTargetInstance;
            } else {
                // Navigate back through the workflow steps
                match app.state {
                    AppState::SelectingSourceInstance => {
                        // Go back to source project selection
                        app.source_project = None;
                        app.sql_instances.clear();
                        app.selected_instance_index = 0;
                        app.state = AppState::SelectingSourceProject;
                    }
                    AppState::SelectingBackup => {
                        // Go back to source instance selection
                        app.source_instance = None;
                        app.backups.clear();
                        app.selected_backup_index = 0;
                        app.state = AppState::SelectingSourceInstance;
                        // Reload instances for the current project
                        if let Some(project) = &app.source_project.clone() {
                            let _ = app.load_instances(project).await;
                        }
                    }
                    AppState::SelectingTargetProject => {
                        // Go back to backup selection
                        app.selected_backup = None;
                        app.state = AppState::SelectingBackup;
                        // Reload backups for the current source
                        if let (Some(project), Some(instance)) = (&app.source_project.clone(), &app.source_instance.clone()) {
                            let _ = app.load_backups(project, instance).await;
                        }
                    }
                    AppState::SelectingTargetInstance => {
                        // Go back to target project selection
                        app.target_project = None;
                        app.sql_instances.clear();
                        app.selected_instance_index = 0;
                        app.state = AppState::SelectingTargetProject;
                    }
                    AppState::PerformingRestore => {
                        // If currently monitoring a restore, go back to target instance selection
                        if app.restore_result.is_some() {
                            // Keep the restore running but allow navigation back
                            app.state = AppState::SelectingTargetInstance;
                        }
                    }
                    _ => {
                        // For other states, go to the welcome screen
                        app.state = AppState::Welcome;
                    }
                }
            }
        }
        KeyCode::Char('h') => app.toggle_help(),
        KeyCode::Char('p') => {
            if matches!(app.state, AppState::Welcome) {
                app.state = AppState::SelectingSourceProject;
            }
        }
        KeyCode::Up => app.move_selection_up(),
        KeyCode::Down => app.move_selection_down(),
        KeyCode::Enter => {
            match app.state {
                AppState::Welcome => {
                    app.state = AppState::SelectingSourceProject;
                }
                AppState::SelectingSourceProject => {
                    app.start_manual_input("source_project");
                }
                AppState::SelectingTargetProject => {
                    app.start_manual_input("target_project");
                }
                AppState::ConfirmRestore => {
                    app.perform_restore().await?;
                }
                _ => {
                    app.select_current_item().await?;
                }
            }
        }
        KeyCode::Char('m') => {
            // Manual input mode
            match app.state {
                AppState::SelectingSourceProject => {
                    app.start_manual_input("source_project");
                }
                AppState::SelectingTargetProject => {
                    app.start_manual_input("target_project");
                }
                AppState::SelectingSourceInstance | AppState::SelectingTargetInstance => {
                    app.start_manual_input("instance");
                }
                AppState::SelectingBackup => {
                    app.start_manual_input("backup");
                }
                _ => {}
            }
        }
        KeyCode::Char('r') => {
            // Refresh/reload
            match app.state {
                AppState::SelectingSourceProject | AppState::SelectingTargetProject => {
                    // Projects are entered manually, nothing to refresh
                }
                AppState::SelectingSourceInstance | AppState::SelectingTargetInstance => {
                    let project_clone = if let Some(project) = &app.source_project {
                        Some(project.clone())
                    } else {
                        app.target_project.clone()
                    };
                    if let Some(project) = project_clone {
                        app.load_instances(&project).await?;
                    }
                    // Also check restore status if there's an ongoing operation
                    if app.restore_result.is_some() {
                        app.check_restore_status().await?;
                    }
                }
                AppState::SelectingBackup => {
                    let (project_clone, instance_clone) = (app.source_project.clone(), app.source_instance.clone());
                    if let (Some(project), Some(instance)) = (project_clone, instance_clone) {
                        app.load_backups(&project, &instance).await?;
                    }
                }
                _ => {
                    // In any other state, if there's a restore operation, check its status
                    if app.restore_result.is_some() {
                        app.check_restore_status().await?;
                    }
                }
            }
        }
        KeyCode::Char('n') => {
            // Start new restore (reset current operation)
            if app.restore_result.is_some() {
                app.restore_result = None;
                app.restore_status = None;
                app.restore_config = None;
                app.selected_backup = None;
                app.target_instance = None;
                app.state = AppState::SelectingSourceProject;
            }
        }
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
            std::process::exit(0);
        }
        _ => {}
    }
    Ok(())
}

async fn handle_edit_input(app: &mut App, key: KeyCode) -> Result<()> {
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

fn ui(f: &mut Frame, app: &mut App) {
    // Main layout: Header | Content | Footer
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),    // Header
            Constraint::Min(0),       // Content
            Constraint::Length(3),    // Footer
        ])
        .split(f.area());

    render_header(f, main_chunks[0], app);
    render_content(f, main_chunks[1], app);
    render_footer(f, main_chunks[2], app);

    // Handle popups
    if app.show_help {
        render_help_popup(f, app);
    }
    if app.manual_input_active {
        render_manual_input_popup(f, app);
    }
    if matches!(app.state, AppState::ConfirmRestore) {
        render_restore_warning_popup(f, app);
    }
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let title = if app.dry_run_mode {
        " GCP SQL Backup Restore - DRY RUN MODE "
    } else {
        " GCP SQL Backup Restore "
    };
    
    let subtitle = match app.state {
        AppState::Welcome => "Welcome - Press 'p' to start with project selection",
        AppState::CheckingPrerequisites => "Checking Prerequisites...",
        AppState::SelectingSourceProject => "Step 1/5: Select Source Project",
        AppState::SelectingSourceInstance => "Step 2/5: Select Source Instance", 
        AppState::SelectingBackup => "Step 3/5: Select Backup",
        AppState::SelectingTargetProject => "Step 4/5: Select Target Project",
        AppState::SelectingTargetInstance => "Step 5/5: Select Target Instance",
        AppState::ConfirmRestore => "Step 6: Confirm Restoration",
        AppState::PerformingRestore => "Monitoring Restore Progress...",
        AppState::Error(_) => "Error Occurred",
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
        AppState::Welcome => render_welcome(f, area),
        AppState::CheckingPrerequisites => render_loading(f, area, "Checking prerequisites..."),
        AppState::SelectingSourceProject | 
        AppState::SelectingSourceInstance | 
        AppState::SelectingBackup |
        AppState::SelectingTargetProject | 
        AppState::SelectingTargetInstance |
        AppState::ConfirmRestore |
        AppState::PerformingRestore => {
            render_two_section_layout(f, area, app)
        }
        AppState::Error(msg) => render_error(f, area, msg),
    }
}

fn render_two_section_layout(f: &mut Frame, area: Rect, app: &mut App) {
    // Create 2-section horizontal layout like example app
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),  // Source section
            Constraint::Percentage(50),  // Target section
        ])
        .split(area);

    render_source_section(f, main_chunks[0], app);
    render_target_section(f, main_chunks[1], app);
}

fn render_source_section(f: &mut Frame, area: Rect, app: &mut App) {
    let source_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),   // Project
            Constraint::Length(8),   // Instance  
            Constraint::Min(0),      // Backup
        ])
        .split(area);

    // Source Project
    let project_style = if matches!(app.state, AppState::SelectingSourceProject) {
        Style::default().fg(ACCENT_COLOR)
    } else if app.source_project.is_some() {
        Style::default().fg(SUCCESS_COLOR)
    } else {
        Style::default().fg(BORDER_COLOR)
    };

    let project_content = if let Some(project) = &app.source_project {
        format!("✓ {}", project)
    } else if matches!(app.state, AppState::SelectingSourceProject) {
        "→ Press Enter to select...".to_string()
    } else {
        "Pending...".to_string()
    };

    f.render_widget(
        Paragraph::new(project_content)
            .block(
                Block::default()
                    .title("Source Project")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(project_style)
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        source_chunks[0],
    );

    // Source Instance
    if matches!(app.state, AppState::SelectingSourceInstance) && !app.sql_instances.is_empty() && app.source_instance.is_none() {
        render_instance_list(f, source_chunks[1], app, "Source Instance");
    } else {
        let instance_style = if matches!(app.state, AppState::SelectingSourceInstance) && app.source_instance.is_none() {
            Style::default().fg(ACCENT_COLOR)
        } else if app.source_instance.is_some() {
            Style::default().fg(SUCCESS_COLOR)
        } else {
            Style::default().fg(BORDER_COLOR)
        };

        let instance_content = if let Some(instance) = &app.source_instance {
            format!("✓ {}", instance)
        } else if matches!(app.state, AppState::SelectingSourceInstance) {
            if app.loading {
                "→ Loading instances...".to_string()
            } else if app.sql_instances.is_empty() {
                "→ No instances found".to_string()
            } else {
                "→ Select instance...".to_string()
            }
        } else {
            "Pending...".to_string()
        };

        f.render_widget(
            Paragraph::new(instance_content)
                .block(
                    Block::default()
                        .title("Source Instance")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .style(instance_style)
                )
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true }),
            source_chunks[1],
        );
    }

    // Source Backup
    if matches!(app.state, AppState::SelectingBackup) && !app.backups.is_empty() && app.selected_backup.is_none() {
        render_backup_list(f, source_chunks[2], app);
    } else {
        let backup_style = if matches!(app.state, AppState::SelectingBackup) && app.selected_backup.is_none() {
            Style::default().fg(ACCENT_COLOR)
        } else if app.selected_backup.is_some() {
            Style::default().fg(SUCCESS_COLOR)
        } else {
            Style::default().fg(BORDER_COLOR)
        };

        let backup_content = if let Some(backup) = &app.selected_backup {
            format!("✓ {}", backup)
        } else if matches!(app.state, AppState::SelectingBackup) {
            if app.loading {
                "→ Loading backups...".to_string()
            } else if app.backups.is_empty() {
                "→ No backups found".to_string()
            } else {
                format!("→ Choose from {} backups", app.backups.len())
            }
        } else {
            "Pending...".to_string()
        };

        f.render_widget(
            Paragraph::new(backup_content)
                .block(
                    Block::default()
                        .title("Source Backup")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .style(backup_style)
                )
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true }),
            source_chunks[2],
        );
    }
}

fn render_instance_list(f: &mut Frame, area: Rect, app: &mut App, title: &str) {
    let items: Vec<ListItem> = app
        .sql_instances
        .iter()
        .enumerate()
        .map(|(i, instance)| {
            let style = if i == app.selected_instance_index {
                Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(BASE_FG)
            };
            ListItem::new(format!("  {}", instance.name)).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(title)
                .style(Style::default().fg(ACCENT_COLOR))
        )
        .highlight_style(Style::default().bg(HIGHLIGHT_BG).add_modifier(Modifier::BOLD))
        .highlight_symbol("► ");

    let mut state = ListState::default();
    state.select(Some(app.selected_instance_index));

    f.render_stateful_widget(list, area, &mut state);
}

fn render_backup_list(f: &mut Frame, area: Rect, app: &mut App) {
    let items: Vec<ListItem> = app
        .backups
        .iter()
        .enumerate()
        .map(|(i, backup)| {
            let style = if i == app.selected_backup_index {
                Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(BASE_FG)
            };
            
            // Format the date (without time)
            let date_str = backup.start_time
                .map(|t| t.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            
            // Create display text with date and backup ID
            let display_text = format!("  {} | {}", date_str, backup.id);
            
            ListItem::new(display_text).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Source Backup")
                .style(Style::default().fg(ACCENT_COLOR))
        )
        .highlight_style(Style::default().bg(HIGHLIGHT_BG).add_modifier(Modifier::BOLD))
        .highlight_symbol("► ");

    let mut state = ListState::default();
    state.select(Some(app.selected_backup_index));

    f.render_stateful_widget(list, area, &mut state);
}

fn render_target_section(f: &mut Frame, area: Rect, app: &mut App) {
    let target_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),   // Project
            Constraint::Length(8),   // Instance
            Constraint::Min(0),      // Status/Info
        ])
        .split(area);

    // Target Project
    let project_style = if matches!(app.state, AppState::SelectingTargetProject) {
        Style::default().fg(ACCENT_COLOR)
    } else if app.target_project.is_some() {
        Style::default().fg(SUCCESS_COLOR)
    } else {
        Style::default().fg(BORDER_COLOR)
    };

    let project_content = if let Some(project) = &app.target_project {
        format!("✓ {}", project)
    } else if matches!(app.state, AppState::SelectingTargetProject) {
        "→ Press Enter to select...".to_string()
    } else {
        "Pending...".to_string()
    };

    f.render_widget(
        Paragraph::new(project_content)
            .block(
                Block::default()
                    .title("Target Project")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(project_style)
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        target_chunks[0],
    );

    // Target Instance
    if matches!(app.state, AppState::SelectingTargetInstance) && !app.sql_instances.is_empty() && app.target_instance.is_none() {
        render_instance_list(f, target_chunks[1], app, "Target Instance");
    } else {
        let instance_style = if matches!(app.state, AppState::SelectingTargetInstance) && app.target_instance.is_none() {
            Style::default().fg(ACCENT_COLOR)
        } else if app.target_instance.is_some() {
            Style::default().fg(SUCCESS_COLOR)
        } else {
            Style::default().fg(BORDER_COLOR)
        };

        let instance_content = if let Some(instance) = &app.target_instance {
            format!("✓ {}", instance)
        } else if matches!(app.state, AppState::SelectingTargetInstance) {
            if app.loading {
                "→ Loading instances...".to_string()
            } else if app.sql_instances.is_empty() {
                "→ No instances found".to_string()
            } else {
                "→ Select instance...".to_string()
            }
        } else {
            "Pending...".to_string()
        };

        f.render_widget(
            Paragraph::new(instance_content)
                .block(
                    Block::default()
                        .title("Target Instance")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .style(instance_style)
                )
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true }),
            target_chunks[1],
        );
    }

    // Status/Info section - Now shows restore progress with actual status
    let status_content = if let Some(_operation_id) = &app.restore_result {
        match app.restore_status.as_deref() {
            Some("DONE") => "✅ Restore completed successfully!\nBackup has been applied.",
            Some("RUNNING") => "🔄 Restore in progress...\nPlease wait, this may take several minutes.",
            Some("PENDING") => "⏳ Restore is pending...\nOperation is queued for execution.",
            Some("FAILED") | Some("ERROR") => "❌ Restore failed!\nCheck logs for details.",
            _ => "📊 Checking restore status...\nMonitoring progress...",
        }
    } else if app.target_instance.is_some() && app.selected_backup.is_some() {
        "✅ Ready to restore!\nPress Enter to confirm."
    } else {
        "Complete source\nselection first."
    };

    let status_style = if let Some(_) = &app.restore_result {
        match app.restore_status.as_deref() {
            Some("DONE") => Style::default().fg(SUCCESS_COLOR),
            Some("RUNNING") => Style::default().fg(WARNING_COLOR),
            Some("PENDING") => Style::default().fg(ACCENT_COLOR),
            Some("FAILED") | Some("ERROR") => Style::default().fg(Color::Red),
            _ => Style::default().fg(WARNING_COLOR),
        }
    } else if app.target_instance.is_some() && app.selected_backup.is_some() {
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
                    .style(status_style)
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        target_chunks[2],
    );
}

fn render_welcome(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" Welcome ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(BASE_FG).bg(BASE_BG));

    let welcome_text = vec![
        Line::from(""),
        Line::from(Span::styled("GCP SQL Backup Restore Tool", Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("This tool helps you restore SQL backups between GCP projects."),
        Line::from(""),
        Line::from("Steps:"),
        Line::from("  1. Select source project and instance"),
        Line::from("  2. Choose a backup to restore"),  
        Line::from("  3. Select target project and instance"),
        Line::from("  4. Confirm and execute restoration"),
        Line::from("  5. Monitor progress in real-time"),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  • Use ESC to go back to previous steps"),
        Line::from("  • Use Q to quit the application"),
        Line::from(""),
        Line::from(Span::styled("Press 'p' to start with project selection", Style::default().fg(WARNING_COLOR))),
        Line::from(Span::styled("Press 'h' for detailed help", Style::default().fg(BORDER_COLOR))),
    ];

    let paragraph = Paragraph::new(welcome_text)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_loading(f: &mut Frame, area: Rect, message: &str) {
    let loading_text = vec![
        Line::from(""),
        Line::from(Span::styled("⏳ Loading...", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
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

fn render_project_selection(f: &mut Frame, area: Rect, app: &App, title: &str) {
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(BASE_FG).bg(BASE_BG));

    let content = vec![
        Line::from(""),
        Line::from(Span::styled("Manual Project ID Entry", Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("Enter your GCP project ID manually"),
        Line::from("for security and flexibility."),
        Line::from(""),
        Line::from(Span::styled("Press [Enter] to open input", Style::default().fg(INPUT_TEXT))),
    ];

    if !app.remembered_projects.is_empty() {
        let recent_text = format!("Recent: {}", app.remembered_projects.join(", "));
        let content_with_recent = [
            content,
            vec![
                Line::from(""),
                Line::from(Span::styled(recent_text, Style::default().fg(BORDER_COLOR))),
            ]
        ].concat();
        
        let paragraph = Paragraph::new(content_with_recent)
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    } else {
        let paragraph = Paragraph::new(content)
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    }
}

fn render_error(f: &mut Frame, area: Rect, error_msg: &str) {
    let error_text = vec![
        Line::from(""),
        Line::from(Span::styled("❌ ERROR", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(error_msg),
        Line::from(""),
        Line::from(Span::styled("Press 'q' to exit", Style::default().fg(Color::Yellow))),
    ];

    let error = Paragraph::new(error_text)
        .block(Block::default().borders(Borders::ALL).title("Error"))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(error, area);
}

fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    let help_text = if app.manual_input_active {
        " [Enter] Confirm | [Esc] Cancel "
    } else {
        match app.state {
            AppState::Welcome => " [p] Start Project Selection | [h] Help | [q] Quit ",
            AppState::SelectingSourceProject => " [Enter] Manual Input | [Esc] Back to Welcome | [h] Help | [q] Quit ",
            AppState::SelectingTargetProject => {
                " [Enter] Manual Input | [Esc] Go Back | [h] Help | [q] Quit "
            }
            _ => {
                if app.restore_result.is_some() {
                    " [↑/↓] Navigate | [Enter] Select | [Esc] Go Back | [r] Refresh Status | [n] New Restore | [h] Help | [q] Quit "
                } else {
                    " [↑/↓] Navigate | [Enter] Select | [Esc] Go Back | [r] Refresh | [h] Help | [q] Quit "
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
    if let Some(config) = &app.restore_config {
        // Create a large, prominent popup
        let popup_area = centered_rect(85, 60, f.area());
        f.render_widget(Clear, popup_area);

        // Main warning block with red background
        let warning_block = Block::default()
            .title("⚠️  CRITICAL WARNING - BACKUP RESTORATION  ⚠️")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .style(Style::default()
                .fg(Color::White)
                .bg(Color::Rgb(139, 0, 0)) // Dark red background
            );

        f.render_widget(warning_block, popup_area);

        // Inner content area
        let inner_area = Rect {
            x: popup_area.x + 2,
            y: popup_area.y + 2,
            width: popup_area.width.saturating_sub(4),
            height: popup_area.height.saturating_sub(4),
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),   // Warning header
                Constraint::Length(8),   // Configuration details
                Constraint::Length(3),   // Danger notice
                Constraint::Min(0),      // Instructions
            ])
            .split(inner_area);

        // Warning header
        let header_text = vec![
            Line::from(Span::styled(
                "🚨 IRREVERSIBLE DATABASE RESTORATION 🚨",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED)
            )),
        ];
        f.render_widget(
            Paragraph::new(header_text)
                .alignment(Alignment::Center)
                .style(Style::default().bg(Color::Rgb(139, 0, 0))),
            chunks[0],
        );

        // Configuration details with better formatting
        let source_text = format!("{} → {}", config.source_project, config.source_instance);
        let target_text = format!("{} → {}", config.target_project, config.target_instance);
        
        let config_text = vec![
            Line::from(Span::styled("Restoration Configuration:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(vec![
                Span::styled("📂 Source: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(&source_text, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("💾 Backup: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(&config.backup_id, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("🎯 Target: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
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

        // Danger notice
        let danger_text = vec![
            Line::from(Span::styled(
                "⚠️  THIS WILL COMPLETELY REPLACE THE TARGET DATABASE  ⚠️",
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::SLOW_BLINK)
            )),
        ];
        f.render_widget(
            Paragraph::new(danger_text)
                .alignment(Alignment::Center)
                .style(Style::default().bg(Color::Rgb(139, 0, 0))),
            chunks[2],
        );

        // Instructions with contrasting colors
        let instructions_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("• All existing data in ", Style::default().fg(Color::White)),
                Span::styled(&config.target_instance, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(" will be PERMANENTLY LOST", Style::default().fg(Color::White)),
            ]),
            Line::from(Span::styled("• This operation cannot be undone or reversed", Style::default().fg(Color::White))),
            Line::from(Span::styled("• The restoration process may take several minutes", Style::default().fg(Color::White))),
            Line::from(""),
            Line::from(""),
            Line::from(vec![
                Span::styled("[Enter] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled("PROCEED WITH RESTORATION  ", Style::default().fg(Color::White)),
                Span::styled("[Esc] ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
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

fn render_help_popup(f: &mut Frame, _app: &App) {
    let popup_area = centered_rect(80, 70, f.area());
    f.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(Span::styled("🔧 HELP - GCP SQL Backup Restore Tool", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled("Navigation:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from("  ↑/↓       Navigate through lists"),
        Line::from("  Enter     Select current item or confirm action"),
        Line::from("  Esc       Go back to previous step"),
        Line::from(""),
        Line::from(Span::styled("Commands:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from("  M         Manual input mode (enter IDs manually)"),
        Line::from("  R         Refresh current list or restore status"),
        Line::from("  N         Start new restore (when restore is active)"),
        Line::from("  H         Toggle this help screen"),
        Line::from("  Q         Quit application"),
        Line::from("  Ctrl+C    Force quit"),
        Line::from(""),
        Line::from(Span::styled("Workflow:", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
        Line::from("  1. Select source project"),
        Line::from("  2. Select source instance"),
        Line::from("  3. Select backup to restore"),
        Line::from("  4. Select target project"),
        Line::from("  5. Select target instance"),
        Line::from("  6. Confirm and execute restore"),
        Line::from(""),
        Line::from(Span::styled("Status Check Mode:", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD))),
        Line::from("  Use --check-status flag to check operation status"),
        Line::from("  Requires project ID and operation ID"),
        Line::from(""),
        Line::from(Span::styled("Requirements:", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))),
        Line::from("  • Google Cloud SDK (gcloud) installed"),
        Line::from("  • Authenticated with gcloud auth login"),
        Line::from("  • Appropriate IAM permissions"),
        Line::from(""),
        Line::from(Span::styled("Press H or Esc to close this help", Style::default().fg(Color::Yellow))),
    ];

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .wrap(Wrap { trim: true })
        .style(Style::default().bg(Color::Black));

    f.render_widget(help, popup_area);
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
    // Calculate better popup size - minimum width, responsive height
    let area = f.area();
    let min_width = 50;
    let max_width = 80;
    let width = if area.width < min_width + 10 {
        area.width.saturating_sub(4)
    } else {
        (area.width * 60 / 100).min(max_width).max(min_width)
    };
    
    let height = 9; // Fixed reasonable height
    
    let popup_area = Rect {
        x: (area.width.saturating_sub(width)) / 2,
        y: (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };
    
    let title = match app.manual_input_type.as_str() {
        "source_project" => "Enter Source Project ID",
        "target_project" => "Enter Target Project ID", 
        "status_project" => "Enter Project ID for Status Check",
        _ => "Enter Input",
    };

    f.render_widget(Clear, popup_area);

    // Single border layout - no redundant borders
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Input field with title
            Constraint::Min(0),     // Help text/previous inputs
        ])
        .split(popup_area);

    // Input field with title - single border, no redundancy
    let input = Paragraph::new(app.manual_input_buffer.as_str())
        .style(Style::default().fg(INPUT_TEXT))  // Bright visible text
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(title)
                .title_alignment(Alignment::Center)
                .style(Style::default().fg(ACCENT_COLOR).bg(BASE_BG))
        );
    f.render_widget(input, chunks[0]);
    
    // Set cursor position
    f.set_cursor_position((
        chunks[0].x + app.manual_input_buffer.len() as u16 + 1,
        chunks[0].y + 1,
    ));

    // Show remembered projects and controls
    if !app.remembered_projects.is_empty() && app.manual_input_type.contains("project") {
        let content = vec![
            Line::from(""),
            Line::from(Span::styled("Recent projects:", Style::default().fg(BORDER_COLOR))),
            Line::from(Span::styled(app.remembered_projects.join(", "), Style::default().fg(ACCENT_COLOR))),
            Line::from(""),
            Line::from(Span::styled("[Enter] Confirm | [Esc] Cancel", Style::default().fg(WARNING_COLOR))),
        ];
        
        let help = Paragraph::new(content)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(help, chunks[1]);
    } else {
        let help = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("[Enter] Confirm | [Esc] Cancel", Style::default().fg(WARNING_COLOR))),
        ])
        .alignment(Alignment::Center);
        f.render_widget(help, chunks[1]);
    }
}


