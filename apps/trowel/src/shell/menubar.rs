//! The menu bar (design 2.1 section 3, the row under the caption).
//!
//! VectorCraft's own app bar carries its brand mark, a Home button and a Discord
//! button; the design drops all three and puts the menus at the left. So the bar
//! is drawn here and the menu model itself (`vectorcraft_ui_egui::menus`) is
//! reused unchanged — same command ids, same shortcuts, same labels, so the
//! control channel, MCP and the keyboard keep working. The one addition is the
//! `도면` (Drawing) menu, whose items run as Trowel arrives (J003–J019); until
//! then they are the dimmed 「준비 중」.

use egui::{Align, CornerRadius, Layout, Rect, Stroke, Ui, UiBuilder, vec2};
use vectorcraft_ui_egui::{VectorcraftApp, icons, titlebar, widgets};

use super::labels;
use super::tokens;

pub fn show(app: &mut VectorcraftApp, ui: &mut Ui) {
    let custom = app.custom_titlebar;
    let frame =
        egui::Frame::NONE.fill(tokens::SURFACE).inner_margin(egui::Margin { left: 10, right: if custom { 0 } else { 12 }, top: 0, bottom: 0 });
    let bar = egui::Panel::top("trowel_menus").exact_size(tokens::CAPTION).frame(frame.stroke(Stroke::new(1.0, tokens::LINE))).show(ui, |ui| {
        if custom {
            titlebar::drag_area(ui, ui.max_rect());
        }
        ui.horizontal_centered(|ui| {
            // macOS keeps its menu bar at the top of the screen, so the in-window
            // row shows the product name there instead of the menus.
            if app.services.native_menu.is_some() {
                let full = ui.max_rect();
                ui.painter().text(full.center(), egui::Align2::CENTER_CENTER, "Trowel", egui::FontId::proportional(13.0), tokens::INK);
            } else {
                super::menus::show(app, ui);
            }

            // The command palette's field, at the right end before the caption
            // buttons (section 3: 「⌕ 명령 찾기」).
            let full = ui.max_rect();
            let right_edge = if custom { full.right() - titlebar::WIDTH - 10.0 } else { full.right() };
            let lang = app.ui_language().code();
            let rect = Rect::from_min_max(egui::pos2(full.right() - 150.0, full.top() + 5.0), egui::pos2(right_edge, full.bottom() - 5.0));
            let mut rui = ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::right_to_left(Align::Center)));
            let hint = if app.ui.palette_open { labels::T_FIND_COMMANDS.text(lang) } else { "⌕" };
            if widgets::icon_button(&mut rui, "search", labels::T_FIND_COMMANDS.text(lang), app.ui.palette_open, 22.0).clicked() {
                app.ui.palette_open = !app.ui.palette_open;
                app.ui.palette_query.clear();
            }
            rui.label(egui::RichText::new(hint).size(11.5).color(tokens::FAINT));
        });
    });
    if custom {
        titlebar::caption_buttons(app, ui, bar.response.rect);
    }
}

/// The window title as the interface writes it: the drawing's name, a `*` when it
/// has unsaved changes, then the product — `광령리1 * - Trowel` in section 3.
pub fn window_title_shape(drawing: Option<(&str, bool)>, product: &str) -> String {
    match drawing {
        Some((name, dirty)) => format!("{name}{} — {product}", if dirty { " *" } else { "" }),
        None => product.to_string(),
    }
}

/// A row of tiles: an icon above its label, as the mock-up's ribbon draws them.
pub fn tile(ui: &mut Ui, icon: &str, label: &str, held: bool, dim: bool) -> egui::Response {
    let fill = if held { tokens::tile_color(tokens::TileRole::Held) } else { tokens::tile_color(tokens::TileRole::Idle) };
    let label_color = if dim {
        tokens::PENDING
    } else if held {
        tokens::on(fill)
    } else {
        tokens::INK2
    };
    let icon_tint = if dim {
        tokens::PENDING
    } else if held {
        tokens::on(fill)
    } else {
        tokens::MUTED
    };
    let height = tokens::RIBBON_TILE;
    let (rect, resp) = ui.allocate_exact_size(vec2(56.0, height), egui::Sense::click());
    let bg = if dim {
        tokens::WASH
    } else if resp.hovered() {
        tokens::PRESS
    } else {
        fill
    };
    ui.painter().rect_filled(rect, CornerRadius::same(tokens::RADIUS_SMALL as u8), bg);
    let g = rect.shrink(4.0);
    // Glyph above, name below: both inside the tile, the name bottom-aligned so a long one is
    // cut at the tile's edge rather than under it (section 3).
    icons::paint(ui, icon, Rect::from_min_max(egui::pos2(g.left(), g.top() + 2.0), egui::pos2(g.left() + 15.0, g.top() + 17.0)), icon_tint);
    ui.painter().with_clip_rect(g).galley(
        egui::pos2(g.left(), g.bottom() - 12.0),
        ui.painter().layout_no_wrap(label.into(), egui::FontId::proportional(10.5), label_color),
        label_color,
    );
    resp
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::window_title_shape;

    /// The window title is `<drawing> [*] — Trowel`, and just `Trowel` with no
    /// drawing open (design 2.1 section 3: 「광령리1 * - Trowel」).
    #[test]
    fn the_window_title_is_the_drawing_then_the_product() {
        assert_eq!(window_title_shape(None, "Trowel"), "Trowel");
        assert_eq!(window_title_shape(Some(("광령리1", false)), "Trowel"), "광령리1 — Trowel");
        assert_eq!(window_title_shape(Some(("광령리1", true)), "Trowel"), "광령리1 * — Trowel");
    }
}
