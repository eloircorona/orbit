use crate::app::App;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(f, app, chunks[0]);
    render_sessions(f, app, chunks[1]);
    render_footer(f, app, chunks[2]);
}

fn render_header(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let active = app.sessions.iter().filter(|s| s.is_running()).count();
    let dead = app.sessions.iter().filter(|s| !s.is_running()).count();

    let status_text = match (active, dead) {
        (0, 0) => " No sessions".to_string(),
        (a, 0) => format!(" {a} active"),
        (0, d) => format!(" {d} dead"),
        (a, d) => format!(" {a} active  {d} dead"),
    };

    let line = Line::from(vec![
        Span::styled(
            " orbit ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("─", Style::default().fg(Color::DarkGray)),
        Span::styled(status_text, Style::default().fg(Color::DarkGray)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    f.render_widget(Paragraph::new(line).block(block).alignment(Alignment::Left), area);
}

fn render_sessions(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(" Sessions ", Style::default().fg(Color::DarkGray)));

    if app.sessions.is_empty() {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No sessions running.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Run ", Style::default().fg(Color::DarkGray)),
                Span::styled("orbit launch", Style::default().fg(Color::Cyan)),
                Span::styled(" to start a session.", Style::default().fg(Color::DarkGray)),
            ]),
        ];
        f.render_widget(Paragraph::new(text).block(block), area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("ENGINE").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("SCOPE").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("STATUS").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("TMUX").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("STARTED").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
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

fn render_footer(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let line = if let Some(msg) = &app.status_msg {
        Line::from(vec![
            Span::styled(" ! ", Style::default().fg(Color::Yellow)),
            Span::styled(msg.clone(), Style::default().fg(Color::Yellow)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" [↑↓/jk]", Style::default().fg(Color::Cyan)),
            Span::raw(" nav  "),
            Span::styled("[a/↵]", Style::default().fg(Color::Cyan)),
            Span::raw(" attach  "),
            Span::styled("[K]", Style::default().fg(Color::Cyan)),
            Span::raw(" kill  "),
            Span::styled("[c]", Style::default().fg(Color::Cyan)),
            Span::raw(" clean  "),
            Span::styled("[r]", Style::default().fg(Color::Cyan)),
            Span::raw(" refresh  "),
            Span::styled("[q/Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" quit"),
        ])
    };

    f.render_widget(Paragraph::new(line), area);
}
