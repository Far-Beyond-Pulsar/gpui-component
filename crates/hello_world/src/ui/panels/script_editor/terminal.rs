use std::collections::VecDeque;
use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    input::{InputState, TextInput},
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

pub struct Terminal {
    focus_handle: FocusHandle,
    command_input: Entity<InputState>,
    history: VecDeque<TerminalLine>,
    command_history: Vec<String>,
    history_index: Option<usize>,
    max_lines: usize,
    is_visible: bool,
}

impl Terminal {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let command_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Type a command...")
        });

        let mut terminal = Self {
            focus_handle: cx.focus_handle(),
            command_input,
            history: VecDeque::new(),
            command_history: Vec::new(),
            history_index: None,
            max_lines: 1000,
            is_visible: true,
        };

        // Add some sample output
        terminal.add_line("Welcome to Script Editor Terminal".to_string(), TerminalLineType::Output);
        terminal.add_line("Type 'help' for available commands".to_string(), TerminalLineType::Output);

        terminal
    }

    pub fn toggle_visibility(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.is_visible = !self.is_visible;
        cx.notify();
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    fn add_line(&mut self, content: String, line_type: TerminalLineType) {
        self.history.push_back(TerminalLine { content, line_type });

        // Keep history within limits
        if self.history.len() > self.max_lines {
            self.history.pop_front();
        }
    }

    fn execute_command(&mut self, command: String, _window: &mut Window, cx: &mut Context<Self>) {
        // Add command to history
        self.add_line(format!("$ {}", command), TerminalLineType::Command);

        if !command.trim().is_empty() {
            self.command_history.push(command.clone());
            self.history_index = None;
        }

        // Process the command
        match command.trim() {
            "help" => {
                self.add_line("Available commands:".to_string(), TerminalLineType::Output);
                self.add_line("  help         - Show this help message".to_string(), TerminalLineType::Output);
                self.add_line("  clear        - Clear the terminal".to_string(), TerminalLineType::Output);
                self.add_line("  echo <text>  - Echo text".to_string(), TerminalLineType::Output);
                self.add_line("  pwd          - Show current directory".to_string(), TerminalLineType::Output);
                self.add_line("  ls           - List files in current directory".to_string(), TerminalLineType::Output);
                self.add_line("  cargo <args> - Run cargo commands".to_string(), TerminalLineType::Output);
                self.add_line("  npm <args>   - Run npm commands".to_string(), TerminalLineType::Output);
            },
            "clear" => {
                self.history.clear();
                self.add_line("Terminal cleared".to_string(), TerminalLineType::Success);
            },
            cmd if cmd.starts_with("echo ") => {
                let text = &cmd[5..];
                self.add_line(text.to_string(), TerminalLineType::Output);
            },
            "pwd" => {
                match std::env::current_dir() {
                    Ok(path) => self.add_line(path.display().to_string(), TerminalLineType::Output),
                    Err(e) => self.add_line(format!("Error: {}", e), TerminalLineType::Error),
                }
            },
            "ls" => {
                match std::fs::read_dir(".") {
                    Ok(entries) => {
                        for entry in entries.flatten() {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if entry.path().is_dir() {
                                self.add_line(format!("{}/", name), TerminalLineType::Output);
                            } else {
                                self.add_line(name, TerminalLineType::Output);
                            }
                        }
                    },
                    Err(e) => self.add_line(format!("Error: {}", e), TerminalLineType::Error),
                }
            },
            cmd if cmd.starts_with("cargo ") => {
                self.add_line("Executing cargo command...".to_string(), TerminalLineType::Output);
                // In a real implementation, you would execute the actual cargo command
                self.add_line("Note: Cargo execution not implemented in this demo".to_string(), TerminalLineType::Output);
            },
            cmd if cmd.starts_with("npm ") => {
                self.add_line("Executing npm command...".to_string(), TerminalLineType::Output);
                // In a real implementation, you would execute the actual npm command
                self.add_line("Note: NPM execution not implemented in this demo".to_string(), TerminalLineType::Output);
            },
            "" => {
                // Empty command, do nothing
            },
            _ => {
                self.add_line(format!("Command not found: {}", command.trim()), TerminalLineType::Error);
                self.add_line("Type 'help' for available commands".to_string(), TerminalLineType::Output);
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
                            .child("Terminal")
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{} lines", self.history.len()))
                    )
            )
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        Button::new("clear_terminal")
                            .icon(IconName::Delete)
                            .tooltip("Clear Terminal")
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.history.clear();
                                this.add_line("Terminal cleared".to_string(), TerminalLineType::Success);
                                cx.notify();
                            }))
                    )
                    .child(
                        Button::new("new_terminal")
                            .icon(IconName::Plus)
                            .tooltip("New Terminal")
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(|_this, _, _window, _cx| {
                                // TODO: Implement multiple terminal tabs
                            }))
                    )
                    .child(
                        Button::new("split_terminal")
                            .icon(IconName::Copy)
                            .tooltip("Split Terminal")
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(|_this, _, _window, _cx| {
                                // TODO: Implement terminal splitting
                            }))
                    )
                    .child(
                        Button::new("close_terminal")
                            .icon(IconName::CircleX)
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
        div()
            .flex_1()
            .p_3()
            .bg(cx.theme().background)
            .font_family("monospace")
            .text_sm()
            .overflow_hidden()
            .child(
                v_flex()
                    .gap_1()
                    .children(
                        self.history.iter().map(|line| {
                            div()
                                .text_color(self.get_line_color(&line.line_type, cx))
                                .child(line.content.clone())
                        })
                    )
            )
    }

    fn render_command_input(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
            .child(
                div()
                    .flex_1()
                    .h_6()
                    .px_2()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(px(4.0))
                    .text_sm()
                    .font_family("monospace")
                    .text_color(cx.theme().foreground)
                    .child("Press Enter to execute commands (demo mode)")
            )
    }
}

impl Focusable for Terminal {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Terminal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.is_visible {
            return div().into_any_element();
        }

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .child(self.render_terminal_header(cx))
            .child(self.render_terminal_output(cx))
            .child(self.render_command_input(cx))
            .into_any_element()
    }
}