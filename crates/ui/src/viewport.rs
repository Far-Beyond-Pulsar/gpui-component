use gpui::canvas;
use gpui::{
    div, App, Bounds, ContentMask, DismissEvent, Element, ElementId, Entity, EventEmitter,
    FocusHandle, Focusable, GlobalElementId, Hitbox, InteractiveElement, IntoElement, LayoutId,
    ParentElement as _, Pixels, Render, Size, Style, Styled as _, Window,
};

/// A trait for render engines that can render to a framebuffer
pub trait RenderEngine: Send + Sync + 'static {
    /// Render to the given framebuffer
    fn render(&mut self, framebuffer: &mut Framebuffer);
}

/// A zero-copy framebuffer that can be rendered to
pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pub buffer: Vec<u8>, // RGBA format
}

impl Framebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            buffer: vec![0; (width * height * 4) as usize],
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.buffer.resize((width * height * 4) as usize, 0);
    }
}

pub struct Viewport<E: RenderEngine> {
    focus_handle: FocusHandle,
    render_engine: E,
    framebuffer: Framebuffer,
    visible: bool,
    bounds: Bounds<Pixels>,
}

impl<E: RenderEngine> Drop for Viewport<E> {
    fn drop(&mut self) {
        self.hide();
    }
}

impl<E: RenderEngine> Viewport<E> {
    pub fn new(render_engine: E, initial_width: u32, initial_height: u32, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            render_engine,
            framebuffer: Framebuffer::new(initial_width, initial_height),
            visible: true,
            bounds: Bounds::default(),
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }

    pub fn render(&mut self) {
        if self.visible {
            self.render_engine.render(&mut self.framebuffer);
        }
    }

    pub fn framebuffer(&self) -> &Framebuffer {
        &self.framebuffer
    }

    pub fn framebuffer_mut(&mut self) -> &mut Framebuffer {
        &mut self.framebuffer
    }

    pub fn render_engine(&mut self) -> &mut E {
        &mut self.render_engine
    }
}

impl<E: RenderEngine> Focusable for Viewport<E> {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl<E: RenderEngine> EventEmitter<DismissEvent> for Viewport<E> {}

impl<E: RenderEngine> Render for Viewport<E> {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let view = cx.entity().clone();
        
        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .child({
                let view = cx.entity().clone();
                canvas(
                    move |bounds, _, cx| {
                        view.update(cx, |viewport, _| {
                            viewport.bounds = bounds;
                            let width = bounds.size.width.0 as u32;
                            let height = bounds.size.height.0 as u32;
                            if viewport.framebuffer.width != width || viewport.framebuffer.height != height {
                                viewport.framebuffer.resize(width, height);
                            }
                            viewport.render();
                        });
                    },
                    move |frame, bounds, window, cx| {
                        let viewport = view.read(cx);
                        if !viewport.visible {
                            return;
                        }
                        
                        let framebuffer = &viewport.framebuffer;
                        if framebuffer.width == 0 || framebuffer.height == 0 {
                            return;
                        }

                        window.with_content_mask(Some(ContentMask { bounds }), |window| {
                            // Drawing code here
                        });
                        bounds
                    },
                )
                .absolute()
                .size_full()
            })
    }
}

/// A viewport element that renders the framebuffer
pub struct ViewportElement<E: RenderEngine> {
    parent: Entity<Viewport<E>>,
}

impl<E: RenderEngine> ViewportElement<E> {
    pub fn new(
        parent: Entity<Viewport<E>>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self {
        Self { parent }
    }
}

impl<E: RenderEngine> IntoElement for ViewportElement<E> {
    type Element = ViewportElement<E>;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<E: RenderEngine> Element for ViewportElement<E> {
    type RequestLayoutState = ();
    type PrepaintState = Option<Hitbox>;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.flex_grow = 0.0;
        style.flex_shrink = 1.;
        style.size = Size::full();

        let id = window.request_layout(style, [], cx);
        (id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if !self.parent.read(cx).visible() {
            return None;
        }

        // Create a hitbox to handle mouse events
        Some(window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal))
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        _: &mut App,
    ) {
        let bounds = hitbox.clone().map(|h| h.bounds).unwrap_or(bounds);
        window.with_content_mask(Some(ContentMask { bounds }), |_| {});
    }
}
