//! Trowel's own design tokens: the 「quiet plate」 palette and the bar heights of
//! design 2.1 section 3.
//!
//! The colours are the mock-up's, read from its own stylesheet. J001 carries them
//! through the whole interface; J003 needs the ribbon tile roles and the heights, so
//! they live here from the start.
//!
//! VectorCraft's own [`vectorcraft_ui_egui::theme`] still draws the canvas, the
//! panels and the text — Trowel does not fork it.

use egui::{Color32, vec2};

/// Caption (title bar) height — design 2.1 section 3.
pub const CAPTION: f32 = 32.0;
/// One ribbon tile's height — section 3.
pub const RIBBON_TILE: f32 = 44.0;
/// The contextual control bar's height — section 3.
pub const CONTROL_BAR: f32 = 40.0;
/// The document tabs' height — section 3.
pub const TAB: f32 = 32.0;
/// The status bar's height — section 3.
pub const STATUS: f32 = 32.0;
/// Corner radii — section 3 ("radius 8/12", the Strata reference values).
pub const RADIUS_SMALL: f32 = 8.0;
pub const RADIUS_LARGE: f32 = 12.0;
/// Width of one tool in the left column, and of a tile's glyph.
pub const TOOL_W: f32 = 40.0;
/// The 「drawing book」 column, wide enough for `1호 주거지 1:20`.
pub const BOOK_W: f32 = 248.0;
/// The right column of three panels.
pub const INSPECTOR_W: f32 = 300.0;

// — the palette, from the mock-up's stylesheet ———————————————————————————

/// The paper the whole interface sits on.
pub const GROUND: Color32 = Color32::from_rgb(0xFA, 0xF9, 0xF5);
/// A raised surface: panels, tiles.
pub const SURFACE: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
/// A recessed surface: fields, rows at rest.
pub const WASH: Color32 = Color32::from_rgb(0xF5, 0xF4, 0xED);
/// The desk: an idle ribbon tile.
pub const DESK: Color32 = Color32::from_rgb(0xF0, 0xEE, 0xE6);
/// A pressed tile.
pub const PRESS: Color32 = Color32::from_rgb(0xE8, 0xE6, 0xDC);
/// The hand: ink for the tool in hand.
pub const HAND: Color32 = Color32::from_rgb(0x30, 0x30, 0x2E);
pub const INK: Color32 = Color32::from_rgb(0x14, 0x14, 0x13);
pub const INK2: Color32 = Color32::from_rgb(0x3D, 0x3D, 0x3A);
pub const MUTED: Color32 = Color32::from_rgb(0x5E, 0x5D, 0x59);
pub const FAINT: Color32 = Color32::from_rgb(0x73, 0x72, 0x6C);
/// Hairlines: cells are divided by thin rules and lead-ins only (Strata's rule).
pub const LINE: Color32 = Color32::from_rgb(0xDE, 0xDC, 0xD1);
pub const EDGE: Color32 = Color32::from_rgb(0xC2, 0xC0, 0xB6);
pub const RING: Color32 = Color32::from_rgb(0xB0, 0xAE, 0xA5);
/// The accent: an action, an export.
pub const ACTION: Color32 = Color32::from_rgb(0xB5, 0x57, 0x3A);
/// Clay: a view that is on.
pub const CLAY: Color32 = Color32::from_rgb(0xD9, 0x77, 0x57);
/// Unsaved changes — the clay text colour, darker so it reads as text.
pub const CLAY_TEXT: Color32 = Color32::from_rgb(0x9C, 0x4A, 0x2F);
pub const BLOCK: Color32 = Color32::from_rgb(0xA3, 0x3B, 0x3B);
pub const BLOCK_WASH: Color32 = Color32::from_rgb(0xF3, 0xDE, 0xDA);
pub const CAUTION: Color32 = Color32::from_rgb(0x7A, 0x5A, 0x00);
pub const CAUTION_WASH: Color32 = Color32::from_rgb(0xF7, 0xEF, 0xD6);
pub const DONE: Color32 = Color32::from_rgb(0x3F, 0x6B, 0x31);
/// Munsell-ish soil tints the design uses for finds and strata.
pub const FIG: Color32 = Color32::from_rgb(0xC4, 0x66, 0x86);
pub const OLIVE: Color32 = Color32::from_rgb(0x78, 0x8C, 0x5D);
pub const SKY: Color32 = Color32::from_rgb(0x6A, 0x9B, 0xCC);
pub const GCP: Color32 = Color32::from_rgb(0xC9, 0xA8, 0x2D);
pub const FIND: Color32 = Color32::from_rgb(0x6B, 0x4D, 0x9E);
/// The scanned underlay (a pencil survey drawing).
pub const PENCIL: Color32 = Color32::from_rgb(0xA8, 0xA4, 0x9A);

/// What a ribbon tile is doing — design 2.1 section 3: an idle tile in desk colour,
/// the tool in hand in ink, a view that is on in clay, unsaved in the clay text colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileRole {
    /// At rest.
    Idle,
    /// The tool in hand.
    Held,
    /// A view that is on.
    On,
    /// Unsaved changes.
    Unsaved,
}

/// The colour a tile of `role` is drawn in.
pub fn tile_color(role: TileRole) -> Color32 {
    match role {
        TileRole::Idle => DESK,
        TileRole::Held => HAND,
        TileRole::On => CLAY,
        TileRole::Unsaved => CLAY_TEXT,
    }
}

/// The text colour that reads on `bg`.
pub fn on(bg: Color32) -> Color32 {
    // Relative luminance, enough to pick ink or ground for a flat fill.
    let [r, g, b, _] = bg.to_array();
    let l = 0.2126 * f32::from(r) + 0.7152 * f32::from(g) + 0.0722 * f32::from(b);
    if l > 140.0 { INK } else { GROUND }
}

/// The 「준비 중」 (not built yet) grey. Design 2.1 section 3: a feature that does not
/// exist yet is shown dimmed, in words, never as a button that does nothing.
pub const PENDING: Color32 = FAINT;

/// A row of text in `size`, in `color`, with the interface's own faces.
pub fn text(ui: &mut egui::Ui, s: impl Into<String>, size: f32, color: Color32) -> egui::Response {
    ui.label(egui::RichText::new(s).size(size).color(color))
}

/// A section heading: the mock-up sets it in the semibold face, one step up from the body.
pub fn heading(ui: &mut egui::Ui, s: impl Into<String>) -> egui::Response {
    ui.label(egui::RichText::new(s).strong().size(12.5).color(INK))
}

/// A hairline between cells (Strata's rule: thin rules and lead-ins, no boxes).
pub fn rule(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 0.0, LINE);
}

/// A cell frame: a surface with a hairline, at [`RADIUS_SMALL`].
pub fn cell(fill: Color32) -> egui::Frame {
    egui::Frame::NONE.fill(fill).corner_radius(RADIUS_SMALL).inner_margin(egui::Margin::symmetric(8, 5)).stroke(egui::Stroke::new(1.0, LINE))
}

/// A 「준비 중」 line, dimmed — how a feature that arrives in a later task is shown.
pub fn pending(ui: &mut egui::Ui, what: &str) {
    text(ui, format!("준비 중 · {what}"), 11.5, PENDING);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn heights_and_radii_are_the_designed_values() {
        assert_eq!((CAPTION, RIBBON_TILE, CONTROL_BAR, TAB, STATUS), (32.0, 44.0, 40.0, 32.0, 32.0));
        assert_eq!((RADIUS_SMALL, RADIUS_LARGE), (8.0, 12.0));
    }

    #[test]
    fn ink_reads_on_ground_and_ground_reads_on_ink() {
        assert_eq!(on(GROUND), INK);
        assert_eq!(on(SURFACE), INK);
        assert_eq!(on(HAND), GROUND);
    }

    #[test]
    fn the_drawing_book_is_wider_than_a_tool_column() {
        const { assert!(BOOK_W > TOOL_W) };
        const { assert!(INSPECTOR_W > BOOK_W) };
    }
}
