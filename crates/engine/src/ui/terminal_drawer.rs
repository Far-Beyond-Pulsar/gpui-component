//! Terminal Drawer - A full terminal emulator with PTY support
//! Supports PowerShell/cmd on Windows and bash/sh on Linux

use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex, ActiveTheme as _, IconName, StyledExt, Sizable as _,
};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::sync::{Arc, Mutex};
use std::thread;

const MAX_SCROLLBACK: usize = 10000;
const TERMINAL_COLS: u16 = 120;
const TERMINAL_ROWS: u16 = 30;

#[derive(Clone)]
pub struct TerminalLine {
    pub text: String,
}

pub struct TerminalSession {
    id: usize,
    name: String,
    lines: Arc<Mutex<VecDeque<TerminalLine>>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    _reader_thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
    scroll_offset: usize,
}

impl TerminalSession {
    pub fn new(id: usize, name: String, cx: &mut App) -> anyhow::Result<Self> {
        let lines = Arc::new(Mutex::new(VecDeque::new()));

        // Create PTY system
        let pty_system = NativePtySystem::default();

        // Determine shell to use
        let mut cmd = if cfg!(target_os = "windows") {
            // Try PowerShell first, fallback to cmd
            if let Ok(pwsh_path) = which::which("pwsh") {
                CommandBuilder::new(pwsh_path)
            } else if let Ok(ps_path) = which::which("powershell") {
                CommandBuilder::new(ps_path)
            } else {
                CommandBuilder::new("cmd")
            }
        } else {
            // Try bash first, fallback to sh
            if let Ok(bash_path) = which::which("bash") {
                CommandBuilder::new(bash_path)
            } else {
                CommandBuilder::new("sh")
            }
        };

        // Create PTY pair
        let pty_pair = pty_system.openpty(PtySize {
            rows: TERMINAL_ROWS,
            cols: TERMINAL_COLS,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        // Spawn the shell
        let mut child = pty_pair.slave.spawn_command(cmd)?;
        drop(pty_pair.slave);

        // Get reader and writer
        let mut reader = pty_pair.master.try_clone_reader()?;
        let writer = pty_pair.master.take_writer()?;

        // Spawn reader thread
        let lines_clone = lines.clone();
        let reader_thread = thread::spawn(move || {
            let mut buf_reader = BufReader::new(reader);
            let mut line_buf = String::new();

            loop {
                match buf_reader.read_line(&mut line_buf) {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        // Handle terminal output
                        let text = line_buf.trim_end_matches('\n').to_string();
                        if !text.is_empty() {
                            let mut lines = lines_clone.lock().unwrap();
                            lines.push_back(TerminalLine { text });

                            // Limit scrollback
                            while lines.len() > MAX_SCROLLBACK {
                                lines.pop_front();
                            }
                        }
                        line_buf.clear();
                    }
                    Err(_) => break,
                }
            }

            // Reap child process
            let _ = child.wait();
        });

        Ok(Self {
            id,
            name,
            lines,
            writer: Arc::new(Mutex::new(writer)),
            _reader_thread: Arc::new(Mutex::new(Some(reader_thread))),
            scroll_offset: 0,
        })
    }

    pub fn send_input(&self, input: &str) -> anyhow::Result<()> {
        let mut writer = self.writer.lock().unwrap();
        writer.write_all(input.as_bytes())?;
        writer.flush()?;
        Ok(())
    }

    pub fn get_lines(&self) -> Vec<TerminalLine> {
        self.lines.lock().unwrap().iter().cloned().collect()
    }

    pub fn scroll_up(&mut self, amount: usize) {
        let max_scroll = self.lines.lock().unwrap().len().saturating_sub(TERMINAL_ROWS as usize);
        self.scroll_offset = (self.scroll_offset + amount).min(max_scroll);
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn clear_scrollback(&mut self) {
        let mut lines = self.lines.lock().unwrap();
        lines.clear();
        self.scroll_offset = 0;
    }
}

pub struct TerminalDrawer {
    focus_handle: FocusHandle,
    sessions: Vec<TerminalSession>,
    active_session: usize,
    next_session_id: usize,
    input_buffer: String,
}

impl TerminalDrawer {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let mut drawer = Self {
            focus_handle,
            sessions: Vec::new(),
            active_session: 0,
            next_session_id: 0,
            input_buffer: String::new(),
        };

        // Create initial terminal session
        if let Err(e) = drawer.add_session(cx) {
            eprintln!("Failed to create initial terminal session: {}", e);
        }

        drawer
    }

    fn add_session(&mut self, cx: &mut Context<Self>) -> anyhow::Result<()> {
        let session_name = format!("Terminal {}", self.next_session_id + 1);
        let session = TerminalSession::new(self.next_session_id, session_name, cx)?;

        self.sessions.push(session);
        self.active_session = self.sessions.len() - 1;
        self.next_session_id += 1;
        cx.notify();

        Ok(())
    }

    fn close_session(&mut self, index: usize, cx: &mut Context<Self>) {
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

    fn switch_session(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.sessions.len() {
            self.active_session = index;
            cx.notify();
        }
    }

    fn send_input(&mut self, cx: &mut Context<Self>) {
        if let Some(session) = self.sessions.get(self.active_session) {
            let input = self.input_buffer.clone() + "\n";
            if let Err(e) = session.send_input(&input) {
                eprintln!("Failed to send input to terminal: {}", e);
            }
            self.input_buffer.clear();
            cx.notify();
        }
    }

    fn handle_keydown(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if let Some(session) = self.sessions.get_mut(self.active_session) {
            // Handle special keys
            match event.keystroke.key.as_str() {
                "enter" => {
                    self.send_input(cx);
                    return;
                }
                "backspace" => {
                    self.input_buffer.pop();
                    cx.notify();
                    return;
                }
                "up" => {
                    session.scroll_up(1);
                    cx.notify();
                    return;
                }
                "down" => {
                    session.scroll_down(1);
                    cx.notify();
                    return;
                }
                "pageup" => {
                    session.scroll_up(10);
                    cx.notify();
                    return;
                }
                "pagedown" => {
                    session.scroll_down(10);
                    cx.notify();
                    return;
                }
                _ => {}
            }

            // Handle character input
            if event.keystroke.key.len() == 1 {
                self.input_buffer.push_str(&event.keystroke.key);
                cx.notify();
            }
        }
    }
}

impl Focusable for TerminalDrawer {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TerminalDrawer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_session_name = self.sessions.get(self.active_session)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "Terminal".to_string());

        let active_session_lines = self.sessions.get(self.active_session)
            .map(|s| s.lines.lock().unwrap().len())
            .unwrap_or(0);

        let lines = self.sessions.get(self.active_session)
            .map(|s| s.get_lines())
            .unwrap_or_default();

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
                                    .child(active_session_name)
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("{} lines", active_session_lines))
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
                                        if let Some(session) = this.sessions.get_mut(this.active_session) {
                                            session.clear_scrollback();
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
                                        if let Err(e) = this.add_session(cx) {
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

                            h_flex()
                                .px_3()
                                .py_1()
                                .rounded(px(4.0))
                                .gap_2()
                                .items_center()
                                .cursor_pointer()
                                .when(is_active, |this| {
                                    this.bg(cx.theme().primary)
                                        .text_color(cx.theme().primary_foreground)
                                })
                                .when(!is_active, |this| {
                                    this.hover(|this| this.bg(cx.theme().accent))
                                })
                                .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _window, cx| {
                                    this.switch_session(index, cx);
                                }))
                                .child(
                                    div()
                                        .text_xs()
                                        .child(session.name.clone())
                                )
                                .when(self.sessions.len() > 1, |this| {
                                    this.child(
                                        div()
                                            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _window, cx| {
                                                cx.stop_propagation();
                                                this.close_session(index, cx);
                                            }))
                                            .child(
                                                gpui_component::Icon::new(IconName::Close)
                                                    .size_3()
                                            )
                                    )
                                })
                        })
                    )
            )
            .child(
                // Terminal output
                div()
                    .flex_1()
                    .w_full()
                    .p_3()
                    .bg(hsla(0.0, 0.0, 0.05, 1.0)) // Dark terminal background
                    .overflow_y_scroll()
                    .font_family("monospace")
                    .text_sm()
                    .child(
                        v_flex()
                            .gap_0p5()
                            .w_full()
                            .children(
                                lines.iter().map(|line| {
                                    div()
                                        .w_full()
                                        .text_color(hsla(0.0, 0.0, 0.9, 1.0))
                                        .child(line.text.clone())
                                })
                            )
                    )
            )
            .child(
                // Input line
                h_flex()
                    .w_full()
                    .px_3()
                    .py_2()
                    .bg(cx.theme().secondary)
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_family("monospace")
                            .text_color(cx.theme().primary)
                            .child("$")
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .font_family("monospace")
                            .text_color(cx.theme().foreground)
                            .child(self.input_buffer.clone())
                    )
            )
    }
}
