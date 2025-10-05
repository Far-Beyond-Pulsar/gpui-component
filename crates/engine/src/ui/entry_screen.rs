use gpui::{prelude::*, *};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex, StyledExt, Icon, IconName, ActiveTheme as _, TitleBar, Placement, ContextModal
};


#[derive(Clone, Copy, PartialEq, Eq)]
enum EntryScreenView {
    Recent,
    Templates,
}

/// EntryScreen: Modern entry UI with sidebar navigation for recent projects and templates.
pub struct EntryScreen {
    view: EntryScreenView,
}

impl EntryScreen {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            view: EntryScreenView::Recent,
        }
    }

    //
}

impl EventEmitter<crate::ui::project_selector::ProjectSelected> for EntryScreen {}

impl Render for EntryScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        v_flex()
            .size_full()
            .bg(theme.background)
            // Title bar at the top
            .child(TitleBar::new())
            // Main content area
            .child(
                h_flex()
                    .size_full()
                    .child(
                        // Sidebar with icons and tooltips
                        v_flex()
                            .w(px(72.))
                            .h_full()
                            .bg(theme.sidebar)
                            .border_r_1()
                            .border_color(theme.border)
                            .gap_4()
                            .items_center()
                            .pt_8()
                            .child(
                                Button::new("recent-projects")
                                    .icon(IconName::FolderClosed)
                                    .label("")
                                    .tooltip("Recent Projects")
                                    .with_variant(gpui_component::button::ButtonVariant::Ghost)
                                    .on_click(cx.listener(|this: &mut Self, _, _, _| this.view = EntryScreenView::Recent))
                            )
                            .child(
                                Button::new("templates")
                                    .icon(IconName::Star)
                                    .label("")
                                    .tooltip("Templates")
                                    .with_variant(gpui_component::button::ButtonVariant::Ghost)
                                    .on_click(cx.listener(|this: &mut Self, _, _, _| this.view = EntryScreenView::Templates))
                            )
                            .child(
                                Button::new("settings")
                                    .icon(IconName::Settings)
                                    .label("")
                                    .tooltip("Settings")
                                    .with_variant(gpui_component::button::ButtonVariant::Ghost)
                            )
                    )
                    .child(
                        // Main area: grid of cards, progress, dividers, glass, etc.
                        v_flex()
                            .flex_1()
                            .h_full()
                            .scrollable(gpui_component::scroll::ScrollbarAxis::Vertical)
                            .bg(theme.background)
                            .gap_y_8()
                            .p_12()
                            .child(
                                match self.view {
                                    EntryScreenView::Recent => {
                                        // Dummy data for recent projects (replace with real data source)
                                        let recent_projects = vec![
                                            ("Project Alpha", "/path/to/alpha", true, Some("2025-10-01")),
                                            ("Project Beta", "/path/to/beta", false, Some("2025-09-20")),
                                            ("Project Gamma", "/path/to/gamma", true, None),
                                            ("Project Delta", "/path/to/delta", false, Some("2025-08-15")),
                                            ("Project Epsilon", "/path/to/epsilon", true, Some("2025-07-30")),
                                            ("Project Zeta", "/path/to/zeta", false, None),
                                            ("Project Eta", "/path/to/eta", true, Some("2025-06-10")),
                                            ("Project Theta", "/path/to/theta", false, Some("2025-05-05")),
                                            ("Project Iota", "/path/to/iota", true, None),
                                        ];
                                        
                                        // Build rows of 3 cards each
                                        let mut container = v_flex().gap_8();
                                        let mut row = h_flex().gap_8();
                                        let mut count = 0;
                                        
                                        for (proj_name, proj_path, is_git, last_opened) in recent_projects.into_iter() {
                                            let icon = if is_git { IconName::Star } else { IconName::Cube };
                                            let proj_name = proj_name.to_string();
                                            let proj_path = proj_path.to_string();
                                            let proj_last_opened = last_opened.map(|s| s.to_string());
                                            
                                            let card = v_flex()
                                                .h_full()
                                                .border_1()
                                                .border_color(theme.border)
                                                .rounded_lg()
                                                .p_6()
                                                .bg(theme.sidebar)
                                                .shadow_lg()
                                                .child(
                                                    h_flex()
                                                        .gap_2()
                                                        .items_center()
                                                        .child(Icon::new(icon).size(px(28.)).text_color(theme.primary))
                                                        .child(div().font_semibold().child(proj_name.clone()))
                                                        .child(
                                                            gpui_component::badge::Badge::new()
                                                                .count(1)
                                                                .color(theme.primary)
                                                        )
                                                )
                                                .child(div().text_color(theme.muted_foreground).text_sm().child(proj_path.clone()))
                                                .child(
                                                    Button::new(format!("details-{}", proj_path).as_str())
                                                        .label("Details")
                                                        .icon(IconName::ArrowRight)
                                                        .tooltip("Show project details")
                                                        .with_variant(gpui_component::button::ButtonVariant::Secondary)
                                                        .on_click(cx.listener({
                                                            let proj_name = proj_name.clone();
                                                            let proj_path = proj_path.clone();
                                                            let proj_last_opened = proj_last_opened.clone();
                                                            let is_git = is_git;
                                                            move |_screen, _, window, cx| {
                                                                let proj_name = proj_name.clone();
                                                                let proj_path = proj_path.clone();
                                                                let proj_last_opened = proj_last_opened.clone();
                                                                <Window as ContextModal>::open_drawer_at(window, Placement::Right, cx, move |drawer, _window, _cx| {
                                                                    let proj_path_for_button = proj_path.clone();
                                                                    drawer
                                                                        .title(format!("{} Details", proj_name))
                                                                        .child(
                                                                            v_flex()
                                                                                .gap_y_2()
                                                                                .child(div().child(format!("Path: {}", proj_path)))
                                                                                .child(div().child(format!("Last Opened: {}", proj_last_opened.as_deref().unwrap_or("Never"))))
                                                                                .child(div().child(format!("Git Project: {}", if is_git { "Yes" } else { "No" })))
                                                                        )
                                                                        .footer(
                                                                            Button::new("launch-btn")
                                                                                .label("Launch")
                                                                                .with_variant(gpui_component::button::ButtonVariant::Primary)
                                                                                .on_click(move |_, window, cx| {
                                                                                    let _ = proj_path_for_button; // Capture for future use
                                                                                    window.close_drawer(cx);
                                                                                    // Note: We can't emit events from here as we don't have view context
                                                                                    // The actual project loading would need to be handled differently
                                                                                })
                                                                        )
                                                                });
                                                            }
                                                        }))
                                                );
                                            
                                            row = row.child(card);
                                            count += 1;
                                            
                                            if count == 3 {
                                                container = container.child(row);
                                                row = h_flex().gap_8();
                                                count = 0;
                                            }
                                        }
                                        
                                        // Add remaining items if any
                                        if count > 0 {
                                            container = container.child(row);
                                        }
                                        
                                        container
                                    }
                                    EntryScreenView::Templates => {
                                        v_flex()
                                            .gap_y_6()
                                            .child(div().text_2xl().font_bold().child("Templates"))
                                            .child(gpui_component::divider::Divider::horizontal())
                                            .child({
                                                let templates = [
                                                    ("Blank Project", "A new empty project", IconName::Rocket, "create_blank"),
                                                    ("2D Platformer", "A 2D platformer template", IconName::Cube, "create_2d"),
                                                    ("3D First-Person", "A 3D FPS template", IconName::Star, "create_3d"),
                                                    ("Top-Down RPG", "A top-down RPG starter", IconName::Cube, "create_rpg"),
                                                    ("Visual Novel", "A visual novel template", IconName::Star, "create_vn"),
                                                    ("Puzzle Game", "A puzzle game base", IconName::Cube, "create_puzzle"),
                                                    ("Platformer Advanced", "Advanced 2D platformer", IconName::Star, "create_platformer_adv"),
                                                    ("Shooter", "A 2D shooter template", IconName::Cube, "create_shooter"),
                                                    ("Strategy", "A strategy game base", IconName::Star, "create_strategy"),
                                                    ("Card Game", "A card game template", IconName::Cube, "create_card"),
                                                    ("Roguelike", "A roguelike starter", IconName::Star, "create_roguelike"),
                                                    ("Metroidvania", "A metroidvania base", IconName::Cube, "create_metroidvania"),
                                                    ("Farming Sim", "A farming sim template", IconName::Star, "create_farming"),
                                                    ("Idle Game", "An idle/clicker game base", IconName::Cube, "create_idle"),
                                                    ("Racing", "A racing game template", IconName::Star, "create_racing"),
                                                    ("Fighting Game", "A fighting game base", IconName::Cube, "create_fighting"),
                                                    ("MOBA", "A MOBA starter", IconName::Star, "create_moba"),
                                                    ("MMO", "An MMO base", IconName::Cube, "create_mmo"),
                                                    ("Sandbox", "A sandbox game template", IconName::Star, "create_sandbox"),
                                                    ("Survival", "A survival game base", IconName::Cube, "create_survival"),
                                                    ("Adventure", "An adventure game template", IconName::Star, "create_adventure"),
                                                    ("Board Game", "A board game base", IconName::Cube, "create_board"),
                                                    ("Simulation", "A simulation game template", IconName::Star, "create_simulation"),
                                                    ("Educational", "An educational game base", IconName::Cube, "create_educational"),
                                                    ("Music Game", "A rhythm/music game template", IconName::Star, "create_music"),
                                                    ("Sports", "A sports game base", IconName::Cube, "create_sports"),
                                                    ("VR Game", "A VR game template", IconName::Star, "create_vr"),
                                                    ("AR Game", "An AR game base", IconName::Cube, "create_ar"),
                                                    ("Text Adventure", "A text adventure template", IconName::Star, "create_textadv"),
                                                    ("Pinball", "A pinball game base", IconName::Cube, "create_pinball"),
                                                    ("Maze", "A maze game template", IconName::Star, "create_maze"),
                                                    ("Stealth", "A stealth game base", IconName::Cube, "create_stealth"),
                                                    ("Horror", "A horror game template", IconName::Star, "create_horror"),
                                                    ("Platformer 3D", "A 3D platformer base", IconName::Cube, "create_platformer3d"),
                                                    ("FPS", "A first-person shooter template", IconName::Star, "create_fps"),
                                                    ("TPS", "A third-person shooter base", IconName::Cube, "create_tps"),
                                                    ("Battle Royale", "A battle royale template", IconName::Star, "create_battleroyale"),
                                                    ("City Builder", "A city builder base", IconName::Cube, "create_citybuilder"),
                                                    ("Tycoon", "A tycoon game template", IconName::Star, "create_tycoon"),
                                                    ("Match-3", "A match-3 puzzle base", IconName::Cube, "create_match3"),
                                                    ("Endless Runner", "An endless runner template", IconName::Star, "create_runner"),
                                                    ("Arcade", "An arcade game base", IconName::Cube, "create_arcade"),
                                                    ("Party Game", "A party game template", IconName::Star, "create_party"),
                                                    ("Trivia", "A trivia game base", IconName::Cube, "create_trivia"),
                                                    ("Quiz", "A quiz game template", IconName::Star, "create_quiz"),
                                                    ("Platformer Physics", "A physics platformer base", IconName::Cube, "create_platformer_phys"),
                                                    ("Sandbox Physics", "A physics sandbox template", IconName::Star, "create_sandbox_phys"),
                                                    ("RTS", "A real-time strategy base", IconName::Cube, "create_rts"),
                                                    ("Tower Defense", "A tower defense template", IconName::Star, "create_towerdef"),
                                                    ("Platformer Puzzle", "A puzzle platformer base", IconName::Cube, "create_platformer_puzzle"),
                                                    ("Platformer Shooter", "A shooter platformer template", IconName::Star, "create_platformer_shooter"),
                                                    ("Platformer RPG", "A platformer RPG base", IconName::Cube, "create_platformer_rpg"),
                                                    ("Platformer Stealth", "A stealth platformer template", IconName::Star, "create_platformer_stealth"),
                                                ];
                                                
                                                // Build rows of 3 cards each
                                                let mut container = v_flex().gap_8();
                                                let mut row = h_flex().gap_8();
                                                let mut count = 0;
                                                
                                                for (name_str, desc_str, icon_name, btn_id_str) in templates.into_iter() {
                                                    let icon = icon_name;
                                                    let name = name_str.to_string();
                                                    let desc = desc_str.to_string();
                                                    
                                                    let card = v_flex()
                                                        .h_full()
                                                        .border_1()
                                                        .border_color(theme.border)
                                                        .rounded_lg()
                                                        .p_6()
                                                        .bg(theme.sidebar)
                                                        .shadow_lg()
                                                        .child(
                                                            h_flex()
                                                                .gap_2()
                                                                .items_center()
                                                                .child(Icon::new(icon).size(px(28.)).text_color(theme.primary))
                                                                .child(div().font_semibold().child(name.clone()))
                                                                .child(
                                                                    gpui_component::badge::Badge::new()
                                                                        .dot()
                                                                        .color(theme.primary)
                                                                )
                                                        )
                                                        .child(div().text_color(theme.muted_foreground).text_sm().child(desc.clone()))
                                                        .child(
                                                            Button::new(SharedString::from(btn_id_str.to_string()))
                                                                .label("Details")
                                                                .icon(IconName::ArrowRight)
                                                                .tooltip("Show template details and config")
                                                                .with_variant(gpui_component::button::ButtonVariant::Secondary)
                                                                .on_click(cx.listener({
                                                                    let name = name.clone();
                                                                    let desc = desc.clone();
                                                                    move |_screen, _, window, cx| {
                                                                        let name = name.clone();
                                                                        let desc = desc.clone();
                                                                        <Window as ContextModal>::open_drawer_at(window, Placement::Right, cx, move |drawer, _window, _cx| {
                                                                            let name_for_button = name.clone();
                                                                            drawer
                                                                                .title(format!("{} Template", name))
                                                                                .child(
                                                                                    v_flex()
                                                                                        .gap_y_2()
                                                                                        .child(div().child(desc))
                                                                                )
                                                                                .footer(
                                                                                    Button::new("launch-template-btn")
                                                                                        .label("Launch")
                                                                                        .with_variant(gpui_component::button::ButtonVariant::Primary)
                                                                                        .on_click(move |_, window, cx| {
                                                                                            let _ = name_for_button; // Capture for future use
                                                                                            window.close_drawer(cx);
                                                                                            // Note: We can't emit events from here as we don't have view context
                                                                                            // The actual template loading would need to be handled differently
                                                                                        })
                                                                                )
                                                                        });
                                                                    }
                                                                }))
                                                        );
                                                    
                                                    row = row.child(card);
                                                    count += 1;
                                                    
                                                    if count == 3 {
                                                        container = container.child(row);
                                                        row = h_flex().gap_8();
                                                        count = 0;
                                                    }
                                                }
                                                
                                                // Add remaining items if any
                                                if count > 0 {
                                                    container = container.child(row);
                                                }
                                                
                                                container
                                            })
                                    }
                                }
                            )
                    )
            )
    }
}
