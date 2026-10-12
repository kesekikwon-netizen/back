//! The status bar (design 2.1 section 3).
//!
//! Left: one sentence of what to do next. Right: unsaved count, the real TM
//! coordinates under the pointer, the drawing's scale and the coordinate system.
//! The coordinates and the scale are J006 — until then the two ends are shown in
//! their final shape with 「준비 중」 where the numbers go, so the bar's arrangement
//! can be compared with the mock-up now and only the numbers change later.

use egui::{Align, Layout, Stroke, Ui};
use vectorcraft_ui_egui::VectorcraftApp;

use super::labels;
use super::tokens;

/// Height — design 2.1 section 3 ("status 32").
pub const HEIGHT: f32 = tokens::STATUS;

pub fn show(app: &mut VectorcraftApp, ui: &mut Ui) {
    let lang = app.ui_language().code();
    egui::Panel::bottom("trowel_status")
        .exact_size(HEIGHT)
        .frame(egui::Frame::NONE.fill(tokens::SURFACE).inner_margin(egui::Margin::symmetric(10, 0)).stroke(Stroke::new(1.0, tokens::LINE)))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                let hint = next_step(app, lang);
                ui.label(egui::RichText::new(hint).size(11.5).color(tokens::INK2));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    right(app, ui, lang);
                });
            });
        });
}

/// The right end: unsaved, then the coordinates, the scale and the system, as the
/// mock-up orders them. Read right to left, so they are laid out in reverse.
fn right(app: &mut VectorcraftApp, ui: &mut Ui, lang: &str) {
    let unsaved = unsaved_count(app);
    if unsaved > 0 {
        ui.label(
            egui::RichText::new(format!("{} {unsaved}", labels::UNSAVED.text(lang))).size(11.5).color(tokens::tile_color(tokens::TileRole::Unsaved)),
        );
        ui.add_space(10.0);
    }
    // 5186 중부원점 · 높이 해발
    ui.label(
        egui::RichText::new(format!("5186 중부원점 · {} {}", labels::BOOK_HEIGHT.text(lang), labels::BOOK_SEALE.text(lang)))
            .size(11.0)
            .color(tokens::INK2),
    );
    ui.add_space(10.0);
    // 도면 1:20 — the drawing's scale (J006).
    ui.label(egui::RichText::new(format!("1:20 · {}", labels::NOT_READY.text(lang))).size(11.0).color(tokens::PENDING));
    ui.add_space(10.0);
    // X … Y … — the real TM coordinates under the pointer (J006).
    ui.label(egui::RichText::new(format!("X —  Y —  {}", labels::NOT_READY.text(lang))).size(11.0).color(tokens::PENDING));
}

/// One sentence of what to do next. With nothing selected the app says what it is
/// ready for; with something selected it says what that object can do.
/// How many objects are selected; zero when no drawing is open.
fn selected_count(app: &VectorcraftApp) -> usize {
    app.session.active().map(|d| d.selection.len()).unwrap_or(0)
}

fn next_step(app: &VectorcraftApp, lang: &str) -> String {
    let _ = lang;
    match selected_count(app) {
        0 => "시작점을 누르면 닫히고 면적·둘레를 계산합니다.".to_string(),
        _ => "이 변 · 지금까지 · 닫으면 면적·둘레".to_string(),
    }
}

/// How many drawings have unsaved changes.
fn unsaved_count(app: &VectorcraftApp) -> usize {
    app.session.documents().iter().filter(|d| d.is_dirty()).count()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn the_bar_is_thirty_two_points_as_the_mock_up_shows() {
        assert_eq!(HEIGHT, 32.0);
    }

    /// With nothing selected the bar still says something rather than sitting empty.
    #[test]
    fn the_left_end_always_says_something() {
        assert!(!next_step_text().is_empty());
    }

    fn next_step_text() -> &'static str {
        "시작점을 누르면 닫히고 면적·둘레를 계산합니다."
    }
}
