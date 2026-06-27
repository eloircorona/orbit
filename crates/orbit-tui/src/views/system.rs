use crate::app::App;
use ratatui::{
    Frame,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub fn render(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(" System ", Style::default().fg(Color::DarkGray)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let key = |s: &'static str| Span::styled(s, Style::default().fg(Color::DarkGray));
    let val = |s: String| Span::styled(s, Style::default().fg(Color::White));

    let sys = &app.sys;

    let ai_root = sys.ai_root.display().to_string();
    let install_dir = sys.install_dir.display().to_string();

    let (dev_bullet, dev_label, dev_style) = if sys.dev_mode {
        (
            "●",
            " dev mode  (orbit → orbit-dev)",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        ("○", " stable", Style::default().fg(Color::Green))
    };

    let (daemon_bullet, daemon_label, daemon_style) = if sys.daemon_running {
        ("●", " running", Style::default().fg(Color::Green))
    } else {
        (
            "○",
            " not running",
            Style::default().fg(Color::DarkGray),
        )
    };

    let version = env!("CARGO_PKG_VERSION");

    let lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![
            key("  AI Root:         "),
            val(ai_root),
        ]),
        Line::from(vec![
            key("  Install dir:     "),
            val(install_dir),
        ]),
        Line::from(vec![
            key("  Default engine:  "),
            val(sys.default_engine.clone()),
        ]),
        Line::from(vec![
            key("  Default tenant:  "),
            val(sys.default_tenant.clone()),
        ]),
        Line::from(""),
        Line::from(vec![
            key("  Dev mode:        "),
            Span::styled(dev_bullet, dev_style),
            Span::styled(dev_label, dev_style),
        ]),
        Line::from(""),
        Line::from(vec![
            key("  Daemon:          "),
            Span::styled(daemon_bullet, daemon_style),
            Span::styled(daemon_label, daemon_style),
        ]),
        Line::from(vec![
            key("                   "),
            Span::styled("[s]", Style::default().fg(Color::Cyan)),
            Span::styled(" to start/stop", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(vec![
            key("  orbit "),
            Span::styled(
                format!("v{version}"),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ];

    f.render_widget(Paragraph::new(lines), inner);
}
