//! The start screen (design 2.1 §3, J004).
//!
//! VectorCraft's own Home view is private to `ui-egui` and it is VectorCraft's: new-file presets,
//! Open, recent files — no archaeology in it. Trowel does not modify `ui-egui` (the whole rule is
//! that the public API is enough), so the start screen is drawn here, in the place the canvas
//! would have been, whenever no document is open.
//!
//! The three blocks are the design's own (§3):
//!
//! 1. **새 도면** — a new plan, section, find or plate drawing.
//! 2. **저장소 열기** — open a drawing, a recent one, or a Strata survey file.
//! 3. **새 그리기** — trace a survey: the dashed redraw of a scan and an orthophoto.

use egui::{Stroke, StrokeKind, Ui, vec2};
use serde_json::json;
use vectorcraft_ui_egui::VectorcraftApp;

use super::labels;
use super::tokens;

/// The three blocks, in the order the design lays them out.
#[allow(dead_code)] // the design's order, stated once and checked by the test below
pub const BLOCKS: [&str; 3] = ["new drawing", "open store", "new tracing"];

/// The four kinds of drawing the design offers (§3: 평면·단면·유물·도판), in that order.
pub const KINDS: [&str; 4] = ["plan", "section", "find", "plate"];

/// The artboard a new drawing starts on, in points (a plate is the A2 the design measures
/// against: 420 × 594 mm at 72 pt/inch).
fn artboard(kind: &str) -> (f64, f64) {
    match kind {
        // 도판 — the plate itself, A2 portrait in millimetres.
        "plate" => (420.0 / 25.4 * 72.0, 594.0 / 25.4 * 72.0),
        // The others draw at 1:1 inside the plate and start on a working sheet.
        _ => (1190.0, 1684.0),
    }
}

/// Draw the start screen when nothing is open. Returns whether it drew.
pub fn show_if_empty(app: &mut VectorcraftApp, ui: &mut Ui, lang: &str) -> bool {
    if !app.session.documents().is_empty() {
        return false;
    }
    show(app, ui, lang);
    true
}

/// Trowel's start screen.
pub fn show(app: &mut VectorcraftApp, ui: &mut Ui, lang: &str) {
    let frame = egui::Frame::NONE.fill(tokens::SURFACE);
    frame.show(ui, |ui| {
        let pad = 28.0;
        ui.add_space(pad * 2.0);
        ui.horizontal(|ui| {
            ui.add_space(pad);
            tokens::text(ui, labels::PRODUCT_NAME.text(lang), 26.0, tokens::INK);
        });
        ui.horizontal(|ui| {
            ui.add_space(pad);
            ui.label(egui::RichText::new(labels::START_LEAD.text(lang)).size(12.0).color(tokens::MUTED));
        });
        ui.add_space(pad);

        // 1 · 새 도면 — the four kinds.
        block(ui, labels::START_NEW_DRAWING.text(lang));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 0.0);
            for kind in KINDS {
                kind_tile(app, ui, lang, kind);
            }
        });

        // 2 · 저장소 열기.
        block(ui, labels::START_OPEN_STORE.text(lang));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 0.0);
            action(app, ui, lang, labels::T_OPEN, "file.open", json!({}));
            action(app, ui, lang, labels::START_RECENT, "file.recentFiles", json!({}));
            // Strata survey files are a GPKG layer set; that is J019's dependency decision, so
            // the button says so rather than opening something else.
            pending(app, ui, lang, labels::START_STRATA, "J019");
        });

        // 3 · 새 그리기.
        block(ui, labels::START_NEW_TRACING.text(lang));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 0.0);
            // The scan and the orthophoto need a site and a survey file, which J019 opens.
            pending(app, ui, lang, labels::START_PICK_SITE, "J019");
            pending(app, ui, lang, labels::START_LAST_SITE, "J019");
        });
    });
}

/// A block's name, set apart by a rule rather than a box (Strata's rule).
fn block(ui: &mut Ui, name: &str) {
    ui.add_space(10.0);
    tokens::rule(ui);
    ui.add_space(6.0);
    ui.label(egui::RichText::new(name).strong().size(13.0).color(tokens::INK));
}

/// One drawing kind: 평면 · 단면 · 유물 · 도판. Clicking it makes that drawing.
fn kind_tile(app: &mut VectorcraftApp, ui: &mut Ui, lang: &str, kind: &str) {
    let label = kind_label(kind, lang);
    let (w, h) = artboard(kind);
    let size = vec2(150.0, 74.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    ui.painter().rect_filled(rect, tokens::RADIUS_SMALL, tokens::DESK);
    ui.painter().rect_stroke(rect, tokens::RADIUS_SMALL, Stroke::new(1.0, tokens::LINE), StrokeKind::Middle);
    ui.painter().text(
        egui::pos2(rect.left() + 12.0, rect.top() + 14.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::proportional(13.0),
        tokens::INK,
    );
    // The sheet's proportion, drawn small — a plate is tall, a drawing sheet is not.
    let sheet_w = 26.0_f32.min(size.x - 24.0);
    let sheet_h = (sheet_w * h as f32 / w as f32).min(size.y - 44.0).max(8.0);
    let sheet = egui::Rect::from_min_size(egui::pos2(rect.left() + 12.0, rect.top() + 40.0), vec2(sheet_w, sheet_h));
    ui.painter().rect_stroke(sheet, 2.0, Stroke::new(1.0, tokens::EDGE), StrokeKind::Middle);
    let hint = format!("{} · {w:.0} × {h:.0} pt", label);
    let clicked = resp.on_hover_text(hint).clicked();
    if clicked {
        note(app.run("file.new", json!({ "width": w, "height": h })).err().unwrap_or_default());
    }
}

/// A command the start screen offers, as a quiet text button.
fn action(app: &mut VectorcraftApp, ui: &mut Ui, lang: &str, label: labels::Label, id: &str, params: serde_json::Value) {
    let text = egui::RichText::new(label.text(lang)).size(12.0).color(tokens::INK2);
    if ui.add(egui::Button::new(text).fill(tokens::DESK).stroke(Stroke::new(1.0, tokens::LINE))).clicked() {
        note(app.run(id, params).err().unwrap_or_default());
    }
}

/// Something the design asks for whose task has not reached it: named, not hidden.
fn pending(_app: &mut VectorcraftApp, ui: &mut Ui, lang: &str, label: labels::Label, task: &str) {
    let text = egui::RichText::new(format!("{} · {task}", label.text(lang))).size(12.0).color(tokens::PENDING);
    ui.add_enabled(false, egui::Button::new(text).fill(tokens::SURFACE).stroke(Stroke::new(1.0, tokens::LINE)));
}

/// What the screen could not do, kept until something changes.
fn note(msg: String) {
    if !msg.is_empty() {
        log::warn!("trowel start screen: {msg}");
    }
}

fn kind_label(kind: &str, lang: &str) -> &'static str {
    match kind {
        "section" => labels::T_SECTION.text(lang),
        "find" => labels::T_FIND.text(lang),
        "plate" => labels::T_PLATE.text(lang),
        _ => labels::T_PLAN.text(lang),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    /// The three blocks, in the design's order (§3).
    #[test]
    fn the_three_blocks_are_in_the_designed_order() {
        assert_eq!(BLOCKS, ["new drawing", "open store", "new tracing"]);
    }

    /// The four drawing kinds the design offers, in that order (§3).
    #[test]
    fn the_four_kinds_are_in_the_designed_order() {
        assert_eq!(KINDS, ["plan", "section", "find", "plate"]);
        assert_eq!(kind_label("plan", "ko"), labels::T_PLAN.ko);
        assert_eq!(kind_label("section", "ko"), labels::T_SECTION.ko);
        assert_eq!(kind_label("find", "ko"), labels::T_FIND.ko);
        assert_eq!(kind_label("plate", "ko"), labels::T_PLATE.ko);
        assert_eq!(kind_label("nonsense", "en"), labels::T_PLAN.text("en"), "an unknown kind falls back to a plan");
    }

    /// The plate is the A2 the design measures against — 420 × 594 mm — and the others are the
    /// working sheets it draws on.
    #[test]
    fn the_plate_is_a2_in_points() {
        let (w, h) = artboard("plate");
        assert!((w - 420.0 / 25.4 * 72.0).abs() < 0.001, "plate width {w} pt");
        assert!((h - 594.0 / 25.4 * 72.0).abs() < 0.001, "plate height {h} pt");
        assert!(h > w, "a plate stands taller than it is wide");
        for kind in ["plan", "section", "find"] {
            let (w, h) = artboard(kind);
            assert!(w > 0.0 && h > 0.0, "{kind} has no sheet");
        }
    }

    /// Every label the screen draws is the design's own word.
    #[test]
    fn the_labels_are_the_designs_own_words() {
        assert_eq!(labels::START_NEW_DRAWING.ko, "새 도면");
        assert_eq!(labels::START_OPEN_STORE.ko, "저장소 열기");
        assert_eq!(labels::START_NEW_TRACING.ko, "새 그리기");
        assert_eq!(labels::START_STRATA.ko, "Strata 조사 파일");
        for lang in ["ko", "en"] {
            assert_eq!(labels::START_NEW_DRAWING.text(lang), if lang == "ko" { "새 도면" } else { "New drawing" });
        }
    }
}
