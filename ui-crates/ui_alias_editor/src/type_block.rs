use gpui::{prelude::*, *};
use ui::{h_flex, v_flex, button::{Button, ButtonVariants}, ActiveTheme, StyledExt};
use ui_types_common::TypeAstNode;

/// Represents a visual block for a type node (like Scratch blocks)
#[derive(Clone, Debug)]
pub enum TypeBlock {
    /// Primitive type block (leaf node)
    Primitive {
        name: String,
        color: BlockColor,
    },
    /// Path type block (leaf node)
    Path {
        path: String,
        color: BlockColor,
    },
    /// Alias reference block (leaf node)
    AliasRef {
        alias: String,
        color: BlockColor,
    },
    /// Constructor block with slots for nested types
    Constructor {
        name: String,
        color: BlockColor,
        slots: Vec<Option<Box<TypeBlock>>>,
        expected_params: usize,
    },
    /// Tuple block with multiple element slots
    Tuple {
        color: BlockColor,
        elements: Vec<Option<Box<TypeBlock>>>,
    },
}

#[derive(Clone, Debug, Copy)]
pub enum BlockColor {
    Primitive,   // Blue
    Path,        // Green
    Alias,       // Purple
    Constructor, // Orange
    Tuple,       // Yellow
}

impl BlockColor {
    pub fn to_hsla(&self) -> Hsla {
        match self {
            BlockColor::Primitive => hsla(0.6, 0.7, 0.5, 1.0),   // Blue
            BlockColor::Path => hsla(0.35, 0.7, 0.5, 1.0),       // Green
            BlockColor::Alias => hsla(0.75, 0.7, 0.5, 1.0),      // Purple
            BlockColor::Constructor => hsla(0.08, 0.8, 0.6, 1.0), // Orange
            BlockColor::Tuple => hsla(0.15, 0.8, 0.6, 1.0),      // Yellow
        }
    }
}

impl TypeBlock {
    /// Create a primitive type block
    pub fn primitive(name: impl Into<String>) -> Self {
        TypeBlock::Primitive {
            name: name.into(),
            color: BlockColor::Primitive,
        }
    }

    /// Create a path type block
    pub fn path(path: impl Into<String>) -> Self {
        TypeBlock::Path {
            path: path.into(),
            color: BlockColor::Path,
        }
    }

    /// Create an alias reference block
    pub fn alias(alias: impl Into<String>) -> Self {
        TypeBlock::AliasRef {
            alias: alias.into(),
            color: BlockColor::Alias,
        }
    }

    /// Create a constructor block (Box, Arc, Vec, etc.)
    pub fn constructor(name: impl Into<String>, param_count: usize) -> Self {
        let name = name.into();
        let slots = vec![None; param_count];

        TypeBlock::Constructor {
            name,
            color: BlockColor::Constructor,
            slots,
            expected_params: param_count,
        }
    }

    /// Create a tuple block
    pub fn tuple(element_count: usize) -> Self {
        TypeBlock::Tuple {
            color: BlockColor::Tuple,
            elements: vec![None; element_count],
        }
    }

    /// Convert to AST node for code generation
    pub fn to_ast(&self) -> Option<TypeAstNode> {
        match self {
            TypeBlock::Primitive { name, .. } => Some(TypeAstNode::Primitive {
                name: name.clone(),
            }),
            TypeBlock::Path { path, .. } => Some(TypeAstNode::Path {
                path: path.clone(),
            }),
            TypeBlock::AliasRef { alias, .. } => Some(TypeAstNode::AliasRef {
                alias: alias.clone(),
            }),
            TypeBlock::Constructor { name, slots, .. } => {
                let params: Vec<_> = slots
                    .iter()
                    .filter_map(|slot| slot.as_ref().and_then(|b| b.to_ast()))
                    .collect();

                // Check if all slots are filled
                if params.len() != slots.len() {
                    return None;
                }

                Some(TypeAstNode::Constructor {
                    name: name.clone(),
                    params,
                    lifetimes: vec![],
                    const_generics: vec![],
                })
            }
            TypeBlock::Tuple { elements, .. } => {
                let element_nodes: Vec<_> = elements
                    .iter()
                    .filter_map(|el| el.as_ref().and_then(|b| b.to_ast()))
                    .collect();

                if element_nodes.len() != elements.len() {
                    return None;
                }

                Some(TypeAstNode::Tuple {
                    elements: element_nodes,
                })
            }
        }
    }

    /// Create from AST node
    pub fn from_ast(node: &TypeAstNode) -> Self {
        match node {
            TypeAstNode::Primitive { name } => TypeBlock::primitive(name.clone()),
            TypeAstNode::Path { path } => TypeBlock::path(path.clone()),
            TypeAstNode::AliasRef { alias } => TypeBlock::alias(alias.clone()),
            TypeAstNode::Constructor { name, params, .. } => {
                let mut block = TypeBlock::constructor(name.clone(), params.len());
                if let TypeBlock::Constructor { slots, .. } = &mut block {
                    for (i, param) in params.iter().enumerate() {
                        if i < slots.len() {
                            slots[i] = Some(Box::new(TypeBlock::from_ast(param)));
                        }
                    }
                }
                block
            }
            TypeAstNode::Tuple { elements } => {
                let mut block = TypeBlock::tuple(elements.len());
                if let TypeBlock::Tuple { elements: el, .. } = &mut block {
                    for (i, elem) in elements.iter().enumerate() {
                        if i < el.len() {
                            el[i] = Some(Box::new(TypeBlock::from_ast(elem)));
                        }
                    }
                }
                block
            }
            TypeAstNode::FnPointer { .. } => {
                // For now, represent as a placeholder
                TypeBlock::primitive("FnPtr")
            }
        }
    }

    /// Get display name for the block
    pub fn display_name(&self) -> String {
        match self {
            TypeBlock::Primitive { name, .. } => name.clone(),
            TypeBlock::Path { path, .. } => path.clone(),
            TypeBlock::AliasRef { alias, .. } => alias.clone(),
            TypeBlock::Constructor { name, .. } => name.clone(),
            TypeBlock::Tuple { .. } => "Tuple".to_string(),
        }
    }

    /// Get the color for this block
    pub fn color(&self) -> BlockColor {
        match self {
            TypeBlock::Primitive { color, .. }
            | TypeBlock::Path { color, .. }
            | TypeBlock::AliasRef { color, .. }
            | TypeBlock::Constructor { color, .. }
            | TypeBlock::Tuple { color, .. } => *color,
        }
    }

    /// Check if this is a container block (has slots)
    pub fn is_container(&self) -> bool {
        matches!(self, TypeBlock::Constructor { .. } | TypeBlock::Tuple { .. })
    }

    /// Get number of slots (0 for leaf nodes)
    pub fn slot_count(&self) -> usize {
        match self {
            TypeBlock::Constructor { slots, .. } => slots.len(),
            TypeBlock::Tuple { elements, .. } => elements.len(),
            _ => 0,
        }
    }

    /// Check if a slot is filled
    pub fn is_slot_filled(&self, index: usize) -> bool {
        match self {
            TypeBlock::Constructor { slots, .. } => {
                slots.get(index).and_then(|s| s.as_ref()).is_some()
            }
            TypeBlock::Tuple { elements, .. } => {
                elements.get(index).and_then(|e| e.as_ref()).is_some()
            }
            _ => false,
        }
    }
}

/// Visual representation of a type block
pub struct TypeBlockView {
    block: TypeBlock,
    id: ElementId,
}

impl TypeBlockView {
    pub fn new(block: TypeBlock, id: impl Into<ElementId>) -> Self {
        Self {
            block,
            id: id.into(),
        }
    }

    fn render_leaf_block(&self, cx: &App) -> impl IntoElement {
        let color = self.block.color().to_hsla();

        h_flex()
            .px_3()
            .py_2()
            .gap_2()
            .bg(color)
            .rounded(px(6.0))
            .border_1()
            .border_color(color.lighten(0.1))
            .shadow_sm()
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .text_color(gpui::white())
                    .child(self.block.display_name())
            )
    }

    fn render_container_block(&self, cx: &App) -> impl IntoElement {
        let color = self.block.color().to_hsla();

        match &self.block {
            TypeBlock::Constructor { name, slots, .. } => {
                v_flex()
                    .gap_1()
                    .child(
                        // Header
                        h_flex()
                            .px_3()
                            .py_2()
                            .bg(color)
                            .rounded_t(px(6.0))
                            .border_1()
                            .border_color(color.lighten(0.1))
                            .child(
                                div()
                                    .text_sm()
                                    .font_bold()
                                    .text_color(gpui::white())
                                    .child(format!("{}<", name))
                            )
                    )
                    .child(
                        // Slots
                        v_flex()
                            .px_3()
                            .py_2()
                            .gap_2()
                            .bg(color.opacity(0.2))
                            .border_x_1()
                            .border_color(color.lighten(0.1))
                            .children(slots.iter().enumerate().map(|(i, slot)| {
                                self.render_slot(i, slot, cx)
                            }))
                    )
                    .child(
                        // Footer
                        h_flex()
                            .px_3()
                            .py_1()
                            .bg(color)
                            .rounded_b(px(6.0))
                            .border_1()
                            .border_color(color.lighten(0.1))
                            .child(
                                div()
                                    .text_sm()
                                    .font_bold()
                                    .text_color(gpui::white())
                                    .child(">")
                            )
                    )
            }
            TypeBlock::Tuple { elements, .. } => {
                h_flex()
                    .gap_1()
                    .items_center()
                    .child(
                        div()
                            .px_2()
                            .py_2()
                            .bg(color)
                            .rounded_l(px(6.0))
                            .border_1()
                            .border_color(color.lighten(0.1))
                            .text_sm()
                            .font_bold()
                            .text_color(gpui::white())
                            .child("(")
                    )
                    .children(
                        elements.iter().enumerate().map(|(i, el)| {
                            h_flex()
                                .gap_1()
                                .child(self.render_slot(i, el, cx))
                                .when(i < elements.len() - 1, |this| {
                                    this.child(
                                        div()
                                            .text_sm()
                                            .text_color(color)
                                            .child(",")
                                    )
                                })
                        })
                    )
                    .child(
                        div()
                            .px_2()
                            .py_2()
                            .bg(color)
                            .rounded_r(px(6.0))
                            .border_1()
                            .border_color(color.lighten(0.1))
                            .text_sm()
                            .font_bold()
                            .text_color(gpui::white())
                            .child(")")
                    )
            }
            _ => self.render_leaf_block(cx),
        }
    }

    fn render_slot(&self, index: usize, slot: &Option<Box<TypeBlock>>, cx: &App) -> Div {
        if let Some(block) = slot {
            let nested_view = TypeBlockView::new(
                *block.clone(),
                format!("{}-slot-{}", self.id, index),
            );

            div().child(nested_view.render(cx))
        } else {
            // Empty slot - drop zone
            div()
                .px_4()
                .py_3()
                .bg(hsla(0.0, 0.0, 0.3, 0.3))
                .rounded(px(4.0))
                .border_2()
                .border_color(hsla(0.0, 0.0, 0.5, 0.5))
                .border_dashed()
                .text_xs()
                .text_color(hsla(0.0, 0.0, 0.6, 1.0))
                .child("Drop type here")
        }
    }
}

impl IntoElement for TypeBlockView {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        div().id(self.id.clone())
    }
}

impl RenderOnce for TypeBlockView {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        if self.block.is_container() {
            self.render_container_block(cx.app())
        } else {
            self.render_leaf_block(cx.app())
        }
    }
}
