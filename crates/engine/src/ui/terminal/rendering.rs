//! Professional terminal rendering with batched text runs
//! Based on Zed's terminal_element.rs implementation

use super::terminal_core::TerminalBounds;
use alacritty_terminal::{
    index::Point as AlacPoint,
    term::cell::{Cell, Flags},
    vte::ansi::{Color as AnsiColor, NamedColor},
};
use gpui::*;

/// A batched text run combining multiple adjacent cells with the same style
#[derive(Debug)]
pub struct BatchedTextRun {
    pub start_point: AlacPoint,
    pub text: String,
    pub cell_count: usize,
    pub style: TextRun,
    pub font_size: AbsoluteLength,
}

impl BatchedTextRun {
    fn new_from_char(
        start_point: AlacPoint,
        c: char,
        style: TextRun,
        font_size: AbsoluteLength,
    ) -> Self {
        let mut text = String::with_capacity(100);
        text.push(c);
        BatchedTextRun {
            start_point,
            text,
            cell_count: 1,
            style,
            font_size,
        }
    }

    fn can_append(&self, other_style: &TextRun) -> bool {
        self.style.font == other_style.font
            && self.style.color == other_style.color
            && self.style.background_color == other_style.background_color
            && self.style.underline == other_style.underline
            && self.style.strikethrough == other_style.strikethrough
    }

    fn append_char(&mut self, c: char) {
        self.text.push(c);
        self.cell_count += 1;
    }

    pub fn paint(
        &self,
        origin: Point<Pixels>,
        dimensions: &TerminalBounds,
        window: &mut Window,
        cx: &mut App,
    ) {
        let pos = Point::new(
            origin.x + self.start_point.column.0 as f32 * dimensions.cell_width,
            origin.y + self.start_point.line.0 as f32 * dimensions.line_height,
        );

        let shaped = window
            .text_system()
            .shape_line(
                self.text.clone().into(),
                self.font_size.to_pixels(window.rem_size()),
                std::slice::from_ref(&self.style),
                Some(dimensions.cell_width),  // Use cell width like Zed for monospace alignment
            );
        
        let _ = shaped.paint(pos, dimensions.line_height, window, cx);
    }
}

/// Background rectangle for terminal cells
#[derive(Clone, Debug, Default)]
pub struct LayoutRect {
    point: AlacPoint,
    num_of_cells: usize,
    color: Hsla,
}

impl LayoutRect {
    fn new(point: AlacPoint, num_of_cells: usize, color: Hsla) -> LayoutRect {
        LayoutRect {
            point,
            num_of_cells,
            color,
        }
    }

    pub fn paint(&self, origin: Point<Pixels>, dimensions: &TerminalBounds, window: &mut Window) {
        let position = point(
            (origin.x + self.point.column.0 as f32 * dimensions.cell_width).floor(),
            origin.y + self.point.line.0 as f32 * dimensions.line_height,
        );
        let size = point(
            (dimensions.cell_width * self.num_of_cells as f32).ceil(),
            dimensions.line_height,
        )
        .into();

        window.paint_quad(fill(Bounds::new(position, size), self.color));
    }
}

/// Convert ANSI color to GPUI Hsla
fn convert_color(color: &AnsiColor, theme: &gpui_component::Theme) -> Hsla {
    match color {
        AnsiColor::Named(named) => named_color(*named, theme),
        AnsiColor::Spec(rgb) => {
            let r = rgb.r as f32 / 255.0;
            let g = rgb.g as f32 / 255.0;
            let b = rgb.b as f32 / 255.0;
            // Simple RGB to grayscale for now
            let gray = (r + g + b) / 3.0;
            hsla(0.0, 0.0, gray, 1.0)
        }
        AnsiColor::Indexed(idx) => indexed_color(*idx),
    }
}

fn named_color(color: NamedColor, theme: &gpui_component::Theme) -> Hsla {
    
    match color {
        NamedColor::Black => hsla(0.0, 0.0, 0.0, 1.0),
        NamedColor::Red => hsla(0.0, 1.0, 0.4, 1.0),
        NamedColor::Green => hsla(120.0 / 360.0, 0.8, 0.4, 1.0),
        NamedColor::Yellow => hsla(60.0 / 360.0, 1.0, 0.5, 1.0),
        NamedColor::Blue => hsla(240.0 / 360.0, 1.0, 0.5, 1.0),
        NamedColor::Magenta => hsla(300.0 / 360.0, 1.0, 0.5, 1.0),
        NamedColor::Cyan => hsla(180.0 / 360.0, 1.0, 0.5, 1.0),
        NamedColor::White => hsla(0.0, 0.0, 0.9, 1.0),
        NamedColor::BrightBlack => hsla(0.0, 0.0, 0.4, 1.0),
        NamedColor::BrightRed => hsla(0.0, 1.0, 0.6, 1.0),
        NamedColor::BrightGreen => hsla(120.0 / 360.0, 0.8, 0.6, 1.0),
        NamedColor::BrightYellow => hsla(60.0 / 360.0, 1.0, 0.7, 1.0),
        NamedColor::BrightBlue => hsla(240.0 / 360.0, 1.0, 0.7, 1.0),
        NamedColor::BrightMagenta => hsla(300.0 / 360.0, 1.0, 0.7, 1.0),
        NamedColor::BrightCyan => hsla(180.0 / 360.0, 1.0, 0.7, 1.0),
        NamedColor::BrightWhite => hsla(0.0, 0.0, 1.0, 1.0),
        NamedColor::Foreground => hsla(0.0, 0.0, 0.9, 1.0),
        NamedColor::Background => hsla(0.0, 0.0, 0.05, 1.0),
        _ => hsla(0.0, 0.0, 0.9, 1.0),
    }
}

fn indexed_color(idx: u8) -> Hsla {
    if idx < 16 {
        // Use basic colors
        let colors = [
            hsla(0.0, 0.0, 0.0, 1.0),      // Black
            hsla(0.0, 1.0, 0.4, 1.0),      // Red
            hsla(120.0 / 360.0, 0.8, 0.4, 1.0), // Green
            hsla(60.0 / 360.0, 1.0, 0.5, 1.0),  // Yellow
            hsla(240.0 / 360.0, 1.0, 0.5, 1.0), // Blue
            hsla(300.0 / 360.0, 1.0, 0.5, 1.0), // Magenta
            hsla(180.0 / 360.0, 1.0, 0.5, 1.0), // Cyan
            hsla(0.0, 0.0, 0.9, 1.0),      // White
            hsla(0.0, 0.0, 0.4, 1.0),      // Bright Black
            hsla(0.0, 1.0, 0.6, 1.0),      // Bright Red
            hsla(120.0 / 360.0, 0.8, 0.6, 1.0), // Bright Green
            hsla(60.0 / 360.0, 1.0, 0.7, 1.0),  // Bright Yellow
            hsla(240.0 / 360.0, 1.0, 0.7, 1.0), // Bright Blue
            hsla(300.0 / 360.0, 1.0, 0.7, 1.0), // Bright Magenta
            hsla(180.0 / 360.0, 1.0, 0.7, 1.0), // Bright Cyan
            hsla(0.0, 0.0, 1.0, 1.0),      // Bright White
        ];
        colors[idx as usize]
    } else if idx >= 232 {
        // Grayscale
        let gray = ((idx - 232) as f32) / 23.0;
        hsla(0.0, 0.0, gray, 1.0)
    } else {
        // 6x6x6 color cube - approximate to grayscale
        let idx = idx - 16;
        let r = ((idx / 36) as f32) / 5.0;
        let g = (((idx % 36) / 6) as f32) / 5.0;
        let b = ((idx % 6) as f32) / 5.0;
        let gray = (r + g + b) / 3.0;
        hsla(0.0, 0.0, gray, 1.0)
    }
}

/// Cell style conversion from Alacritty to GPUI
fn cell_style(
    cell: &Cell,
    fg: AnsiColor,
    bg: AnsiColor,
    theme: &gpui_component::Theme,
    font: &Font,
) -> TextRun {
    let flags = cell.flags;
    let mut fg_color = convert_color(&fg, theme);
    let bg_color = convert_color(&bg, theme);

    // Handle DIM flag
    if flags.contains(Flags::DIM) {
        fg_color.a *= 0.7;
    }

    let underline = (flags.contains(Flags::UNDERLINE) || flags.contains(Flags::DOUBLE_UNDERLINE))
        .then(|| UnderlineStyle {
            color: Some(fg_color),
            thickness: px(1.0),
            wavy: flags.contains(Flags::UNDERCURL),
        });

    let strikethrough = flags.contains(Flags::STRIKEOUT).then(|| StrikethroughStyle {
        color: Some(fg_color),
        thickness: px(1.0),
    });

    let weight = if flags.contains(Flags::BOLD) {
        FontWeight::BOLD
    } else {
        font.weight
    };

    let style = if flags.contains(Flags::ITALIC) {
        FontStyle::Italic
    } else {
        FontStyle::Normal
    };

    TextRun {
        len: cell.c.len_utf8(),
        color: fg_color,
        background_color: None,
        font: Font {
            family: font.family.clone(),
            features: font.features.clone(),
            weight,
            style,
            fallbacks: font.fallbacks.clone(),
        },
        underline,
        strikethrough,
    }
}

/// Check if cell is blank
fn is_blank(cell: &Cell) -> bool {
    cell.c == ' ' && !cell.flags.contains(Flags::INVERSE)
}

/// Layout terminal grid into batched text runs and background rects (Zed approach)
pub fn layout_grid(
    grid_iter: impl Iterator<Item = crate::ui::terminal::terminal_core::IndexedCell>,
    display_offset: usize,
    text_style: &TextStyle,
    font: &Font,
    theme: &gpui_component::Theme,
) -> (Vec<LayoutRect>, Vec<BatchedTextRun>) {
    use itertools::Itertools;
    
    let font_size = text_style.font_size;

    let mut batched_runs = Vec::new();
    let mut rects = Vec::new();
    let mut current_batch: Option<BatchedTextRun> = None;
    let mut current_rect: Option<LayoutRect> = None;

    // Group cells by line, then enumerate to get viewport line numbers
    // This matches Zed's approach: regardless of what line the cells claim to be on,
    // we render them sequentially starting from line 0 in the viewport.
    // This is crucial for proper scrolling: when display_offset > 0 (scrolled up),
    // Alacritty's display_iter returns cells from scrollback, but we need to render
    // them at viewport positions 0, 1, 2, ... not at their original line numbers.
    let linegroups = grid_iter.chunk_by(|cell| cell.point.line);
    
    for (viewport_line, (_, line)) in linegroups.into_iter().enumerate() {
        let viewport_line = viewport_line as i32;
        
        // Flush batches when starting a new line
        if let Some(batch) = current_batch.take() {
            batched_runs.push(batch);
        }
        if let Some(rect) = current_rect.take() {
            rects.push(rect);
        }

        for indexed_cell in line {
            let cell = &indexed_cell.cell;
        let mut fg = cell.fg;
        let mut bg = cell.bg;
        if cell.flags.contains(Flags::INVERSE) {
            std::mem::swap(&mut fg, &mut bg);
        }

        // Handle background color
        if !matches!(bg, AnsiColor::Named(NamedColor::Background)) {
            let color = convert_color(&bg, theme);
            let col = indexed_cell.point.column.0;

            if let Some(ref mut rect) = current_rect {
                if rect.color == color
                    && rect.point.line.0 == viewport_line
                    && (rect.point.column.0 as usize + rect.num_of_cells) == col as usize
                {
                    rect.num_of_cells += 1;
                } else {
                    rects.push(current_rect.take().unwrap());
                    current_rect = Some(LayoutRect::new(
                        AlacPoint::new(alacritty_terminal::index::Line(viewport_line), alacritty_terminal::index::Column(col)),
                        1,
                        color,
                    ));
                }
            } else {
                current_rect = Some(LayoutRect::new(
                    AlacPoint::new(alacritty_terminal::index::Line(viewport_line), alacritty_terminal::index::Column(col)),
                    1,
                    color,
                ));
            }
        }

        // Skip wide character spacers
        if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
            continue;
        }

        // Layout current cell text
        if !is_blank(cell) {
            let cell_style_run = cell_style(cell, fg, bg, theme, font);
            let cell_point = AlacPoint::new(
                alacritty_terminal::index::Line(viewport_line),
                indexed_cell.point.column
            );

            // Try to batch with existing run
            if let Some(ref mut batch) = current_batch {
                if batch.can_append(&cell_style_run)
                    && batch.start_point.line.0 == cell_point.line.0
                    && (batch.start_point.column.0 as usize + batch.cell_count) == cell_point.column.0 as usize
                {
                    batch.append_char(cell.c);
                } else {
                    // Flush current batch and start new one
                    batched_runs.push(current_batch.take().unwrap());
                    current_batch = Some(BatchedTextRun::new_from_char(
                        cell_point,
                        cell.c,
                        cell_style_run,
                        font_size,
                    ));
                }
            } else {
                // Start new batch
                current_batch = Some(BatchedTextRun::new_from_char(
                    cell_point,
                    cell.c,
                    cell_style_run,
                    font_size,
                ));
            }
        }
    }
    }

    // Flush any remaining batches
    if let Some(batch) = current_batch {
        batched_runs.push(batch);
    }
    if let Some(rect) = current_rect {
        rects.push(rect);
    }

    (rects, batched_runs)
}
