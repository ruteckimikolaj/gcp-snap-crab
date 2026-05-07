use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use super::{ACCENT_COLOR, BASE_BG, BORDER_COLOR, INPUT_TEXT, WARNING_COLOR};

pub(super) fn render_error_popup(f: &mut Frame, app: &mut App) {
    if let Some(error_msg) = &app.error {
        let popup_area = centered_rect(60, 25, f.area());
        f.render_widget(Clear, popup_area);

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

pub(super) fn render_restore_warning_popup(f: &mut Frame, app: &App) {
    if let Some(config) = &app.restore_flow.config {
        let popup_area = centered_rect(85, 60, f.area());
        f.render_widget(Clear, popup_area);

        let warning_block = Block::default()
            .title("⚠️  CRITICAL WARNING - BACKUP RESTORATION  ⚠️")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .style(
                Style::default().fg(Color::White).bg(Color::Rgb(139, 0, 0)),
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

pub(super) fn render_create_backup_warning_popup(f: &mut Frame, app: &App) {
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

pub(super) fn render_help_popup(f: &mut Frame, _app: &App) {
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

pub(super) fn render_manual_input_popup(f: &mut Frame, app: &App) {
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
