//! Terminal core using Alacritty's Term

use alacritty_terminal::{
    Term,
    event::{Event as AlacTermEvent, EventListener, Notify, WindowSize},
    event_loop::{EventLoop, Msg, Notifier},
    grid::Dimensions,
    index::{Column, Line, Point as AlacPoint},
    sync::FairMutex,
    term::{Config, TermMode},
    tty,
};
use anyhow::{Context as _, Result};
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use gpui::*;
use std::sync::Arc;
use std::path::PathBuf;

/// Events that flow upward from the terminal
#[derive(Clone, Debug)]
pub enum Event {
    TitleChanged,
    CloseTerminal,
    Bell,
    Wakeup,
}

/// Terminal bounds for rendering
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerminalBounds {
    pub cell_width: Pixels,
    pub line_height: Pixels,
    pub bounds: Bounds<Pixels>,
}

impl TerminalBounds {
    pub fn new(line_height: Pixels, cell_width: Pixels, bounds: Bounds<Pixels>) -> Self {
        TerminalBounds {
            cell_width,
            line_height,
            bounds,
        }
    }

    pub fn num_lines(&self) -> usize {
        (self.bounds.size.height / self.line_height).floor() as usize
    }

    pub fn num_columns(&self) -> usize {
        (self.bounds.size.width / self.cell_width).floor() as usize
    }
}

impl Default for TerminalBounds {
    fn default() -> Self {
        TerminalBounds::new(
            px(20.0),
            px(8.0),
            Bounds {
                origin: Point::default(),
                size: Size {
                    width: px(800.0),
                    height: px(600.0),
                },
            },
        )
    }
}

impl From<TerminalBounds> for WindowSize {
    fn from(val: TerminalBounds) -> Self {
        WindowSize {
            num_lines: val.num_lines() as u16,
            num_cols: val.num_columns() as u16,
            cell_width: f32::from(val.cell_width) as u16,
            cell_height: f32::from(val.line_height) as u16,
        }
    }
}

impl Dimensions for TerminalBounds {
    fn total_lines(&self) -> usize {
        self.screen_lines()
    }

    fn screen_lines(&self) -> usize {
        self.num_lines()
    }

    fn columns(&self) -> usize {
        self.num_columns()
    }
}

/// Listener for Alacritty events
#[derive(Clone)]
pub struct ZedListener(pub UnboundedSender<AlacTermEvent>);

impl EventListener for ZedListener {
    fn send_event(&self, event: AlacTermEvent) {
        self.0.unbounded_send(event).ok();
    }
}

/// A single terminal session with PTY
pub struct TerminalSession {
    pub id: usize,
    pub name: String,
    term: Arc<FairMutex<Term<ZedListener>>>,
    pty_tx: Notifier,
    _io_thread: Option<std::thread::JoinHandle<()>>,
    events_rx: UnboundedReceiver<AlacTermEvent>,
    bounds: TerminalBounds,
    title: String,
}

impl TerminalSession {
    pub fn new(id: usize, name: String, working_directory: Option<PathBuf>, cx: &mut App) -> Result<Self> {
        // Create Alacritty configuration
        let config = Config {
            scrolling_history: 10000,
            ..Config::default()
        };

        // Set up event communication
        let (events_tx, events_rx) = unbounded();

        // Create the terminal with default bounds
        let term = Term::new(
            config.clone(),
            &TerminalBounds::default(),
            ZedListener(events_tx.clone()),
        );

        let term = Arc::new(FairMutex::new(term));

        // Determine shell to use
        let shell = if cfg!(target_os = "windows") {
            // Try PowerShell first, fallback to cmd
            if let Ok(pwsh_path) = which::which("pwsh") {
                tty::Shell::new(pwsh_path.to_string_lossy().to_string(), vec![])
            } else if let Ok(ps_path) = which::which("powershell") {
                tty::Shell::new(ps_path.to_string_lossy().to_string(), vec![])
            } else {
                tty::Shell::new("cmd".to_string(), vec![])
            }
        } else {
            // Try bash first, fallback to sh
            if let Ok(bash_path) = which::which("bash") {
                tty::Shell::new(bash_path.to_string_lossy().to_string(), vec![])
            } else {
                tty::Shell::new("sh".to_string(), vec![])
            }
        };

        // Set up PTY options
        let pty_options = tty::Options {
            shell: Some(shell),
            working_directory: working_directory.clone(),
            drain_on_exit: true,
            env: Default::default(),
        };

        // Create PTY
        let pty = tty::new(&pty_options, TerminalBounds::default().into(), 0)
            .context("failed to create PTY")?;

        // Connect terminal and PTY via event loop
        let event_loop = EventLoop::new(
            term.clone(),
            ZedListener(events_tx),
            pty,
            true, // drain_on_exit
            false, // hold
        )
        .context("failed to create event loop")?;

        let pty_tx = event_loop.channel();
        let _io_thread = event_loop.spawn();

        Ok(Self {
            id,
            name: name.clone(),
            term,
            pty_tx: Notifier(pty_tx),
            _io_thread: None, // We don't need to track this - Alacritty handles it
            events_rx,
            bounds: TerminalBounds::default(),
            title: name,
        })
    }

    /// Send input to the terminal
    pub fn send_input(&mut self, input: &str) {
        let bytes = input.as_bytes().to_vec();
        self.pty_tx.0.send(Msg::Input(bytes.into())).ok();
    }

    /// Resize the terminal
    pub fn resize(&mut self, bounds: TerminalBounds) {
        if self.bounds != bounds {
            self.bounds = bounds;
            let window_size = WindowSize::from(bounds);
            self.term.lock().resize(bounds);
            // Send resize notification to PTY
            self.pty_tx.0.send(Msg::Resize(window_size)).ok();
        }
    }

    /// Get the terminal for rendering
    pub fn term(&self) -> &Arc<FairMutex<Term<ZedListener>>> {
        &self.term
    }

    /// Process pending events
    pub fn process_events(&mut self, cx: &mut Context<Terminal>) -> Vec<Event> {
        let mut events = Vec::new();

        while let Ok(Some(event)) = self.events_rx.try_next() {
            match event {
                AlacTermEvent::Title(title) => {
                    self.title = title;
                    events.push(Event::TitleChanged);
                }
                AlacTermEvent::Bell => {
                    events.push(Event::Bell);
                }
                AlacTermEvent::Wakeup => {
                    events.push(Event::Wakeup);
                    cx.notify();
                }
                AlacTermEvent::Exit => {
                    events.push(Event::CloseTerminal);
                }
                _ => {}
            }
        }

        events
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn clear(&mut self) {
        // Clear the terminal by writing ANSI escape sequence
        self.send_input("\x1b[2J\x1b[H");
    }
}

/// Terminal with multiple sessions
pub struct Terminal {
    focus_handle: FocusHandle,
    sessions: Vec<TerminalSession>,
    active_session: usize,
    next_session_id: usize,
}

impl Terminal {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Result<Self> {
        let focus_handle = cx.focus_handle();
        let mut terminal = Self {
            focus_handle,
            sessions: Vec::new(),
            active_session: 0,
            next_session_id: 0,
        };

        // Create initial session
        terminal.add_session(None, cx)?;

        Ok(terminal)
    }

    pub fn add_session(&mut self, working_directory: Option<PathBuf>, cx: &mut Context<Self>) -> Result<()> {
        let session_name = format!("Terminal {}", self.next_session_id + 1);
        let session = TerminalSession::new(self.next_session_id, session_name, working_directory, cx)?;

        self.sessions.push(session);
        self.active_session = self.sessions.len() - 1;
        self.next_session_id += 1;
        cx.notify();

        Ok(())
    }

    pub fn close_session(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.sessions.len() <= 1 {
            return; // Keep at least one session
        }

        self.sessions.remove(index);

        if self.active_session >= self.sessions.len() {
            self.active_session = self.sessions.len() - 1;
        } else if index < self.active_session {
            self.active_session -= 1;
        }

        cx.notify();
    }

    pub fn switch_session(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.sessions.len() {
            self.active_session = index;
            cx.notify();
        }
    }

    pub fn active_session(&self) -> Option<&TerminalSession> {
        self.sessions.get(self.active_session)
    }

    pub fn active_session_mut(&mut self) -> Option<&mut TerminalSession> {
        self.sessions.get_mut(self.active_session)
    }

    pub fn sessions(&self) -> &[TerminalSession] {
        &self.sessions
    }

    fn handle_keydown(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(session) = self.active_session_mut() {
            // Handle special keys
            match event.keystroke.key.as_str() {
                "enter" => {
                    session.send_input("\n");
                }
                "backspace" => {
                    session.send_input("\x7f");
                }
                "tab" => {
                    session.send_input("\t");
                }
                "escape" => {
                    session.send_input("\x1b");
                }
                "up" => {
                    session.send_input("\x1b[A");
                }
                "down" => {
                    session.send_input("\x1b[B");
                }
                "right" => {
                    session.send_input("\x1b[C");
                }
                "left" => {
                    session.send_input("\x1b[D");
                }
                key if key.len() == 1 => {
                    // Handle Ctrl combinations
                    if event.keystroke.modifiers.control {
                        if let Some(ch) = key.chars().next() {
                            if ch.is_ascii_alphabetic() {
                                // Ctrl+A = 0x01, Ctrl+B = 0x02, etc.
                                let ctrl_code = (ch.to_ascii_lowercase() as u8 - b'a' + 1) as char;
                                session.send_input(&ctrl_code.to_string());
                                return;
                            }
                        }
                    }
                    session.send_input(key);
                }
                _ => {}
            }
            cx.notify();
        }
    }

    pub fn update_events(&mut self, cx: &mut Context<Self>) {
        if let Some(session) = self.active_session_mut() {
            session.process_events(cx);
        }
    }
}

impl Focusable for Terminal {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<Event> for Terminal {}

impl Render for Terminal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui_component::{v_flex, h_flex, StyledExt, ActiveTheme};

        let active_session = self.active_session().map(|s| s.name.clone()).unwrap_or_default();

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .on_key_down(cx.listener(Self::handle_keydown))
            .child(
                // Header
                h_flex()
                    .w_full()
                    .px_3()
                    .py_2()
                    .bg(cx.theme().secondary)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child(active_session)
                    )
            )
            .child(
                // Terminal rendering will go here via TerminalElement
                div()
                    .flex_1()
                    .w_full()
                    .bg(hsla(0.0, 0.0, 0.05, 1.0))
                    .child("Terminal rendering")
            )
    }
}
