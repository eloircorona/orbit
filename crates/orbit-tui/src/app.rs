use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use orbit_core::{
    engine::Engine,
    ipc::socket_path,
    session::Session,
    user_config::UserConfig,
};
use ratatui::{backend::CrosstermBackend, widgets::TableState, Terminal};
use std::{
    io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::{widget::TextInput, LaunchParams};

// ── tabs ──────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Sessions,
    Launch,
    System,
}

impl Tab {
    pub fn next(self) -> Self {
        match self {
            Tab::Sessions => Tab::Launch,
            Tab::Launch => Tab::System,
            Tab::System => Tab::Sessions,
        }
    }
}

// ── mode (popup overlays) ─────────────────────────────────────────────────────

pub enum Mode {
    Normal,
    Help,
    ConfirmKill(Session),
    SessionDetails(Session),
}

// ── engines ───────────────────────────────────────────────────────────────────

pub const ENGINES: [Engine; 3] = [Engine::Opencode, Engine::Gemini, Engine::Claude];

// ── launch state ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LaunchField {
    Engine,
    Tenant,
    Project,
    Repository,
    NoTmux,
    Launch,
}

impl LaunchField {
    pub fn next(self) -> Self {
        match self {
            LaunchField::Engine => LaunchField::Tenant,
            LaunchField::Tenant => LaunchField::Project,
            LaunchField::Project => LaunchField::Repository,
            LaunchField::Repository => LaunchField::NoTmux,
            LaunchField::NoTmux => LaunchField::Launch,
            LaunchField::Launch => LaunchField::Engine,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            LaunchField::Engine => LaunchField::Launch,
            LaunchField::Tenant => LaunchField::Engine,
            LaunchField::Project => LaunchField::Tenant,
            LaunchField::Repository => LaunchField::Project,
            LaunchField::NoTmux => LaunchField::Repository,
            LaunchField::Launch => LaunchField::NoTmux,
        }
    }

    pub fn is_text_input(self) -> bool {
        matches!(
            self,
            LaunchField::Tenant | LaunchField::Project | LaunchField::Repository
        )
    }
}

pub struct LaunchState {
    pub engine_idx: usize,
    pub tenant: TextInput,
    pub project: TextInput,
    pub repository: TextInput,
    pub no_tmux: bool,
    pub focused: LaunchField,
}

impl LaunchState {
    pub fn new(default_tenant: &str, default_engine: &str) -> Self {
        let engine_idx = ENGINES
            .iter()
            .position(|e| e.as_str() == default_engine)
            .unwrap_or(0);

        Self {
            engine_idx,
            tenant: TextInput::new("Tenant", "AIDEV").with_value(default_tenant),
            project: TextInput::new("Project", "my-project"),
            repository: TextInput::new("Repo", "(optional)"),
            no_tmux: false,
            focused: LaunchField::Engine,
        }
    }

    pub fn engine(&self) -> Engine {
        ENGINES[self.engine_idx]
    }

    pub fn to_params(&self) -> LaunchParams {
        LaunchParams {
            engine: self.engine(),
            tenant: self.tenant.as_str().to_string(),
            project: self.project.as_str().to_string(),
            repository: self.repository.as_str().to_string(),
            no_tmux: self.no_tmux,
        }
    }

    pub fn focused_input_mut(&mut self) -> Option<&mut TextInput> {
        match self.focused {
            LaunchField::Tenant => Some(&mut self.tenant),
            LaunchField::Project => Some(&mut self.project),
            LaunchField::Repository => Some(&mut self.repository),
            _ => None,
        }
    }
}

// ── system state ──────────────────────────────────────────────────────────────

pub struct SystemState {
    pub ai_root: PathBuf,
    pub default_engine: String,
    pub default_tenant: String,
    pub install_dir: PathBuf,
    pub dev_mode: bool,
    pub daemon_running: bool,
}

impl SystemState {
    pub fn load() -> Self {
        let cfg = UserConfig::load();
        let ai_root = cfg.ai_root_expanded();
        let install_dir = cfg.install_dir_expanded();
        let dev_mode = is_dev_mode(&install_dir);
        let daemon_running = socket_path().exists();

        Self {
            ai_root,
            default_engine: cfg.engine.default.clone(),
            default_tenant: cfg.engine.default_tenant.clone(),
            install_dir,
            dev_mode,
            daemon_running,
        }
    }

    pub fn refresh(&mut self) {
        self.dev_mode = is_dev_mode(&self.install_dir);
        self.daemon_running = socket_path().exists();
    }
}

fn is_dev_mode(install_dir: &Path) -> bool {
    let orbit = install_dir.join("orbit");
    let orbit_dev = install_dir.join("orbit-dev");

    if !orbit
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        return false;
    }

    orbit
        .read_link()
        .map(|target| {
            let abs = if target.is_absolute() {
                target
            } else {
                orbit.parent().unwrap_or(Path::new(".")).join(target)
            };
            abs == orbit_dev
        })
        .unwrap_or(false)
}

// ── async actions ─────────────────────────────────────────────────────────────

pub enum AsyncAction {
    DaemonStart,
    DaemonStop,
}

// ── post-exit actions ─────────────────────────────────────────────────────────

enum PostAction {
    Attach(Session),
    Launch(LaunchParams),
}

// ── app ───────────────────────────────────────────────────────────────────────

pub struct App {
    pub tab: Tab,
    pub mode: Mode,
    pub sessions: Vec<Session>,
    pub table_state: TableState,
    pub launch: LaunchState,
    pub sys: SystemState,
    pub status_msg: Option<String>,
    pub pending_async: Option<AsyncAction>,
    should_quit: bool,
    post_action: Option<PostAction>,
}

impl App {
    fn new() -> Self {
        let sys = SystemState::load();
        let launch = LaunchState::new(&sys.default_tenant, &sys.default_engine);

        let sessions = Session::load_all();
        let mut table_state = TableState::default();
        if !sessions.is_empty() {
            table_state.select(Some(0));
        }

        Self {
            tab: Tab::Sessions,
            mode: Mode::Normal,
            sessions,
            table_state,
            launch,
            sys,
            status_msg: None,
            pending_async: None,
            should_quit: false,
            post_action: None,
        }
    }

    pub fn refresh_sessions(&mut self) {
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
                    "Session {} was not launched in tmux.",
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
        self.refresh_sessions();
    }

    pub fn handle_key(&mut self, code: KeyCode, _mods: KeyModifiers) {
        if !matches!(self.mode, Mode::Normal) {
            self.handle_popup_key(code);
            return;
        }

        // ? always opens help (except when typing in Launch tab)
        if code == KeyCode::Char('?') && !self.launch.focused.is_text_input() {
            self.mode = Mode::Help;
            return;
        }

        // Tab switching: skip when typing in a text field
        let in_text_input = self.tab == Tab::Launch && self.launch.focused.is_text_input();
        if !in_text_input {
            match code {
                KeyCode::Tab | KeyCode::BackTab => {
                    self.tab = self.tab.next();
                    return;
                }
                KeyCode::Char('1') => {
                    self.tab = Tab::Sessions;
                    return;
                }
                KeyCode::Char('2') => {
                    self.tab = Tab::Launch;
                    return;
                }
                KeyCode::Char('3') => {
                    self.tab = Tab::System;
                    return;
                }
                _ => {}
            }
        }

        match self.tab {
            Tab::Sessions => self.handle_sessions_key(code),
            Tab::Launch => self.handle_launch_key(code),
            Tab::System => self.handle_system_key(code),
        }
    }

    fn handle_popup_key(&mut self, code: KeyCode) {
        match std::mem::replace(&mut self.mode, Mode::Normal) {
            Mode::Help => {
                // any key closes help; mode already set to Normal
            }
            Mode::ConfirmKill(session) => {
                if matches!(code, KeyCode::Char('y') | KeyCode::Char('Y')) {
                    let id = session.id.clone();
                    send_sigterm(session.pid);
                    let _ = session.delete();
                    self.status_msg = Some(format!("Killed session {id}"));
                    self.refresh_sessions();
                }
                // any other key: cancel (mode already Normal)
            }
            Mode::SessionDetails(session) => match code {
                KeyCode::Char('a') => {
                    if session.has_tmux() && session.is_running() {
                        self.post_action = Some(PostAction::Attach(session));
                        self.should_quit = true;
                    } else {
                        self.status_msg =
                            Some("Cannot attach: not in tmux or not running.".into());
                    }
                }
                KeyCode::Char('K') => {
                    let id = session.id.clone();
                    send_sigterm(session.pid);
                    let _ = session.delete();
                    self.status_msg = Some(format!("Killed session {id}"));
                    self.refresh_sessions();
                }
                _ => {
                    // Esc or any other key: close popup
                }
            },
            Mode::Normal => {}
        }
    }

    fn handle_sessions_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up | KeyCode::Char('k') => self.move_up(),
            KeyCode::Down | KeyCode::Char('j') => self.move_down(),
            KeyCode::Char('a') | KeyCode::Enter => self.attach_selected(),
            KeyCode::Char('K') => {
                if let Some(session) = self.selected_session().cloned() {
                    self.mode = Mode::ConfirmKill(session);
                }
            }
            KeyCode::Char('d') => {
                if let Some(session) = self.selected_session().cloned() {
                    self.mode = Mode::SessionDetails(session);
                }
            }
            KeyCode::Char('c') => self.clean_dead(),
            KeyCode::Char('r') => self.refresh_sessions(),
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_launch_key(&mut self, code: KeyCode) {
        // Route to focused text input first
        let consumed = {
            if let Some(input) = self.launch.focused_input_mut() {
                input.handle_key(code)
            } else {
                false
            }
        };
        if consumed {
            return;
        }

        match code {
            KeyCode::Up => self.launch.focused = self.launch.focused.prev(),
            KeyCode::Down => self.launch.focused = self.launch.focused.next(),
            KeyCode::Left => {
                let n = ENGINES.len();
                self.launch.engine_idx = (self.launch.engine_idx + n - 1) % n;
            }
            KeyCode::Right => {
                self.launch.engine_idx = (self.launch.engine_idx + 1) % ENGINES.len();
            }
            KeyCode::Char(' ') => {
                if self.launch.focused == LaunchField::NoTmux {
                    self.launch.no_tmux = !self.launch.no_tmux;
                }
            }
            KeyCode::Enter => {
                if self.launch.focused == LaunchField::Launch {
                    let params = self.launch.to_params();
                    self.post_action = Some(PostAction::Launch(params));
                    self.should_quit = true;
                } else {
                    self.launch.focused = self.launch.focused.next();
                }
            }
            KeyCode::Esc => self.tab = Tab::Sessions,
            _ => {}
        }
    }

    fn handle_system_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('s') => {
                self.pending_async = Some(if self.sys.daemon_running {
                    AsyncAction::DaemonStop
                } else {
                    AsyncAction::DaemonStart
                });
            }
            KeyCode::Char('r') => self.sys.refresh(),
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            _ => {}
        }
    }
}

// ── event loop ────────────────────────────────────────────────────────────────

async fn handle_async_action(action: AsyncAction, app: &mut App) {
    match action {
        AsyncAction::DaemonStart => {
            if let Ok(exe) = std::env::current_exe() {
                let _ = std::process::Command::new(&exe)
                    .args(["daemon", "serve"])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .stdin(std::process::Stdio::null())
                    .spawn();
            }
            tokio::time::sleep(Duration::from_millis(400)).await;
            app.sys.refresh();
            app.status_msg = Some("Daemon started.".into());
        }
        AsyncAction::DaemonStop => {
            match orbit_client::ipc::shutdown().await {
                Ok(()) => {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    app.sys.refresh();
                    app.status_msg = Some("Daemon stopped.".into());
                }
                Err(e) => {
                    app.status_msg = Some(format!("Stop failed: {e}"));
                    app.sys.refresh();
                }
            }
        }
    }
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<Option<PostAction>> {
    let mut last_refresh = Instant::now();

    loop {
        terminal.draw(|f| crate::views::render(f, app))?;

        if let Some(action) = app.pending_async.take() {
            handle_async_action(action, app).await;
        }

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.status_msg = None;
                    app.handle_key(key.code, key.modifiers);
                }
            }
        }

        if last_refresh.elapsed() > Duration::from_secs(2) {
            app.refresh_sessions();
            last_refresh = Instant::now();
        }

        if app.should_quit {
            return Ok(app.post_action.take());
        }
    }
}

// ── public entry point ────────────────────────────────────────────────────────

pub async fn run() -> Result<Option<LaunchParams>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app).await;

    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    match result? {
        Some(PostAction::Attach(session)) => {
            attach_to_session(&session)?;
            Ok(None)
        }
        Some(PostAction::Launch(params)) => Ok(Some(params)),
        None => Ok(None),
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

pub fn attach_to_session(session: &Session) -> Result<()> {
    let tmux_name = session
        .tmux_session
        .as_deref()
        .expect("attach called on session without tmux");

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

pub fn send_sigterm(pid: u32) {
    let _ = std::process::Command::new("kill")
        .arg(pid.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}
