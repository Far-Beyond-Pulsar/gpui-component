use gpui::{prelude::*, div, px, rgb, App, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable, MouseButton, Render, SharedString, Window};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex, input::InputState, input::TextInput, v_flex, ActiveTheme as _, Icon, IconName,
    Sizable as _, StyledExt,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CommandType {
    Files,
    OpenSettings,
    ToggleTerminal,
    ToggleProblems,
    ToggleFileManager,
    BuildProject,
    RunProject,
    RestartAnalyzer,
    StopAnalyzer,
}

#[derive(Clone, Debug)]
pub struct Command {
    pub name: String,
    pub description: String,
    pub icon: IconName,
    pub command_type: CommandType,
    pub keywords: Vec<String>,
}

impl Command {
    fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        icon: IconName,
        command_type: CommandType,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            icon,
            command_type,
            keywords: vec![],
        }
    }

    fn with_keywords(mut self, keywords: Vec<impl Into<String>>) -> Self {
        self.keywords = keywords.into_iter().map(|k| k.into()).collect();
        self
    }

    fn matches(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        
        let query = query.to_lowercase();
        
        // Check name
        if self.name.to_lowercase().contains(&query) {
            return true;
        }
        
        // Check description
        if self.description.to_lowercase().contains(&query) {
            return true;
        }
        
        // Check keywords
        for keyword in &self.keywords {
            if keyword.to_lowercase().contains(&query) {
                return true;
            }
        }
        
        false
    }
}

#[derive(Clone, Debug)]
pub struct CommandSelected {
    pub command: Command,
}

pub struct CommandPalette {
    search_input: Entity<InputState>,
    focus_handle: FocusHandle,
    commands: Vec<Command>,
    filtered_commands: Vec<Command>,
    selected_index: usize,
    mode: PaletteMode,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PaletteMode {
    Commands,
    Files,
}

impl EventEmitter<CommandSelected> for CommandPalette {}

impl Focusable for CommandPalette {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl CommandPalette {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("Type a command or search files...", window, cx);
            state
        });

        let commands = Self::default_commands();
        let filtered_commands = commands.clone();

        let focus_handle = cx.focus_handle();

        Self {
            search_input,
            focus_handle,
            commands,
            filtered_commands,
            selected_index: 0,
            mode: PaletteMode::Commands,
        }
    }

    fn default_commands() -> Vec<Command> {
        vec![
            Command::new(
                "Search Files",
                "Search and open files in the project",
                IconName::Search,
                CommandType::Files,
            )
            .with_keywords(vec!["file", "open", "find"]),
            Command::new(
                "Toggle File Manager",
                "Show or hide the file explorer",
                IconName::Folder,
                CommandType::ToggleFileManager,
            )
            .with_keywords(vec!["files", "explorer", "sidebar", "ctrl+b"]),
            Command::new(
                "Toggle Terminal",
                "Show or hide the terminal",
                IconName::Terminal,
                CommandType::ToggleTerminal,
            )
            .with_keywords(vec!["console", "shell", "cmd"]),
            Command::new(
                "Toggle Problems",
                "Show or hide the problems panel",
                IconName::TriangleAlert,
                CommandType::ToggleProblems,
            )
            .with_keywords(vec!["errors", "warnings", "diagnostics"]),
            Command::new(
                "Open Settings",
                "Open application settings",
                IconName::Settings,
                CommandType::OpenSettings,
            )
            .with_keywords(vec!["preferences", "config", "configuration"]),
            Command::new(
                "Build Project",
                "Build the current project",
                IconName::Hammer,
                CommandType::BuildProject,
            )
            .with_keywords(vec!["compile", "make", "cargo build"]),
            Command::new(
                "Run Project",
                "Run the current project",
                IconName::Play,
                CommandType::RunProject,
            )
            .with_keywords(vec!["execute", "start", "cargo run"]),
            Command::new(
                "Restart Rust Analyzer",
                "Restart the Rust language server",
                IconName::Undo,
                CommandType::RestartAnalyzer,
            )
            .with_keywords(vec!["lsp", "language server", "intellisense"]),
            Command::new(
                "Stop Rust Analyzer",
                "Stop the Rust language server",
                IconName::X,
                CommandType::StopAnalyzer,
            )
            .with_keywords(vec!["kill", "terminate"]),
        ]
    }

    fn update_filter(&mut self, query: &str, _cx: &mut Context<Self>) {
        if query.is_empty() {
            self.filtered_commands = self.commands.clone();
        } else {
            self.filtered_commands = self
                .commands
                .iter()
                .filter(|cmd| cmd.matches(query))
                .cloned()
                .collect();
        }
        self.selected_index = 0;
    }

    fn select_command(&mut self, cx: &mut Context<Self>) {
        if let Some(command) = self.filtered_commands.get(self.selected_index) {
            cx.emit(CommandSelected {
                command: command.clone(),
            });
        }
    }

    fn move_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        if self.filtered_commands.is_empty() {
            return;
        }

        let new_index = (self.selected_index as isize + delta)
            .rem_euclid(self.filtered_commands.len() as isize) as usize;
        
        self.selected_index = new_index;
        cx.notify();
    }
}

impl Render for CommandPalette {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_index = self.selected_index;
        
        v_flex()
            .w(px(600.))
            .max_h(px(500.))
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(px(8.))
            .shadow_lg()
            .overflow_hidden()
            .track_focus(&self.focus_handle)
            .child(
                // Search input
                h_flex()
                    .p_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        TextInput::new(&self.search_input)
                            .appearance(false)
                            .bordered(false)
                            .prefix(
                                Icon::new(IconName::Search)
                                    .size(px(18.))
                                    .text_color(cx.theme().muted_foreground),
                            )
                            .w_full(),
                    ),
            )
            .child(
                // Command list
                v_flex()
                    .flex_1()
                    .overflow_hidden()
                    .gap_0p5()
                    .p_2()
                    .children(self.filtered_commands.iter().enumerate().map(|(i, cmd)| {
                        let is_selected = i == selected_index;
                        let command = cmd.clone();
                        
                        h_flex()
                            .w_full()
                            .px_3()
                            .py_2()
                            .rounded(px(6.))
                            .gap_3()
                            .items_center()
                            .cursor_pointer()
                            .when(is_selected, |this| {
                                this.bg(cx.theme().primary.opacity(0.15))
                            })
                            .hover(|s| s.bg(cx.theme().muted.opacity(0.2)))
                            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                this.selected_index = i;
                                this.select_command(cx);
                            }))
                            .child(
                                Icon::new(command.icon)
                                    .size(px(20.))
                                    .text_color(if is_selected {
                                        cx.theme().primary
                                    } else {
                                        cx.theme().muted_foreground
                                    }),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_semibold()
                                            .text_color(if is_selected {
                                                cx.theme().foreground
                                            } else {
                                                cx.theme().foreground.opacity(0.9)
                                            })
                                            .child(command.name.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(command.description.clone()),
                                    ),
                            )
                    })),
            )
            .when(self.filtered_commands.is_empty(), |this| {
                this.child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .p_8()
                        .child(
                            v_flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    Icon::new(IconName::Search)
                                        .size(px(48.))
                                        .text_color(cx.theme().muted_foreground.opacity(0.3)),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("No commands found"),
                                ),
                        ),
                )
            })
    }
}
