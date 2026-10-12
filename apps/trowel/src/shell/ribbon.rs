//! The one-line ribbon under the menu bar (design 2.1 section 3).
//!
//! The design's ribbon is eight groups of picture tiles. A tile either runs a
//! command VectorCraft already has (열기 · 저장 · 가져오기 · 펜 · 연필 · 칠통 ·
//! 격자 · 자석 · PDF · DXF), or is dimmed and reads 「준비 중 · <task>」 until the
//! task that builds it lands. A dimmed tile is never a button that would do
//! nothing.

use egui::{CornerRadius, Stroke, Ui};
use vectorcraft_ui_egui::VectorcraftApp;

use super::labels;
use super::menubar::tile;
use super::tokens;

/// One ribbon tile.
pub enum Tile {
    /// Runs a command that exists today.
    Run { icon: &'static str, label: labels::Label, id: &'static str },
    /// Toggles a view flag (there is no command id for these; they are UI state).
    Flag { icon: &'static str, label: labels::Label, get: fn(&VectorcraftApp) -> bool, set: fn(&mut VectorcraftApp) },
    /// Not built yet: dimmed, in words (section 3).
    Pending { icon: &'static str, label: labels::Label, task: &'static str },
}

/// A named run of tiles.
pub struct Group {
    pub label: labels::Label,
    pub tiles: &'static [Tile],
}

/// The ribbon, in the order the mock-up shows it.
pub const GROUPS: &[Group] = &[
    Group {
        label: labels::MENU_FILE,
        tiles: &[
            Tile::Run { icon: "folder-open", label: labels::T_OPEN, id: "file.open" },
            Tile::Run { icon: "save", label: labels::T_SAVE, id: "document.save" },
            Tile::Pending { icon: "file-plus", label: labels::T_IMPORT, task: "J007" },
        ],
    },
    Group {
        label: labels::G_NEW_DRAWING,
        tiles: &[
            Tile::Pending { icon: "map", label: labels::T_PLAN, task: "J004" },
            Tile::Pending { icon: "dc-line", label: labels::T_SECTION, task: "J004" },
            Tile::Pending { icon: "circle", label: labels::T_FIND, task: "J004" },
            Tile::Pending { icon: "layout-grid", label: labels::T_PLATE, task: "J004" },
        ],
    },
    Group {
        label: labels::G_UNDERLAY,
        tiles: &[
            Tile::Pending { icon: "align-center-horizontal", label: labels::T_ALIGN_UNDERLAY, task: "J008" },
            Tile::Run { icon: "image", label: labels::T_UNDERLAY, id: "file.place" },
        ],
    },
    Group {
        label: labels::G_DRAW,
        tiles: &[
            Tile::Run { icon: "pen-tool", label: labels::T_PEN, id: "tool:pen" },
            Tile::Run { icon: "pencil", label: labels::T_PENCIL, id: "tool:pencil" },
            Tile::Pending { icon: "dc-line", label: labels::T_SECTION_LINE, task: "J015" },
            Tile::Pending { icon: "dc-reference-point", label: labels::T_LEVEL_MARK, task: "J009" },
            Tile::Run { icon: "dc-live-bucket", label: labels::T_PAINT, id: "tool:bucket" },
        ],
    },
    Group {
        label: labels::G_ELEMENTS,
        tiles: &[
            Tile::Pending { icon: "compass", label: labels::T_NORTH, task: "J012" },
            Tile::Pending { icon: "ruler", label: labels::T_SCALE, task: "J006" },
            Tile::Pending { icon: "dc-list-view", label: labels::T_LEGEND_TABLE, task: "J018" },
            Tile::Pending { icon: "info", label: labels::T_TITLE_BLOCK, task: "J012" },
        ],
    },
    Group {
        label: labels::G_VIEW,
        tiles: &[
            Tile::Flag { icon: "grid-3x3", label: labels::T_GRID, get: grid_on, set: toggle_grid },
            Tile::Flag { icon: "dc-anchor", label: labels::T_SNAP, get: snap_on, set: toggle_snap },
        ],
    },
    Group {
        label: labels::G_EXPORT,
        tiles: &[
            Tile::Run { icon: "printer", label: labels::T_PDF, id: "document.exportPdf" },
            Tile::Run { icon: "dc-line", label: labels::T_DXF, id: "document.exportDxf" },
            Tile::Pending { icon: "share-2", label: labels::T_TO_STRATA, task: "J018" },
        ],
    },
];

pub fn grid_on(app: &VectorcraftApp) -> bool {
    app.ui.view.grid
}

pub fn toggle_grid(app: &mut VectorcraftApp) {
    app.ui.view.grid = !app.ui.view.grid;
}

/// Smart guides are the magnet: snapping while a shape is drawn.
pub fn snap_on(app: &VectorcraftApp) -> bool {
    app.ui.view.smart_guides
}

pub fn toggle_snap(app: &mut VectorcraftApp) {
    app.ui.view.smart_guides = !app.ui.view.smart_guides;
}

/// Height of the line each group names itself on, under its tiles (section 3).
pub const GROUP_NAME_H: f32 = 16.0;

pub fn show(app: &mut VectorcraftApp, ui: &mut Ui) {
    let lang = app.ui_language().code();
    egui::Panel::top("trowel_ribbon")
        .exact_size(tokens::RIBBON_TILE + GROUP_NAME_H)
        .frame(egui::Frame::NONE.fill(tokens::DESK).stroke(Stroke::new(1.0, tokens::LINE)))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(3.0, 0.0);
            ui.horizontal(|ui| {
                for (i, group) in GROUPS.iter().enumerate() {
                    // A hairline between groups, never a box (Strata's rule).
                    if i > 0 {
                        ui.add_space(6.0);
                        let (r, _) = ui.allocate_exact_size(egui::vec2(1.0, tokens::RIBBON_TILE - 10.0), egui::Sense::hover());
                        ui.painter().rect_filled(r, CornerRadius::ZERO, tokens::EDGE);
                        ui.add_space(4.0);
                    }
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            for t in group.tiles {
                                let clicked = match t {
                                    Tile::Run { icon, label, .. } => tile(ui, icon, label.text(lang), false, false).clicked(),
                                    Tile::Flag { icon, label, get, set } => {
                                        let held = get(app);
                                        if tile(ui, icon, label.text(lang), held, false).clicked() {
                                            set(app);
                                        }
                                        false
                                    }
                                    Tile::Pending { icon, label, task } => {
                                        tile(ui, icon, label.text(lang), false, true)
                                            .on_hover_text(format!("{} · {task}", labels::NOT_READY.text(lang)));
                                        false
                                    }
                                };
                                if clicked {
                                    run(app, t);
                                }
                            }
                        });
                        // The group's name under its tiles, as in the mock-up.
                        let name = group.label.text(lang);
                        let galley = ui.painter().layout_no_wrap(name.into(), egui::FontId::proportional(9.5), tokens::FAINT);
                        ui.painter().galley(egui::pos2(ui.cursor().min.x + 1.0, ui.cursor().min.y), galley, tokens::FAINT);
                    });
                }
            });
        });
}

/// What a `Run` tile does. A command that fails is reported in the status bar like
/// any other command; nothing is swallowed.
fn run(app: &mut VectorcraftApp, tile: &Tile) {
    if let Tile::Run { id, .. } = tile
        && let Err(e) = app.run(id, serde_json::json!({}))
    {
        app.status(format!("{id}: {e}"));
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    /// The design lists eight ribbon groups (section 3): 파일 · 새 도면 · 밑그림 ·
    /// 그리기 · 도면 요소 · 보기 · 내보내기, with the file group named 파일.
    #[test]
    fn the_ribbon_has_the_designed_groups() {
        assert_eq!(GROUPS.len(), 7);
        let names: Vec<&str> = GROUPS.iter().map(|g| g.label.ko).collect();
        assert_eq!(names, ["파일", "새 도면", "밑그림", "그리기", "도면 요소", "보기", "내보내기"]);
    }

    /// Nothing is drawn that is not a tile of one of the groups, and every tile has
    /// an icon and a label.
    #[test]
    fn every_tile_is_named() {
        for g in GROUPS {
            assert!(!g.tiles.is_empty(), "{} has no tiles", g.label.ko);
            for t in g.tiles {
                let (icon, label) = match t {
                    Tile::Run { icon, label, .. } | Tile::Flag { icon, label, .. } | Tile::Pending { icon, label, .. } => (*icon, *label),
                };
                assert!(!icon.is_empty());
                assert!(!label.ko.is_empty(), "{label:?} has no Korean text");
                assert!(!label.key.is_empty(), "{label:?} has no English key");
            }
        }
    }

    /// Every tile that runs a command names one that exists.
    #[test]
    fn run_tiles_name_a_command() {
        const KNOWN: &[&str] =
            &["file.open", "document.save", "file.place", "tool:pen", "tool:pencil", "tool:bucket", "document.exportPdf", "document.exportDxf"];
        for g in GROUPS {
            for t in g.tiles {
                if let Tile::Run { id, .. } = t {
                    assert!(KNOWN.contains(id), "{id} is not a command Trowel wires in J003");
                }
            }
        }
    }
}
