use std::collections::VecDeque;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    input::{InputState, TextInput, InputEvent},
    v_flex, h_flex,
    ActiveTheme as _, StyledExt, Sizable as _,
    IconName,
};

#[derive(Clone)]
pub struct TerminalLine {
    pub content: String,
    pub line_type: TerminalLineType,
}

#[derive(Clone)]
pub enum TerminalLineType {
    Command,
    Output,
    Error,
    Success,
}

#[derive(Clone)]
pub struct TerminalTab {
    pub id: usize,
    pub name: String,
    pub command_input: Entity<InputState>,
    pub history: VecDeque<TerminalLine>,
    pub command_history: Vec<String>,
    pub history_index: Option<usize>,
    pub max_lines: usize,
}

pub struct Terminal {
    focus_handle: FocusHandle,
    tabs: Vec<TerminalTab>,
    active_tab_index: usize,
    next_tab_id: usize,
    is_visible: bool,
}

impl TerminalTab {
    pub fn new(id: usize, name: String, window: &mut Window, cx: &mut Context<Terminal>) -> Self {
        let command_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Type a command...")
        });

        let mut tab = Self {
            id,
            name,
            command_input,
            history: VecDeque::new(),
            command_history: Vec::new(),
            history_index: None,
            max_lines: 1000,
        };

        // Add welcome message for new tabs
        tab.add_line("Welcome to Script Editor Terminal".to_string(), TerminalLineType::Output);
        tab.add_line("Type 'help' for available commands".to_string(), TerminalLineType::Output);

        tab
    }

    fn add_line(&mut self, content: String, line_type: TerminalLineType) {
        self.history.push_back(TerminalLine { content, line_type });

        // Keep history within limits
        if self.history.len() > self.max_lines {
            self.history.pop_front();
        }
    }

    fn navigate_history_up(&mut self, window: &mut Window, cx: &mut Context<Terminal>) {
        if self.command_history.is_empty() {
            return;
        }

        let new_index = match self.history_index {
            None => self.command_history.len() - 1,
            Some(index) => {
                if index > 0 {
                    index - 1
                } else {
                    return;
                }
            }
        };

        self.history_index = Some(new_index);
        let command = self.command_history[new_index].clone();
        // Set the command history text in the input
        self.command_input.update(cx, |input, cx| {
            input.set_value(&command, window, cx);
        });
    }

    fn navigate_history_down(&mut self, window: &mut Window, cx: &mut Context<Terminal>) {
        if self.command_history.is_empty() {
            return;
        }

        let new_index = match self.history_index {
            None => return,
            Some(index) => {
                if index < self.command_history.len() - 1 {
                    Some(index + 1)
                } else {
                    None
                }
            }
        };

        self.history_index = new_index;

        let command = match new_index {
            Some(index) => self.command_history[index].clone(),
            None => String::new(),
        };

        // Set the command history text in the input
        self.command_input.update(cx, |input, cx| {
            input.set_value(&command, window, cx);
        });
    }
}

impl Terminal {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let first_tab = TerminalTab::new(0, "Terminal 1".to_string(), window, cx);

        let mut terminal = Self {
            focus_handle: cx.focus_handle(),
            tabs: vec![first_tab],
            active_tab_index: 0,
            next_tab_id: 1,
            is_visible: true,
        };

        // Subscribe to input events for the first tab
        terminal.subscribe_to_tab_events(0, window, cx);

        terminal
    }

    fn subscribe_to_tab_events(&mut self, tab_index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.tabs.get(tab_index) {
            let command_input = tab.command_input.clone();
            let tab_index_copy = tab_index;
            cx.subscribe(&command_input, move |this, _input, event: &InputEvent, cx| {
                match event {
                    InputEvent::PressEnter { .. } => {
                        if let Some(active_tab) = this.tabs.get_mut(this.active_tab_index) {
                            let command = active_tab.command_input.read(cx).text().to_string();
                            this.execute_command(command, cx);
                            // Note: Clearing input would need window context, which isn't available here
                            // The input will be cleared on the next focus or user can clear manually
                        }
                    },
                    _ => {}
                }
            }).detach();
        }
    }

    /// Handle key down for command history navigation
    fn handle_key_down(&mut self, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if let Some(active_tab) = self.tabs.get_mut(self.active_tab_index) {
            if event.keystroke.key == "up" && event.keystroke.modifiers == gpui::Modifiers::default() {
                active_tab.navigate_history_up(window, cx);
                cx.notify();
                return true;
            } else if event.keystroke.key == "down" && event.keystroke.modifiers == gpui::Modifiers::default() {
                active_tab.navigate_history_down(window, cx);
                cx.notify();
                return true;
            }
        }
        false
    }

    pub fn toggle_visibility(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.is_visible = !self.is_visible;
        cx.notify();
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    fn add_new_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tab_name = format!("Terminal {}", self.next_tab_id + 1);
        let new_tab = TerminalTab::new(self.next_tab_id, tab_name, window, cx);

        self.tabs.push(new_tab);
        self.active_tab_index = self.tabs.len() - 1;

        // Subscribe to events for the new tab
        self.subscribe_to_tab_events(self.active_tab_index, window, cx);

        self.next_tab_id += 1;
        cx.notify();
    }

    fn close_tab(&mut self, tab_index: usize, cx: &mut Context<Self>) {
        if self.tabs.len() <= 1 {
            return; // Don't close the last tab
        }

        self.tabs.remove(tab_index);

        // Adjust active tab index if necessary
        if self.active_tab_index >= self.tabs.len() {
            self.active_tab_index = self.tabs.len() - 1;
        } else if tab_index < self.active_tab_index {
            self.active_tab_index -= 1;
        }

        cx.notify();
    }

    fn switch_to_tab(&mut self, tab_index: usize, cx: &mut Context<Self>) {
        if tab_index < self.tabs.len() {
            self.active_tab_index = tab_index;
            cx.notify();
        }
    }

    pub fn execute_command(&mut self, command: String, cx: &mut Context<Self>) {
        if let Some(active_tab) = self.tabs.get_mut(self.active_tab_index) {
            // Add command to history
            active_tab.add_line(format!("$ {}", command), TerminalLineType::Command);

            if !command.trim().is_empty() {
                active_tab.command_history.push(command.clone());
                active_tab.history_index = None;
            }

            // Run everything entered in the shell asynchronously.
            let cmd_trim = command.trim();
            if cmd_trim.is_empty() {
                // do nothing for empty input
            } else if cmd_trim == "clear" {
                active_tab.history.clear();
                active_tab.add_line("Terminal cleared".to_string(), TerminalLineType::Success);
            } else {
                active_tab.add_line(format!("$ {}", cmd_trim), TerminalLineType::Command);

                let (tx, rx) = flume::unbounded::<(TerminalLineType, String)>();
                let to_run = cmd_trim.to_string();

                // Spawn blocking execution on a thread
                std::thread::spawn(move || {
                    use std::process::Command;

                    let output = if cfg!(target_os = "windows") {
                        Command::new("cmd").arg("/C").arg(&to_run).output()
                    } else {
                        Command::new("sh").arg("-c").arg(&to_run).output()
                    };

                    match output {
                        Ok(out) => {
                            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                            for line in stdout.lines() {
                                let _ = tx.send((TerminalLineType::Output, line.to_string()));
                            }
                            for line in stderr.lines() {
                                let _ = tx.send((TerminalLineType::Error, line.to_string()));
                            }
                        }
                        Err(e) => {
                            let _ = tx.send((TerminalLineType::Error, format!("failed to execute command: {}", e)));
                        }
                    }
                });

                // UI task to collect output and append to terminal
                cx.spawn(async move |this, cx| {
                    while let Ok((line_type, text)) = rx.recv_async().await {
                        this.update(cx, |this, cx| {
                            if let Some(tab) = this.tabs.get_mut(this.active_tab_index) {
                                tab.add_line(text.clone(), line_type.clone());
                            }
                        })
                        .ok();
                    }
                })
                .detach();
            }
        }

        cx.notify();
    }


    fn get_line_color(&self, line_type: &TerminalLineType, cx: &Context<Self>) -> Hsla {
        match line_type {
            TerminalLineType::Command => cx.theme().primary,
            TerminalLineType::Output => cx.theme().foreground,
            TerminalLineType::Error => cx.theme().danger,
            TerminalLineType::Success => cx.theme().success,
        }
    }

    fn render_terminal_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let active_tab_name = self.tabs.get(self.active_tab_index)
            .map(|tab| tab.name.clone())
            .unwrap_or_else(|| "Terminal".to_string());

        let active_tab_lines = self.tabs.get(self.active_tab_index)
            .map(|tab| tab.history.len())
            .unwrap_or(0);

        h_flex()
            .w_full()
            .p_2()
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
                            .child(active_tab_name)
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{} lines", active_tab_lines))
                    )
            )
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        Button::new("clear_terminal")
                            .icon(IconName::Trash)
                            .tooltip("Clear Terminal")
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(|this, _, _window, cx| {
                                if let Some(active_tab) = this.tabs.get_mut(this.active_tab_index) {
                                    active_tab.history.clear();
                                    active_tab.add_line("Terminal cleared".to_string(), TerminalLineType::Success);
                                }
                                cx.notify();
                            }))
                    )
                    .child(
                        Button::new("new_terminal")
                            .icon(IconName::Plus)
                            .tooltip("New Terminal")
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.add_new_tab(window, cx);
                            }))
                    )
                    .child(
                        Button::new("close_terminal")
                            .icon(IconName::WindowClose)
                            .tooltip("Close Terminal")
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.toggle_visibility(window, cx);
                            }))
                    )
            )
    }

    fn render_terminal_output(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let default_history = VecDeque::new();
        let history = self.tabs.get(self.active_tab_index)
            .map(|tab| &tab.history)
            .unwrap_or(&default_history);

        div()
            .w_full()
            .h_full()
            .overflow_hidden()
            .child(
                div()
                    .w_full()
                    .h_full()
                    .p_3()
                    .bg(cx.theme().background)
                    .font_family("monospace")
                    .text_sm()
                    .scrollable(Axis::Vertical)
                    .child(
                        v_flex()
                            .gap_1()
                            .w_full()
                            .children(
                                history.iter().map(|line| {
                                    div()
                                        .w_full()
                                        .text_color(self.get_line_color(&line.line_type, cx))
                                        .child(line.content.clone())
                                })
                            )
                    )
            )
    }

    fn render_command_input(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let command_input = self.tabs.get(self.active_tab_index)
            .map(|tab| tab.command_input.clone());

        h_flex()
            .w_full()
            .p_2()
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
            .when_some(command_input, |this, input| {
                this.child(
                    div()
                        .flex_1()
                        .child(
                            TextInput::new(&input)
                        )
                )
            })
    }

    fn render_vertical_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(px(120.0))
            .h_full()
            .bg(cx.theme().secondary)
            .border_l_1()
            .border_color(cx.theme().border)
            .p_1()
            .gap_1()
            .children(
                self.tabs.iter().enumerate().map(|(index, tab)| {
                    let is_active = index == self.active_tab_index;

                    div()
                        .w_full()
                        .h(px(32.0))
                        .px_2()
                        .py_1()
                        .rounded(px(4.0))
                        .cursor_pointer()
                        .when(is_active, |this| {
                            this.bg(cx.theme().primary)
                                .text_color(cx.theme().primary_foreground)
                        })
                        .when(!is_active, |this| {
                            this.bg(cx.theme().background)
                                .text_color(cx.theme().foreground)
                                .hover(|this| this.bg(cx.theme().accent))
                        })
                        .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                            this.switch_to_tab(index, cx);
                        }))
                        .child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_medium()
                                        .truncate()
                                        .child(tab.name.clone())
                                )
                                .when(self.tabs.len() > 1, |this| {
                                    this.child(
                                        Button::new(("close_tab", index))
                                            .icon(IconName::Close)
                                            .ghost()
                                            .xsmall()
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.close_tab(index, cx);
                                            }))
                                    )
                                })
                        )
                })
            )
            .child(
                // Add new tab button at the bottom
                div()
                    .w_full()
                    .h(px(32.0))
                    .px_2()
                    .py_1()
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .bg(cx.theme().background)
                    .text_color(cx.theme().muted_foreground)
                    .hover(|this| this.bg(cx.theme().accent))
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                        this.add_new_tab(window, cx);
                    }))
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_xs()
                                    .child("+ New")
                            )
                    )
            )
    }
}

impl Focusable for Terminal {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Terminal {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.is_visible {
            return div().into_any_element();
        }

        h_flex()
            .size_full()
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .child(
                // Main terminal area
                v_flex()
                    .flex_1()
                    .h_full()
                    .overflow_hidden()
                    .child(
                        // Fixed header
                        div()
                            .flex_none()
                            .child(self.render_terminal_header(cx))
                    )
                    .child(
                        // Scrollable output area
                        div()
                            .flex_1()
                            .min_h_0()
                            .child(self.render_terminal_output(cx))
                    )
                    .child(
                        // Fixed input
                        div()
                            .flex_none()
                            .child(self.render_command_input(cx))
                    )
            )
            .child(
                // Vertical tab bar on the right
                self.render_vertical_tab_bar(cx)
            )
            .into_any_element()
    }
}