use crate::ast::*;
use crate::component::*;
use crate::error::*;
use crate::hot_reload::*;
use crate::parser::GPMLParser;
use crate::renderer::GPMLRenderer;
use gpui::*;
use gpui_component::*;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use notify::{RecommendedWatcher, Watcher};

/// Main GPML canvas component that loads and renders GPML files dynamically
pub struct GPMLCanvas {
    /// Path to the main GPML file
    root_path: PathBuf,
    /// Current parsed document
    current_document: Option<GPMLNode>,
    /// Component resolution context
    context: Option<GPMLContext>,
    /// Component resolver for handling imports
    resolver: ComponentResolver,
    /// Hot reload manager
    hot_reload_manager: HotReloadManager,
    /// Error state
    error: Option<String>,
    /// Loading state
    is_loading: bool,
    /// Runtime variables that can be injected
    runtime_vars: HashMap<String, AttributeValue>,
    /// File watcher for hot reload (kept alive for the canvas lifetime)
    file_watcher: Option<RecommendedWatcher>,
}

impl GPMLCanvas {
    /// Create a new GPML canvas with the given root file
    pub fn new(root_path: impl AsRef<Path>) -> Self {
        let root_path = root_path.as_ref().to_path_buf();
        
        Self {
            root_path,
            current_document: None,
            context: None,
            resolver: ComponentResolver::new(),
            hot_reload_manager: HotReloadManager::new(),
            error: None,
            is_loading: false,
            runtime_vars: HashMap::new(),
            file_watcher: None,
        }
    }

    /// Create a new GPML canvas with runtime variables
    pub fn with_variables(mut self, vars: HashMap<String, AttributeValue>) -> Self {
        self.runtime_vars = vars;
        self
    }

    /// Add a runtime variable
    pub fn add_variable(&mut self, name: String, value: AttributeValue) {
        self.runtime_vars.insert(name, value);
    }

    /// Load the GPML file and all its dependencies
    pub fn load(&mut self) -> GPMLResult<()> {
        tracing::info!("GPMLCanvas::load called for path: {:?}", self.root_path);
        self.is_loading = true;
        self.error = None;

        match self.load_internal() {
            Ok(()) => {
                self.is_loading = false;
                tracing::info!("GPML file loaded successfully");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to load GPML file: {}", e);
                self.error = Some(format!("{}", e));
                self.is_loading = false;
                Err(e)
            }
        }
    }

    fn load_internal(&mut self) -> GPMLResult<()> {
        tracing::info!("Loading internal - checking file exists: {:?}", self.root_path);
        
        // Check if file exists first
        if !self.root_path.exists() {
            let error_msg = format!("File does not exist: {}", self.root_path.display());
            tracing::error!("{}", error_msg);
            return Err(GPMLError::FileNotFound {
                path: self.root_path.display().to_string(),
            });
        }

        // Load the context with all components and imports
        tracing::info!("Loading context and resolving components");
        let mut context = self.resolver.load_file(&self.root_path)?;
        
        // Add runtime variables to context
        for (name, value) in &self.runtime_vars {
            tracing::debug!("Adding runtime variable: {} = {:?}", name, value);
            context.variables.insert(name.clone(), value.clone());
        }
        
        self.context = Some(context);
        tracing::info!("Context loaded successfully");

        // Parse the main document
        tracing::info!("Reading file content from: {:?}", self.root_path);
        let content = std::fs::read_to_string(&self.root_path)
            .map_err(|e| {
                tracing::error!("Failed to read file {}: {}", self.root_path.display(), e);
                GPMLError::FileNotFound {
                    path: self.root_path.display().to_string(),
                }
            })?;

        tracing::info!("File content read, length: {} chars", content.len());
        tracing::debug!("File content preview: {}", 
            if content.len() > 200 { 
                format!("{}...", &content[..200]) 
            } else { 
                content.clone() 
            }
        );

        tracing::info!("Parsing GPML document");
        let document = GPMLParser::parse_file(&content)
            .map_err(|e| {
                tracing::error!("Parse error: {}", e);
                GPMLError::ParseError { 
                    message: e, 
                    line: 0, 
                    column: 0 
                }
            })?;
        
        tracing::info!("Document parsed successfully");
        if let GPMLNode::Document { imports, components, root } = &document {
            tracing::info!("Document structure - imports: {}, components: {}, has_root: {}", 
                imports.len(), components.len(), root.is_some());
            if let Some(root_elem) = root {
                tracing::info!("Root element: tag={}, children={}", root_elem.tag, root_elem.children.len());
            }
        }
        
        self.current_document = Some(document);
        tracing::info!("Document loaded into canvas successfully");

        Ok(())
    }

    /// Start hot reloading for this canvas
    pub fn start_hot_reload(&mut self, cx: &mut Context<Self>) -> GPMLResult<()> {
        tracing::info!("Starting hot reload for path: {:?}", self.root_path);
        
        // Convert to absolute path if needed
        let absolute_path = if self.root_path.is_absolute() {
            self.root_path.clone()
        } else {
            std::env::current_dir()
                .map_err(|e| GPMLError::IoError(e))?
                .join(&self.root_path)
        };
        
        tracing::info!("Hot reload absolute path: {:?}", absolute_path);
        tracing::info!("File exists: {}", absolute_path.exists());
        
        // Additional debugging
        if let Ok(metadata) = std::fs::metadata(&absolute_path) {
            tracing::info!("File metadata - size: {}, is_file: {}, modified: {:?}", 
                metadata.len(), metadata.is_file(), metadata.modified());
        } else {
            tracing::error!("Failed to get file metadata for: {:?}", absolute_path);
        }
        
        // Spawn a background task to watch for file changes with debouncing
        let (tx, rx) = smol::channel::bounded(10); // Smaller buffer to prevent flooding
        let watched_file = absolute_path.clone();
        
        tracing::info!("Creating file watcher for: {:?}", watched_file);
        
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            tracing::debug!("File watcher event received: {:?}", res);
            if let Ok(event) = &res {
                tracing::debug!("Event kind: {:?}, paths: {:?}", event.kind, event.paths);
                match event.kind {
                    // Accept any modify event that indicates file content change
                    notify::EventKind::Modify(notify::event::ModifyKind::Data(_)) |
                    notify::EventKind::Modify(notify::event::ModifyKind::Any) => {
                        tracing::info!("File modification event detected: {:?}", event.kind);
                        for path in &event.paths {
                            tracing::info!("Checking path: {:?} against watched file: {:?}", path, watched_file);
                            // Only react to changes to our specific file
                            if path == &watched_file && path.extension().and_then(|s| s.to_str()) == Some("gpml") {
                                tracing::info!("GPML file change detected, sending to channel: {:?}", path);
                                // Use try_send to avoid blocking - if channel is full, skip this event
                                match tx.try_send(path.clone()) {
                                    Ok(_) => tracing::info!("File change event sent successfully"),
                                    Err(e) => tracing::warn!("Failed to send file change event: {:?}", e),
                                }
                                break; // Only send once per event
                            }
                        }
                    }
                    _ => {
                        tracing::debug!("Ignoring event kind: {:?}", event.kind);
                    } // Ignore other event types to reduce noise
                }
            } else {
                tracing::error!("File watcher error: {:?}", res);
            }
        }).map_err(|e| GPMLError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create file watcher: {}", e)
        )))?;
        
        use notify::Watcher;
        // Only watch the specific file, not the directory
        tracing::info!("Attempting to watch file: {:?}", absolute_path);
        watcher.watch(&absolute_path, notify::RecursiveMode::NonRecursive).map_err(|e| {
            tracing::error!("Failed to watch path {:?}: {}", absolute_path, e);
            GPMLError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to watch path: {}", e)
            ))
        })?;
        
        // Store the watcher in the struct to keep it alive
        self.file_watcher = Some(watcher);
        
        tracing::info!("File watcher started successfully for: {:?}", absolute_path);
        
        cx.spawn(async move |this, mut cx| {
            tracing::info!("Hot reload background task started");
            let mut last_reload = std::time::Instant::now();
            const DEBOUNCE_DURATION: std::time::Duration = std::time::Duration::from_millis(500);
            
            while let Ok(changed_path) = rx.recv().await {
                tracing::info!("Received file change event in background task: {:?}", changed_path);
                
                // Debounce: only reload if enough time has passed
                let now = std::time::Instant::now();
                if now.duration_since(last_reload) < DEBOUNCE_DURATION {
                    tracing::info!("Debouncing file change (too recent), skipping reload");
                    continue;
                }
                last_reload = now;
                
                tracing::info!("GPML file changed (debounced): {:?}", changed_path);
                
                // Update the canvas on the main thread
                let update_result = this.update(cx, |canvas, cx| {
                    tracing::info!("Updating canvas after file change");
                    // Clear resolver cache for changed file
                    canvas.resolver.remove_from_cache(&changed_path);
                    
                    // Reload the canvas
                    if let Err(e) = canvas.load() {
                        tracing::error!("Failed to reload after file change: {}", e);
                    } else {
                        tracing::info!("Successfully reloaded after file change");
                    }
                    
                    // Notify for re-render
                    cx.notify();
                });
                
                if let Err(e) = update_result {
                    tracing::error!("Failed to update canvas: {:?}", e);
                }
            }
            tracing::warn!("Hot reload background task ended (channel closed)");
        }).detach();
        
        tracing::info!("Hot reload setup complete");
        Ok(())
    }

    /// Check for changes and reload if necessary
    pub fn check_and_reload(&mut self) -> GPMLResult<bool> {
        let changes = self.hot_reload_manager.check_for_changes();
        
        if !changes.is_empty() {
            tracing::debug!("GPML files changed: {:?}", changes);
            
            // Clear resolver cache for changed files
            for changed_path in &changes {
                self.resolver.remove_from_cache(changed_path);
            }
            
            // Reload everything
            self.load()?;
            return Ok(true);
        }
        
        Ok(false)
    }

    /// Force reload the canvas
    pub fn reload(&mut self) -> GPMLResult<()> {
        self.resolver.clear_cache();
        self.load()
    }

    /// Get the current error if any
    pub fn get_error(&self) -> Option<&String> {
        self.error.as_ref()
    }

    /// Check if the canvas is currently loading
    pub fn is_loading(&self) -> bool {
        self.is_loading
    }

    /// Check if the canvas has loaded content
    pub fn is_loaded(&self) -> bool {
        self.current_document.is_some() && self.context.is_some()
    }

    /// Get the root element from the document
    pub fn get_root_element(&self) -> Option<&GPMLElement> {
        if let Some(GPMLNode::Document { root: Some(root), .. }) = &self.current_document {
            Some(root)
        } else {
            None
        }
    }

    /// Load GPML from a string instead of a file
    pub fn load_from_string(&mut self, content: &str, base_path: Option<&Path>) -> GPMLResult<()> {
        self.is_loading = true;
        self.error = None;

        let base_path = base_path.unwrap_or_else(|| Path::new("."));
        let mut context = GPMLContext::new(base_path);
        
        // Add runtime variables
        for (name, value) in &self.runtime_vars {
            context.variables.insert(name.clone(), value.clone());
        }

        let document = GPMLParser::parse_file(content)
            .map_err(|e| GPMLError::ParseError { 
                message: e, 
                line: 0, 
                column: 0 
            })?;

        // Process imports and components from the document
        self.resolver.clear_cache();
        
        if let GPMLNode::Document { imports, components, .. } = &document {
            for component in components {
                context.add_component(component.clone());
            }
            
            // Note: imports won't work with string content unless base_path is set properly
            if !imports.is_empty() && base_path == Path::new(".") {
                tracing::warn!("GPML imports found but no base path set - imports will not resolve");
            }
        }

        self.current_document = Some(document);
        self.context = Some(context);
        self.is_loading = false;

        Ok(())
    }

    /// Update a runtime variable and trigger re-render if canvas is loaded
    pub fn update_variable(&mut self, name: String, value: AttributeValue) -> bool {
        self.runtime_vars.insert(name.clone(), value.clone());
        
        if let Some(ref mut context) = self.context {
            context.variables.insert(name, value);
            true
        } else {
            false
        }
    }

    /// Get current runtime variables
    pub fn get_variables(&self) -> &HashMap<String, AttributeValue> {
        &self.runtime_vars
    }

    /// Clear all runtime variables
    pub fn clear_variables(&mut self) {
        self.runtime_vars.clear();
        if let Some(ref mut context) = self.context {
            // Keep only the original variables from the document
            context.variables.retain(|k, _| !self.runtime_vars.contains_key(k));
        }
    }
}

impl Render for GPMLCanvas {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        tracing::info!("GPMLCanvas::render called");
        tracing::info!("Canvas state - loading: {}, error: {:?}, document loaded: {}, context loaded: {}", 
            self.is_loading, 
            self.error.as_ref().map(|e| e.as_str()),
            self.current_document.is_some(),
            self.context.is_some()
        );

        // Handle different states
        if self.is_loading {
            tracing::info!("Rendering loading state");
            return self.render_loading_state(window, cx);
        }

        if let Some(error) = &self.error {
            tracing::error!("Rendering error state: {}", error);
            return self.render_error_state(error, window, cx);
        }

        if let (Some(root_element), Some(context)) = (self.get_root_element(), &self.context) {
            tracing::info!("Rendering GPML element: tag={}, children={}", root_element.tag, root_element.children.len());
            match GPMLRenderer::render_element(root_element, context, &self.resolver, cx) {
                Ok(element) => {
                    tracing::info!("Successfully rendered GPML element");
                    element
                },
                Err(e) => {
                    tracing::error!("GPML render error: {}", e);
                    self.render_error_state(&format!("{}", e), window, cx)
                }
            }
        } else {
            tracing::warn!("No root element or context available - rendering empty state");
            tracing::debug!("Root element available: {}, Context available: {}", 
                self.get_root_element().is_some(),
                self.context.is_some()
            );
            self.render_empty_state(window, cx)
        }
    }
}

impl GPMLCanvas {
    fn render_loading_state(&self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .items_center()
            .justify_center()
            .size_full()
            .gap_4()
            .child(
                Icon::new(IconName::Loader)
                    .size(px(24.0))
                    .text_color(cx.theme().muted_foreground)
            )
            .child(
                div()
                    .text_size(px(14.0))
                    .text_color(cx.theme().muted_foreground)
                    .child("Loading GPML...")
            )
            .into_any_element()
    }

    fn render_error_state(&self, error: &String, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .items_center()
            .justify_center()
            .size_full()
            .gap_4()
            .p_4()
            .child(
                Icon::new(IconName::TriangleAlert)
                    .size(px(24.0))
                    .text_color(gpui::red())
            )
            .child(
                div()
                    .text_size(px(16.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(gpui::red())
                    .child("GPML Error")
            )
            .child(
                div()
                    .text_size(px(14.0))
                    .text_color(cx.theme().muted_foreground)
                    //TODO:.text_wrap()
                    .max_w(px(600.0))
                    .child(error.clone())
            )
            .child(
                button::Button::new("reload-button")
                    .child("Reload")
                    .on_click(cx.listener(|canvas, _event, _window, _cx| {
                        if let Err(e) = canvas.reload() {
                            tracing::error!("Failed to reload GPML: {}", e);
                        }
                    }))
            )
            .into_any_element()
    }

    fn render_empty_state(&self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .items_center()
            .justify_center()
            .size_full()
            .gap_4()
            .child(
                Icon::new(IconName::Folder)
                    .size(px(24.0))
                    .text_color(cx.theme().muted_foreground)
            )
            .child(
                div()
                    .text_size(px(14.0))
                    .text_color(cx.theme().muted_foreground)
                    .child("No GPML content loaded")
            )
            .into_any_element()
    }
}

/// Create a GPML canvas view entity
pub fn create_gpml_canvas<V>(
    root_path: impl AsRef<Path>,
    cx: &mut Context<V>,
) -> GPMLCanvas
where
    V: Render + 'static,
{
    let mut canvas = GPMLCanvas::new(root_path);
    
    // Try to load the file
    if let Err(e) = canvas.load() {
        tracing::error!("Failed to load GPML file: {}", e);
    }
    
    canvas
}

/// Create a GPML canvas view with runtime variables
pub fn create_gpml_canvas_with_vars<V>(
    root_path: impl AsRef<Path>,
    variables: HashMap<String, AttributeValue>,
    cx: &mut Context<V>,
) -> GPMLCanvas
where
    V: Render + 'static,
{
    let mut canvas = GPMLCanvas::new(root_path).with_variables(variables);
    
    if let Err(e) = canvas.load() {
        tracing::error!("Failed to load GPML file: {}", e);
    }
    
    canvas
}
