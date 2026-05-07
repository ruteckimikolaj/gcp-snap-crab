use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame,
};

use crate::app::App;
use crate::types::{InputMode, OperationMode};
use super::{ACCENT_COLOR, BASE_FG, BORDER_COLOR, HIGHLIGHT_BG, INPUT_TEXT, SUCCESS_COLOR, WARNING_COLOR};

pub(super) fn render_step_box(
    f: &mut Frame,
    area: Rect,
    title: &str,
    value: Option<&str>,
    active_hint: Option<&str>,
) {
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

pub(super) fn render_instance_list(f: &mut Frame, area: Rect, app: &mut App, title: &str) {
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

pub(super) fn render_backup_list(f: &mut Frame, area: Rect, app: &mut App) {
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

pub(super) fn render_loading(f: &mut Frame, area: Rect, message: &str) {
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

pub(super) fn render_search_bar(f: &mut Frame, area: Rect, query: &str) {
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

pub(super) fn render_error(f: &mut Frame, area: Rect, error_msg: &str) {
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
