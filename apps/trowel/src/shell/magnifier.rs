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
//!
//! # J002 — the drag
//!
//! «왼쪽으로 끌면 확대 끈 만큼 커짐 누른 자리를 가운데 두고 커집니다. 누른 채 가만히 있으면
//! 천천히 계속 커집니다» · «오른쪽으로 끌면 축소 끈 만큼 작아짐 방향은 환경설정에서 바꿀 수
//! 있습니다 (Illustrator는 오른쪽이 확대)» (spec §10, full-spec.txt:1971-1973).
//!
//! The engine's own Zoom tool already scrubs — Performance › Animated Zoom drags it to the
//! right to zoom in — and that direction is the reference app's, not Trowel's, so the drag is
//! polled here instead, about the press and in the direction the preferences set
//! (`trowel.magnifierDrag`, `crates/trowel-engine/src/cmd/magnifier.rs`). The canvas is left
//! out of it: the host takes the magnifier's press out of the frame's input in
//! `raw_input_hook` ([`Hold::take_pointer`]), so the Zoom tool's own drag never runs
//! alongside this one.

use egui::Key;
use trowel_engine::cmd::magnifier::Side;
use vectorcraft_ui_egui::VectorcraftApp;
use vectorcraft_ui_egui::View;
use vectorcraft_ui_egui::canvas::Xf;

/// The tool the magnifier key lends: the Zoom tool (돋보기).
const MAGNIFIER_TOOL: &str = "zoom";

/// J002: how much one point of sideways drag zooms (100 points on the zooming-in side double
/// it: the zoom is `exp(points × DRAG_ZOOM)`).
const DRAG_ZOOM: f64 = std::f64::consts::LN_2 / 100.0;
/// J002: how far the pointer moves before a press on the magnifier is a drag, in points.
const DRAG_SLOP: f32 = 3.0;
/// J002: how long a press on the magnifier is held still before it zooms on, in seconds…
const HOLD_DELAY: f32 = 0.3;
/// …and how fast it zooms then (as `exp(seconds × HOLD_ZOOM)`: twice as close each second).
const HOLD_ZOOM: f64 = std::f64::consts::LN_2;
/// J002: the zoom range the magnifier drags within (the canvas's own `zoom_about` clamp).
const MIN_ZOOM: f64 = 0.0313;
const MAX_ZOOM: f64 = 640.0;

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
    /// J002: which way a drag of the magnifier zooms in — the preferences («방향은
    /// 환경설정에서 바꿀 수 있습니다», design 2.1 §10), which the host pushes in each frame.
    zoom_in: Side,
    /// J002: the press the magnifier took out of the frame's input, while it is held.
    pressed: Option<Scrub>,
}

/// J002: a press on the magnifier being dragged or held still.
struct Scrub {
    /// Where the press was, in screen points — the point that stays put («누른 자리를 가운데
    /// 두고 커집니다»).
    start: egui::Pos2,
    /// How long the press has been held so far, in seconds.
    held: f32,
    /// Whether the pointer has moved far enough for the press to be a drag.
    moved: bool,
}

impl Hold {
    /// J002: which way a drag of the magnifier zooms in.
    pub fn set_zoom_in(&mut self, side: Side) {
        self.zoom_in = side;
    }

    /// J002: this frame's pointer input, with a press on the magnifier taken out of it.
    ///
    /// The host runs this in `raw_input_hook`, before egui sees the input: left in, the Zoom
    /// tool's own press would drag a marquee (or, with Animated Zoom as it is on by default,
    /// scrub its own way) alongside this. The press and its release are taken; the moves that
    /// carry the pointer stay, so the cursor and the hover keep up. A press off the canvas is
    /// not the magnifier's: the panel or ribbon it lands on keeps it.
    ///
    /// Once the magnifier has taken the press it owns the button, so it takes the release
    /// wherever that comes up — over a panel, over the ribbon, outside the canvas. The release
    /// reaches nobody if it is left in (egui never saw the press, so none of its widgets began
    /// an interaction to end), and the magnifier would go on holding a press whose button is
    /// already up: «누른 채 가만히 있으면 천천히 계속 커집니다» would keep zooming, with nothing
    /// pressed, at the full rate. The pointer leaving the window is *not* such an end: the
    /// button can still be down, and coming back carries the drag on.
    pub fn take_pointer(&mut self, app: &VectorcraftApp, raw: &mut egui::RawInput) {
        let Some(canvas) = app.canvas_rect else { return };
        if app.session.tool_id() != MAGNIFIER_TOOL {
            return;
        }
        let mut mine = self.pressed.is_some();
        let mut press = None;
        let mut released = false;
        raw.events.retain(|e| match e {
            egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, .. } => {
                // Not on the canvas: the panel's press is its own — but the release of a press
                // this magnifier is already holding is this one's, wherever it comes up.
                if !canvas.contains(*pos) && !(mine && !*pressed) {
                    return true;
                }
                if *pressed {
                    press = Some(*pos);
                    mine = true;
                } else {
                    released = true;
                    mine = false;
                }
                false
            }
            _ => true,
        });
        // A press and its release in one frame is a click: it drags nothing.
        if let Some(at) = press {
            self.pressed = Some(Scrub { start: at, held: 0.0, moved: false });
        }
        if released {
            self.pressed = None;
        }
    }

    /// One frame of the magnifier key. Runs before `VectorcraftApp::logic`, so the tool
    /// recorded on the press is the one in hand before the Z shortcut's own dispatch.
    pub fn update(&mut self, app: &mut VectorcraftApp, ctx: &egui::Context) {
        // J002: the drag on the magnifier, whatever the magnifier key is doing — a dialog or a
        // tool mid-gesture takes the keyboard, not the pointer.
        self.drag(app, ctx);
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

    /// J002: one frame of the press on the magnifier. Dragged sideways it zooms by the drag,
    /// held still it zooms on («누른 채 가만히 있으면 천천히 계속 커집니다») — both about
    /// where it was pressed («누른 자리를 가운데 두고 커집니다», design 2.1 §10).
    fn drag(&mut self, app: &mut VectorcraftApp, ctx: &egui::Context) {
        // The magnifier let go while the pointer is still down (Z released mid-drag): the
        // pointer belongs to the tool in hand now.
        if app.session.tool_id() != MAGNIFIER_TOOL {
            self.pressed = None;
            return;
        }
        let Some(scrub) = self.pressed.as_mut() else { return };
        let Some(rect) = app.canvas_rect else { return };
        // A zoom that goes on with the pointer held still needs the next frame to come.
        ctx.request_repaint();
        let (dt, dx, at, alt) = ctx.input(|i| (i.unstable_dt, i.pointer.delta().x, i.pointer.latest_pos(), i.modifiers.alt));
        // A stalled frame counts a quarter second at most, as the canvas's own scrub has it.
        let dt = dt.min(0.25);
        scrub.held += dt;
        scrub.moved = scrub.moved || at.is_some_and(|p| (p - scrub.start).length() > DRAG_SLOP);
        let start = scrub.start;
        // The drag's own amount, in the direction the preferences set: left zooms in unless
        // they say otherwise (§10: «방향은 환경설정에서 바꿀 수 있습니다»).
        let side = match self.zoom_in {
            Side::Left => -1.0,
            Side::Right => 1.0,
        };
        let ln = if scrub.moved {
            f64::from(dx) * DRAG_ZOOM * side
        } else if scrub.held > HOLD_DELAY {
            // Alt holds it the other way, as the Zoom tool's own scrub has it.
            f64::from(dt) * HOLD_ZOOM * if alt { -1.0 } else { 1.0 }
        } else {
            0.0
        };
        if ln == 0.0 || !ln.is_finite() {
            return;
        }
        let Some(vm) = app.view_mut() else { return };
        let zoom = vm.zoom * ln.exp();
        zoom_about(vm, rect, start, zoom);
    }
}

/// Zoom the view `vm` of the canvas `rect` to `zoom` (clamped to the zoom range), keeping the
/// document point under screen point `at` where it is. The canvas's own `zoom_about` is
/// private to ui-egui, so the arithmetic is here.
fn zoom_about(vm: &mut View, rect: egui::Rect, at: egui::Pos2, zoom: f64) {
    let pinned = Xf::new(rect, vm).to_doc(at);
    vm.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    vm.center += pinned - Xf::new(rect, vm).to_doc(at);
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
        let mut raw = egui::RawInput { screen_rect: Some(screen()), events, ..Default::default() };
        // `raw_input_hook` runs before egui sees the input, which is where the magnifier's
        // press is taken out of it (J002).
        hold.take_pointer(app, &mut raw);
        let mut out = ctx.run_ui(raw, |ui| {
            hold.update(app, ui.ctx());
            app.logic(ui.ctx());
        });
        // A headless frame uploads no textures; the deltas it reports are dropped (the way
        // ui-egui's own frame tests do).
        out.textures_delta.clear();
    }

    /// The window the frames run on, 800 × 600 as ui-egui's own canvas tests draw it.
    fn screen() -> egui::Rect {
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))
    }

    /// The canvas the window lays out in it. A headless frame draws no canvas, so a drag test
    /// gives the app one the way `canvas::show` does.
    fn canvas() -> egui::Rect {
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))
    }

    /// The primary pointer button at `pos` going down or coming up.
    fn pointer(pos: egui::Pos2, pressed: bool) -> Event {
        Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
    }

    /// Press the magnifier at `at`, move the pointer by `by`, hold it there for `frames`
    /// frames (a 60th of a second each) and let go — through `frames`, so the same drag runs
    /// on a bare frame or on one that draws the canvas.
    #[allow(clippy::too_many_arguments)]
    fn drag(
        app: &mut VectorcraftApp,
        ctx: &Context,
        hold: &mut Hold,
        at: egui::Pos2,
        by: egui::Vec2,
        held_frames: usize,
        frames: fn(&mut VectorcraftApp, &Context, Vec<Event>, &mut Hold),
    ) {
        frames(app, ctx, vec![Event::PointerMoved(at)], hold);
        frames(app, ctx, vec![pointer(at, true)], hold);
        frames(app, ctx, vec![Event::PointerMoved(at + by)], hold);
        for _ in 0..held_frames {
            frames(app, ctx, vec![], hold);
        }
        frames(app, ctx, vec![pointer(at + by, false)], hold);
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

    /// One frame the window draws: the host takes the magnifier's press out of the input, egui
    /// runs the frame, and the canvas the window puts in it draws — the order `main.rs` runs
    /// them in, so the Zoom tool's own drag (Animated Zoom is on by default) is in the loop.
    fn canvas_frame(app: &mut VectorcraftApp, ctx: &Context, events: Vec<Event>, hold: &mut Hold) {
        let mut raw = egui::RawInput { screen_rect: Some(screen()), events, ..Default::default() };
        hold.take_pointer(app, &mut raw);
        let mut out = ctx.run_ui(raw, |ui| {
            hold.update(app, ui.ctx());
            app.logic(ui.ctx());
            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| vectorcraft_ui_egui::canvas::show(app, ui));
        });
        out.textures_delta.clear();
    }

    /// Spec §10: «왼쪽으로 끌면 확대 끈 만큼 커짐 누른 자리를 가운데 두고 커집니다» — dragged
    /// 100 points left the magnifier doubles, about where it was pressed.
    #[test]
    fn a_drag_left_of_the_magnifier_zooms_in_about_the_press() {
        let mut app = app_with("zoom");
        let ctx = Context::default();
        let mut hold = Hold::default();
        app.canvas_rect = Some(canvas());
        // The window fits the view the first time the drawing shows; a headless frame draws
        // no canvas, so the view is materialised the way `view_mut` does.
        assert!(app.view_mut().is_some(), "the drawing has a view to zoom");
        let rect = canvas();
        let at = rect.center() - egui::vec2(100.0, 50.0);
        let z0 = app.view_info().zoom;
        let pinned = vectorcraft_ui_egui::canvas::Xf::new(rect, app.view().unwrap()).to_doc(at);
        drag(&mut app, &ctx, &mut hold, at, egui::vec2(-100.0, 0.0), 0, frame);
        let z1 = app.view_info().zoom;
        assert!((z1 / z0 - 2.0).abs() < 1e-6, "100 points left doubles the zoom: {z0} → {z1}");
        let moved = vectorcraft_ui_egui::canvas::Xf::new(rect, app.view().unwrap()).to_doc(at).distance(pinned);
        assert!(moved < 1e-6, "about the press: {moved}");
    }

    /// «오른쪽으로 끌면 축소 끈 만큼 작아짐» — 100 points right halves it, about the press.
    #[test]
    fn a_drag_right_of_the_magnifier_zooms_out_by_the_drag() {
        let mut app = app_with("zoom");
        let ctx = Context::default();
        let mut hold = Hold::default();
        app.canvas_rect = Some(canvas());
        assert!(app.view_mut().is_some(), "the drawing has a view to zoom");
        let rect = canvas();
        let at = rect.center() + egui::vec2(60.0, 40.0);
        let z0 = app.view_info().zoom;
        let pinned = vectorcraft_ui_egui::canvas::Xf::new(rect, app.view().unwrap()).to_doc(at);
        drag(&mut app, &ctx, &mut hold, at, egui::vec2(100.0, 0.0), 0, frame);
        let z1 = app.view_info().zoom;
        assert!((z1 / z0 - 0.5).abs() < 1e-6, "100 points right halves the zoom: {z0} → {z1}");
        let moved = vectorcraft_ui_egui::canvas::Xf::new(rect, app.view().unwrap()).to_doc(at).distance(pinned);
        assert!(moved < 1e-6, "about the press: {moved}");
    }

    /// «누른 채 가만히 있으면 천천히 계속 커집니다» — held still it zooms on, about the press.
    #[test]
    fn holding_the_magnifier_still_zooms_in_slowly_about_the_press() {
        let mut app = app_with("zoom");
        let ctx = Context::default();
        let mut hold = Hold::default();
        app.canvas_rect = Some(canvas());
        assert!(app.view_mut().is_some(), "the drawing has a view to zoom");
        let rect = canvas();
        let at = rect.center();
        let z0 = app.view_info().zoom;
        let pinned = vectorcraft_ui_egui::canvas::Xf::new(rect, app.view().unwrap()).to_doc(at);
        // A third of a second held still is not a zoom yet: the press is still a click.
        drag(&mut app, &ctx, &mut hold, at, egui::Vec2::ZERO, 6, frame);
        assert!(app.view_info().zoom == z0, "held under the delay, nothing zooms");
        drag(&mut app, &ctx, &mut hold, at, egui::Vec2::ZERO, 40, frame);
        let z1 = app.view_info().zoom;
        assert!(z1 > z0 * 1.2, "held still, it zooms on: {z0} → {z1}");
        let moved = vectorcraft_ui_egui::canvas::Xf::new(rect, app.view().unwrap()).to_doc(at).distance(pinned);
        assert!(moved < 1e-6, "about the press: {moved}");
    }

    /// «방향은 환경설정에서 바꿀 수 있습니다» — flipped, the other side zooms in and this one
    /// zooms out («Illustrator는 오른쪽이 확대»).
    #[test]
    fn the_flip_in_the_preferences_swaps_the_two_sides() {
        let mut app = app_with("zoom");
        let ctx = Context::default();
        let mut hold = Hold::default();
        app.canvas_rect = Some(canvas());
        assert!(app.view_mut().is_some(), "the drawing has a view to zoom");
        hold.set_zoom_in(trowel_engine::cmd::magnifier::Side::Right);
        let at = canvas().center();
        let z0 = app.view_info().zoom;
        drag(&mut app, &ctx, &mut hold, at, egui::vec2(100.0, 0.0), 0, frame);
        assert!((app.view_info().zoom / z0 - 2.0).abs() < 1e-6, "to the right, it zooms in");
        drag(&mut app, &ctx, &mut hold, at, egui::vec2(-100.0, 0.0), 0, frame);
        assert!((app.view_info().zoom / z0 - 1.0).abs() < 1e-6, "and back to the left, out");
    }

    /// The magnifier's press is the magnifier's: with the Zoom tool's own drag left in the
    /// input it would zoom to the area dragged across (or scrub it its own way) alongside, so
    /// the press and its release are taken and the moves that carry the pointer stay.
    #[test]
    fn a_press_on_the_magnifier_is_not_the_zoom_tools_own_drag() {
        let mut app = app_with("zoom");
        let mut hold = Hold::default();
        app.canvas_rect = Some(canvas());
        let at = canvas().center();
        let mut raw = egui::RawInput {
            screen_rect: Some(screen()),
            events: vec![pointer(at, true), Event::PointerMoved(at + egui::vec2(-80.0, 0.0)), pointer(at, false)],
            ..Default::default()
        };
        hold.take_pointer(&app, &mut raw);
        let left: Vec<_> = raw.events.iter().filter(|e| matches!(e, Event::PointerButton { .. })).collect();
        assert_eq!(left.len(), 0, "the press and its release never reach egui: {left:?}");
        assert_eq!(raw.events.len(), 1, "the move stays, so the pointer is still where the user left it");
    }

    /// A press off the canvas is not the magnifier's: the panel or ribbon it lands on keeps
    /// it, as it does with the Zoom tool in hand.
    #[test]
    fn a_press_off_the_canvas_is_not_the_magnifiers() {
        let mut app = app_with("zoom");
        let mut hold = Hold::default();
        let rect = canvas();
        app.canvas_rect = Some(rect);
        let outside = egui::pos2(rect.max.x + 40.0, rect.center().y);
        let mut raw = egui::RawInput { screen_rect: Some(screen()), events: vec![pointer(outside, true)], ..Default::default() };
        hold.take_pointer(&app, &mut raw);
        assert_eq!(raw.events.len(), 1, "a press outside the canvas stays in the input");
    }

    /// The press ends wherever the button comes up: only a release *on* the canvas is taken out
    /// of the input (a press on the panel is the panel's), so a release over the panel — or the
    /// window losing the pointer — has to end the magnifier's press as well. Otherwise the press
    /// goes on being held after the button is up, and «누른 채 가만히 있으면 천천히 계속
    /// 커집니다» keeps zooming with nothing pressed at all.
    #[test]
    fn a_release_off_the_canvas_ends_the_press() {
        let mut app = app_with("zoom");
        let ctx = Context::default();
        let mut hold = Hold::default();
        let rect = canvas();
        app.canvas_rect = Some(rect);
        assert!(app.view_mut().is_some(), "the drawing has a view to zoom");
        // Pressed at the canvas's edge and let go over the panel beside it, without travelling
        // far enough to be a drag — so the hold is what would go on.
        let at = egui::pos2(rect.right() - 1.0, rect.center().y);
        let off = egui::pos2(rect.right() + 1.0, rect.center().y);
        frame(&mut app, &ctx, vec![Event::PointerMoved(at)], &mut hold);
        frame(&mut app, &ctx, vec![pointer(at, true)], &mut hold);
        frame(&mut app, &ctx, vec![Event::PointerMoved(off), pointer(off, false)], &mut hold);
        let up = app.view_info().zoom;
        for _ in 0..60 {
            frame(&mut app, &ctx, vec![], &mut hold);
        }
        let after = app.view_info().zoom;
        assert_eq!(after, up, "the button is up, so nothing zooms on: {up} → {after}");
    }

    /// The drag is the zoom, with the canvas drawn in the same frame: the Zoom tool's own drag
    /// scrubs to the right (Animated Zoom is on by default), so a drag left doubling the zoom
    /// exactly is the magnifier's alone.
    #[test]
    fn the_magnifier_drags_without_the_zoom_tool_zooming_too() {
        let mut app = app_with("zoom");
        let ctx = Context::default();
        let mut hold = Hold::default();
        canvas_frame(&mut app, &ctx, vec![], &mut hold);
        let rect = app.canvas_rect.expect("the canvas laid out");
        let at = rect.center() - egui::vec2(100.0, 50.0);
        let z0 = app.view_info().zoom;
        drag(&mut app, &ctx, &mut hold, at, egui::vec2(-100.0, 0.0), 0, canvas_frame);
        let z1 = app.view_info().zoom;
        assert!((z1 / z0 - 2.0).abs() < 1e-6, "100 points left doubles the zoom: {z0} → {z1}");
    }
}
