use crate::app::App;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

pub fn render(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let active = app.sessions.iter().filter(|s| s.is_running()).count();
    let dead = app.sessions.iter().filter(|s| !s.is_running()).count();

    let title = match (active, dead) {
        (0, 0) => " Sessions — none ".to_string(),
        (a, 0) => format!(" Sessions — {a} active "),
        (0, d) => format!(" Sessions — {d} dead "),
        (a, d) => format!(" Sessions — {a} active, {d} dead "),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(title, Style::default().fg(Color::DarkGray)));

    if app.sessions.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No sessions running.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Press ", Style::default().fg(Color::DarkGray)),
                Span::styled("[2]", Style::default().fg(Color::Cyan)),
                Span::styled(" or ", Style::default().fg(Color::DarkGray)),
                Span::styled("[Tab]", Style::default().fg(Color::Cyan)),
                Span::styled(
                    " to open the Launch tab.",
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        ];
        f.render_widget(Paragraph::new(lines).block(block).alignment(Alignment::Left), area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("ENGINE")
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("SCOPE")
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("STATUS")
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("TMUX")
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("STARTED")
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ])
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .sessions
        .iter()
        .map(|s| {
            let alive = s.is_running();
            let status_cell = if alive {
                Cell::from("● alive").style(Style::default().fg(Color::Green))
            } else {
                Cell::from("○ dead").style(Style::default().fg(Color::DarkGray))
            };
            let tmux_cell = if s.has_tmux() {
                Cell::from("yes").style(Style::default().fg(Color::Cyan))
            } else {
                Cell::from("no").style(Style::default().fg(Color::DarkGray))
            };
            let row_style = if alive {
                Style::default()
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Row::new(vec![
                Cell::from(s.engine.clone()),
                Cell::from(s.scope_label()),
                status_cell,
                tmux_cell,
                Cell::from(s.started_ago()),
            ])
            .style(row_style)
        })
        .collect();

    let widths = [
        Constraint::Length(10),
        Constraint::Min(20),
        Constraint::Length(9),
        Constraint::Length(6),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(table, area, &mut app.table_state);
}
