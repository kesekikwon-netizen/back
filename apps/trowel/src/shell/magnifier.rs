//! J002b — the magnifier for as long as Z is held (design 2.1 §10, §11).
//!
//! «Z를 쥐는 동안만. 놓으면 쓰던 도구로. 펜으로 그리다가 Z를 쥐고 끌어 보고, 놓으면 다시 펜입니다.»
//! (spec §10, full-spec.txt:1975). The shortcut table (spec §11, full-spec.txt:2106-2108) names
//! it «Z를 쥐는 동안만 돋보기 / Z 길게 / 새로 · J002b», beside the unchanged «잠깐 돋보기
//! Space / Ctrl+Space» the canvas already lends (crates/ui-egui/src/canvas.rs).
//!
//! «Shift 를 먼저 누르고 Z 를 누르면 표고 도구입니다. 돋보기는 Z 만 누릅니다.» (spec §11,
//! full-spec.txt:2170) — so with Shift (or Cmd, Alt, Ctrl) held the key is not the
//! magnifier's alone: it lends nothing, and Shift arriving while the magnifier is lent lets
//! it go.
//!
//! It mirrors the canvas's held-Space behaviour rather than adding a second temporary-tool
//! mechanism to the engine: the key state is polled once a frame, the lent tool is the Zoom
//! tool (돋보기), and putting it back goes through `Session::switch_tool`, the engine's own
//! «back from a temporary tool» path (crates/engine/src/tooling.rs) — the same one the
//! pointer-lent selection tool takes on its release.

use egui::Key;
use vectorcraft_ui_egui::VectorcraftApp;

/// The tool the magnifier key lends: the Zoom tool (돋보기).
const MAGNIFIER_TOOL: &str = "zoom";

/// The key that holds the magnifier: the Zoom tool's own shortcut, when it is one plain key.
/// «편집 › 단축키» may move it (spec §11), and a moved key takes the hold with it; a shortcut
/// carrying a modifier is not a hold and lends nothing.
fn magnifier_key() -> Option<Key> {
    let sc = vectorcraft_ui_egui::shortcut_editor::tool_shortcut(MAGNIFIER_TOOL).and_then(vectorcraft_ui_egui::shortcuts::parse)?;
    if sc.modifiers.shift || sc.modifiers.command || sc.modifiers.ctrl || sc.modifiers.alt {
        return None;
    }
    Some(sc.logical_key)
}

/// The magnifier key's hold: which frame lends the magnifier and which puts the tool back.
/// One per app window, kept by the host (`main.rs`) beside its frame counter.
#[derive(Default)]
pub struct Hold {
    /// The tool held before the magnifier key went down, while it is down.
    lent_from: Option<&'static str>,
}

impl Hold {
    /// One frame of the magnifier key. Runs before `VectorcraftApp::logic`, so the tool
    /// recorded on the press is the one in hand before the Z shortcut's own dispatch.
    pub fn update(&mut self, app: &mut VectorcraftApp, ctx: &egui::Context) {
        let Some(key) = magnifier_key() else { return };
        // The quiet the app's own shortcut handling asks for, mirrored: a dialog or the
        // command palette, a field being typed in or composed in, the Type tool's text, a
        // tool mid-gesture (its drag keeps the keyboard to itself, as the held Space does).
        if app.ui.dialog.is_some() || app.ui.palette_open {
            return;
        }
        if ctx.egui_wants_keyboard_input() || app.session.tool_wants_text() || app.session.tool_composing() || app.session.tool_busy() {
            return;
        }
        let m = ctx.input(|i| i.modifiers);
        let down = ctx.input(|i| i.key_down(key));
        // «돋보기는 Z 만 누릅니다»: with Shift, Cmd, Alt or Ctrl beside it the key is not the
        // magnifier's alone.
        let alone = !(m.shift || m.command || m.ctrl || m.alt);
        match self.lent_from.take() {
            // Still down, still alone: the magnifier goes on lending.
            Some(lent_from) if down && alone => self.lent_from = Some(lent_from),
            // Up, or no longer alone: the tool held before it is back (a hidden perspective
            // grid stays hidden — `switch_tool`, not `select_tool`).
            Some(lent_from) => {
                let view = app.view_info();
                if let Err(e) = app.session.switch_tool(lent_from, view) {
                    log::error!("trowel: back from the magnifier to {lent_from}: {e}");
                }
            }
            // Down, alone, and not already the magnifier: lend it and remember what to put
            // back. The Z shortcut's own dispatch in `logic` lands on the same tool after
            // this, and `select_tool` of the tool in hand is a no-op.
            None if down && alone && app.session.tool_id() != MAGNIFIER_TOOL => {
                let lent_from = app.session.tool_id();
                app.select_tool(MAGNIFIER_TOOL);
                if app.session.tool_id() == MAGNIFIER_TOOL {
                    self.lent_from = Some(lent_from);
                }
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use egui::{Context, Event, Key, Modifiers};
    use serde_json::json;
    use vectorcraft_engine::Session;
    use vectorcraft_ui_egui::VectorcraftApp;

    /// An app with a drawing open and `tool` in hand.
    fn app_with(tool: &str) -> VectorcraftApp {
        let mut app = VectorcraftApp::new(Session::new(), Default::default());
        app.session.execute("file.new", &json!({"width": 400, "height": 300})).unwrap();
        let view = app.view_info();
        app.session.select_tool(tool, view).unwrap();
        app
    }

    /// One frame carrying `events`, the whole frame path running in it the way `main.rs`'s
    /// `logic_frame` runs it: the hook first, then `logic` and the Z shortcut's own dispatch.
    fn frame(app: &mut VectorcraftApp, ctx: &Context, events: Vec<Event>, hold: &mut Hold) {
        let raw = egui::RawInput { events, ..Default::default() };
        let mut out = ctx.run_ui(raw, |ui| {
            hold.update(app, ui.ctx());
            app.logic(ui.ctx());
        });
        // A headless frame uploads no textures; the deltas it reports are dropped (the way
        // ui-egui's own frame tests do).
        out.textures_delta.clear();
    }

    /// `key` going down or coming up. egui keeps the modifier state between frames, so it
    /// travels as its own event ([`Event::ModifiersChanged`]).
    fn key(k: Key, pressed: bool, m: Modifiers) -> Event {
        Event::Key { key: k, physical_key: Some(k), pressed, repeat: false, modifiers: m }
    }

    /// Spec §10: «Z를 쥐는 동안만. 놓으면 쓰던 도구로» — held, the Zoom tool (돋보기) is lent;
    /// let go, the tool held before it is back (here the Pen, «놓으면 다시 펜입니다»).
    /// The frames carry what a real keyboard sends (the key and the text it types), so the
    /// Z shortcut's own dispatch runs in the loop with the hook.
    #[test]
    fn holding_z_lends_the_magnifier_and_letting_go_puts_the_held_tool_back() {
        let mut app = app_with("pen");
        let ctx = Context::default();
        let mut hold = Hold::default();
        frame(&mut app, &ctx, vec![key(Key::Z, true, Modifiers::NONE), Event::Text("z".into())], &mut hold);
        assert_eq!(app.session.tool_id(), "zoom", "Z held is the magnifier");
        frame(&mut app, &ctx, vec![], &mut hold);
        assert_eq!(app.session.tool_id(), "zoom", "and stays it while the key is down");
        frame(&mut app, &ctx, vec![key(Key::Z, false, Modifiers::NONE)], &mut hold);
        assert_eq!(app.session.tool_id(), "pen", "letting go puts the pen back");
    }

    /// A tap is a short hold: the magnifier is lent and put back within a couple of frames.
    #[test]
    fn a_tap_of_the_magnifier_key_puts_the_tool_straight_back() {
        let mut app = app_with("pen");
        let ctx = Context::default();
        let mut hold = Hold::default();
        frame(&mut app, &ctx, vec![key(Key::Z, true, Modifiers::NONE), Event::Text("z".into())], &mut hold);
        assert_eq!(app.session.tool_id(), "zoom", "the magnifier comes on");
        frame(&mut app, &ctx, vec![key(Key::Z, false, Modifiers::NONE)], &mut hold);
        assert_eq!(app.session.tool_id(), "pen", "and goes straight off again");
    }

    /// Spec §11 (full-spec.txt:2170): «Shift 를 먼저 누르고 Z 를 누르면 표고 도구입니다.
    /// 돋보기는 Z 만 누릅니다.» — Shift held with Z is the elevation key, never the magnifier.
    #[test]
    fn shift_held_with_z_is_the_elevation_key_not_the_magnifier() {
        let mut app = app_with("pen");
        let ctx = Context::default();
        let mut hold = Hold::default();
        frame(&mut app, &ctx, vec![Event::ModifiersChanged(Modifiers::SHIFT), key(Key::Z, true, Modifiers::SHIFT)], &mut hold);
        assert_eq!(app.session.tool_id(), "pen", "Shift first, then Z: the elevation tool, not the magnifier");
    }

    /// «돋보기는 Z 만 누릅니다» — Shift joining the held key leaves it no longer Z alone, so the
    /// magnifier lets go and the tool held before is back.
    #[test]
    fn shift_joining_the_held_magnifier_lets_it_go() {
        let mut app = app_with("pen");
        let ctx = Context::default();
        let mut hold = Hold::default();
        frame(&mut app, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], &mut hold);
        assert_eq!(app.session.tool_id(), "zoom", "Z held is the magnifier");
        frame(&mut app, &ctx, vec![Event::ModifiersChanged(Modifiers::SHIFT)], &mut hold);
        assert_eq!(app.session.tool_id(), "pen", "with Shift beside it the key is not Z alone");
        frame(&mut app, &ctx, vec![Event::ModifiersChanged(Modifiers::NONE), key(Key::Z, false, Modifiers::NONE)], &mut hold);
        assert_eq!(app.session.tool_id(), "pen", "letting go changes nothing more");
    }

    /// Already holding the magnifier, the key has nothing to lend or to put back.
    #[test]
    fn the_magnifier_held_over_the_magnifier_keeps_it() {
        let mut app = app_with("zoom");
        let ctx = Context::default();
        let mut hold = Hold::default();
        frame(&mut app, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], &mut hold);
        assert_eq!(app.session.tool_id(), "zoom", "the magnifier stays the tool in hand");
        frame(&mut app, &ctx, vec![key(Key::Z, false, Modifiers::NONE)], &mut hold);
        assert_eq!(app.session.tool_id(), "zoom", "and letting go keeps it");
    }

    /// A dialog open (or the command palette) has the keyboard, as it does for every other
    /// shortcut: the magnifier key lends nothing until it closes.
    #[test]
    fn a_dialog_open_keeps_the_key_from_the_magnifier() {
        let mut app = app_with("pen");
        app.ui.dialog = Some(vectorcraft_ui_egui::state::Dialog::new("move", json!({"dx": "0 pt", "dy": "0 pt"})));
        let ctx = Context::default();
        let mut hold = Hold::default();
        frame(&mut app, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], &mut hold);
        assert_eq!(app.session.tool_id(), "pen", "a dialog open keeps the key from the magnifier");
        frame(&mut app, &ctx, vec![key(Key::Z, false, Modifiers::NONE)], &mut hold);
        assert_eq!(app.session.tool_id(), "pen", "and letting go changes nothing");
    }

    /// The Type tool editing text takes the keyboard for its text, so the key types (Z inserts
    /// «z», as it does today) and lends no magnifier.
    #[test]
    fn the_type_tool_editing_keeps_the_key_to_its_text() {
        let mut app = app_with("type");
        let created = app.session.execute("text.create", &json!({"x": 100, "y": 150, "text": "Hello world", "size": 24})).unwrap();
        let id = vectorcraft_doc::NodeId(created["id"].as_u64().unwrap());
        let at = app.session.active().unwrap().doc.node(id).unwrap().geometric_bounds().unwrap().center();
        let view = app.view_info();
        let ev = || vectorcraft_tools::PointerEvent::new(vectorcraft_tools::PointerKind::Down, at.x, at.y);
        app.session.pointer(&ev(), view).unwrap();
        app.session.pointer(&ev(), view).unwrap();
        assert!(app.session.tool_wants_text(), "the Type tool edits the text");
        let ctx = Context::default();
        let mut hold = Hold::default();
        frame(&mut app, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], &mut hold);
        assert_eq!(app.session.tool_id(), "type", "the key belongs to the text, not the magnifier");
        frame(&mut app, &ctx, vec![key(Key::Z, false, Modifiers::NONE)], &mut hold);
        assert_eq!(app.session.tool_id(), "type", "and letting go changes nothing");
    }

    /// A tool mid-gesture has the keyboard to itself, as it does with the held Space: the key
    /// lends nothing until the gesture is over (the canvas's «Space during a drag doesn't pan»).
    #[test]
    fn a_tool_mid_gesture_keeps_the_key_to_itself() {
        let mut app = app_with("rectangle");
        let ctx = Context::default();
        let mut hold = Hold::default();
        let view = app.view_info();
        let at = (70.0, 70.0);
        let to = (210.0, 170.0);
        let ev = |kind, x: f64, y: f64| vectorcraft_tools::PointerEvent::new(kind, x, y);
        app.session.pointer(&ev(vectorcraft_tools::PointerKind::Down, at.0, at.1), view).unwrap();
        app.session.pointer(&ev(vectorcraft_tools::PointerKind::Drag, to.0, to.1), view).unwrap();
        assert!(app.session.tool_busy(), "mid-drag the tool has the gesture");
        frame(&mut app, &ctx, vec![key(Key::Z, true, Modifiers::NONE)], &mut hold);
        assert_eq!(app.session.tool_id(), "rectangle", "the key lends nothing while the drag runs");
        frame(&mut app, &ctx, vec![key(Key::Z, false, Modifiers::NONE)], &mut hold);
        assert_eq!(app.session.tool_id(), "rectangle", "and letting go leaves it");
        app.session.pointer(&ev(vectorcraft_tools::PointerKind::Up, to.0, to.1), view).unwrap();
    }
}
