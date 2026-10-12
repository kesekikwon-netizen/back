//! The 「drawing book」 panel (design 2.1 section 3).
//!
//! The one panel that is new for archaeology: plans, sections, finds and plates of
//! one excavation in one list, each with its scale, so the whole site is in view
//! from the left edge. Under the list it states what the open drawing is — its
//! scale, its coordinate system, its origin, its height datum — and how well the
//! scanned underlay is fitted.
//!
//! The list itself is J006 (a drawing's real scale and TM coordinates) and J004
//! (the 「도면」 folder beside the excavation file). J003 gives the panel its place,
//! its name, its four groups and its shape, and says 「준비 중」 where the numbers
//! will go.

use egui::{Stroke, Ui, vec2};
use vectorcraft_ui_egui::{VectorcraftApp, icons};

use super::labels;
use super::tokens;

/// The four groups of the design's list, with the heading it gives each.
const GROUPS: &[labels::Label] = &[labels::BOOK_PLAN, labels::BOOK_SECTION, labels::BOOK_FIND, labels::BOOK_PLATE];

pub fn show(app: &mut VectorcraftApp, ui: &mut Ui) {
    let lang = app.ui_language().code();
    let frame = egui::Frame::NONE
        .fill(tokens::SURFACE)
        .inner_margin(egui::Margin { left: 0, right: 0, top: 0, bottom: 0 })
        .stroke(Stroke::new(1.0, tokens::LINE));
    egui::Panel::left("trowel_drawing_book").resizable(false).exact_size(tokens::BOOK_W).frame(frame).show(ui, |ui| {
        header(app, ui, lang);
        tokens::rule(ui);
        list(ui, lang);
        ui.add_space(6.0);
        info(ui, lang);
        ui.add_space(6.0);
        footer(app, ui, lang);
    });
}

/// `광령리1 ＋` — the excavation's name, and the button that starts another one.
fn header(app: &mut VectorcraftApp, ui: &mut Ui, lang: &str) {
    ui.horizontal(|ui| {
        icons::paint(
            ui,
            "layers",
            egui::Rect::from_min_max(
                egui::pos2(ui.cursor().min.x, ui.cursor().min.y + 3.0),
                egui::pos2(ui.cursor().min.x + 14.0, ui.cursor().min.y + 17.0),
            ),
            tokens::MUTED,
        );
        ui.add_space(4.0);
        let name = active_title(app);
        let galley = ui.painter().layout_no_wrap(name, egui::FontId::proportional(12.5), tokens::INK);
        ui.painter().galley(ui.cursor().min, galley, tokens::INK);
        ui.add_space(6.0);
        // 「＋」 — a new drawing in this excavation (J004).
        let (r, resp) = ui.allocate_exact_size(vec2(18.0, 18.0), egui::Sense::click());
        ui.painter().rect_filled(r, 4.0, if resp.hovered() { tokens::PRESS } else { tokens::WASH });
        icons::paint(ui, "plus", egui::Rect::from_center_size(r.center(), vec2(12.0, 12.0)), tokens::MUTED);
        resp.on_hover_text(format!("{} · J004", labels::NOT_READY.text(lang)));
    });
    ui.add_space(4.0);
}

/// The excavation's name: the drawing that is open, or the placeholder the design
/// shows with nothing open.
fn active_title(app: &VectorcraftApp) -> String {
    match app.session.active() {
        Some(d) => d.title(),
        None => "광령리1".to_string(),
    }
}

/// The four groups. Their drawings and scales arrive with J006.
fn list(ui: &mut Ui, lang: &str) {
    egui::ScrollArea::vertical().id_salt("trowel_drawing_book_list").auto_shrink([false, false]).max_height(360.0).show(ui, |ui| {
        for (i, g) in GROUPS.iter().enumerate() {
            if i > 0 {
                ui.add_space(6.0);
            }
            ui.label(egui::RichText::new(g.text(lang)).strong().size(11.0).color(tokens::INK2));
            tokens::pending(ui, &format!("{} · J006", g.text(lang)));
        }
    });
}

/// What the open drawing is: its scale, its coordinate system, its origin, its height
/// datum and how well the underlay is fitted. J006 fills in the numbers.
fn info(ui: &mut Ui, lang: &str) {
    tokens::cell(tokens::WASH).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(labels::BOOK_PLAN.text(lang)).size(11.0).color(tokens::MUTED));
                ui.label(egui::RichText::new("· 축척 1:20 · EPSG:5186 중부원점").size(11.0).color(tokens::PENDING));
            });
            ui.label(egui::RichText::new(format!("{} X —  Y —", labels::BOOK_ORIGIN.text(lang))).size(11.0).color(tokens::PENDING));
            ui.label(
                egui::RichText::new(format!("{} — · {}", labels::BOOK_HEIGHT.text(lang), labels::BOOK_SEALE.text(lang)))
                    .size(11.0)
                    .color(tokens::PENDING),
            );
            ui.label(egui::RichText::new(format!("{} — · 0.012 m", "스캔")).size(11.0).color(tokens::PENDING));
        });
    });
}

/// The 「홈」 row and the unsaved mark the design puts at the foot of the list.
pub fn footer(app: &mut VectorcraftApp, ui: &mut Ui, lang: &str) {
    ui.horizontal(|ui| {
        let (r, resp) = ui.allocate_exact_size(vec2(18.0, 18.0), egui::Sense::click());
        ui.painter().rect_filled(r, 4.0, if resp.hovered() { tokens::PRESS } else { tokens::WASH });
        icons::paint(ui, "house", egui::Rect::from_center_size(r.center(), vec2(12.0, 12.0)), tokens::MUTED);
        if resp.clicked() {
            // VectorCraft's own Home view, reused as it is.
            let _ = app.run("app.home", serde_json::json!({}));
        }
        resp.on_hover_text(labels::BOOK_HOME.text(lang));
        ui.add_space(6.0);
        if let Some(d) = app.session.active() {
            let mark = if d.is_dirty() { " *" } else { "" };
            ui.label(egui::RichText::new(format!("{}{mark}", d.title())).size(11.0).color(tokens::INK2));
        } else {
            ui.label(egui::RichText::new(labels::NOT_READY.text(lang)).size(11.0).color(tokens::PENDING));
        }
    });
}

/// The name the window carries: the drawing, a `*` when it has unsaved changes, then
/// the product — `광령리1 * — Trowel` in section 3.
pub fn titled(app: &VectorcraftApp, product: &str) -> String {
    let drawing = app.session.active().map(|d| (d.title(), d.is_dirty()));
    match drawing {
        Some((name, dirty)) => super::menubar::window_title_shape(Some((name.as_str(), dirty)), product),
        None => super::menubar::window_title_shape(None, product),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    /// The design's four groups, in its order (section 3: 평면 · 단면 · 유물 · 도판).
    #[test]
    fn the_book_lists_the_four_kinds() {
        let ko: Vec<&str> = GROUPS.iter().map(|g| g.ko).collect();
        assert_eq!(ko, ["평면", "단면", "유물", "도판"]);
    }

    #[test]
    fn the_book_is_wider_than_a_tool_column() {
        const { assert!(tokens::BOOK_W >= 200.0) };
    }
}
