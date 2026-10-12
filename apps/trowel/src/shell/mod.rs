//! Trowel's window arrangement — design 2.1 section 3 (J003).
//!
//! `VectorcraftApp::ui_frame` is private and draws VectorCraft's own arrangement
//! (its brand mark, its Discord button, its fixed three-tab right dock). Trowel
//! keeps everything underneath — the canvas, the tools, the dialogs, the command
//! palette, the panels — and replaces only the arrangement, which is what this
//! module is.
//!
//! Reused as it stands: [`chrome::control_bar`] (the contextual bar that follows
//! the tool in hand), [`toolbar::show`] (the left tool column), [`chrome::doc_tabs`]
//! and [`canvas::show`]. Drawn here: the menu bar (without the VectorCraft brand
//! mark or Discord button, which the design drops), the one-line ribbon, the
//! 「Archaeology」 tool group, the 「drawing book」, the right column of three panels,
//! and the status bar.

pub mod arch_tools;
pub mod drawing_book;
pub mod fonts;
pub mod labels;
pub mod menubar;
pub mod menus;
pub mod ribbon;
pub mod right_column;
pub mod start_screen;
pub mod status;
// The palette and the bar heights are the design's, complete on purpose: J003 draws with
// the ribbon tile roles and the heights, and J001 carries the rest through the interface.
// A constant with no user yet is the one whose surface has not arrived.
#[allow(dead_code)]
pub mod tokens;

use egui::Ui;
use vectorcraft_ui_egui::{VectorcraftApp, canvas, chrome, dialogs, floating, native_menu, palette, toolbar};

// The window's arrangement, in the order design 2.1 section 3 lays it out.
/// Draw one frame of Trowel's window.
pub fn trowel_frame(app: &mut VectorcraftApp, ui: &mut Ui, fonts: &mut fonts::Fallback) {
    let ctx = ui.ctx().clone();
    let t = vectorcraft_ui_egui::theme::Tokens::get(&ctx);
    // `screen_mode`: 0 and 1 show everything, 2 drops the bars, 3 is Presentation
    // Mode (canvas only). The same meaning VectorCraft gives it.
    let mode = app.ui.screen_mode;
    let bars = mode < 2;
    let chrome_on = mode < 3;

    if bars {
        menubar::show(app, ui);
        ribbon::show(app, ui);
        if app.ui.control_bar {
            chrome::control_bar(app, ui);
        }
    }

    if chrome_on {
        // The tool column draws its own panel, so the archaeology group that
        // follows it is a panel of its own rather than a row at the column's foot.
        if app.ui.toolbar {
            toolbar::show(app, ui);
            arch_tools::show(app, ui);
        }
        if app.ui.dock {
            drawing_book::show(app, ui);
            right_column::show(app, ui);
        }
        if app.ui.status_bar {
            status::show(app, ui);
        }
    }

    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.pasteboard)).show(ui, |ui| {
        if chrome_on {
            chrome::doc_tabs(app, ui);
        }
        // With nothing open, the canvas would show VectorCraft's Home view. Trowel draws its
        // own start screen there instead (J004), so no archaeology ever appears as VectorCraft.
        if !start_screen::show_if_empty(app, ui, app.ui_language().code()) {
            canvas::show(app, ui);
        }
    });

    // Panels torn out of their dock, the modal dialogs, the command palette.
    vectorcraft_ui_egui::dock::floating_panel(app, &ctx);
    floating::show(app, &ctx);
    dialogs::show(app, &ctx);
    palette::show(app, &ctx);
    if app.custom_titlebar {
        vectorcraft_ui_egui::titlebar::resize_zones(ui);
    }
    native_menu::sync(app, &ctx);
    // The interface fonts carry no Hangul: what this frame painted that they lack is looked
    // for in the installed fonts and joins every family as its last fallback for the next frame.
    fonts.after_frame(&ctx);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    /// The heights in design 2.1 section 3 (the caption is 32 in the mock-up's
    /// "caption 32 · ribbon tile 44 · control 40 · tab 32 · status 32" line).
    #[test]
    fn the_bars_are_the_heights_the_mock_up_shows() {
        assert_eq!(tokens::CAPTION, 32.0);
        assert_eq!(tokens::RIBBON_TILE, 44.0);
        assert_eq!(tokens::CONTROL_BAR, 40.0);
        assert_eq!(tokens::TAB, 32.0);
        assert_eq!(tokens::STATUS, 32.0);
    }

    /// The right column holds the drawing palette, the legend and the properties,
    /// in that order from the top, and the legend is the largest of the three.
    #[test]
    fn the_right_column_is_palette_legend_properties() {
        assert_eq!(right_column::SECTIONS, ["drawing palette", "legend · layers", "properties"]);
    }

    /// The ribbon's tiles carry the four roles the design names: an idle tile in
    /// desk colour, the tool in hand in ink, an on view in clay, unsaved in clay-text.
    #[test]
    fn ribbon_tile_roles_have_the_designed_colours() {
        assert_eq!(tokens::tile_color(tokens::TileRole::Idle).to_array(), [0xF0, 0xEE, 0xE6, 0xFF]);
        assert_eq!(tokens::tile_color(tokens::TileRole::Held).to_array(), [0x30, 0x30, 0x2E, 0xFF]);
        assert_eq!(tokens::tile_color(tokens::TileRole::On).to_array(), [0xD9, 0x77, 0x57, 0xFF]);
        assert_eq!(tokens::tile_color(tokens::TileRole::Unsaved).to_array(), [0x9C, 0x4A, 0x2F, 0xFF]);
    }
}
