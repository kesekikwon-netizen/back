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
//! properties (J009, J010, J011).
//!
//! The legend is the rule of section 7 made visible: one line is one layer, and the
//! line's weight and line type are the shape of every stroke on that layer. A stroke
//! the user set by hand is left alone and counted, so the line reads 「따로 2」, and
//! 「범례대로」 puts it back. Every action here is an engine command (`legend.*`), so
//! the control channel and the MCP reach all of it.

use std::cell::RefCell;

use egui::{Align, Stroke, Ui, vec2};
use serde_json::{Value, json};
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

/// What the weight-at-once control is set to (§7: 「어디 · 어떻게 · 값」 plus
/// 「따로 바꾼 선도 함께」). The panel is state, not a model, so it lives here rather
/// than in the domain — the domain only ever sees the four values a command takes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScaleState {
    pub where_: &'static str,
    pub how: &'static str,
    pub factor: f64,
    pub include: bool,
}

impl Default for ScaleState {
    /// 「이 범례 · 비율 값 ×0.8」 — the design's own example of the control (§7).
    fn default() -> Self {
        Self { where_: "set", how: "ratio", factor: 0.8, include: false }
    }
}

/// The three scopes, in the order the control lists them.
pub const SCOPE_SET: &str = "set";
pub const SCOPE_DRAWING: &str = "drawing";
pub const SCOPE_BOOK: &str = "book";
/// The two ways a weight carries across the lines it covers.
pub const HOW_RATIO: &str = "ratio";
pub const HOW_STEP: &str = "step";

thread_local! {
    /// The panel's own state, one per thread. Nothing here is the drawing's: the legend
    /// table, the lines' weights and the 「따로」 counts all live in the engine.
    static PANEL: RefCell<Panel> = RefCell::new(Panel::default());
}

#[derive(Debug, Default)]
struct Panel {
    /// The line whose chips and layer are showing.
    open: Option<String>,
    scale: ScaleState,
    /// The last thing the panel could not do, drawn under the head until it changes.
    note: String,
}

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

    // 굵기 — the ladder of section 10, decision 4, thickest first.
    row(ui, labels::WEIGHT.text(lang));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(3.0, 0.0);
        for pt in WEIGHTS {
            weight_chip(ui, pt, lang, |_| {});
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

/// One weight chip, drawn at the weight it stands for. `on_pick` runs the weight when the
/// chip is clicked; the drawing palette has nothing yet to change, so it passes a no-op.
fn weight_chip(ui: &mut Ui, pt: f64, lang: &str, mut on_pick: impl FnMut(f64)) {
    let (r, resp) = ui.allocate_exact_size(vec2(30.0, 20.0), egui::Sense::click());
    ui.painter().rect_filled(r, 6.0, tokens::DESK);
    // A chip thinner than a plot holds is drawn as a hairline outline, so the two ends of
    // the ladder read differently at a glance (section 7).
    if pt >= labels::PRINTER_SAFE_WEIGHT {
        let a = egui::pos2(r.left() + 6.0, r.center().y);
        let b = egui::pos2(r.right() - 6.0, r.center().y);
        ui.painter().line_segment([a, b], egui::Stroke::new(pt as f32, tokens::INK));
    } else {
        let a = egui::pos2(r.left() + 6.0, r.center().y);
        let b = egui::pos2(r.right() - 6.0, r.center().y);
        ui.painter().line_segment([a, b], egui::Stroke::new(0.5, tokens::INK));
    }
    ui.painter().with_clip_rect(r).galley(
        egui::pos2(r.left() + 2.0, r.bottom() - 10.0),
        ui.painter().layout_no_wrap(labels::weight_label(pt), egui::FontId::monospace(8.5), tokens::FAINT),
        tokens::FAINT,
    );
    if resp.clicked() {
        on_pick(pt);
    }
    resp.on_hover_text(weight_note(pt, lang));
}

/// 「범례 · 레이어」 — one line is one layer, and the line's weight and line type are the
/// shape of every stroke on it (§7).
fn legend_layers(app: &mut VectorcraftApp, ui: &mut Ui, lang: &str) {
    let listed = legend_list(app);
    let group = listed.as_ref().and_then(|v| v.get("group")).and_then(Value::as_str).unwrap_or("plan").to_string();
    ui.horizontal(|ui| {
        section_head(ui, labels::LEGEND_LAYERS.text(lang));
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.label(egui::RichText::new(set_label(group.as_str(), lang)).size(11.0).color(tokens::MUTED));
        });
    });
    set_switch(app, ui, lang, &group);
    let Some(listed) = listed else {
        tokens::pending(ui, labels::LEGEND_LAYERS.text(lang));
        return;
    };
    let lines = listed.get("lines").and_then(Value::as_array).cloned().unwrap_or_default();
    let open = PANEL.with(|p| p.borrow().open.clone());

    egui::ScrollArea::vertical().id_salt("trowel_legend").auto_shrink([false, false]).max_height(ui.available_height() - 62.0).show(ui, |ui| {
        if lines.is_empty() {
            tokens::pending(ui, set_label(&group, lang));
            return;
        }
        for line in &lines {
            legend_line(app, ui, lang, line, open.as_deref());
        }
    });

    weight_at_once(app, ui, lang);
}

/// The three sets, one switch apart (§7: a new drawing lays out one of them).
fn set_switch(app: &mut VectorcraftApp, ui: &mut Ui, lang: &str, current: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
        for id in ["plan", "section", "find"] {
            let on = id == current;
            let text = set_label(id, lang);
            let b = egui::Button::new(egui::RichText::new(text).size(11.0).color(if on { tokens::CLAY_TEXT } else { tokens::MUTED }))
                .fill(if on { tokens::BLOCK_WASH } else { tokens::SURFACE })
                .stroke(Stroke::new(1.0, if on { tokens::CLAY } else { tokens::LINE }));
            if ui.add(b).on_hover_text(text).clicked() && !on {
                note(app.run("legend.setGroup", json!({ "group": id })).err().unwrap_or_default());
                PANEL.with(|p| p.borrow_mut().open = None);
            }
        }
    });
}

/// One line of the legend: its name, its weight, and — while it is the current one — the
/// chips that change its weight, the layer it is bound to, and 「범례대로」 when strokes
/// on it were set by hand.
fn legend_line(app: &mut VectorcraftApp, ui: &mut Ui, lang: &str, line: &Value, open: Option<&str>) {
    let layer = line.get("layer").and_then(Value::as_str).unwrap_or("").to_string();
    let name = line.get("name").and_then(Value::as_str).unwrap_or("").to_string();
    let weight = line.get("weight").and_then(Value::as_f64).unwrap_or(0.0);
    let separate = line.get("separate").and_then(Value::as_u64).unwrap_or(0);
    let frequent = line.get("frequent").and_then(Value::as_bool).unwrap_or(false);
    let dash = line.get("dash").and_then(Value::as_str).unwrap_or("solid").to_string();
    let from_survey = line.get("fromSurvey").and_then(Value::as_bool).unwrap_or(true);
    let is_open = open == Some(layer.as_str());

    let clicked = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
            if is_open {
                ui.label(egui::RichText::new("▾").size(10.0).color(tokens::FAINT));
            } else {
                ui.label(egui::RichText::new("▸").size(10.0).color(tokens::FAINT));
            }
            let mut t = egui::RichText::new(format!("{} {name}", if frequent { "★" } else { "" })).size(11.5);
            if is_open {
                t = t.strong().color(tokens::INK);
            } else {
                t = t.color(tokens::INK2);
            }
            let resp = ui.label(t);
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                ui.label(egui::RichText::new(labels::weight_label(weight)).size(11.0).color(tokens::INK));
                if separate > 0 {
                    let chip = egui::RichText::new(format!("{} {separate}", labels::SEPARATE.text(lang))).size(9.5).color(tokens::CAUTION);
                    ui.label(chip);
                }
            });
            resp.clicked()
        })
        .response
        .clicked();

    if clicked {
        note(app.run("legend.choose", json!({ "layer": layer })).err().unwrap_or_default());
        PANEL.with(|p| {
            let mut p = p.borrow_mut();
            p.open = if is_open { None } else { Some(layer.clone()) };
        });
    }

    if !is_open {
        return;
    }

    // 선종 · 빗금 · 글자 — what the line stands for, named as section 7 writes it.
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        let hatch = line.get("hatch").and_then(Value::as_str).unwrap_or("none");
        let label_pt = line.get("labelPt").and_then(Value::as_f64).unwrap_or(0.0);
        let mut t = format!("{} · {}", dash_label(dash.as_str(), lang), hatch_label(hatch, lang));
        if label_pt > 0.0 {
            t.push_str(&format!(" · {} {label_pt} pt", labels::TEXT_SIZE.text(lang)));
        }
        ui.label(egui::RichText::new(t).size(10.0).color(tokens::MUTED));
    });
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        ui.label(
            egui::RichText::new(format!(
                "{} · {}",
                layer,
                if from_survey { labels::FROM_SURVEY.text(lang) } else { labels::TROWEL_ONLY_LINES.text(lang) }
            ))
            .size(10.0)
            .color(tokens::FAINT),
        );
    });

    // 굵기 칩 — picking one changes every stroke on the line's layer at once (§7).
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
        for pt in WEIGHTS {
            let layer = layer.clone();
            weight_chip(ui, pt, lang, |pt| {
                note(app.run("legend.setWeight", json!({ "layer": layer, "weight": pt })).err().unwrap_or_default());
            });
        }
    });

    // 「범례대로」 — only when the line has strokes the user set by hand.
    if separate > 0 {
        let layer = layer.clone();
        let text = format!("{} {separate}", labels::TO_THE_LINE.text(lang));
        if ui
            .add(
                egui::Button::new(egui::RichText::new(text).size(10.5).color(tokens::CLAY_TEXT))
                    .fill(tokens::CAUTION_WASH)
                    .stroke(Stroke::new(1.0, tokens::CAUTION)),
            )
            .clicked()
        {
            note(app.run("legend.resetOverrides", json!({ "layer": layer })).err().unwrap_or_default());
        }
    }
}

/// 「굵기 한 번에」 — 「어디 · 어떻게 · 값」 and 「따로 바꾼 선도 함께」 (§7).
fn weight_at_once(app: &mut VectorcraftApp, ui: &mut Ui, lang: &str) {
    let scale = PANEL.with(|p| p.borrow().scale);
    tokens::rule(ui);
    ui.label(egui::RichText::new(labels::WEIGHT_AT_ONCE.text(lang)).strong().size(11.0).color(tokens::INK));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
        let mut next = scale;
        egui::ComboBox::from_id_salt("legend_scope").selected_text(scope_label(scale.where_, lang)).width(78.0).show_ui(ui, |ui| {
            for (id, label) in [(SCOPE_SET, labels::WHERE_SET), (SCOPE_DRAWING, labels::WHERE_DRAWING), (SCOPE_BOOK, labels::WHERE_BOOK)] {
                ui.selectable_value(&mut next.where_, id, label.text(lang));
            }
        });
        egui::ComboBox::from_id_salt("legend_how")
            .selected_text(if scale.how == HOW_STEP { labels::HOW_STEP.text(lang) } else { labels::HOW_RATIO.text(lang) })
            .width(72.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut next.how, HOW_RATIO, labels::HOW_RATIO.text(lang));
                ui.selectable_value(&mut next.how, HOW_STEP, labels::HOW_STEP.text(lang));
            });
        if next.how == HOW_RATIO {
            ui.add(egui::DragValue::new(&mut next.factor).speed(0.02).range(0.05..=4.0).prefix("×").custom_formatter(|v, _| format!("{v:.2}")));
        }
        ui.checkbox(&mut next.include, egui::RichText::new(labels::WITH_SEPARATE.text(lang)).size(10.0).color(tokens::MUTED));
        PANEL.with(|p| p.borrow_mut().scale = next);
        if ui.add(egui::Button::new(egui::RichText::new("→").size(12.0))).on_hover_text(labels::WEIGHT_AT_ONCE.text(lang)).clicked() {
            let mut params = json!({ "where": next.where_, "how": next.how, "includeOverrides": next.include });
            if next.how == HOW_RATIO {
                params["factor"] = json!(next.factor);
            }
            note(app.run("legend.applyScale", params).err().unwrap_or_default());
        }
    });
    let note = PANEL.with(|p| p.borrow().note.clone());
    if !note.is_empty() {
        ui.label(egui::RichText::new(note).size(10.0).color(tokens::CAUTION));
    }
}

/// What the panel could not do, kept until something changes.
fn note(msg: String) {
    PANEL.with(|p| p.borrow_mut().note = msg);
}

/// The legend as the engine has it, or `None` when it could not be read this frame.
fn legend_list(app: &mut VectorcraftApp) -> Option<Value> {
    app.run("legend.list", json!({})).ok()
}

fn set_label(id: &str, lang: &str) -> &'static str {
    match id {
        "section" => labels::SET_SECTION.text(lang),
        "find" => labels::SET_FIND.text(lang),
        _ => labels::SET_PLAN.text(lang),
    }
}

fn scope_label(id: &str, lang: &str) -> &'static str {
    match id {
        SCOPE_DRAWING => labels::WHERE_DRAWING.text(lang),
        SCOPE_BOOK => labels::WHERE_BOOK.text(lang),
        _ => labels::WHERE_SET.text(lang),
    }
}

fn dash_label(id: &str, lang: &str) -> &'static str {
    match id {
        "dashed" => labels::DASH_DASHED.text(lang),
        "dotted" => labels::DASH_DOTTED.text(lang),
        "dashDot" => labels::DASH_DASHDOT.text(lang),
        _ => labels::DASH_SOLID.text(lang),
    }
}

fn hatch_label(id: &str, lang: &str) -> &'static str {
    match id {
        "deg45" => "45°",
        "deg135" => "135°",
        "alternating" => labels::HATCH_ALTERNATING.text(lang),
        _ => labels::HATCH_NONE.text(lang),
    }
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

    /// The weight-at-once control starts on the design's own example: 이 범례 · 비율 값 ×0.8,
    /// and the strokes set by hand stay out until the switch is ticked (§7).
    #[test]
    fn the_weight_at_once_starts_on_the_designs_example() {
        let s = ScaleState::default();
        assert_eq!(s.where_, SCOPE_SET);
        assert_eq!(s.how, HOW_RATIO);
        assert_eq!(s.factor, 0.8);
        assert!(!s.include, "따로 바꾼 선도 함께 is off by default");
    }

    /// The three scopes and the two ways are exactly what the commands accept (§7).
    #[test]
    fn the_control_offers_exactly_what_the_commands_take() {
        assert_eq!([SCOPE_SET, SCOPE_DRAWING, SCOPE_BOOK], ["set", "drawing", "book"]);
        assert_eq!([HOW_RATIO, HOW_STEP], ["ratio", "step"]);
    }

    /// Every set has its own name in the panel (§7: 평면용 · 단면·입면용 · 유물용).
    #[test]
    fn each_set_has_its_own_name() {
        for lang in ["ko", "en"] {
            assert_eq!(set_label("plan", lang), labels::SET_PLAN.text(lang));
            assert_eq!(set_label("section", lang), labels::SET_SECTION.text(lang));
            assert_eq!(set_label("find", lang), labels::SET_FIND.text(lang));
            // An unknown id falls back to the plan set rather than showing nothing.
            assert_eq!(set_label("nonsense", lang), labels::SET_PLAN.text(lang));
        }
        assert_eq!(labels::SET_PLAN.ko, "평면용");
        assert_eq!(labels::SET_SECTION.ko, "단면·입면용");
        assert_eq!(labels::SET_FIND.ko, "유물용");
    }

    /// The line types are written as section 7 writes them: 실선 · 파선 · 점선 · 일점쇄선.
    #[test]
    fn the_line_types_are_written_as_the_design_writes_them() {
        assert_eq!(labels::DASH_SOLID.ko, "실선");
        assert_eq!(labels::DASH_DASHED.ko, "파선");
        assert_eq!(labels::DASH_DOTTED.ko, "점선");
        assert_eq!(labels::DASH_DASHDOT.ko, "일점쇄선");
        for lang in ["ko", "en"] {
            assert_eq!(dash_label("solid", lang), labels::DASH_SOLID.text(lang));
            assert_eq!(dash_label("dashDot", lang), labels::DASH_DASHDOT.text(lang));
            assert_eq!(dash_label("dotted", lang), labels::DASH_DOTTED.text(lang));
            assert_eq!(dash_label("dashed", lang), labels::DASH_DASHED.text(lang));
            assert_eq!(dash_label("nonsense", lang), labels::DASH_SOLID.text(lang));
        }
    }

    /// 「범례대로」 is the way back, and it is the design's own word (§7).
    #[test]
    fn back_to_the_line_is_the_designs_own_word() {
        assert_eq!(labels::TO_THE_LINE.ko, "범례대로");
        assert_eq!(labels::SEPARATE.ko, "따로");
        assert_eq!(labels::WEIGHT_AT_ONCE.ko, "굵기 한 번에");
        assert_eq!(labels::WITH_SEPARATE.ko, "따로 바꾼 선도 함께");
    }

    /// The panel keeps no model of its own: the weights it draws come from the ladder, and
    /// the counts it shows come from the engine.
    #[test]
    fn the_panel_shows_the_ladder_it_offers() {
        assert_eq!(WEIGHTS, [1.0, 0.8, 0.5, 0.25, 0.2, 0.1]);
        // Every chip's written label is the weight itself, so a line reading 「0.8」 is the
        // 0.8 pt of §7 and not a rounded something else.
        assert_eq!(labels::weight_label(0.8), "0.8");
        assert_eq!(labels::weight_label(1.0), "1");
    }
}
