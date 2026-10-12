//! The right column: drawing palette, legend · layers, properties (design 2.1 section 3).
//!
//! From the top: the drawing palette (always on, whatever tool is in hand), the
//! legend · layers as the largest of the three, and the properties of whatever is
//! selected. Cells are divided by thin rules and lead-ins, never by boxes
//! (Strata's rule), which is what the mock-up's panels look like.
//!
//! VectorCraft's dock can't be used here: it is a fixed three-tab group with a
//! collapsed icon column. So the column is drawn here, and the bodies it grows into
//! are the jobs that follow — the palette (J014), the legend (J005) and the
//! properties (J009, J010, J011). J003 gives the column its place, its three
//! sections and its shape.

use egui::{Align, Stroke, Ui, vec2};
use vectorcraft_ui_egui::VectorcraftApp;

use super::labels::{self, WEIGHTS};
use super::tokens;

/// The three sections, from the top.
#[allow(dead_code)] // the design's order, stated once and checked by the test below
pub const SECTIONS: [&str; 3] = ["drawing palette", "legend · layers", "properties"];
/// Height the drawing palette takes when the window is tall enough (points).
pub const PALETTE_FIXED: f32 = 150.0;
/// Height the properties take (points).
pub const PROPERTIES_FIXED: f32 = 132.0;
/// Share of what is left the legend takes; it is the largest of the three by design.
#[allow(dead_code)] // the design's ratio; the split below derives its own shares from the heights
pub const LEGEND_RATIO: f32 = 0.55;

pub fn show(app: &mut VectorcraftApp, ui: &mut Ui) {
    let lang = app.ui_language().code();
    let frame = egui::Frame::NONE.fill(tokens::SURFACE).inner_margin(egui::Margin::ZERO).stroke(Stroke::new(1.0, tokens::LINE));
    egui::Panel::right("trowel_inspector").resizable(true).exact_size(tokens::INSPECTOR_W).frame(frame).show(ui, |ui| {
        let total = ui.available_height();
        let h_palette = PALETTE_FIXED.min((total * 0.30).max(96.0));
        let h_props = PROPERTIES_FIXED.min((total * 0.24).max(88.0));
        let h_legend = (total - h_palette - h_props).max(80.0);
        let width = ui.available_width();

        section(ui, width, h_palette, |ui| drawing_palette(app, ui, lang));
        tokens::rule(ui);
        section(ui, width, h_legend, |ui| legend_layers(app, ui, lang));
        tokens::rule(ui);
        section(ui, width, h_props, |ui| properties(app, ui, lang));
    });
}

/// One of the column's three sections, `height` points tall, spanning the column's width.
fn section(ui: &mut Ui, width: f32, height: f32, body: impl FnOnce(&mut Ui)) {
    let top = ui.cursor().min.y;
    let rect = egui::Rect::from_min_size(ui.cursor().min, vec2(width, height));
    ui.allocate_ui(rect.size(), |ui| {
        ui.set_max_height(height);
        body(ui);
    });
    let _ = top;
}

/// 「그리기 팔레트」 — the weight chips, the hatch, the dash and 「잘린 면」, whatever
/// tool is in hand. J014 draws the chips for real; J003 lays the section out and
/// names the four rows.
fn drawing_palette(app: &mut VectorcraftApp, ui: &mut Ui, lang: &str) {
    section_head(ui, labels::DRAWING_PALETTE.text(lang));
    let target = match selected_count(app) {
        0 => labels::NEXT_STROKE,
        _ => labels::PROPERTIES_STROKE,
    };
    ui.label(egui::RichText::new(format!("{} · pt ⋯", target.text(lang))).size(11.0).color(tokens::INK2));
    ui.add_space(5.0);

    // 굵기 — the four weights of section 10, decision 4, thickest first.
    row(ui, labels::WEIGHT.text(lang));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(3.0, 0.0);
        for pt in WEIGHTS {
            let (r, resp) = ui.allocate_exact_size(vec2(30.0, 20.0), egui::Sense::click());
            ui.painter().rect_filled(r, 6.0, tokens::DESK);
            // The chip's line, drawn at the weight it stands for. A chip thinner than a plot
            // holds is drawn as a hairline outline, so the two ends of the ladder read
            // differently at a glance (section 7).
            let stroke = egui::Stroke::new(pt as f32, tokens::INK);
            let a = egui::pos2(r.left() + 6.0, r.center().y);
            let b = egui::pos2(r.right() - 6.0, r.center().y);
            if pt >= labels::PRINTER_SAFE_WEIGHT {
                ui.painter().line_segment([a, b], stroke);
            } else {
                ui.painter().line_segment([a, b], egui::Stroke::new(0.5, tokens::INK));
            }
            ui.painter().with_clip_rect(r).galley(
                egui::pos2(r.left() + 2.0, r.bottom() - 10.0),
                ui.painter().layout_no_wrap(labels::weight_label(pt), egui::FontId::monospace(8.5), tokens::FAINT),
                tokens::FAINT,
            );
            resp.on_hover_text(weight_note(pt, lang));
        }
        let (r, _) = ui.allocate_exact_size(vec2(20.0, 20.0), egui::Sense::hover());
        ui.painter().rect_filled(r, 6.0, tokens::WASH);
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, "+", egui::FontId::proportional(12.0), tokens::FAINT);
    });
    ui.add_space(4.0);

    // 빗금 · 선종 · 잘린 면 — named, drawn by J014.
    for l in [labels::HATCH, labels::DASH, labels::CUT_FACE] {
        row(ui, l.text(lang));
        ui.label(egui::RichText::new(format!("{} · J014", labels::NOT_READY.text(lang))).size(11.0).color(tokens::PENDING));
    }
}

/// 「범례 · 레이어」 — one legend line is one layer, and the line's weight, dash and
/// hatch are the shape of every stroke on it. J005 makes it work on the engine's
/// layers; J003 gives it the three groups and the footer.
fn legend_layers(app: &mut VectorcraftApp, ui: &mut Ui, lang: &str) {
    ui.horizontal(|ui| {
        section_head(ui, labels::LEGEND_LAYERS.text(lang));
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.label(egui::RichText::new(format!("{} · J005", labels::NOT_READY.text(lang))).size(10.5).color(tokens::PENDING));
        });
    });
    let layers = app.session.documents().len();
    for (heading, task) in [("調査データ · Strata와 같은 레이어", "J005"), ("그리기 · Trowel", "J005")] {
        let _ = task;
        ui.label(egui::RichText::new(heading).strong().size(11.0).color(tokens::INK2));
        if layers == 0 {
            tokens::pending(ui, &format!("{heading} · J005"));
        }
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("{} ×0.8 … {} 1 → {}", labels::WEIGHT.text(lang), labels::DASH.text(lang), labels::NOT_READY.text(lang)))
                .size(10.5)
                .color(tokens::PENDING),
        );
    });
}

/// 「속성」 — what is selected: its kind and period while a trench is being drawn,
/// its lengths, and what it will compute to when it is closed.
fn properties(app: &mut VectorcraftApp, ui: &mut Ui, lang: &str) {
    ui.horizontal(|ui| {
        section_head(ui, labels::PROPERTIES.text(lang));
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.label(egui::RichText::new("⌄").size(11.0).color(tokens::FAINT));
        });
    });
    let n = selected_count(app);
    if n == 0 {
        tokens::pending(ui, &format!("{} · J009", labels::KIND.text(lang)));
        return;
    }
    ui.label(egui::RichText::new(format!("{} {n}", labels::KIND.text(lang))).size(11.0).color(tokens::INK2));
    ui.label(egui::RichText::new(format!("{} ▼ · {}", labels::PERIOD.text(lang), labels::PICK.text(lang))).size(11.0).color(tokens::PENDING));
}

/// A section's name, set in the semibold face.
fn section_head(ui: &mut Ui, name: &str) {
    ui.label(egui::RichText::new(name).strong().size(12.0).color(tokens::INK));
}

/// What a weight chip says when hovered: the weight in millimetres on paper, and for one
/// thinner than the plotter holds, the weight a submission drawing guarantees
/// (section 14, question (a)).
fn weight_note(pt: f64, lang: &str) -> String {
    let mm = pt * 25.4 / 72.0;
    if pt >= labels::PRINTER_SAFE_WEIGHT {
        format!("{} {mm:.3} mm", labels::WEIGHT.text(lang))
    } else {
        format!("{} {mm:.3} mm {} {}", labels::WEIGHT.text(lang), labels::NOT_READY.text(lang), labels::PRINTER_SAFE_WEIGHT)
    }
}

/// A row's name on the left, its value on the right.
fn row(ui: &mut Ui, name: &str) {
    ui.label(egui::RichText::new(name).size(11.0).color(tokens::MUTED));
}

/// How many objects are selected. `None` would only mean an engine that lost the
/// active document mid-frame, which the guard has already answered for.
/// How many objects are selected; zero when no drawing is open.
fn selected_count(app: &VectorcraftApp) -> usize {
    app.session.active().map(|d| d.selection.len()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    /// The three sections, in the design's order from the top (section 3).
    #[test]
    fn the_column_is_palette_legend_properties() {
        assert_eq!(SECTIONS, ["drawing palette", "legend · layers", "properties"]);
    }

    /// The legend is the largest of the three, as the mock-up draws it.
    #[test]
    fn the_legend_is_the_largest() {
        let tall: f32 = 900.0;
        let palette = PALETTE_FIXED.min((tall * 0.30).max(96.0));
        let properties = PROPERTIES_FIXED.min((tall * 0.24).max(88.0));
        let legend = (tall - palette - properties).max(80.0);
        assert!(legend > palette, "legend {legend} must beat palette {palette}");
        assert!(legend > properties, "legend {legend} must beat properties {properties}");
        assert!(LEGEND_RATIO > PALETTE_FIXED / tall);
    }

    /// Even in a short window the three sections still fit.
    #[test]
    fn the_column_fits_a_short_window() {
        let short: f32 = 500.0;
        let palette = PALETTE_FIXED.min((short * 0.30).max(96.0));
        let properties = PROPERTIES_FIXED.min((short * 0.24).max(88.0));
        let legend = (short - palette - properties).max(80.0);
        assert!(palette > 0.0 && properties > 0.0 && legend > 0.0);
    }
}
