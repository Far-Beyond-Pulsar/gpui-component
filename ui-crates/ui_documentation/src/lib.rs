use gpui::{prelude::*, *};
use rust_embed::RustEmbed;
use ui::{
    ActiveTheme, Root, Sizable, StyledExt,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex, IconName,
};
use std::collections::HashMap;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../target/doc"]
pub struct DocAssets;

pub struct DocumentationWindow {
    focus_handle: FocusHandle,
    current_page: String,
    search_query: String,
    search_input_state: Entity<ui::input::InputState>,
    search_results: Vec<SearchResult>,
    doc_index: HashMap<String, DocPage>,
    sidebar_items: Vec<SidebarItem>,
    breadcrumbs: Vec<String>,
}

#[derive(Clone, Debug)]
struct DocPage {
    path: String,
    title: String,
    content: String,
    module_path: Vec<String>,
}

#[derive(Clone, Debug)]
struct SearchResult {
    title: String,
    path: String,
    snippet: String,
    relevance: f32,
}

#[derive(Clone, Debug)]
struct SidebarItem {
    name: String,
    path: String,
    level: usize,
    children: Vec<SidebarItem>,
    is_expanded: bool,
}

impl DocumentationWindow {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input_state = cx.new(|cx| ui::input::InputState::new(window, cx));
        
        let mut window = Self {
            focus_handle: cx.focus_handle(),
            current_page: String::from("index.html"),
            search_query: String::new(),
            search_input_state,
            search_results: Vec::new(),
            doc_index: HashMap::new(),
            sidebar_items: Vec::new(),
            breadcrumbs: vec!["Pulsar Engine".to_string()],
        };
        
        // Index all documentation
        window.index_documentation();
        window.build_sidebar();
        
        window
    }
    
    fn index_documentation(&mut self) {
        // Parse all HTML files from embedded docs
        for file_path in DocAssets::iter() {
            let file_name = file_path.as_ref();
            
            // Only process HTML files (skip CSS, JS, images, etc.)
            if !file_name.ends_with(".html") {
                continue;
            }
            
            // Skip files in static.files, src, etc.
            if file_name.starts_with("static.files/") || 
               file_name.starts_with("src/") ||
               file_name.starts_with("search.index/") ||
               file_name.starts_with("trait.impl/") ||
               file_name.starts_with("type.impl/") {
                continue;
            }
            
            if let Some(content) = DocAssets::get(file_name) {
                if let Ok(html) = std::str::from_utf8(&content.data) {
                    let doc_page = self.parse_doc_page(file_name, html);
                    self.doc_index.insert(file_name.to_string(), doc_page);
                }
            }
        }
        
        // If no docs were loaded, add placeholder
        if self.doc_index.is_empty() {
            self.doc_index.insert("index.html".to_string(), DocPage {
                path: "index.html".to_string(),
                title: "Pulsar Engine Documentation".to_string(),
                content: "<p>Welcome to Pulsar Engine documentation. Run <code>cargo doc --workspace</code> to generate full documentation.</p>".to_string(),
                module_path: vec!["pulsar_engine".to_string()],
            });
        }
    }
    
    fn parse_doc_page(&self, path: &str, html: &str) -> DocPage {
        // Extract title from HTML
        let title = html
            .find("<title>")
            .and_then(|start| {
                let title_start = start + 7;
                html[title_start..].find("</title>").map(|end| {
                    html[title_start..title_start + end].to_string()
                })
            })
            .unwrap_or_else(|| path.to_string());
        
        // Extract module path from path
        let module_path: Vec<String> = path
            .trim_end_matches(".html")
            .split('/')
            .map(String::from)
            .collect();
        
        // Extract main content
        let content = html
            .find("<main>")
            .and_then(|start| {
                html[start..].find("</main>").map(|end| {
                    html[start..start + end].to_string()
                })
            })
            .unwrap_or_else(|| html.to_string());
        
        DocPage {
            path: path.to_string(),
            title,
            content,
            module_path,
        }
    }
    
    fn build_sidebar(&mut self) {
        // Build hierarchical sidebar from doc index
        let mut root_items: HashMap<String, SidebarItem> = HashMap::new();
        
        for (path, page) in &self.doc_index {
            if page.module_path.is_empty() {
                continue;
            }
            
            let crate_name = &page.module_path[0];
            let item = root_items.entry(crate_name.clone()).or_insert_with(|| {
                SidebarItem {
                    name: crate_name.clone(),
                    path: format!("{}/index.html", crate_name),
                    level: 0,
                    children: Vec::new(),
                    is_expanded: true, // Start expanded
                }
            });
            
            // Add nested items
            if page.module_path.len() > 1 {
                let child = SidebarItem {
                    name: page.title.clone(),
                    path: path.clone(),
                    level: 1,
                    children: Vec::new(),
                    is_expanded: false,
                };
                item.children.push(child);
            }
        }
        
        self.sidebar_items = root_items.into_values().collect();
        self.sidebar_items.sort_by(|a, b| a.name.cmp(&b.name));
    }
    
    fn toggle_sidebar_item(&mut self, path: &str, cx: &mut Context<Self>) {
        for item in &mut self.sidebar_items {
            if item.path == path {
                item.is_expanded = !item.is_expanded;
                cx.notify();
                return;
            }
        }
    }
    
    fn perform_search(&mut self, query: &str) {
        self.search_results.clear();
        
        if query.is_empty() {
            return;
        }
        
        let query_lower = query.to_lowercase();
        
        for (path, page) in &self.doc_index {
            let title_lower = page.title.to_lowercase();
            let content_lower = page.content.to_lowercase();
            
            // Calculate relevance
            let mut relevance = 0.0;
            
            // Title match is most relevant
            if title_lower.contains(&query_lower) {
                relevance += 10.0;
            }
            
            // Content match
            if content_lower.contains(&query_lower) {
                relevance += 1.0;
            }
            
            // Exact match bonus
            if title_lower == query_lower {
                relevance += 20.0;
            }
            
            if relevance > 0.0 {
                // Extract snippet
                let snippet = if let Some(pos) = content_lower.find(&query_lower) {
                    let start = pos.saturating_sub(50);
                    let end = (pos + query.len() + 50).min(page.content.len());
                    format!("...{}...", &page.content[start..end])
                } else {
                    page.content.chars().take(100).collect()
                };
                
                self.search_results.push(SearchResult {
                    title: page.title.clone(),
                    path: path.clone(),
                    snippet,
                    relevance,
                });
            }
        }
        
        // Sort by relevance
        self.search_results.sort_by(|a, b| {
            b.relevance.partial_cmp(&a.relevance).unwrap()
        });
    }
    
    fn navigate_to(&mut self, path: &str, cx: &mut Context<Self>) {
        self.current_page = path.to_string();
        
        // Update breadcrumbs
        if let Some(page) = self.doc_index.get(path) {
            self.breadcrumbs = page.module_path.clone();
            if self.breadcrumbs.is_empty() {
                self.breadcrumbs.push("Pulsar Engine".to_string());
            }
        }
        
        cx.notify();
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
                // Header with search
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
                            .child("📚 Documentation")
                    )
                    .child(
                        div()
                            .flex_1()
                            .max_w_96()
                            .child(
                                ui::input::TextInput::new(&self.search_input_state)
                            )
                    )
                    .child(
                        Button::new("refresh-docs")
                            .icon(IconName::Refresh)
                            .ghost()
                            .tooltip("Refresh Documentation")
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                this.doc_index.clear();
                                this.sidebar_items.clear();
                                this.index_documentation();
                                this.build_sidebar();
                                cx.notify();
                            }))
                    )
            )
            .child(
                // Breadcrumbs
                div()
                    .w_full()
                    .h_10()
                    .flex()
                    .items_center()
                    .px_4()
                    .gap_2()
                    .bg(theme.sidebar)
                    .border_b_1()
                    .border_color(theme.border)
                    .children(
                        self.breadcrumbs.iter().enumerate().flat_map(|(i, crumb)| {
                            let mut elements = vec![
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child(crumb.clone())
                                    .into_any_element()
                            ];
                            
                            if i < self.breadcrumbs.len() - 1 {
                                elements.push(
                                    div()
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child("›")
                                        .into_any_element()
                                );
                            }
                            
                            elements
                        })
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
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .id("docs-sidebar-content")
                                    .flex_1()
                                    .overflow_y_scroll()
                                    .child(
                                        v_flex()
                                            .w_full()
                                            .p_2()
                                            .gap_1()
                                            .children(
                                                self.sidebar_items.iter().enumerate().map(|(i, item)| {
                                                    self.render_sidebar_item(item, i, cx)
                                                })
                                            )
                                    )
                            )
                    )
                    .child(
                        // Content area
                        div()
                            .flex_1()
                            .h_full()
                            .child(
                                if !self.search_query.is_empty() && !self.search_results.is_empty() {
                                    self.render_search_results(cx)
                                } else {
                                    self.render_doc_content(cx)
                                }
                            )
                    )
            )
    }
}

impl DocumentationWindow {
    fn render_sidebar_item(&self, item: &SidebarItem, _index: usize, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let is_current = self.current_page == item.path;
        let has_children = !item.children.is_empty();
        
        let item_path = item.path.clone();
        let children = item.children.clone();
        let is_expanded = item.is_expanded;
        
        v_flex()
            .w_full()
            .child(
                Button::new(SharedString::from(format!("sidebar-item-{}", item_path)))
                    .w_full()
                    .justify_start()
                    .ghost()
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        this.navigate_to(&item_path, cx);
                    }))
                    .label(item.name.clone())
            )
            .when(is_expanded && has_children, |this| {
                this.children(
                    children.iter().map(|child| {
                        let child_path = child.path.clone();
                        let child_is_current = self.current_page == child.path;
                        
                        Button::new(SharedString::from(format!("sidebar-child-{}", child_path)))
                            .w_full()
                            .justify_start()
                            .ghost()
                            .small()
                            .pl_6()
                            .label(child.name.clone())
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                this.navigate_to(&child_path, cx);
                            }))
                            .into_any_element()
                    })
                )
            })
            .into_any_element()
    }
    
    fn render_search_results(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        
        div()
            .id("docs-search-results")
            .size_full()
            .overflow_y_scroll()
            .child(
                v_flex()
                    .w_full()
                    .p_6()
                    .gap_4()
                    .child(
                        div()
                            .text_lg()
                            .font_bold()
                            .text_color(theme.foreground)
                            .child(format!("{} results", self.search_results.len()))
                    )
                    .children(
                        self.search_results.iter().map(|result| {
                            let result_path = result.path.clone();
                            
                            Button::new(SharedString::from(format!("search-result-{}", result_path)))
                                .w_full()
                                .ghost()
                                .justify_start()
                                .on_click(cx.listener(move |this, _event, _window, cx| {
                                    this.navigate_to(&result_path, cx);
                                    // Clear search
                                    this.search_query.clear();
                                    this.search_results.clear();
                                    cx.notify();
                                }))
                                .child(
                                    v_flex()
                                        .gap_2()
                                        .child(
                                            div()
                                                .text_base()
                                                .font_semibold()
                                                .text_color(theme.accent)
                                                .child(result.title.clone())
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(theme.muted_foreground)
                                                .child(result.snippet.clone())
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(theme.muted_foreground.opacity(0.7))
                                                .child(result.path.clone())
                                        )
                                )
                                .into_any_element()
                        })
                    )
            )
            .into_any_element()
    }
    
    fn render_doc_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        
        if let Some(page) = self.doc_index.get(&self.current_page) {
            // Extract text content from HTML (simple HTML stripping)
            let plain_text = self.html_to_text(&page.content);
            
            div()
                .id("docs-content")
                .size_full()
                .overflow_y_scroll()
                .child(
                    div()
                        .w_full()
                        .max_w(px(1200.0))
                        .mx_auto()
                        .p_8()
                        .child(
                            v_flex()
                                .gap_6()
                                .child(
                                    div()
                                        .text_3xl()
                                        .font_bold()
                                        .text_color(theme.foreground)
                                        .child(page.title.clone())
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child(page.module_path.join(" > "))
                                )
                                .child(
                                    div()
                                        .w_full()
                                        .h_px()
                                        .bg(theme.border)
                                )
                                .child(
                                    div()
                                        .text_base()
                                        .text_color(theme.foreground)
                                        .child(plain_text)
                                )
                        )
                )
                .into_any_element()
        } else {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    v_flex()
                        .gap_4()
                        .items_center()
                        .child(
                            div()
                                .text_2xl()
                                .child("📄")
                        )
                        .child(
                            div()
                                .text_lg()
                                .text_color(theme.muted_foreground)
                                .child("Page not found")
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(format!("Looking for: {}", self.current_page))
                        )
                )
                .into_any_element()
        }
    }
    
    fn html_to_text(&self, html: &str) -> String {
        // Simple HTML tag removal for display
        let mut text = String::new();
        let mut in_tag = false;
        let mut in_script = false;
        let mut in_style = false;
        
        let html_lower = html.to_lowercase();
        
        for (i, c) in html.chars().enumerate() {
            // Check for script/style tags
            if html_lower[i..].starts_with("<script") {
                in_script = true;
            } else if html_lower[i..].starts_with("</script") {
                in_script = false;
            } else if html_lower[i..].starts_with("<style") {
                in_style = true;
            } else if html_lower[i..].starts_with("</style") {
                in_style = false;
            }
            
            if in_script || in_style {
                continue;
            }
            
            match c {
                '<' => in_tag = true,
                '>' => {
                    in_tag = false;
                    text.push(' ');
                }
                _ if !in_tag => text.push(c),
                _ => {}
            }
        }
        
        // Clean up whitespace
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(5000) // Limit length
            .collect()
    }
}

/// Helper to create documentation window with Root wrapper
pub fn create_documentation_window(window: &mut Window, cx: &mut App) -> Entity<Root> {
    let docs = cx.new(|cx| DocumentationWindow::new(window, cx));
    cx.new(|cx| Root::new(docs.into(), window, cx))
}
