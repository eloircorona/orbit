use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            " System ",
            Style::default().fg(Color::DarkGray),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let sections = Layout::vertical([
        Constraint::Length(6), // compact info
        Constraint::Min(0),    // MCP section
    ])
    .split(inner);

    render_info(f, app, sections[0]);
    render_mcp(f, app, sections[1]);
}

fn render_info(f: &mut Frame, app: &App, area: Rect) {
    let sys = &app.sys;

    let (dev_bullet, dev_label, dev_style) = if sys.dev_mode {
        (
            "●",
            " dev  (orbit → orbit-dev)",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        ("○", " stable", Style::default().fg(Color::Green))
    };

    let (d_bullet, d_label, d_style) = if sys.daemon_running {
        ("●", " running", Style::default().fg(Color::Green))
    } else {
        ("○", " stopped", Style::default().fg(Color::DarkGray))
    };

    let version = env!("CARGO_PKG_VERSION");

    let k = |s: &'static str| Span::styled(s, Style::default().fg(Color::DarkGray));

    let lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![
            k("  AI Root:  "),
            Span::styled(
                sys.ai_root.display().to_string(),
                Style::default().fg(Color::White),
            ),
            k("   Engine: "),
            Span::styled(
                sys.default_engine.clone(),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            k("  Install:  "),
            Span::styled(
                sys.install_dir.display().to_string(),
                Style::default().fg(Color::White),
            ),
            k("   Tenant: "),
            Span::styled(
                sys.default_tenant.clone(),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            k("  Dev:      "),
            Span::styled(dev_bullet, dev_style),
            Span::styled(dev_label, dev_style),
            k("         Daemon: "),
            Span::styled(d_bullet, d_style),
            Span::styled(d_label, d_style),
            k("  [s]"),
            k("   orbit "),
            Span::styled(format!("v{version}"), Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ──────────────────────────────────────────────────────────────────────",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    f.render_widget(Paragraph::new(lines), area);
}

fn render_mcp(f: &mut Frame, app: &App, area: Rect) {
    if area.height < 2 {
        return;
    }

    let sections = Layout::vertical([
        Constraint::Length(2), // header + column labels
        Constraint::Min(0),    // list
    ])
    .split(area);

    let header_lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled(
                "  MCP Servers",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("                                        "),
            Span::styled("[a]", Style::default().fg(Color::Cyan)),
            Span::raw(" add  "),
            Span::styled("[x]", Style::default().fg(Color::Cyan)),
            Span::raw(" remove"),
        ]),
        Line::from(Span::styled(
            "  SCOPE               NAME              COMMAND",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )),
    ];
    f.render_widget(Paragraph::new(header_lines), sections[0]);

    let list_area = sections[1];

    if app.sys.mcp_entries.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  No MCP servers configured. Press [a] to add one.",
                Style::default().fg(Color::DarkGray),
            ))),
            list_area,
        );
        return;
    }

    let items: Vec<ListItem> = app
        .sys
        .mcp_entries
        .iter()
        .map(|e| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("  {:<20}", truncate(&e.scope, 20)),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{:<18}", truncate(&e.name, 18)),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    truncate(&e.command_display, 36),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶");

    let mut list_state = ListState::default();
    list_state.select(Some(app.sys.mcp_selected));

    f.render_stateful_widget(list, list_area, &mut list_state);
}

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_string()
    } else {
        let t: String = chars[..max.saturating_sub(2)].iter().collect();
        format!("{t}..")
    }
}
