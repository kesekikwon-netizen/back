//! Trowel's menu bar (design 2.1 section 3).
//!
//! The design's row is 파일 편집 오브젝트 문자 선택 효과 **도면** 보기 창 도움말 — VectorCraft's
//! own menus, in the same order, with one addition: `도면` (Drawing), which holds the
//! archaeological operations as they arrive.
//!
//! VectorCraft's menu *model* is reused whole — [`menus::menu_tree_named`] gives the same tree,
//! [`menus::entry`] turns each item into its label, shortcut, enabled and checked state with the
//! user's overrides applied, and [`menus::invoke`] runs what was chosen — so command ids,
//! shortcuts and the control channel are untouched. Only the drawing is Trowel's, because
//! `menus::menu_bar` cannot take a tree of its own.

use egui::{RichText, Ui};
use serde_json::Value;
use vectorcraft_ui_egui::menus::{self, Entry, Item};
use vectorcraft_ui_egui::{VectorcraftApp, tl};

use super::labels;
use super::tokens::{FAINT, INK};

/// The `도면` (Drawing) menu, in the design's order. An item whose task has not landed is a
/// [`Item::Todo`], which the menus draw disabled — the same dimmed 「준비 중」 the ribbon uses.
fn drawing_menu() -> Vec<Item> {
    vec![
        Item::Header("New drawing"),
        Item::Todo("Plan…", "Shift+N"),
        Item::Todo("Section…", "Shift+M"),
        Item::Todo("Find…", ""),
        Item::Todo("Plate…", ""),
        Item::Sep,
        Item::Header("Draw"),
        Item::Todo("Section line (A–A′)", "Shift+A"),
        Item::Todo("Elevation lines", "Shift+Z"),
        Item::Todo("Level mark", ""),
        Item::Todo("Trench", ""),
        Item::Sep,
        Item::Header("Drawing elements"),
        Item::Todo("North point", ""),
        Item::Todo("Scale", ""),
        Item::Todo("Legend table", ""),
        Item::Todo("Title block", ""),
        Item::Sep,
        Item::Header("Exchange"),
        Item::Todo("Import DXF…", ""),
        Item::Todo("Export DXF…", ""),
        Item::Todo("Strata bundle…", ""),
    ]
}

/// The bar's tree: VectorCraft's menus, without its application menu (the bar starts at File, as
/// in Illustrator), with `도면` in the design's place between 효과 and 보기.
pub fn tree(app: &VectorcraftApp) -> Vec<(&'static str, Vec<Item>)> {
    let mut tree: Vec<(&'static str, Vec<Item>)> =
        menus::menu_tree_named(app.session.prefs.font_names_in_english).into_iter().filter(|(title, _)| *title != "VectorCraft").collect();
    let at = tree.iter().position(|(title, _)| *title == "View").unwrap_or(tree.len());
    tree.insert(at, ("Drawing", drawing_menu()));
    tree
}

/// The title a menu is drawn with: Trowel's own word for the ten the design names, and
/// VectorCraft's translated title for the rest.
fn title(app: &VectorcraftApp, key: &str) -> String {
    let _ = app;
    match key {
        "File" => labels::MENU_FILE.ko.to_string(),
        "Edit" => labels::MENU_EDIT.ko.to_string(),
        "Object" => labels::MENU_OBJECT.ko.to_string(),
        "Type" => labels::MENU_TYPE.ko.to_string(),
        "Select" => labels::MENU_SELECT.ko.to_string(),
        "Effect" => labels::MENU_EFFECT.ko.to_string(),
        "Drawing" => labels::MENU_DRAWING.ko.to_string(),
        "View" => labels::MENU_VIEW.ko.to_string(),
        "Window" => labels::MENU_WINDOW.ko.to_string(),
        "Help" => labels::MENU_HELP.ko.to_string(),
        other => tl!(other).to_string(),
    }
}

/// Draw the bar; answers where its right edge ended, as the caption buttons need.
pub fn show(app: &mut VectorcraftApp, ui: &mut Ui) -> f32 {
    let mut chosen: Option<(String, Value)> = None;
    egui::MenuBar::new().ui(ui, |ui| {
        let tree = tree(app);
        for (key, items) in &tree {
            let text = RichText::new(title(app, key)).size(13.0).color(INK);
            ui.menu_button(text, |ui| body(app, ui, items, &mut chosen));
        }
    });
    if let Some((id, params)) = chosen {
        menus::invoke(app, &id, params);
    }
    // `MenuBar::ui` answers the rectangle it drew in; the bar runs to the window's right edge.
    ui.max_rect().right()
}

/// A menu's items: what is there now, drawn the way the design draws them — a disabled label
/// for what has not arrived, a check for what is on, the shortcut at the right.
fn body(app: &mut VectorcraftApp, ui: &mut Ui, items: &[Item], chosen: &mut Option<(String, Value)>) {
    for item in items {
        let Some(entry) = menus::entry(app, item) else { continue };
        match entry {
            Entry::Sep => {
                ui.separator();
            }
            Entry::Header(text) => {
                ui.label(RichText::new(text).strong().size(11.0).color(FAINT));
            }
            Entry::Sub(label, children) => {
                if ui.menu_button(format!("{label}  ▸"), |ui| body(app, ui, children, chosen)).response.clicked() {
                    ui.close();
                }
            }
            Entry::Item(row) => {
                let mark = match row.checked {
                    Some(true) => "✓  ",
                    Some(false) => "     ",
                    None => "",
                };
                let text = format!("{mark}{}", row.label);
                let button = egui::Button::new(RichText::new(text).size(13.0)).shortcut_text(row.shortcut.unwrap_or(""));
                let resp = ui.add_enabled(row.enabled, button);
                if resp.clicked()
                    && row.enabled
                    && let Some(target) = row.target()
                {
                    *chosen = Some(target);
                    ui.close();
                }
                if let Some(sc) = row.shortcut {
                    resp.on_hover_text(sc.to_string());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    /// The bar the design shows, in its order, with `도면` between 효과 and 보기. Built from
    /// VectorCraft's tree plus Trowel's own, so the check walks what the code actually makes.
    #[test]
    fn the_bar_is_the_designed_row_with_drawing_in_it() {
        let app = VectorcraftApp::new(vectorcraft_engine::Session::new(), Default::default());
        let names: Vec<String> = tree(&app).iter().map(|(t, _)| title(&app, t)).collect();
        let drawing = names.iter().position(|n| n == "도면").unwrap_or(0);
        assert!(drawing > 0, "도면 is in the bar");
        assert_eq!(names.get(drawing + 1).map(String::as_str), Some("보기"));
        for want in ["파일", "편집", "오브젝트", "문자", "선택", "효과", "창", "도움말"] {
            assert!(names.iter().any(|n| n == want), "{want} is missing from {names:?}");
        }
        assert!(!names.iter().any(|n| n == "VectorCraft"), "the application menu is not in the window bar");
    }

    /// Nothing in the `도면` menu is live yet: every item is a dimmed 「준비 중」 until its task.
    #[test]
    fn every_drawing_item_is_still_a_task() {
        let items = drawing_menu();
        assert!(items.len() > 8, "the menu is written out");
        for i in &items {
            match i {
                Item::Todo(label, _) => assert!(!label.is_empty()),
                Item::Header(label) => assert!(!label.is_empty()),
                Item::Sep => {}
                other => panic!("the drawing menu holds no live command yet: {other:?}"),
            }
        }
    }
}
