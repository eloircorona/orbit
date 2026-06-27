use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use orbit_core::session::Session;
use ratatui::{backend::CrosstermBackend, widgets::TableState, Terminal};
use std::{
    io,
    time::{Duration, Instant},
};

// ── state ─────────────────────────────────────────────────────────────────────

pub struct App {
    pub sessions: Vec<Session>,
    pub table_state: TableState,
    pub status_msg: Option<String>,
    should_quit: bool,
    post_action: Option<PostAction>,
}

enum PostAction {
    Attach(Session),
}

impl App {
    fn new() -> Self {
        let sessions = Session::load_all();
        let mut table_state = TableState::default();
        if !sessions.is_empty() {
            table_state.select(Some(0));
        }
        Self {
            sessions,
            table_state,
            status_msg: None,
            should_quit: false,
            post_action: None,
        }
    }

    pub fn refresh(&mut self) {
        let selected = self.table_state.selected();
        self.sessions = Session::load_all();
        if self.sessions.is_empty() {
            self.table_state.select(None);
        } else {
            let i = selected.unwrap_or(0).min(self.sessions.len() - 1);
            self.table_state.select(Some(i));
        }
    }

    pub fn move_up(&mut self) {
        let n = self.sessions.len();
        if n == 0 {
            return;
        }
        let i = self.table_state.selected().unwrap_or(0);
        self.table_state.select(Some(if i == 0 { n - 1 } else { i - 1 }));
    }

    pub fn move_down(&mut self) {
        let n = self.sessions.len();
        if n == 0 {
            return;
        }
        let i = self.table_state.selected().unwrap_or(0);
        self.table_state.select(Some((i + 1) % n));
    }

    pub fn selected_session(&self) -> Option<&Session> {
        self.table_state.selected().and_then(|i| self.sessions.get(i))
    }

    pub fn attach_selected(&mut self) {
        match self.selected_session() {
            None => {
                self.status_msg = Some("No session selected.".to_string());
            }
            Some(s) if !s.has_tmux() => {
                self.status_msg = Some(format!(
                    "Session {} was not launched in tmux — cannot attach.",
                    s.id
                ));
            }
            Some(s) if !s.is_running() => {
                self.status_msg = Some(format!("Session {} is no longer running.", s.id));
            }
            Some(s) => {
                self.post_action = Some(PostAction::Attach(s.clone()));
                self.should_quit = true;
            }
        }
    }

    pub fn kill_selected(&mut self) {
        let Some(idx) = self.table_state.selected() else {
            return;
        };
        let Some(session) = self.sessions.get(idx).cloned() else {
            return;
        };

        let id = session.id.clone();
        send_sigterm(session.pid);
        let _ = session.delete();

        self.status_msg = Some(format!("Killed session {id}"));
        self.refresh();
    }

    pub fn clean_dead(&mut self) {
        let sessions = std::mem::take(&mut self.sessions);
        let mut cleaned = 0usize;
        for s in &sessions {
            if !s.is_running() {
                let _ = s.delete();
                cleaned += 1;
            }
        }
        self.status_msg = Some(if cleaned > 0 {
            format!("Cleaned {cleaned} dead session(s).")
        } else {
            "No dead sessions to clean.".to_string()
        });
        self.refresh();
    }
}

// ── event loop ────────────────────────────────────────────────────────────────

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<Option<PostAction>> {
    let mut last_refresh = Instant::now();

    loop {
        terminal.draw(|f| crate::views::render(f, app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.status_msg = None;
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
                        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
                        KeyCode::Char('a') | KeyCode::Enter => app.attach_selected(),
                        KeyCode::Char('K') => app.kill_selected(),
                        KeyCode::Char('c') => app.clean_dead(),
                        KeyCode::Char('r') => app.refresh(),
                        _ => {}
                    }
                }
            }
        }

        if last_refresh.elapsed() > Duration::from_secs(2) {
            app.refresh();
            last_refresh = Instant::now();
        }

        if app.should_quit {
            return Ok(app.post_action.take());
        }
    }
}

// ── public entry point ────────────────────────────────────────────────────────

pub async fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app);

    // Always restore terminal before handling result
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    match result? {
        Some(PostAction::Attach(session)) => attach_to_session(&session)?,
        None => {}
    }

    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn attach_to_session(session: &Session) -> Result<()> {
    let tmux_name = session
        .tmux_session
        .as_deref()
        .expect("attach_to_session called on session without tmux");

    let cmd = if std::env::var("TMUX").is_ok() {
        "switch-client"
    } else {
        "attach-session"
    };

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = std::process::Command::new("tmux")
            .args([cmd, "-t", tmux_name])
            .exec();
        anyhow::bail!("Failed to exec tmux: {err}");
    }

    #[cfg(not(unix))]
    anyhow::bail!("tmux attach is only supported on Unix");
}

fn send_sigterm(pid: u32) {
    let _ = std::process::Command::new("kill")
        .arg(pid.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}
