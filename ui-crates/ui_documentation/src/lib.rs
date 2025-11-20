use gpui::{prelude::*, *};
use rust_embed::RustEmbed;
use ui::{
    ActiveTheme, Root, Sizable, StyledExt,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex, IconName,
    markdown::Markdown,
};
use std::collections::HashMap;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../target/doc"]
pub struct DocAssets;

pub struct DocumentationWindow {
    focus_handle: FocusHandle,
    current_crate: Option<String>,
    current_section: Option<String>,
    crates: Vec<CrateDoc>,
    markdown_content: String,
}

#[derive(Clone, Debug)]
struct CrateDoc {
    name: String,
    path: String,
    sections: Vec<Section>,
    is_expanded: bool,
}

#[derive(Clone, Debug)]
struct Section {
    name: String,
    content_key: String,
}

impl DocumentationWindow {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut window = Self {
            focus_handle: cx.focus_handle(),
            current_crate: None,
            current_section: None,
            crates: Vec::new(),
            markdown_content: "# Pulsar Engine Documentation\n\nSelect a crate from the sidebar to view its documentation.".to_string(),
        };
        
        window.load_documentation();
        window
    }
    
    fn load_documentation(&mut self) {
        // Load all .md files from embedded docs
        let mut crate_map: HashMap<String, Vec<String>> = HashMap::new();
        
        for file_path in DocAssets::iter() {
            let file_name = file_path.as_ref();
            
            if file_name.ends_with(".md") {
                // Extract crate name from path like "pulsar_engine.md" or "ui/index.md"
                let parts: Vec<&str> = file_name.split('/').collect();
                let crate_name = if parts.len() > 1 {
                    parts[0].to_string()
                } else {
                    file_name.trim_end_matches(".md").to_string()
                };
                
                crate_map.entry(crate_name).or_default().push(file_name.to_string());
            }
        }
        
        // Build crate list with sections
        for (crate_name, _files) in crate_map {
            self.crates.push(CrateDoc {
                name: crate_name.clone(),
                path: format!("{}.md", crate_name),
                sections: vec![
                    Section { name: "Overview".to_string(), content_key: format!("{}.md", crate_name) },
                    Section { name: "Functions".to_string(), content_key: format!("{}_fns.md", crate_name) },
                    Section { name: "Macros".to_string(), content_key: format!("{}_macros.md", crate_name) },
                    Section { name: "Structs".to_string(), content_key: format!("{}_structs.md", crate_name) },
                    Section { name: "Traits".to_string(), content_key: format!("{}_traits.md", crate_name) },
                ],
                is_expanded: false,
            });
        }
        
        self.crates.sort_by(|a, b| a.name.cmp(&b.name));
    }
    
    fn load_crate_content(&mut self, crate_name: &str, section: Option<&str>) {
        self.current_crate = Some(crate_name.to_string());
        self.current_section = section.map(String::from);
        
        let file_key = if let Some(sec) = section {
            format!("{}_{}.md", crate_name, sec.to_lowercase())
        } else {
            format!("{}.md", crate_name)
        };
        
        if let Some(content) = DocAssets::get(&file_key) {
            if let Ok(markdown) = std::str::from_utf8(&content.data) {
                self.markdown_content = markdown.to_string();
            } else {
                self.markdown_content = format!("# {}\n\nNo documentation available.", crate_name);
            }
        } else {
            self.markdown_content = format!("# {}\n\nNo documentation available for this section.", crate_name);
        }
    }
    
    fn toggle_crate(&mut self, crate_name: &str, cx: &mut Context<Self>) {
        for crate_doc in &mut self.crates {
            if crate_doc.name == crate_name {
                crate_doc.is_expanded = !crate_doc.is_expanded;
                
                // If expanding and no section selected, load overview
                if crate_doc.is_expanded && self.current_crate.as_deref() != Some(crate_name) {
                    self.load_crate_content(crate_name, None);
                }
                
                cx.notify();
                return;
            }
        }
    }
}

impl Focusable for DocumentationWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DocumentationWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        
        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .child(
                // Header
                div()
                    .w_full()
                    .h_16()
                    .flex()
                    .items_center()
                    .px_4()
                    .gap_4()
                    .bg(theme.sidebar)
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_lg()
                            .font_bold()
                            .text_color(theme.foreground)
                            .child("📚 Pulsar Engine Documentation")
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("refresh-docs")
                            .icon(IconName::Refresh)
                            .ghost()
                            .tooltip("Refresh Documentation")
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                this.crates.clear();
                                this.load_documentation();
                                cx.notify();
                            }))
                    )
            )
            .child(
                // Main content area
                h_flex()
                    .flex_1()
                    .overflow_hidden()
                    .child(
                        // Sidebar
                        div()
                            .id("docs-sidebar")
                            .w_64()
                            .h_full()
                            .bg(theme.sidebar)
                            .border_r_1()
                            .border_color(theme.border)
                            .child(
                                div()
                                    .id("docs-sidebar-content")
                                    .h_full()
                                    .overflow_y_scroll()
                                    .child(
                                        v_flex()
                                            .w_full()
                                            .p_2()
                                            .gap_1()
                                            .children(
                                                self.crates.iter().map(|crate_doc| {
                                                    self.render_crate_item(crate_doc, cx)
                                                })
                                            )
                                    )
                            )
                    )
                    .child(
                        // Content area with markdown
                        div()
                            .id("docs-content")
                            .flex_1()
                            .h_full()
                            .overflow_y_scroll()
                            .child(
                                div()
                                    .w_full()
                                    .max_w(px(1200.0))
                                    .mx_auto()
                                    .p_8()
                                    .child(
                                        Markdown::new(
                                            self.markdown_content.clone(),
                                            None,
                                            None,
                                            cx,
                                        )
                                    )
                            )
                    )
            )
    }
}

impl DocumentationWindow {
    fn render_crate_item(&self, crate_doc: &CrateDoc, cx: &mut Context<Self>) -> AnyElement {
        let crate_name = crate_doc.name.clone();
        let is_expanded = crate_doc.is_expanded;
        let is_selected = self.current_crate.as_deref() == Some(&crate_doc.name);
        
        v_flex()
            .w_full()
            .child(
                Button::new(SharedString::from(format!("crate-{}", crate_name)))
                    .w_full()
                    .justify_start()
                    .ghost()
                    .label(format!("📦 {}", crate_doc.name))
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        this.toggle_crate(&crate_name, cx);
                    }))
            )
            .when(is_expanded, |this| {
                this.children(
                    crate_doc.sections.iter().map(|section| {
                        let section_name = section.name.clone();
                        let crate_name_clone = crate_name.clone();
                        let is_current = self.current_crate.as_deref() == Some(&crate_name) 
                            && self.current_section.as_deref() == Some(&section.name);
                        
                        Button::new(SharedString::from(format!("section-{}-{}", crate_name, section_name)))
                            .w_full()
                            .justify_start()
                            .ghost()
                            .small()
                            .pl_8()
                            .label(section.name.clone())
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                this.load_crate_content(&crate_name_clone, Some(&section_name));
                                cx.notify();
                            }))
                            .into_any_element()
                    })
                )
            })
            .into_any_element()
    }
}

/// Helper to create documentation window with Root wrapper
pub fn create_documentation_window(window: &mut Window, cx: &mut App) -> Entity<Root> {
    let docs = cx.new(|cx| DocumentationWindow::new(window, cx));
    cx.new(|cx| Root::new(docs.into(), window, cx))
}
