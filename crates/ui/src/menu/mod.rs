use gpui::App;

mod menu_item;
mod app_menu_bar;

pub mod context_menu;
pub mod popup_menu;

pub(crate) fn init(cx: &mut App) {
    app_menu_bar::init(cx);
    popup_menu::init(cx);
}
