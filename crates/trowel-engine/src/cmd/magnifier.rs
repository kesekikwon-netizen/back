//! J002 — the magnifier's drag direction (design 2.1 §10).
//!
//! «왼쪽으로 끌면 확대 끈 만큼 커짐 누른 자리를 가운데 두고 커집니다. 누른 채 가만히 있으면
//! 천천히 계속 커집니다» · «오른쪽으로 끌면 축소 끈 만큼 작아짐 방향은 환경설정에서 바꿀 수
//! 있습니다 (Illustrator는 오른쪽이 확대)» (full-spec.txt:1971-1973).
//!
//! The *dragging* is the shell's — `apps/trowel/src/shell/magnifier.rs` polls the pointer
//! once a frame and zooms about the press. What lives here is the **direction**, because a
//! preference a command flips has to be somewhere a command can reach: the same shape
//! [`super::legend`] keeps its legend in, and the reason Trowel's commands are added to the
//! session at startup instead of written into the engine's registry (see [`super`]).
//!
//! It is persisted by the host in its own preferences file
//! (`%APPDATA%\Trowel\trowel-ui.json`), not in the engine preferences the VectorCraft
//! Preferences dialog shows: a Trowel setting has no answer for the other products this
//! engine carries.

use std::cell::Cell;

use serde_json::{Value, json};
use vectorcraft_engine::cmd::{CommandSpec, always};
use vectorcraft_engine::{EngineError, Result, Session};

/// Which way a drag of the magnifier zooms in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Side {
    /// Dragging left zooms in — Trowel's own default (§10's note: «Illustrator는 오른쪽이 확대»).
    #[default]
    Left,
    /// Dragging right zooms in, as the reference app has it.
    Right,
}

impl Side {
    /// The side a preference value names, or `None` for anything else.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            _ => None,
        }
    }

    /// The value the preference file holds.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

thread_local! {
    /// Which way a drag of the magnifier zooms in. One per thread: the drawing is
    /// single-session, and both the command and the host reach the direction here.
    static SIDE: Cell<Side> = const { Cell::new(Side::Left) };
}

/// Which way a drag of the magnifier zooms in now.
pub fn side() -> Side {
    SIDE.with(Cell::get)
}

/// Which way a drag of the magnifier zooms in from now on.
pub fn set_side(side: Side) {
    SIDE.with(|s| s.set(side));
}

/// The command that flips the direction, and reads it back.
pub const DRAG: &str = "trowel.magnifierDrag";

fn bad(side: &str) -> EngineError {
    EngineError::BadParams { cmd: DRAG.into(), msg: format!("`side` is left or right, not `{side}`") }
}

/// `{side?: left|right}` → `{side}`: which way a drag of the magnifier zooms in, and the side
/// the parameter switches it to. With no `side` the direction stands and is answered, which is
/// how the interface reads it and how the host writes it into its preferences file.
fn drag(_: &mut Session, p: &Value) -> Result<Value> {
    if let Some(want) = p.get("side").and_then(Value::as_str) {
        set_side(Side::parse(want).ok_or_else(|| bad(want))?);
    }
    Ok(json!({ "side": side().as_str() }))
}

/// The magnifier command Trowel adds to the session at startup.
pub fn specs() -> Vec<CommandSpec> {
    vec![CommandSpec {
        id: DRAG,
        label: "Magnifier Drag",
        // Trowel's menus are drawn by the shell, which is where the items are (design 2.1 §3);
        // the legend's commands leave this empty for the same reason.
        menu: &[],
        shortcut: None,
        params: "{side?: left|right} → {side} — which way a drag of the magnifier zooms in, and the side it is switched to (design 2.1 §10)",
        enabled: always,
        run: drag,
        // A preference is not a document edit: it is never a step of the undo.
        journal: false,
    }]
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use serde_json::json;
    use vectorcraft_engine::Session;

    use super::*;

    /// Design 2.1 §10: «왼쪽으로 끌면 확대» — left is Trowel's own default (the reference app
    /// zooms in to the right, which the note names: «Illustrator는 오른쪽이 확대»).
    #[test]
    fn left_is_the_default() {
        assert_eq!(Side::default(), Side::Left);
        assert_eq!(Side::default().as_str(), "left");
        assert_eq!(Side::parse("left"), Some(Side::Left));
        assert_eq!(Side::parse("right"), Some(Side::Right));
        assert_eq!(Side::parse("upwards"), None, "a value that is not a direction names none");
    }

    /// «방향은 환경설정에서 바꿀 수 있습니다» — the direction is a preference, and the command
    /// is what the preferences (and the interface, the control channel and the MCP) set it
    /// with: a side sets it, none answers it.
    #[test]
    fn the_command_sets_the_direction_and_answers_it() {
        let mut s = Session::new();
        s.add_commands(specs());
        set_side(Side::Left);
        let r = s.execute("trowel.magnifierDrag", &json!({"side": "right"})).unwrap();
        assert_eq!(r["side"], json!("right"));
        assert_eq!(side(), Side::Right, "the drag zooms in to the right now");
        assert_eq!(
            s.execute("trowel.magnifierDrag", &json!({})).unwrap()["side"],
            json!("right"),
            "no side answers the direction instead of changing it"
        );
        s.execute("trowel.magnifierDrag", &json!({"side": "left"})).unwrap();
        assert_eq!(side(), Side::Left);
    }

    /// A side that is not a direction is refused by name, and nothing changes.
    #[test]
    fn a_side_that_is_not_a_direction_is_refused() {
        let mut s = Session::new();
        s.add_commands(specs());
        set_side(Side::Left);
        let e = s.execute("trowel.magnifierDrag", &json!({"side": "upwards"})).unwrap_err();
        assert!(e.to_string().contains("left or right"), "{e}");
        assert_eq!(side(), Side::Left, "and the direction is what it was");
    }

    /// The command joins the session at startup, as the legend's do: once per launch, and
    /// reachable through the session like any built-in one.
    #[test]
    fn the_command_is_added_to_the_session_once() {
        let mut s = Session::new();
        assert_eq!(s.add_commands(specs()), 1);
        assert_eq!(s.add_commands(specs()), 0, "a repeated id is dropped");
        assert_eq!(s.execute("trowel.magnifierDrag", &json!({"side": "right"})).unwrap()["side"], json!("right"));
    }
}
