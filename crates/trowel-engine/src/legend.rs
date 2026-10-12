//! The legend, and the rule that a legend line **is** a layer (design 2.1 sections 7, 10 #5).
//!
//! One line of the legend panel is one layer of the drawing. The line's weight, dash and hatch
//! are the shape of **every** stroke on that layer: change the line's weight and every stroke
//! on the layer takes it at once. A stroke the user changed by hand is left alone and counted —
//! the line then reads 「따로 2」, and one 「범례대로」 puts it back (section 7).
//!
//! A line is bound to its layer **by name**, because the design's own table pairs each legend
//! line with a Strata survey layer (`유구_면 feature_poly`, `단면선 section_line`, …) and names
//! the lines that exist only on the drawing `Trowel` (section 7). Nothing here touches the
//! engine or the file format: the table is a value, and the caller runs the commands.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::style::{Dash, Hatch, Weight};

/// Which of the three sets a line belongs to. A new drawing lays one of them out, and each
/// excavation rewrites it (section 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Group {
    /// 평면용 — plans.
    #[default]
    Plan,
    /// 단면·입면용 — sections and elevations.
    Section,
    /// 유물용 — finds.
    Find,
}

impl Group {
    /// The interface's own name for the set.
    pub fn label(self) -> &'static str {
        match self {
            Group::Plan => "Plan",
            Group::Section => "Section",
            Group::Find => "Find",
        }
    }

    /// The three sets, in the order the panel shows them.
    pub const ALL: [Group; 3] = [Group::Plan, Group::Section, Group::Find];
}

/// The layer name a drawing-only line carries: Strata has no such layer, these are lines that
/// exist only on the drawing (section 7).
pub const TROWEL_ONLY: &str = "Trowel";

/// One line of the legend: a layer, and the shape every stroke on it takes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegendEntry {
    /// What the line says, in the drawing's own words (굴광선 (유구 면)).
    pub name: String,
    /// The layer it is: a Strata survey layer's name, or [`TROWEL_ONLY`]. This is the binding,
    /// so two drawings from the same survey share it.
    pub layer: String,
    /// The weight on the report paper, in points.
    pub weight: Weight,
    pub dash: Dash,
    /// The fill inside a cut face.
    pub hatch: Hatch,
    /// The size of the line's own label on paper, in points; `0` when the line is a symbol or a
    /// pattern and writes no text.
    #[serde(default)]
    pub label_pt: f64,
    /// `★`: a line used often, pinned to the top (section 3, decision 6).
    #[serde(default)]
    pub frequent: bool,
    /// Whether the line prints. The underlay and the survey lines are for the screen only.
    #[serde(default = "yes")]
    pub print: bool,
}

fn yes() -> bool {
    true
}

impl LegendEntry {
    /// A line with the given layer, name and weight; solid, no hatch, printing, not pinned.
    pub fn new(name: &str, layer: &str, weight: Weight) -> Self {
        Self {
            name: name.to_string(),
            layer: layer.to_string(),
            weight,
            dash: Dash::Solid,
            hatch: Hatch::None,
            label_pt: 0.0,
            frequent: false,
            print: true,
        }
    }

    /// Whether this line's layer exists in the Strata survey file (as opposed to being a
    /// drawing-only line).
    pub fn from_survey(&self) -> bool {
        self.layer != TROWEL_ONLY
    }
}

/// What a weight change asks the drawing to do.
#[derive(Debug, Clone, PartialEq)]
pub struct StrokeChange {
    /// The layer whose strokes change.
    pub layer: String,
    /// The weight they take.
    pub weight: Weight,
    /// The strokes to restyle.
    pub ids: Vec<u64>,
    /// How many strokes on the layer are left as the user set them.
    pub kept: usize,
}

impl StrokeChange {
    /// How many strokes the change touches.
    pub fn touched(&self) -> usize {
        self.ids.len()
    }
}

/// How the legend's weight-at-once control works (section 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Where_ {
    /// 평면용 범례 — only the lines of the set being shown.
    #[default]
    Set,
    /// 이 도면 — every line the open drawing uses.
    Drawing,
    /// 도면집 전체 — every line of every drawing in the excavation.
    Book,
}

/// How a weight change carries across the lines it covers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum How {
    /// A ratio: 「×0.8」 (section 7).
    Ratio(f64),
    /// One step along the ladder: 「한 단계」.
    OneStep,
}

/// What was wrong with a legend request.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum LegendError {
    #[error("no legend line for the layer `{0}`")]
    NoSuchLayer(String),
    #[error("the set already has a line named `{0}`")]
    DuplicateName(String),
    #[error("1.0 pt is the survey-area boundary's and the frame's alone")]
    WideWeightReserved,
    #[error("a factor that is not a number scales nothing")]
    BadFactor,
}

/// One of the three sets, with the strokes the user set by hand.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LegendTable {
    pub group: Group,
    pub entries: Vec<LegendEntry>,
    /// Strokes changed by hand, by layer. They are the 「따로」 ones, and a weight change leaves
    /// them alone (section 7).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub overrides: BTreeMap<String, Vec<u64>>,
}

impl LegendTable {
    pub fn new(group: Group) -> Self {
        Self { group, entries: Vec::new(), overrides: BTreeMap::new() }
    }

    /// The line bound to `layer`.
    pub fn line(&self, layer: &str) -> Option<&LegendEntry> {
        self.entries.iter().find(|e| e.layer == layer)
    }

    /// The line bound to `layer`, for change.
    pub fn line_mut(&mut self, layer: &str) -> Option<&mut LegendEntry> {
        self.entries.iter_mut().find(|e| e.layer == layer)
    }

    /// Add a line, refusing a second line on the same layer (two lines would both claim it) and
    /// a name already in the set (the panel names lines by text).
    pub fn add(&mut self, entry: LegendEntry) -> Result<(), LegendError> {
        if self.line(&entry.layer).is_some() {
            return Err(LegendError::NoSuchLayer(entry.layer.clone()));
        }
        if self.entries.iter().any(|e| e.name == entry.name) {
            return Err(LegendError::DuplicateName(entry.name.clone()));
        }
        self.entries.push(entry);
        Ok(())
    }

    /// The lines as the panel shows them: the pinned ones (`★`) first, then the rest, each group
    /// in the order the set lays them down (section 3, decision 6).
    pub fn in_panel_order(&self) -> Vec<&LegendEntry> {
        let mut out: Vec<&LegendEntry> = self.entries.iter().filter(|e| e.frequent).collect();
        out.extend(self.entries.iter().filter(|e| !e.frequent));
        out
    }

    /// How many strokes on `layer` the user set by hand — the 「따로 N」 the line shows.
    pub fn override_count(&self, layer: &str) -> usize {
        self.overrides.get(layer).map(|v| v.len()).unwrap_or(0)
    }

    /// Put every stroke of `layer` back to its line's shape (「범례대로」, section 7).
    ///
    /// `ids` are the strokes on the layer; the ones the user had set by hand are returned so the
    /// caller can restyle exactly those.
    pub fn reset_overrides(&mut self, layer: &str, ids: &[u64]) -> Result<StrokeChange, LegendError> {
        let entry = self.line(layer).ok_or_else(|| LegendError::NoSuchLayer(layer.to_string()))?;
        let weight = entry.weight;
        let restored = self.overrides.remove(layer).unwrap_or_default();
        // Only strokes that are still on the layer, and not ones the user has since changed
        // again, are put back.
        let ids: Vec<u64> = ids.iter().copied().filter(|id| restored.contains(id)).collect();
        Ok(StrokeChange { layer: layer.to_string(), weight, ids, kept: 0 })
    }

    /// Give `layer`'s line the weight `weight`, and say which strokes to restyle (section 7:
    /// 「줄의 굵기를 고치면 그 줄의 선이 모두 바뀝니다」).
    ///
    /// `ids` are every stroke on the layer. The ones in the 「따로」 set keep what the user set,
    /// and `kept` says how many that was.
    pub fn set_weight(&mut self, layer: &str, weight: Weight, ids: &[u64]) -> Result<StrokeChange, LegendError> {
        let kept_ids = self.overrides.get(layer).cloned().unwrap_or_default();
        let entry = self.line_mut(layer).ok_or_else(|| LegendError::NoSuchLayer(layer.to_string()))?;
        // 1.0 pt is reserved for the survey-area boundary and the frame (section 7).
        if weight.pt() >= crate::style::WIDE_WEIGHT_PT && !is_wide_layer(layer) {
            return Err(LegendError::WideWeightReserved);
        }
        entry.weight = weight;
        let ids: Vec<u64> = ids.iter().copied().filter(|id| !kept_ids.contains(id)).collect();

        Ok(StrokeChange { layer: layer.to_string(), weight, kept: kept_ids.len(), ids })
    }

    /// The weight-at-once control (section 7): 「어디 · 어떻게 · 값」.
    ///
    /// `layers` are the layers the chosen scope covers. Strokes the user set by hand keep what
    /// they have unless `include_overrides` says otherwise — the panel's 「따로 바꾼 선도 함께」.
    pub fn scale(
        &mut self,
        where_: Where_,
        how: How,
        layers: &[String],
        ids_by_layer: &BTreeMap<String, Vec<u64>>,
        include_overrides: bool,
    ) -> Result<Vec<StrokeChange>, LegendError> {
        let targets: Vec<String> = match where_ {
            Where_::Set => self.entries.iter().map(|e| e.layer.clone()).collect(),
            Where_::Drawing | Where_::Book => layers.to_vec(),
        };
        let mut changes = Vec::new();
        for layer in targets {
            let ids = ids_by_layer.get(&layer).cloned().unwrap_or_default();
            let current = self.line(&layer).ok_or_else(|| LegendError::NoSuchLayer(layer.clone()))?;
            let next = match how {
                How::Ratio(f) => {
                    if !f.is_finite() {
                        return Err(LegendError::BadFactor);
                    }
                    current.weight.scaled(f).map_err(|_| LegendError::BadFactor)?
                }
                How::OneStep => current.weight.one_step_down(),
            };
            if let Some(e) = self.line_mut(&layer) {
                e.weight = next;
            }
            // 「따로 바꾼 선도 함께」: the kept strokes join the change and stop being kept.
            // Without it they keep what the user set, and the line still remembers them.
            let kept_ids: Vec<u64> = if include_overrides {
                self.overrides.remove(&layer).unwrap_or_default()
            } else {
                self.overrides.get(&layer).cloned().unwrap_or_default()
            };
            let kept = kept_ids.len();
            let ids: Vec<u64> = if include_overrides { ids } else { ids.into_iter().filter(|id| !kept_ids.contains(id)).collect() };
            changes.push(StrokeChange { layer, weight: next, ids, kept });
        }
        Ok(changes)
    }

    /// The legend table a plate's legend table is built from: only the lines this drawing
    /// actually used (section 7).
    pub fn used_only(&self, used: &[String]) -> LegendTable {
        LegendTable {
            group: self.group,
            entries: self.entries.iter().filter(|e| used.contains(&e.layer)).cloned().collect(),
            overrides: BTreeMap::new(),
        }
    }
}

/// Whether `layer` is one of the two that may be [`crate::style::WIDE_WEIGHT_PT`].
fn is_wide_layer(layer: &str) -> bool {
    matches!(layer, "survey_area" | "frame")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn table() -> LegendTable {
        let mut t = LegendTable::new(Group::Plan);
        t.add(LegendEntry::new("굴광선 (유구 면)", "feature_poly", Weight(0.8))).unwrap();
        t.add(LegendEntry::new("바닥선", "feature_floor", Weight(0.5))).unwrap();
        t.add(LegendEntry::new("단면 위치선", "section_line", Weight(0.2))).unwrap();
        t
    }

    fn ids() -> Vec<u64> {
        vec![10, 11, 12, 13]
    }

    /// One line is one layer: the line is found by the layer's name, and two lines cannot
    /// claim the same layer.
    #[test]
    fn a_line_is_its_layer() {
        let t = table();
        assert_eq!(t.line("feature_poly").map(|e| e.name.as_str()), Some("굴광선 (유구 면)"));
        assert!(t.line("nothing_here").is_none());
        let mut t = t;
        assert_eq!(t.add(LegendEntry::new("다른 이름", "feature_poly", Weight(0.5))), Err(LegendError::NoSuchLayer("feature_poly".into())));
        assert_eq!(t.add(LegendEntry::new("굴광선 (유구 면)", "other", Weight(0.5))), Err(LegendError::DuplicateName("굴광선 (유구 면)".into())));
    }

    /// ① Changing a line's weight changes every stroke on its layer at once — the done-criterion
    /// 「줄의 굵기를 고치면 그 줄의 선이 모두 바뀝니다」.
    #[test]
    fn changing_a_line_changes_every_stroke_on_its_layer() {
        let mut t = table();
        let change = t.set_weight("feature_poly", Weight(0.5), &ids()).unwrap();
        assert_eq!(change.ids, ids(), "every stroke on the layer is restyled");
        assert_eq!(change.touched(), 4);
        assert_eq!(change.weight, Weight(0.5));
        assert_eq!(t.line("feature_poly").map(|e| e.weight), Some(Weight(0.5)));
        // The other lines are untouched.
        assert_eq!(t.line("feature_floor").map(|e| e.weight), Some(Weight(0.5)));
        assert_eq!(t.line("section_line").map(|e| e.weight), Some(Weight(0.2)));
    }

    /// A stroke the user set by hand keeps what it has, and the line says how many: 「따로 2」.
    #[test]
    fn a_stroke_changed_by_hand_is_kept_and_counted() {
        let mut t = table();
        t.overrides.insert("feature_poly".into(), vec![11, 13]);
        assert_eq!(t.override_count("feature_poly"), 2);
        let change = t.set_weight("feature_poly", Weight(0.2), &ids()).unwrap();
        assert_eq!(change.ids, vec![10, 12], "the two the user set are left alone");
        assert_eq!(change.kept, 2);
        // The line takes the new weight all the same.
        assert_eq!(t.line("feature_poly").map(|e| e.weight), Some(Weight(0.2)));
    }

    /// 「범례대로」 once puts the kept strokes back to the line's shape.
    #[test]
    fn back_to_the_line_puts_the_kept_strokes_back() {
        let mut t = table();
        t.overrides.insert("feature_poly".into(), vec![11, 13]);
        let change = t.reset_overrides("feature_poly", &ids()).unwrap();
        assert_eq!(change.ids, vec![11, 13], "exactly the kept ones");
        assert_eq!(change.weight, Weight(0.8), "the line's weight is what they return to");
        assert_eq!(t.override_count("feature_poly"), 0);
        // A second 「범례대로」 has nothing left to do, and says so rather than failing.
        assert_eq!(t.reset_overrides("feature_poly", &ids()).unwrap().ids, Vec::<u64>::new());
    }

    /// A stroke the user set again after 「범례대로」 is not put back.
    #[test]
    fn a_stroke_changed_again_is_not_put_back() {
        let mut t = table();
        t.overrides.insert("feature_poly".into(), vec![11]);
        let change = t.reset_overrides("feature_poly", &ids()).unwrap();
        assert_eq!(change.ids, vec![11]);
        assert!(t.reset_overrides("feature_poly", &[99]).unwrap().ids.is_empty(), "a stroke not on the layer is not touched");
    }

    /// The weight-at-once control: 「어디 · 어떻게 · 값」, over the set, over one drawing or
    /// over the whole book, and with or without the strokes the user set.
    #[test]
    fn the_weight_at_once_control_covers_the_chosen_scope() {
        let mut t = table();
        let mut by_layer = BTreeMap::new();
        by_layer.insert("feature_poly".to_string(), ids());
        by_layer.insert("feature_floor".to_string(), vec![20, 21]);
        let changes = t.scale(Where_::Set, How::Ratio(0.8), &[], &by_layer, false).unwrap();
        assert_eq!(changes.len(), 3, "every line of the set");
        assert_eq!(t.line("feature_poly").map(|e| e.weight.pt()), Some(0.64));
        assert_eq!(t.line("feature_floor").map(|e| e.weight.pt()), Some(0.4));
        assert_eq!(t.line("section_line").map(|e| e.weight.pt()), Some(0.16));

        let mut t = table();
        let changes = t.scale(Where_::Drawing, How::OneStep, &["feature_poly".into()], &by_layer, false).unwrap();
        assert_eq!(changes.len(), 1, "only the layers this drawing uses");
        assert_eq!(t.line("feature_poly").map(|e| e.weight.pt()), Some(0.5));
    }

    /// 「따로 바꾼 선도 함께」 is the switch that includes them.
    #[test]
    fn the_kept_strokes_join_only_when_asked() {
        let mut t = table();
        t.overrides.insert("feature_poly".into(), vec![11]);
        let mut by_layer = BTreeMap::new();
        by_layer.insert("feature_poly".to_string(), ids());
        let changes = t.scale(Where_::Set, How::Ratio(0.5), &[], &by_layer, false).unwrap();
        let poly = changes.iter().find(|c| c.layer == "feature_poly").unwrap();
        assert_eq!(poly.ids, vec![10, 12, 13], "the kept one is left out");
        assert_eq!(poly.kept, 1);

        let mut t = table();
        t.overrides.insert("feature_poly".into(), vec![11]);
        let changes = t.scale(Where_::Set, How::Ratio(0.5), &[], &by_layer, true).unwrap();
        let poly = changes.iter().find(|c| c.layer == "feature_poly").unwrap();
        assert_eq!(poly.ids, ids(), "the kept one joins");
        assert_eq!(poly.kept, 1, "but the line remembers there was one");
    }

    /// A ratio that is not a number scales nothing and is refused.
    #[test]
    fn a_broken_ratio_is_refused() {
        let mut t = table();
        assert_eq!(t.scale(Where_::Set, How::Ratio(f64::NAN), &[], &BTreeMap::new(), false), Err(LegendError::BadFactor));
        assert_eq!(t.line("feature_poly").map(|e| e.weight.pt()), Some(0.8), "nothing changed");
    }

    /// 1.0 pt is the survey-area boundary's and the frame's alone (section 7).
    #[test]
    fn the_wide_weight_is_reserved() {
        let mut t = table();
        assert_eq!(t.set_weight("feature_poly", Weight(1.0), &ids()), Err(LegendError::WideWeightReserved));
        t.add(LegendEntry::new("조사구역 경계", "survey_area", Weight(0.5))).unwrap();
        assert!(t.set_weight("survey_area", Weight(1.0), &ids()).is_ok(), "the survey area may be 1.0 pt");
    }

    /// The pinned lines come first, and the rest keep the order the set lays them down.
    #[test]
    fn the_pinned_lines_come_first() {
        let mut t = table();
        t.line_mut("section_line").unwrap().frequent = true;
        let names: Vec<&str> = t.in_panel_order().iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["단면 위치선", "굴광선 (유구 면)", "바닥선"]);
    }

    /// The plate's legend table is built from the lines the drawing actually used (section 7).
    #[test]
    fn the_plate_table_takes_only_the_lines_used() {
        let t = table();
        let used = t.used_only(&["feature_poly".into(), "section_line".into()]);
        assert_eq!(used.entries.len(), 2);
        assert!(used.entries.iter().all(|e| e.layer != "feature_floor"));
        assert!(used.overrides.is_empty(), "a plate table carries no kept strokes");
    }

    /// The three sets, in the panel's order.
    #[test]
    fn there_are_three_sets() {
        assert_eq!(Group::ALL, [Group::Plan, Group::Section, Group::Find]);
        assert_eq!(Group::default(), Group::Plan);
        assert_eq!(Group::Find.label(), "Find");
    }

    /// A line bound to `Trowel` is a drawing-only one, and says so.
    #[test]
    fn trowel_only_lines_are_drawn_on_the_drawing_alone() {
        let e = LegendEntry::new("내부 시설", TROWEL_ONLY, Weight(0.5));
        assert!(!e.from_survey());
        let e = LegendEntry::new("굴광선", "feature_poly", Weight(0.8));
        assert!(e.from_survey());
    }
}
