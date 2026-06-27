use orbit_core::session::Session;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

// ── layout helper ─────────────────────────────────────────────────────────────

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let h_margin = area.width.saturating_sub(width) / 2;
    let v_margin = area.height.saturating_sub(height) / 2;
    Rect {
        x: area.x + h_margin,
        y: area.y + v_margin,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

fn render_popup(f: &mut Frame, area: Rect, title: &str, lines: Vec<Line>) {
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            format!(" {title} "),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(lines), inner);
}

// ── help popup ────────────────────────────────────────────────────────────────

pub fn render_help(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(52, 22, area);

    let k = |s: &'static str| Span::styled(s, Style::default().fg(Color::Cyan));
    let d = |s: &'static str| Span::raw(s);

    let lines = vec![
        Line::from(""),
        Line::from(vec![k("  Tab / [1-3]  "), d("  Switch tabs")]),
        Line::from(""),
        Line::from(Span::styled(
            "  Sessions tab:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![k("  ↑↓ / jk     "), d("  Navigate sessions")]),
        Line::from(vec![k("  a / Enter   "), d("  Attach to session")]),
        Line::from(vec![k("  K           "), d("  Kill session (confirm)")]),
        Line::from(vec![k("  d           "), d("  Session details")]),
        Line::from(vec![k("  c           "), d("  Clean dead sessions")]),
        Line::from(vec![k("  r           "), d("  Refresh")]),
        Line::from(vec![k("  q / Esc     "), d("  Quit")]),
        Line::from(""),
        Line::from(Span::styled(
            "  Launch tab:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![k("  ↑↓          "), d("  Move between fields")]),
        Line::from(vec![k("  ←→          "), d("  Cycle engine")]),
        Line::from(vec![k("  Space        "), d("  Toggle no-tmux")]),
        Line::from(vec![k("  Enter        "), d("  Confirm / Launch")]),
        Line::from(vec![k("  Esc          "), d("  Back to Sessions")]),
        Line::from(""),
        Line::from(Span::styled(
            "  Press any key to close",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
    ];

    render_popup(f, popup_area, "Keybindings", lines);
}

// ── confirm kill popup ────────────────────────────────────────────────────────

pub fn render_confirm_kill(f: &mut Frame, area: Rect, session: Session) {
    let popup_area = centered_rect(54, 9, area);

    let alive = if session.is_running() { "alive" } else { "dead" };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Kill session "),
            Span::styled(
                session.id.clone(),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("?"),
        ]),
        Line::from(vec![
            Span::styled(
                format!("  {} │ {}  ({})", session.engine, session.scope_label(), alive),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("       "),
            Span::styled(
                "[y] Confirm",
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("       "),
            Span::styled("[Esc/n] Cancel", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
    ];

    render_popup(f, popup_area, "Kill Session", lines);
}

// ── session details popup ─────────────────────────────────────────────────────

pub fn render_details(f: &mut Frame, area: Rect, session: Session) {
    let popup_area = centered_rect(62, 14, area);

    let alive = session.is_running();
    let status_span = if alive {
        Span::styled("● alive", Style::default().fg(Color::Green))
    } else {
        Span::styled("○ dead", Style::default().fg(Color::DarkGray))
    };

    let k = |s: &str| {
        Span::styled(
            format!("{:<12}", s),
            Style::default().fg(Color::DarkGray),
        )
    };

    let work_dir = session.work_dir.display().to_string();
    let tmux = session
        .tmux_session
        .as_deref()
        .unwrap_or("(none)")
        .to_string();

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            k("ID:"),
            Span::raw(session.id.clone()),
        ]),
        Line::from(vec![
            Span::raw("  "),
            k("PID:"),
            Span::raw(session.pid.to_string()),
            Span::styled("   Status: ", Style::default().fg(Color::DarkGray)),
            status_span,
        ]),
        Line::from(vec![
            Span::raw("  "),
            k("Engine:"),
            Span::raw(session.engine.clone()),
        ]),
        Line::from(vec![
            Span::raw("  "),
            k("Scope:"),
            Span::raw(session.scope_label()),
        ]),
        Line::from(vec![
            Span::raw("  "),
            k("Dir:"),
            Span::raw(work_dir),
        ]),
        Line::from(vec![
            Span::raw("  "),
            k("Tmux:"),
            Span::raw(tmux),
        ]),
        Line::from(vec![
            Span::raw("  "),
            k("Started:"),
            Span::raw(session.started_ago()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("[a]", Style::default().fg(Color::Cyan)),
            Span::raw(" Attach   "),
            Span::styled("[K]", Style::default().fg(Color::Red)),
            Span::raw(" Kill   "),
            Span::styled("[Esc]", Style::default().fg(Color::DarkGray)),
            Span::raw(" Close"),
        ]),
        Line::from(""),
    ];

    render_popup(f, popup_area, "Session Details", lines);
}
