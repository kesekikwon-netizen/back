//! Trowel's legend commands — the rule that a legend line **is** a layer (design 2.1 §7).
//!
//! One line of the legend is one layer of the drawing, and the line's weight, dash and hatch are
//! the shape of **every** stroke on that layer. Change the line and the whole layer takes it at
//! once; a stroke the user nudged by hand is left alone and counted, so the line reads 「따로 2」,
//! and one `legend.resetOverrides` puts it back.
//!
//! # Why these commands live here and not in the engine
//!
//! `vectorcraft_engine`'s command labels are VectorCraft **interface strings**: every one of them
//! is scanned into the menu-string catalog and must be translated into each language that claims
//! complete menus. Trowel's archaeology vocabulary — 굴광선, 토층선, 따로 — is not VectorCraft
//! vocabulary, and paying seven catalogs per command for the next fifteen tasks is not a design.
//! So the engine grew a generic hook instead ([`Session::add_commands`]) and the commands are
//! registered by the shell at startup. They behave exactly like built-in ones: enablement, the
//! journal, the undo guard, and the control channel.
//!
//! # Where the legend lives
//!
//! The set is a **library**, not a field of the drawing: §7 pairs each line with a Strata survey
//! layer name and §8 has the sets shared as files. So the state is this module's, per thread, and
//! reaches the interface only through the commands below — the panel never reads it directly.

use std::cell::RefCell;

use serde_json::{Value, json};
use vectorcraft_doc::{Dash, Document, NodeId};
use vectorcraft_engine::cmd::{CommandSpec, always, has_doc};
use vectorcraft_engine::{DocState, EngineError, Result, Session};

use crate::legend::{Group, How, LegendEntry, LegendTable, Where_};
use crate::preset;
use crate::style::{Dash as TrowelDash, Weight};

/// How many nodes one layer walk visits before it stops. A legend line reaching more strokes than
/// this is a broken document, not a drawing; stopping keeps the command responsive and the report
/// honest instead of grinding on a cycle.
const MAX_NODES: usize = 500_000;

/// The legend the session is showing, and which line is current.
#[derive(Debug, Clone)]
pub struct LegendState {
    pub table: LegendTable,
    /// The layer name of the line the user is on.
    pub current: Option<String>,
}

impl Default for LegendState {
    fn default() -> Self {
        Self { table: preset::plan(), current: None }
    }
}

thread_local! {
    /// One legend per thread. The drawing is single-session, and every read and write goes
    /// through the commands below.
    static STATE: RefCell<LegendState> = RefCell::new(LegendState::default());
}

/// The legend, for a read.
pub fn legend() -> LegendState {
    STATE.with(|s| s.borrow().clone())
}

/// The legend, for a change.
fn with_legend<T>(f: impl FnOnce(&mut LegendState) -> T) -> T {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

/// The line bound to `layer`, as a clone: the borrow of the state must not outlive it.
fn line_of(layer: &str) -> Option<LegendEntry> {
    legend().table.line(layer).cloned()
}

// ---------- the small parameter helpers, local so this module depends on no engine internals ----------

fn bad(cmd: &str, msg: impl Into<String>) -> EngineError {
    EngineError::BadParams { cmd: cmd.into(), msg: msg.into() }
}

fn str_param<'a>(p: &'a Value, key: &str) -> Option<&'a str> {
    p.get(key).and_then(Value::as_str)
}

fn f64_or(p: &Value, key: &str, default: f64) -> f64 {
    p.get(key).and_then(Value::as_f64).unwrap_or(default)
}

fn f64_req(p: &Value, key: &str, cmd: &str) -> Result<f64> {
    p.get(key).and_then(Value::as_f64).ok_or_else(|| bad(cmd, format!("missing number `{key}`")))
}

/// An `ids` array of node handles. A value that is not a handle is skipped rather than guessed at.
fn ids_param(p: &Value, key: &str) -> Option<Vec<NodeId>> {
    let arr = p.get(key)?.as_array()?;
    Some(arr.iter().filter_map(Value::as_u64).map(NodeId).collect())
}

/// `CommandSpec` is built by value here; the engine's own `cmd!` macro is crate-private on purpose.
macro_rules! cmd {
    ($id:literal, $label:literal, [$($m:literal),*], $sc:expr, $params:literal, $en:expr, $run:expr) => {
        CommandSpec { id: $id, label: $label, menu: &[$($m),*], shortcut: $sc, params: $params, enabled: $en, run: $run, journal: true }
    };
    (query $id:literal, $label:literal, [$($m:literal),*], $sc:expr, $params:literal, $en:expr, $run:expr) => {
        CommandSpec { id: $id, label: $label, menu: &[$($m),*], shortcut: $sc, params: $params, enabled: $en, run: $run, journal: false }
    };
}

// ---------- the drawing side ----------

/// The layer named `name`, whatever its depth. The binding is by name because the design's own
/// table pairs each line with a Strata survey layer (`유구_면 feature_poly`).
fn layer_id(d: &Document, name: &str) -> Option<NodeId> {
    let mut found = None;
    d.walk(|n| {
        if found.is_none() && n.is_layer() && n.name.as_deref() == Some(name) {
            found = Some(n.id);
        }
    });
    found
}

/// The strokes on `root` and on its sublayers — everything on the line but the line's own layer
/// nodes, which carry no stroke of their own.
fn strokes_under(d: &Document, root: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    let mut seen = 0usize;
    while let Some(id) = stack.pop() {
        seen += 1;
        if seen > MAX_NODES {
            break;
        }
        let Some(kids) = d.children(Some(id)) else { continue };
        for k in kids {
            if k.is_layer() {
                stack.push(k.id);
            } else {
                out.push(k.id);
            }
        }
    }
    out
}

/// The strokes of `layer` in the open drawing, and its node id when the drawing has one.
///
/// A line whose layer is not on the canvas yet has no strokes and no error: a legend line is a
/// *definition*, and an excavation writes its set before the survey file arrives. The commands
/// that need the layer itself (`legend.choose`) ask with [`require_layer`] and say so.
fn line_strokes(st: &DocState, layer: &str) -> (Option<NodeId>, Vec<u64>) {
    let d = &st.doc;
    match layer_id(d, layer) {
        Some(root) => (Some(root), strokes_under(d, root).iter().map(|i| i.0).collect()),
        None => (None, Vec::new()),
    }
}

/// The layer `layer`, or an error saying the drawing has no such layer.
fn require_layer(st: &DocState, cmd: &str, layer: &str) -> Result<NodeId> {
    layer_id(&st.doc, layer).ok_or_else(|| bad(cmd, format!("the drawing has no layer named `{layer}`")))
}

/// The dashes of a line, in the document's own form.
fn dash_of(dash: TrowelDash, weight: Weight) -> Option<Dash> {
    dash.pattern(weight).map(|pattern| Dash { pattern, offset: 0.0, align_corners: false })
}

/// Hand `ids` to `stroke.set` — the tested stroke path, one undo step, the weight and the line
/// type together.
fn restyle(s: &mut Session, ids: &[u64], weight: Weight, dash: TrowelDash) -> Result<Value> {
    let mut params = json!({ "ids": ids, "weight": weight.pt() });
    let pattern = dash_of(dash, weight);
    params["dash"] = pattern.as_ref().map(|d| serde_json::to_value(d).unwrap_or(Value::Null)).unwrap_or(Value::Null);
    s.execute("stroke.set", &params)
}

// ---------- the commands ----------

/// The commands Trowel adds to the engine at startup.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            query "legend.list",
            "Legend Lines",
            [],
            None,
            "{} — the lines of the set being shown, each with its layer, weight, line type, hatch, label size and how many strokes the user set by hand",
            always,
            list
        ),
        cmd!(
            "legend.setGroup",
            "Legend: Set Group",
            [],
            None,
            "{group: plan|section|find} — lay out one of the three sets (§7). This replaces the set being shown; nothing in the drawing changes",
            always,
            set_group
        ),
        cmd!(
            "legend.import",
            "Legend: Read Set File",
            [],
            None,
            "{text} — read a `.trowellegend` set from its text (§8). Refuses a file that is not one, naming the line it could not read",
            always,
            import
        ),
        cmd!(
            query "legend.export",
            "Legend: Write Set File",
            [],
            None,
            "{} — the set as `.trowellegend` text, to be written to the file the user names",
            always,
            export
        ),
        cmd!(
            "legend.choose",
            "Legend: Go To Line",
            [],
            None,
            "{layer, moveSelection?: bool} — the line becomes the current one: the drawing moves to its layer, the pen takes the line's weight and line type, and selected strokes move onto that layer (§7)",
            has_doc,
            choose
        ),
        cmd!(
            "legend.setWeight",
            "Legend: Set Line Weight",
            [],
            None,
            "{layer, weight: pt} — give the line that weight; every stroke on its layer takes it at once, except the ones the user set by hand, which are counted as 「따로」 (§7). 1.0 pt is the survey-area boundary's and the frame's alone",
            has_doc,
            set_weight
        ),
        cmd!(
            "legend.markOverride",
            "Legend: Mark Stroke As Set By Hand",
            [],
            None,
            "{ids, layer} — the strokes the user set by hand: a weight change leaves them alone and the line counts them (§7)",
            has_doc,
            mark_override
        ),
        cmd!(
            "legend.resetOverrides",
            "Legend: Back To The Line",
            [],
            None,
            "{layer} — 「범례대로」: put every stroke the user set by hand back to the line's shape (§7)",
            has_doc,
            reset_overrides
        ),
        cmd!(
            "legend.applyScale",
            "Legend: Weight At Once",
            [],
            None,
            "{where?: set|drawing|book, how: ratio|step, factor?: number, includeOverrides?: bool} — change the weight across the chosen scope at once: by a ratio (「×0.8」) or one step along the ladder (§7). Strokes the user set by hand join only when `includeOverrides` says so",
            has_doc,
            apply_scale
        ),
        cmd!(
            query "legend.layerContents",
            "Legend: Layer Contents",
            [],
            None,
            "{layer} — the strokes on the line's layer, each with its id, its name and the weight it actually carries. The design opens a legend line to see its layer's strokes one by one (§7), so they are read from the drawing rather than assumed from the line",
            has_doc,
            layer_contents
        ),
    ]
}

/// What a line's layer holds right now, for the rows the panel shows under an opened line.
/// A line whose layer is not on the drawing yet simply has none: it is a definition, and the
/// panel says so rather than failing.
fn layer_contents(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "legend.layerContents";
    let layer = str_param(p, "layer").ok_or_else(|| bad(C, "missing `layer`"))?;
    let st = s.doc()?;
    let Some(root) = layer_id(&st.doc, layer) else {
        return Ok(json!({ "layer": layer, "onDrawing": false, "objects": [] }));
    };
    let objects: Vec<Value> = strokes_under(&st.doc, root)
        .iter()
        .filter_map(|id| st.doc.node(*id).map(|n| (id, n)))
        .map(|(id, n)| {
            let stroke = n.appearance.stroke();
            json!({
                "id": id.0,
                "name": n.name.clone(),
                "weight": stroke.map(|st| st.width),
                "dashed": stroke.is_some_and(|st| st.dash.as_ref().is_some_and(|d| d.is_dashed())),
            })
        })
        .collect();
    Ok(json!({ "layer": layer, "onDrawing": true, "objects": objects }))
}

/// The `Group` a name stands for, for the panel's three-set switch.
fn group_of(name: &str) -> Option<Group> {
    match name {
        "plan" | "Plan" => Some(Group::Plan),
        "section" | "Section" => Some(Group::Section),
        "find" | "Find" => Some(Group::Find),
        _ => None,
    }
}

fn list(_s: &mut Session, _p: &Value) -> Result<Value> {
    let l = legend();
    let lines: Vec<Value> = l
        .table
        .in_panel_order()
        .iter()
        .map(|e| {
            json!({
                "name": e.name,
                "layer": e.layer,
                "weight": e.weight.pt(),
                "dash": e.dash,
                "hatch": e.hatch,
                "labelPt": e.label_pt,
                "frequent": e.frequent,
                "print": e.print,
                "fromSurvey": e.from_survey(),
                "separate": l.table.override_count(&e.layer),
            })
        })
        .collect();
    Ok(json!({ "group": l.table.group, "current": l.current, "lines": lines }))
}

fn set_group(_s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "legend.setGroup";
    let name = str_param(p, "group").ok_or_else(|| bad(C, "missing `group`"))?;
    let group = group_of(name).ok_or_else(|| bad(C, format!("`{name}` is not plan, section or find")))?;
    with_legend(|l| {
        l.table = preset::builtin(group);
        l.current = None;
    });
    Ok(json!({ "group": group }))
}

fn import(_s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "legend.import";
    let text = str_param(p, "text").ok_or_else(|| bad(C, "missing `text`"))?;
    let table = crate::library::read(text).map_err(|e| bad(C, e.to_string()))?;
    let count = table.entries.len();
    with_legend(|l| {
        l.table = table;
        l.current = None;
    });
    Ok(json!({ "lines": count }))
}

fn export(_s: &mut Session, _p: &Value) -> Result<Value> {
    let text = crate::library::write(&legend().table);
    Ok(json!({ "text": text, "extension": crate::library::EXTENSION }))
}

fn choose(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "legend.choose";
    let layer = str_param(p, "layer").ok_or_else(|| bad(C, "missing `layer`"))?;
    let entry = line_of(layer).ok_or_else(|| bad(C, format!("no legend line for the layer `{layer}` (see legend.list)")))?;
    let (weight, dash) = (entry.weight, entry.dash);
    // Selected strokes move onto the line's layer, so drawing the line is drawing on it (§7).
    let selection_moves = p.get("moveSelection").and_then(Value::as_bool).unwrap_or(true);
    let moved = if selection_moves { move_selection_to_layer(s, layer)? } else { 0 };
    // The pen takes the line's shape, so the next stroke drawn is already this line's.
    s.paint.stroke_width = weight.pt();
    let root = require_layer(s.doc()?, C, layer)?;
    s.execute("layer.setCurrent", &json!({ "id": root.0 }))?;
    let pattern = dash_of(dash, weight).map(|d| d.pattern);
    s.execute("stroke.set", &json!({ "dash": pattern }))?;
    with_legend(|l| l.current = Some(layer.to_string()));
    Ok(json!({
        "layer": layer,
        "weight": weight.pt(),
        "dash": dash,
        "moved": moved,
        "strokes": line_strokes(s.doc()?, layer).1.len(),
    }))
}

/// Move the selected strokes onto the layer named `layer`; how many moved.
fn move_selection_to_layer(s: &mut Session, layer: &str) -> Result<usize> {
    let st = s.doc()?;
    let ids: Vec<u64> = st.selection.objects.iter().map(|i| i.0).collect();
    if ids.is_empty() {
        return Ok(0);
    }
    let root = require_layer(st, "legend.choose", layer)?;
    s.execute("layer.move", &json!({ "ids": ids, "target": root.0, "place": "inside" }))?;
    Ok(ids.len())
}

fn set_weight(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "legend.setWeight";
    let layer = str_param(p, "layer").ok_or_else(|| bad(C, "missing `layer`"))?;
    let weight = Weight::new(f64_req(p, "weight", C)?).map_err(|e| bad(C, e.to_string()))?;
    let (_, all) = line_strokes(s.doc()?, layer);
    let dash = line_of(layer).ok_or_else(|| bad(C, format!("no legend line for the layer `{layer}` (see legend.list)")))?.dash;
    let change = with_legend(|l| l.table.set_weight(layer, weight, &all)).map_err(|e| bad(C, e.to_string()))?;
    // The change runs even when the drawing has no such layer yet: the line's own value is the
    // point, and the strokes come when the layer does.
    if !change.ids.is_empty() {
        restyle(s, &change.ids, weight, dash)?;
    }
    Ok(json!({
        "layer": layer,
        "weight": weight.pt(),
        "strokes": change.touched(),
        "separate": change.kept,
    }))
}

fn mark_override(_s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "legend.markOverride";
    let layer = str_param(p, "layer").ok_or_else(|| bad(C, "missing `layer`"))?;
    if line_of(layer).is_none() {
        return Err(bad(C, format!("no legend line for the layer `{layer}` (see legend.list)")));
    }
    let ids: Vec<u64> = ids_param(p, "ids").ok_or_else(|| bad(C, "missing `ids`"))?.iter().map(|i| i.0).collect();
    let separate = with_legend(|l| {
        let entry = l.table.overrides.entry(layer.to_string()).or_default();
        for id in ids {
            if !entry.contains(&id) {
                entry.push(id);
            }
        }
        entry.sort_unstable();
        entry.len()
    });
    Ok(json!({ "layer": layer, "separate": separate }))
}

fn reset_overrides(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "legend.resetOverrides";
    let layer = str_param(p, "layer").ok_or_else(|| bad(C, "missing `layer`"))?;
    let (_, all) = line_strokes(s.doc()?, layer);
    let dash = line_of(layer).ok_or_else(|| bad(C, format!("no legend line for the layer `{layer}` (see legend.list)")))?.dash;
    let change = with_legend(|l| l.table.reset_overrides(layer, &all)).map_err(|e| bad(C, e.to_string()))?;
    if !change.ids.is_empty() {
        restyle(s, &change.ids, change.weight, dash)?;
    }
    Ok(json!({ "layer": layer, "restored": change.ids.len(), "weight": change.weight.pt() }))
}

fn apply_scale(s: &mut Session, p: &Value) -> Result<Value> {
    const C: &str = "legend.applyScale";
    let where_ = match str_param(p, "where") {
        None | Some("set") => Where_::Set,
        Some("drawing") => Where_::Drawing,
        Some("book") => Where_::Book,
        Some(o) => return Err(bad(C, format!("`where` must be set, drawing or book, got `{o}`"))),
    };
    let how = match str_param(p, "how") {
        Some("step") => How::OneStep,
        Some("ratio") => How::Ratio(f64_or(p, "factor", 1.0)),
        Some(o) => return Err(bad(C, format!("`how` must be ratio or step, got `{o}`"))),
        None => return Err(bad(C, "missing `how` (ratio or step)")),
    };
    let include = p.get("includeOverrides").and_then(Value::as_bool).unwrap_or(false);

    // Which layers the scope covers, and the strokes each holds in the open drawing.
    let mut targets: Vec<String> = with_legend(|l| l.table.entries.iter().map(|e| e.layer.clone()).collect());
    if matches!(where_, Where_::Drawing | Where_::Book) {
        let mut found: Vec<String> = Vec::new();
        let d = &s.doc()?.doc;
        d.walk(|n| {
            if n.is_layer()
                && let Some(name) = n.name.as_deref()
            {
                found.push(name.to_string());
            }
        });
        targets.retain(|t| found.contains(t));
    }
    let st = s.doc()?;
    let mut ids_by_layer = std::collections::BTreeMap::new();
    for t in &targets {
        let (_, ids) = line_strokes(st, t);
        if !ids.is_empty() {
            ids_by_layer.insert(t.clone(), ids);
        }
    }
    let changes = with_legend(|l| l.table.scale(where_, how, &targets, &ids_by_layer, include)).map_err(|e| bad(C, e.to_string()))?;
    let mut done = Vec::new();
    for c in &changes {
        if let Some(entry) = line_of(&c.layer)
            && !c.ids.is_empty()
        {
            restyle(s, &c.ids, c.weight, entry.dash)?;
        }
        done.push(json!({ "layer": c.layer, "weight": c.weight.pt(), "strokes": c.touched(), "separate": c.kept }));
    }
    Ok(json!({ "lines": done }))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use vectorcraft_engine::cmd::find_command;

    /// A session with the legend registered, as the shell has at launch, and a fresh legend so
    /// one test's set does not leak into the next.
    fn fresh() -> Session {
        with_legend(|l| *l = LegendState::default());
        let mut s = Session::new();
        assert_eq!(s.add_commands(specs()), 10, "the fixture must register the commands");
        s
    }

    /// A drawing with two named layers and a stroke on each — the shape a survey arrives in.
    /// A new object goes on the current layer, so the fixture sets it before drawing.
    fn drawing() -> Session {
        let mut s = fresh();
        s.execute("file.new", &json!({ "width": 1000, "height": 1000 })).unwrap();
        for name in ["feature_poly", "feature_floor"] {
            s.execute("layer.new", &json!({ "name": name })).unwrap();
            let layer = layer_named(&s, name).expect("the layer was just made");
            s.execute("layer.setCurrent", &json!({ "id": layer.0 })).unwrap();
            s.execute("shape.rectangle", &json!({ "x": 10, "y": 10, "width": 50, "height": 50 })).unwrap();
        }
        s
    }

    fn layer_named(s: &Session, name: &str) -> Option<NodeId> {
        layer_id(&s.doc().ok()?.doc, name)
    }

    fn weight_on(s: &Session, id: NodeId) -> Option<f64> {
        s.doc().ok()?.doc.node(id).and_then(|n| n.appearance.stroke().map(|st| st.width))
    }

    fn first_stroke(s: &Session, layer: &str) -> Option<NodeId> {
        strokes_under(&s.doc().ok()?.doc, layer_named(s, layer)?).first().copied()
    }

    /// Opening a legend line shows that layer's strokes one by one (§7). They come from the
    /// drawing, and a stroke the user set by hand carries its own weight, not the line's.
    #[test]
    fn an_opened_line_shows_the_strokes_on_its_layer() {
        let mut s = drawing();
        let layer = layer_named(&s, "feature_poly").unwrap();
        s.execute("layer.setCurrent", &json!({ "id": layer.0 })).unwrap();
        let stray = s.execute("shape.rectangle", &json!({ "x": 300, "y": 300, "width": 40, "height": 40 })).unwrap();
        let stray = NodeId(stray["id"].as_u64().unwrap_or(0));
        s.execute("stroke.set", &json!({ "ids": [stray.0], "weight": 1.6, "dash": [2.0, 1.0] })).unwrap();

        let out = s.execute("legend.layerContents", &json!({ "layer": "feature_poly" })).unwrap();
        assert_eq!(out["onDrawing"], true);
        let objects = out["objects"].as_array().map(Vec::len).unwrap_or(0);
        assert_eq!(objects, 2, "both strokes on the line's layer");
        let row = out["objects"].as_array().unwrap().iter().find(|o| o["id"] == stray.0).unwrap();
        assert_eq!(row["weight"], 1.6, "the weight the stroke actually carries");
        assert_eq!(row["dashed"], true);
        // The other line's layer is its own.
        let other = s.execute("legend.layerContents", &json!({ "layer": "feature_floor" })).unwrap();
        assert_eq!(other["objects"].as_array().map(Vec::len).unwrap_or(0), 1);
    }

    /// A line whose layer is not on the drawing yet has none, and says so instead of failing.
    #[test]
    fn a_line_without_its_layer_has_no_contents() {
        let mut s = drawing();
        let out = s.execute("legend.layerContents", &json!({ "layer": "frame" })).unwrap();
        assert_eq!(out["onDrawing"], false);
        assert_eq!(out["objects"].as_array().map(Vec::len).unwrap_or(0), 0);
        assert!(s.execute("legend.layerContents", &json!({})).is_err(), "a missing `layer` is an error");
    }

    /// Every command is well formed and none of them is a built-in id.
    #[test]
    fn the_legend_commands_are_well_formed() {
        let specs = specs();
        assert_eq!(specs.len(), 10);
        let mut ids: Vec<&str> = specs.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n, "a legend id is used twice");
        for c in &specs {
            assert!(!c.label.is_empty() && !c.params.is_empty(), "{}", c.id);
            assert!(c.id.starts_with("legend."), "{}", c.id);
            assert!(c.menu.is_empty(), "{} is reached from the panel, not a menu", c.id);
            assert!(find_command(c.id).is_none(), "{} would shadow a built-in", c.id);
        }
    }

    /// A session that has registered them runs them like any built-in command.
    #[test]
    fn the_shell_registers_them_and_they_run() {
        with_legend(|l| *l = LegendState::default());
        let mut s = Session::new();
        assert!(s.execute("legend.list", &json!({})).is_err(), "not registered yet");
        assert_eq!(s.add_commands(specs()), 10);
        assert!(s.execute("legend.list", &json!({})).is_ok());
        assert_eq!(s.extra_commands().len(), 10);
    }

    /// Registering twice adds nothing the second time, and a built-in id is never shadowed.
    #[test]
    fn registering_twice_adds_nothing() {
        with_legend(|l| *l = LegendState::default());
        let mut s = Session::new();
        assert_eq!(s.add_commands(specs()), 10);
        assert_eq!(s.add_commands(specs()), 0);
        let mut squatter = specs();
        squatter[0].id = "object.group";
        assert_eq!(s.add_commands(squatter), 0, "a built-in id is not shadowed");
        assert!(!s.extra_commands().iter().any(|c| c.id == "object.group"), "and it is not among the added");
        assert!(find_command("object.group").is_some(), "the built-in is still registered");
    }

    /// A new session shows the plan set (§7), and `legend.list` says so.
    #[test]
    fn a_new_session_shows_the_plan_set() {
        let mut s = drawing();
        let out = s.execute("legend.list", &json!({})).unwrap();
        assert_eq!(out["group"], "plan");
        let lines = out["lines"].as_array().map(Vec::len).unwrap_or(0);
        assert!(lines > 10, "the plan set has {lines} lines");
        assert!(out["current"].is_null(), "no line is current until one is chosen");
    }

    /// The three sets are one command apart.
    #[test]
    fn the_three_sets_are_one_command_apart() {
        let mut s = drawing();
        for (name, first) in [("plan", "feature_poly"), ("section", "section_line"), ("find", "find_axis")] {
            s.execute("legend.setGroup", &json!({ "group": name })).unwrap();
            let out = s.execute("legend.list", &json!({})).unwrap();
            assert_eq!(out["group"], name);
            assert!(out["lines"].as_array().unwrap_or(&vec![]).iter().any(|l| l["layer"] == first), "{name} has no line for {first}");
        }
        assert!(s.execute("legend.setGroup", &json!({ "group": "nope" })).is_err(), "a fourth set does not exist");
    }

    /// ② Changing a line's weight changes every stroke on its layer at once (§7).
    #[test]
    fn changing_a_line_changes_every_stroke_on_its_layer() {
        let mut s = drawing();
        let strokes = strokes_under(&s.doc().unwrap().doc, layer_named(&s, "feature_poly").unwrap());
        assert_eq!(strokes.len(), 1, "the fixture has one stroke on the line");
        let out = s.execute("legend.setWeight", &json!({ "layer": "feature_poly", "weight": 0.2 })).unwrap();
        assert_eq!(out["weight"], 0.2);
        assert_eq!(out["strokes"], 1);
        assert_eq!(weight_on(&s, strokes[0]), Some(0.2), "the stroke took the line's weight");
        // The other line's stroke is untouched.
        assert_ne!(weight_on(&s, first_stroke(&s, "feature_floor").unwrap()), Some(0.2));
    }

    /// A weight change reaches every stroke on the layer, not just the first.
    #[test]
    fn a_line_with_several_strokes_changes_all_of_them() {
        let mut s = drawing();
        let layer = layer_named(&s, "feature_poly").unwrap();
        s.execute("layer.setCurrent", &json!({ "id": layer.0 })).unwrap();
        for i in 0..3 {
            s.execute("shape.rectangle", &json!({ "x": 100 + i * 60, "y": 10, "width": 40, "height": 40 })).unwrap();
        }
        let strokes = strokes_under(&s.doc().unwrap().doc, layer);
        assert_eq!(strokes.len(), 4);
        let out = s.execute("legend.setWeight", &json!({ "layer": "feature_poly", "weight": 0.25 })).unwrap();
        assert_eq!(out["strokes"], 4);
        for id in &strokes {
            assert_eq!(weight_on(&s, *id), Some(0.25));
        }
    }

    /// A stroke the user set by hand keeps what it has, and the line counts it: 「따로 1」.
    #[test]
    fn a_stroke_set_by_hand_is_kept_and_counted() {
        let mut s = drawing();
        let kept = first_stroke(&s, "feature_poly").unwrap();
        s.execute("stroke.set", &json!({ "ids": [kept.0], "weight": 1.6 })).unwrap();
        let out = s.execute("legend.markOverride", &json!({ "layer": "feature_poly", "ids": [kept.0] })).unwrap();
        assert_eq!(out["separate"], 1, "the line says 「따로 1」");
        s.execute("legend.setWeight", &json!({ "layer": "feature_poly", "weight": 0.1 })).unwrap();
        assert_eq!(weight_on(&s, kept), Some(1.6), "the stroke the user set is left alone");
        let listed = s.execute("legend.list", &json!({})).unwrap();
        let line = listed["lines"].as_array().unwrap().iter().find(|l| l["layer"] == "feature_poly").unwrap();
        assert_eq!(line["separate"], 1, "and the line still counts it");
        assert_eq!(line["weight"], 0.1, "the line itself took the new weight");
    }

    /// 「범례대로」 puts the kept strokes back to the line's shape (§7).
    #[test]
    fn back_to_the_line_puts_the_kept_stroke_back() {
        let mut s = drawing();
        let kept = first_stroke(&s, "feature_poly").unwrap();
        s.execute("stroke.set", &json!({ "ids": [kept.0], "weight": 1.6 })).unwrap();
        s.execute("legend.markOverride", &json!({ "layer": "feature_poly", "ids": [kept.0] })).unwrap();
        let out = s.execute("legend.resetOverrides", &json!({ "layer": "feature_poly" })).unwrap();
        assert_eq!(out["restored"], 1);
        assert_eq!(weight_on(&s, kept), Some(0.8), "the line's own weight, 굴광선 0.8");
        let listed = s.execute("legend.list", &json!({})).unwrap();
        let line = listed["lines"].as_array().unwrap().iter().find(|l| l["layer"] == "feature_poly").unwrap();
        assert_eq!(line["separate"], 0, "the count is gone");
    }

    /// The weight-at-once control covers the chosen scope and leaves the kept strokes alone
    /// unless 「따로 바꾼 선도 함께」 says otherwise (§7).
    #[test]
    fn the_weight_at_once_control_covers_the_scope() {
        let mut s = drawing();
        let out = s.execute("legend.applyScale", &json!({ "how": "ratio", "factor": 0.5 })).unwrap();
        assert_eq!(out["lines"].as_array().map(Vec::len).unwrap_or(0), legend().table.entries.len());
        assert_eq!(line_of("feature_poly").map(|e| e.weight.pt()), Some(0.4));

        let mut s = drawing();
        let kept = first_stroke(&s, "feature_poly").unwrap();
        s.execute("stroke.set", &json!({ "ids": [kept.0], "weight": 1.6 })).unwrap();
        s.execute("legend.markOverride", &json!({ "layer": "feature_poly", "ids": [kept.0] })).unwrap();
        s.execute("legend.applyScale", &json!({ "how": "ratio", "factor": 0.5 })).unwrap();
        assert_eq!(weight_on(&s, kept), Some(1.6), "left alone without the switch");
        s.execute("legend.applyScale", &json!({ "how": "step", "includeOverrides": true })).unwrap();
        assert_ne!(weight_on(&s, kept), Some(1.6), "and taken with it when asked");
    }

    /// A ratio that is not a number scales nothing and is refused rather than zeroing a drawing.
    #[test]
    fn a_broken_ratio_is_refused() {
        let mut s = drawing();
        assert!(s.execute("legend.applyScale", &json!({ "how": "ratio", "factor": 0 })).is_err(), "×0 would take every line to nothing");
        assert!(s.execute("legend.applyScale", &json!({ "how": "sideways" })).is_err());
        assert!(s.execute("legend.applyScale", &json!({ "where": "everywhere" })).is_err());
        assert!(s.execute("legend.applyScale", &json!({})).is_err());
        assert_eq!(line_of("feature_poly").map(|e| e.weight.pt()), Some(0.8), "nothing changed");
    }

    /// ① Choosing a line moves the drawing to its layer and gives the pen the line's weight (§7).
    #[test]
    fn choosing_a_line_moves_to_its_layer_and_pens_it() {
        let mut s = drawing();
        let out = s.execute("legend.choose", &json!({ "layer": "feature_floor" })).unwrap();
        assert_eq!(out["layer"], "feature_floor");
        assert_eq!(out["weight"], 0.5);
        assert_eq!(out["strokes"], 1);
        assert_eq!(s.paint.stroke_width, 0.5, "the pen took the line's weight");
        assert_eq!(s.active().and_then(|d| d.current_layer()), Some(layer_named(&s, "feature_floor").unwrap()));
        assert_eq!(legend().current.as_deref(), Some("feature_floor"));
    }

    /// Choosing a line with strokes selected moves them onto it (§7).
    #[test]
    fn choosing_a_line_moves_the_selected_strokes_onto_it() {
        let mut s = drawing();
        let stray = first_stroke(&s, "feature_poly").unwrap();
        s.execute("select.set", &json!({ "ids": [stray.0] })).unwrap();
        let out = s.execute("legend.choose", &json!({ "layer": "feature_floor" })).unwrap();
        assert_eq!(out["moved"], 1, "the selected stroke joined the line");
        let floor = layer_named(&s, "feature_floor").unwrap();
        assert!(strokes_under(&s.doc().unwrap().doc, floor).len() > 1);
    }

    /// Choosing a line with nothing selected leaves the selection alone.
    #[test]
    fn choosing_a_line_with_nothing_selected_moves_nothing() {
        let mut s = drawing();
        s.execute("select.none", &json!({})).ok();
        let out = s.execute("legend.choose", &json!({ "layer": "feature_floor" })).unwrap();
        assert_eq!(out["moved"], 0);
    }

    /// A line for a layer the drawing does not have says so, and the command still records the
    /// weight — the line's value is the point; the strokes come when the layer does.
    #[test]
    fn a_line_without_its_layer_still_takes_its_weight() {
        let mut s = drawing();
        let out = s.execute("legend.setWeight", &json!({ "layer": "frame", "weight": 1.0 })).unwrap();
        assert_eq!(out["strokes"], 0, "there is nothing on it to restyle");
        assert_eq!(line_of("frame").map(|e| e.weight.pt()), Some(1.0));
        assert!(s.execute("legend.choose", &json!({ "layer": "frame" })).is_err(), "but choosing it says the layer is missing");
    }

    /// A line that does not exist is an error naming what is there, not a silent no-op.
    #[test]
    fn an_unknown_layer_is_refused() {
        let mut s = drawing();
        assert!(s.execute("legend.setWeight", &json!({ "layer": "no_such_layer", "weight": 0.5 })).is_err());
        assert!(s.execute("legend.markOverride", &json!({ "layer": "no_such_layer", "ids": [1] })).is_err());
        assert!(s.execute("legend.setWeight", &json!({ "layer": "feature_poly" })).is_err(), "a missing weight is an error");
        assert!(s.execute("legend.setWeight", &json!({ "layer": "feature_poly", "weight": -1 })).is_err());
        assert!(s.execute("legend.markOverride", &json!({ "layer": "feature_poly" })).is_err(), "missing `ids` is an error");
    }

    /// 1.0 pt is the survey-area boundary's and the frame's alone (§7).
    #[test]
    fn the_wide_weight_stays_reserved() {
        let mut s = drawing();
        assert!(s.execute("legend.setWeight", &json!({ "layer": "feature_poly", "weight": 1.0 })).is_err());
        assert!(s.execute("legend.setWeight", &json!({ "layer": "survey_area", "weight": 1.0 })).is_ok(), "the survey area may be 1.0 pt");
    }

    /// ③ The set is a file: export then import gives the same set back (§8).
    #[test]
    fn the_set_survives_a_file_round_trip() {
        let mut s = drawing();
        s.execute("legend.setGroup", &json!({ "group": "section" })).unwrap();
        s.execute("legend.setWeight", &json!({ "layer": "section_line", "weight": 0.5 })).unwrap();
        let text = s.execute("legend.export", &json!({})).unwrap()["text"].as_str().unwrap_or("").to_string();
        with_legend(|l| *l = LegendState::default());
        s.execute("legend.import", &json!({ "text": text })).unwrap();
        assert_eq!(
            s.execute("legend.export", &json!({})).unwrap()["text"].as_str().unwrap_or(""),
            text,
            "the set a colleague reads is the set we wrote"
        );
        assert_eq!(line_of("section_line").map(|e| e.weight.pt()), Some(0.5));
    }

    /// A file that is not a set is refused with a reason, never loaded as an empty set.
    #[test]
    fn a_file_that_is_not_a_set_is_refused() {
        let mut s = drawing();
        let before = s.execute("legend.export", &json!({})).unwrap()["text"].as_str().unwrap_or("").to_string();
        assert!(s.execute("legend.import", &json!({ "text": "{\"format\":\"vcswatches\"}" })).is_err());
        assert!(s.execute("legend.import", &json!({ "text": "not a set" })).is_err());
        assert_eq!(s.execute("legend.export", &json!({})).unwrap()["text"].as_str().unwrap_or(""), before, "the set on show is unchanged");
    }

    /// Every weight change is one undo step: undo leaves the drawing as it was.
    #[test]
    fn a_weight_change_is_one_undo_step() {
        let mut s = drawing();
        let id = first_stroke(&s, "feature_poly").unwrap();
        s.execute("legend.setWeight", &json!({ "layer": "feature_poly", "weight": 0.1 })).unwrap();
        assert_eq!(weight_on(&s, id), Some(0.1));
        s.execute("edit.undo", &json!({})).unwrap();
        assert_ne!(weight_on(&s, id), Some(0.1), "one undo takes it back");
    }

    /// The commands that need a drawing say so rather than failing obscurely.
    #[test]
    fn the_drawing_commands_need_a_document() {
        let specs = specs();
        let mut s = fresh();
        for spec in &specs {
            let off = (spec.enabled)(&s).is_err();
            assert_eq!(
                off,
                spec.id != "legend.list" && spec.id != "legend.setGroup" && spec.id != "legend.import" && spec.id != "legend.export",
                "{}",
                spec.id
            );
            if off {
                assert!(s.execute(spec.id, &json!({ "layer": "feature_poly" })).is_err(), "{} should refuse", spec.id);
            }
        }
        assert!(s.execute("legend.list", &json!({})).is_ok());
        assert!(s.execute("legend.setGroup", &json!({ "group": "find" })).is_ok());
        assert!(s.execute("legend.export", &json!({})).is_ok());
    }
}
