//! Terminal core using Alacritty's Term

use alacritty_terminal::{
    Term,
    event::{Event as AlacTermEvent, EventListener, Notify, WindowSize},
    event_loop::{EventLoop, Msg, Notifier},
    grid::Dimensions,
    index::{Column, Line, Point as AlacPoint},
    sync::FairMutex,
    term::{Config, TermMode, RenderableCursor, cell::Cell},
    tty,
};
use anyhow::{Context as _, Result};
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use gpui::*;
use std::sync::Arc;
use std::path::PathBuf;
use std::collections::VecDeque;

/// Indexed cell (from Zed)
#[derive(Debug, Clone)]
pub struct IndexedCell {
    pub point: AlacPoint,
    pub cell: Cell,
}

impl std::ops::Deref for IndexedCell {
    type Target = Cell;

    fn deref(&self) -> &Self::Target {
        &self.cell
    }
}

/// Terminal content cache (from Zed)
#[derive(Clone)]
pub struct TerminalContent {
    pub cells: Vec<IndexedCell>,
    pub mode: TermMode,
    pub display_offset: usize,
    pub cursor: RenderableCursor,
    pub cursor_char: char,
    pub terminal_bounds: TerminalBounds,
}

impl Default for TerminalContent {
    fn default() -> Self {
        TerminalContent {
            cells: Default::default(),
            mode: Default::default(),
            display_offset: Default::default(),
            cursor: RenderableCursor {
                shape: alacritty_terminal::vte::ansi::CursorShape::Block,
                point: AlacPoint::new(Line(0), Column(0)),
            },
            cursor_char: Default::default(),
            terminal_bounds: Default::default(),
        }
    }
}

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
    pub last_content: TerminalContent,  // Cache like Zed does
    events: VecDeque<AlacTermEvent>,    // Event queue
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
            last_content: Default::default(),
            events: VecDeque::with_capacity(10),
        })
    }

    /// Set terminal size (from Zed)
    pub fn set_size(&mut self, new_bounds: TerminalBounds) {
        if self.last_content.terminal_bounds != new_bounds {
            self.bounds = new_bounds;
            self.last_content.terminal_bounds = new_bounds;
            let window_size = WindowSize::from(new_bounds);
            self.term.lock().resize(new_bounds);
            self.pty_tx.0.send(Msg::Resize(window_size)).ok();
        }
    }

    /// Sync terminal content (from Zed)
    pub fn sync(&mut self, window: &mut Window, cx: &mut Context<Terminal>) {
        // Process pending events
        let mut has_new_events = false;
        while let Ok(Some(event)) = self.events_rx.try_next() {
            has_new_events = true;
            self.events.push_back(event);
        }

        // Update last_content from terminal (EXACT Zed approach)
        let term = self.term.lock();
        let old_content = self.last_content.clone();
        self.last_content = Self::make_content(&term, &self.last_content);
        
        // Check if content actually changed to trigger repaint
        let content_changed = old_content.cells.len() != self.last_content.cells.len()
            || old_content.cursor.point != self.last_content.cursor.point
            || has_new_events;
            
        if content_changed {
            drop(term); // Release the lock before notifying
            cx.notify();
        }
    }

    /// Make terminal content from Alacritty term (from Zed - EXACT copy)
    fn make_content(
        term: &parking_lot::lock_api::MutexGuard<parking_lot::RawMutex, Term<ZedListener>>,
        last_content: &TerminalContent
    ) -> TerminalContent {
        let content = term.renderable_content();

        // Pre-allocate with estimated size to reduce reallocations
        let estimated_size = content.display_iter.size_hint().0;
        let mut cells = Vec::with_capacity(estimated_size);

        cells.extend(content.display_iter.map(|ic| IndexedCell {
            point: ic.point,
            cell: ic.cell.clone(),
        }));

        // Get cursor char
        let cursor_char = {
            let cursor_point = content.cursor.point;
            cells.iter()
                .find(|ic| ic.point.line == cursor_point.line && ic.point.column == cursor_point.column)
                .map(|ic| ic.c)
                .unwrap_or(' ')
        };

        TerminalContent {
            cells,
            mode: content.mode,
            display_offset: content.display_offset,
            cursor: content.cursor,
            cursor_char,
            terminal_bounds: last_content.terminal_bounds, // Keep existing bounds
        }
    }

    /// Send input to the terminal
    pub fn send_input(&mut self, input: &str) {
        let bytes = input.as_bytes().to_vec();
        self.pty_tx.0.send(Msg::Input(bytes.into())).ok();
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

    pub fn handle_input(&mut self, text: &str, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(session) = self.active_session_mut() {
            // Regular text input - send as bytes
            session.send_input(text);
            cx.notify();
        }
    }

    pub fn try_keystroke(&mut self, keystroke: &Keystroke, alt_is_meta: bool, cx: &mut Context<Self>) -> bool {
        if let Some(session) = self.active_session_mut() {
            // Convert keystroke to escape sequence (from Zed's to_esc_str)
            let esc = super::mappings::keys::to_esc_str(keystroke, &session.last_content.mode, alt_is_meta);
            if let Some(esc) = esc {
                match esc {
                    std::borrow::Cow::Borrowed(string) => session.send_input(string),
                    std::borrow::Cow::Owned(string) => session.send_input(&string),
                };
                cx.notify();
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn update_events(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(session) = self.active_session_mut() {
            // Update terminal size if needed
            // Sync terminal content
            session.sync(window, cx);
        }
    }

    pub fn scroll_up(&mut self, lines: usize, cx: &mut Context<Self>) {
        if let Some(session) = self.active_session_mut() {
            let mut term = session.term().lock();
            term.scroll_display(alacritty_terminal::grid::Scroll::Delta(lines as i32));
            cx.notify();
        }
    }

    pub fn scroll_down(&mut self, lines: usize, cx: &mut Context<Self>) {
        if let Some(session) = self.active_session_mut() {
            let mut term = session.term().lock();
            term.scroll_display(alacritty_terminal::grid::Scroll::Delta(-(lines as i32)));
            cx.notify();
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui_component::{v_flex, h_flex, StyledExt, ActiveTheme, button::{Button, ButtonVariants}, IconName, Sizable};
        use super::terminal_element::TerminalElement;

        // Update terminal content before rendering
        self.update_events(window, cx);
        
        let active_session = self.active_session().map(|s| s.name.clone()).unwrap_or_default();
        let is_focused = self.focus_handle.is_focused(window);

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                // Header
                h_flex()
                    .w_full()
                    .px_3()
                    .py_2()
                    .bg(cx.theme().secondary)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .justify_between()
                    .items_center()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child(active_session)
                            )
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("clear-terminal")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Trash)
                                    .tooltip("Clear Terminal")
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        if let Some(session) = this.active_session_mut() {
                                            session.clear();
                                        }
                                        cx.notify();
                                    }))
                            )
                            .child(
                                Button::new("new-terminal")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::Plus)
                                    .tooltip("New Terminal")
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        if let Err(e) = this.add_session(None, cx) {
                                            eprintln!("Failed to create new terminal: {}", e);
                                        }
                                    }))
                            )
                    )
            )
            .child(
                // Terminal sessions tabs
                h_flex()
                    .w_full()
                    .px_2()
                    .py_1()
                    .bg(cx.theme().secondary)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .gap_1()
                    .children(
                        self.sessions.iter().enumerate().map(|(index, session)| {
                            let is_active = index == self.active_session;
                            let has_multiple = self.sessions.len() > 1;

                            let mut tab = h_flex()
                                .px_3()
                                .py_1()
                                .rounded(px(4.0))
                                .gap_2()
                                .items_center()
                                .cursor_pointer();
                            
                            if is_active {
                                tab = tab.bg(cx.theme().primary)
                                    .text_color(cx.theme().primary_foreground);
                            } else {
                                tab = tab.hover(|this| this.bg(cx.theme().accent));
                            }
                            
                            tab = tab.on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _window, cx| {
                                this.switch_session(index, cx);
                            }))
                            .child(
                                div()
                                    .text_xs()
                                    .child(session.name.clone())
                            );
                            
                            if has_multiple {
                                tab = tab.child(
                                    div()
                                        .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _window, cx| {
                                            cx.stop_propagation();
                                            this.close_session(index, cx);
                                        }))
                                        .child(
                                            gpui_component::Icon::new(IconName::Close)
                                                .size_3()
                                        )
                                );
                            }
                            
                            tab
                        })
                    )
            )
            .child(
                // Terminal rendering
                div()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
                    .child(TerminalElement::new(
                        cx.entity().clone(),
                        self.focus_handle.clone(),
                        is_focused
                    ))
            )
    }
}
