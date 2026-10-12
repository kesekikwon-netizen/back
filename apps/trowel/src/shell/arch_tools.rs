//! The 「Archaeology」 tool group (design 2.1 section 3).
//!
//! It follows VectorCraft's own left tool column, which the design keeps whole
//! (`V A P N \ M T R S C K I H Z`). `toolbar::show` owns its panel, so the group is
//! a panel of its own beside it rather than a row at that column's foot — the same
//! place on screen, and it leaves the shared toolbar untouched.
//!
//! Every tool here arrives in a later task (section 13): the section line and the
//! elevation lines (J015), the plan tools (J009), the strata (J010) and the find
//! measurements (J011). Until then they read 「준비 중 · <task>」.

use egui::{CornerRadius, Stroke, Ui, vec2};
use vectorcraft_ui_egui::{VectorcraftApp, icons};

use super::labels;
use super::tokens;

/// One tool in the group: its glyph, its name, its key and the task that builds it.
struct ArchTool {
    icon: &'static str,
    label: labels::Label,
    key: &'static str,
    task: &'static str,
}

/// The group, in the order the design lists it after the shared column.
const TOOLS: &[ArchTool] = &[
    ArchTool { icon: "dc-line", label: labels::T_SECTION_LINE, key: "", task: "J015" },
    ArchTool { icon: "dc-reference-point", label: labels::T_LEVEL_MARK, key: "", task: "J009" },
    ArchTool { icon: "spline-pointer", label: labels::T_TRENCH, key: "", task: "J009" },
    ArchTool { icon: "circle", label: labels::T_FIND, key: "", task: "J011" },
    ArchTool { icon: "waves", label: labels::BOOK_SECTION, key: "", task: "J010" },
];

pub fn show(app: &mut VectorcraftApp, ui: &mut Ui) {
    let lang = app.ui_language().code();
    let frame = egui::Frame::NONE
        .fill(tokens::WASH)
        .inner_margin(egui::Margin { left: 2, right: 2, top: 6, bottom: 6 })
        .stroke(Stroke::new(1.0, tokens::LINE));
    egui::Panel::left("trowel_arch_tools").resizable(false).exact_size(tokens::TOOL_W).frame(frame).show(ui, |ui| {
        ui.vertical(|ui| {
            // The group's name, set sideways as the mock-up sets it.
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(labels::G_ARCHAEOLOGY.text(lang)).size(9.5).color(tokens::FAINT));
            });
            ui.add_space(4.0);
            for t in TOOLS {
                let (r, resp) = ui.allocate_exact_size(vec2(tokens::TOOL_W - 4.0, 34.0), egui::Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(r, CornerRadius::same(6), tokens::PRESS);
                }
                icons::paint(ui, t.icon, egui::Rect::from_center_size(r.center() - vec2(0.0, 5.0), vec2(16.0, 16.0)), tokens::PENDING);
                ui.painter().with_clip_rect(r).galley(
                    egui::pos2(r.left() + 2.0, r.bottom() - 13.0),
                    ui.painter().layout_no_wrap(t.label.text(lang).into(), egui::FontId::proportional(8.5), tokens::PENDING),
                    tokens::PENDING,
                );
                resp.on_hover_text(format!("{} · {} {}", labels::NOT_READY.text(lang), t.task, t.key));
            }
        });
    });
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn the_group_names_every_tool_it_shows() {
        assert_eq!(TOOLS.len(), 5);
        for t in TOOLS {
            assert!(!t.icon.is_empty());
            assert!(!t.label.ko.is_empty());
            assert!(t.task.starts_with('J'), "{} has no task", t.label.key);
        }
    }

    #[test]
    fn the_group_fits_the_tool_column_width() {
        const { assert!(tokens::TOOL_W >= 36.0) };
    }
}
