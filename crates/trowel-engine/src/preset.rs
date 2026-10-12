//! The three sets of lines a new drawing lays out (design 2.1 section 7).
//!
//! Each set is transcribed from the design's own table, weight by weight, with the line's
//! Strata layer name as its binding and `Trowel` for the lines that exist only on the drawing.
//! An excavation rewrites its own copy; these are the starting point.

use crate::legend::{Group, LegendEntry, LegendTable, TROWEL_ONLY};
use crate::style::{Dash, Hatch, Weight};

/// A plan row: name, layer, weight, dash, label pt, pinned, from the survey.
type PlanRow = (&'static str, &'static str, f64, Dash, f64, bool, bool);
/// A section row: name, layer, weight, dash, hatch, label pt, pinned, prints.
type SectionRow = (&'static str, &'static str, f64, Dash, Hatch, f64, bool, bool);
/// A find row: name, layer, weight, dash, label pt.
type FindRow = (&'static str, &'static str, f64, Dash, f64);

fn w(pt: f64) -> Weight {
    // The weights here are the design's, written down once. A number that is not a weight
    // would be a typo in this table, and a table entry that cannot be a weight is a build-time
    // mistake rather than something a drawing can hit: fall back to the thinnest chip.
    Weight::new(pt).unwrap_or(Weight(0.1))
}

/// 평면용 — the plan set, paired with the Strata survey layers.
pub fn plan() -> LegendTable {
    let mut t = LegendTable::new(Group::Plan);
    // The survey lines first, each with the weight the design gives it.
    let rows: [PlanRow; 13] = [
        // name, layer, weight, dash, label pt, ★, from survey
        ("조사구역 경계", "survey_area", 1.0, Dash::DashDot, 0.0, false, true),
        ("시굴격자", "trial_trench", 0.5, Dash::Solid, 0.0, false, true),
        ("굴광선 (유구 면)", "feature_poly", 0.8, Dash::Solid, 0.0, true, true),
        ("바닥선", "feature_floor", 0.5, Dash::Solid, 0.0, true, true),
        ("추정선", "feature_inferred", 0.5, Dash::Dashed, 0.0, false, true),
        ("유구 선 (구·벽구)", "feature_line", 0.5, Dash::Solid, 0.0, false, true),
        ("단면 위치선", "section_line", 0.2, Dash::DashDot, 0.0, true, true),
        ("기준점", "control_points", 0.2, Dash::Solid, 0.0, false, true),
        ("유물 출토 위치", "artifact_point", 0.2, Dash::Solid, 0.0, false, true),
        // The drawing's own lines: Strata has no such layer.
        ("내부 시설 (주혈·노지)", TROWEL_ONLY, 0.5, Dash::Solid, 0.0, false, false),
        ("돌", TROWEL_ONLY, 0.5, Dash::Solid, 0.0, false, false),
        ("경사 표시", TROWEL_ONLY, 0.2, Dash::Solid, 0.0, false, false),
        ("표고점", TROWEL_ONLY, 0.1, Dash::Solid, 7.0, false, false),
    ];
    for (name, layer, weight, dash, label_pt, frequent, _) in rows {
        let mut e = LegendEntry::new(name, layer, w(weight));
        e.dash = dash;
        e.label_pt = label_pt;
        e.frequent = frequent;
        // A line whose weight is a drawing line may carry a cut-face hatch; the design gives
        // stones one (section 8: 잘린 돌은 0.8 + 빗금 번갈아). The starting value is the
        // hatch, not the weight: the weight follows the section set.
        if name == "돌" {
            e.hatch = Hatch::Alternating;
        }

        // `add` refuses a duplicate name; the table above has none, and a duplicate would be a
        // typo here rather than anything a drawing can do.
        let _ = t.add(e);
    }
    // 격자 · 도곽 draws at two weights (0.1 · 1.0) and does not print its frame band.
    let mut frame = LegendEntry::new("격자 · 도곽", "frame", w(1.0));
    frame.label_pt = 7.0;
    let _ = t.add(frame);
    // 밑그림 — the scanned survey and the rectified photograph: for the screen alone.
    let mut underlay = LegendEntry::new("밑그림 · 스캔", "underlay", w(0.1));
    underlay.print = false;
    let _ = t.add(underlay);
    t
}

/// 단면·입면용 — sections and elevations, "말씀하신 규칙" (the rule the user set).
pub fn section() -> LegendTable {
    let mut t = LegendTable::new(Group::Section);
    let rows: [SectionRow; 11] = [
        // name, layer, weight, dash, hatch, label pt, ★, prints
        ("단면선 (잘린 땅)", "section_line", 0.8, Dash::Solid, Hatch::None, 0.0, true, true),
        ("잘린 돌·유물·기와", "section_cut", 0.8, Dash::Solid, Hatch::Alternating, 0.0, true, true),
        ("보이는 돌·유물", "section_behind", 0.5, Dash::Solid, Hatch::None, 0.0, false, true),
        ("해발선 50 cm", "section_lev_50", 0.5, Dash::Solid, Hatch::None, 0.0, true, true),
        ("해발선 10 cm", "section_lev_10", 0.2, Dash::Solid, Hatch::None, 0.0, false, true),
        ("해발 숫자", "section_lev_text", 0.1, Dash::Solid, Hatch::None, 7.0, false, true),
        ("토층선", "stratum_line", 0.5, Dash::Solid, Hatch::None, 0.0, true, true),
        ("불분명한 토층선", "stratum_uncertain", 0.5, Dash::Dotted, Hatch::None, 0.0, false, true),
        ("교란", "disturbance", 0.5, Dash::Solid, Hatch::None, 0.0, false, true),
        ("생토", "stratum_natural", 0.1, Dash::Solid, Hatch::None, 0.0, false, true),
        ("토층 번호", "stratum_number", 0.1, Dash::Solid, Hatch::None, 8.0, false, true),
    ];
    for (name, layer, weight, dash, hatch, label_pt, frequent, _) in rows {
        let mut e = LegendEntry::new(name, layer, w(weight));
        e.dash = dash;
        e.hatch = hatch;
        e.label_pt = label_pt;
        e.frequent = frequent;
        let _ = t.add(e);
    }
    // desca 원본 — the 2D DXF as it came in: hidden, for the screen only.
    let mut desca = LegendEntry::new("desca 원본", "desca_source", w(0.1));
    desca.print = false;
    let _ = t.add(desca);
    t
}

/// 유물용 — finds.
pub fn find() -> LegendTable {
    let mut t = LegendTable::new(Group::Find);
    let rows: [FindRow; 6] = [
        ("외형선", "find_outline", 0.8, Dash::Solid, 0.0),
        ("단면", "find_section", 0.8, Dash::Solid, 0.0),
        ("내면선", "find_inner", 0.5, Dash::Solid, 0.0),
        ("중심선", "find_axis", 0.2, Dash::DashDot, 0.0),
        ("문양 · 조정흔", "find_decoration", 0.2, Dash::Solid, 0.0),
        ("복원선", "find_missing", 0.5, Dash::Dashed, 0.0),
    ];
    for (name, layer, weight, dash, label_pt) in rows {
        let mut e = LegendEntry::new(name, layer, w(weight));
        e.dash = dash;
        e.label_pt = label_pt;
        let _ = t.add(e);
    }
    // 계측선 — diameter, height, caliper: the sky colour, on the screen alone.
    let mut measure = LegendEntry::new("계측선", "find_measure", w(0.1));
    measure.print = false;
    let _ = t.add(measure);
    t
}

/// The set of each kind, as a new drawing lays it out.
pub fn builtin(group: Group) -> LegendTable {
    match group {
        Group::Plan => plan(),
        Group::Section => section(),
        Group::Find => find(),
    }
}

/// All three sets.
pub fn all() -> [LegendTable; 3] {
    [plan(), section(), find()]
}

/// The text sizes on the report paper (section 7): 도판 캡션 10 · 유구 이름 9 · 토층 번호 8 ·
/// 해발·표고·좌표·축척 숫자 7.
pub const LABEL_SIZES_PT: [(&str, f64); 4] =
    [("Plate caption", 10.0), ("Feature name", 9.0), ("Stratum number and legend table", 8.0), ("Elevation, level, coordinate, scale figures", 7.0)];

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    /// Every set is written out, every line has a name and a layer, and no name repeats within
    /// a set — a duplicate would make the panel name two lines the same way.
    #[test]
    fn every_builtin_line_is_written_out_and_named() {
        for t in all() {
            assert!(!t.entries.is_empty(), "{} has no lines", t.group.label());
            let mut names: Vec<&str> = t.entries.iter().map(|e| e.name.as_str()).collect();
            let count = names.len();
            names.sort_unstable();
            names.dedup();
            assert_eq!(names.len(), count, "{:?} has a repeated name", t.group.label());
            for e in &t.entries {
                assert!(!e.name.is_empty() && !e.layer.is_empty());
            }
        }
    }

    /// The weights the design gives each line, checked where a mistake would matter most: the
    /// four steps of section 7 and the reserved 1.0 pt.
    #[test]
    fn the_designed_weights_are_in_place() {
        let plan = plan();
        assert_eq!(plan.line("survey_area").map(|e| e.weight.pt()), Some(1.0), "the survey area is the one 1.0 pt line");
        assert_eq!(plan.line("feature_poly").map(|e| e.weight.pt()), Some(0.8), "굴광선 0.8");
        assert_eq!(plan.line("feature_floor").map(|e| e.weight.pt()), Some(0.5));
        assert_eq!(plan.line("feature_line").map(|e| e.weight.pt()), Some(0.5));
        assert_eq!(plan.line("section_line").map(|e| e.weight.pt()), Some(0.2));
        assert_eq!(plan.line("artifact_point").map(|e| e.weight.pt()), Some(0.2));

        let section = section();
        assert_eq!(section.line("section_line").map(|e| e.weight.pt()), Some(0.8), "잘린 땅 0.8");
        assert_eq!(section.line("section_cut").map(|e| e.weight.pt()), Some(0.8));
        assert_eq!(section.line("section_lev_50").map(|e| e.weight.pt()), Some(0.5));
        assert_eq!(section.line("section_lev_10").map(|e| e.weight.pt()), Some(0.2));
        assert_eq!(section.line("section_lev_text").map(|e| e.weight.pt()), Some(0.1), "빗금 0.1 pt");
    }

    /// The dashes the design names: the survey area and the section line are dash-dot, the
    /// inferred line dashed, everything else solid.
    #[test]
    fn the_designed_dashes_are_in_place() {
        let plan = plan();
        assert_eq!(plan.line("survey_area").map(|e| e.dash), Some(Dash::DashDot));
        assert_eq!(plan.line("section_line").map(|e| e.dash), Some(Dash::DashDot));
        assert_eq!(plan.line("feature_inferred").map(|e| e.dash), Some(Dash::Dashed));
        assert_eq!(plan.line("feature_poly").map(|e| e.dash), Some(Dash::Solid));
        let find = find();
        assert_eq!(find.line("find_axis").map(|e| e.dash), Some(Dash::DashDot), "중심선");
        assert_eq!(find.line("find_missing").map(|e| e.dash), Some(Dash::Dashed), "복원선");
    }

    /// The cut faces carry the alternating hatch; nothing else does.
    #[test]
    fn only_the_cut_faces_carry_a_hatch() {
        let section = section();
        assert_eq!(section.line("section_cut").map(|e| e.hatch), Some(Hatch::Alternating));
        assert_eq!(section.line("section_line").map(|e| e.hatch), Some(Hatch::None));
        assert_eq!(plan().line("feature_poly").map(|e| e.hatch), Some(Hatch::None));
    }

    /// The lines the design marks `★` are pinned; the panel shows them first.
    #[test]
    fn the_pinned_lines_are_the_frequent_ones() {
        let p = plan().in_panel_order().iter().filter(|e| e.frequent).count();
        assert_eq!(p, 3, "굴광선 · 바닥선 · 단면 위치선");
        let s = section();
        assert!(s.line("section_line").is_some_and(|e| e.frequent));
        assert!(s.line("section_cut").is_some_and(|e| e.frequent));
    }

    /// The lines the design keeps off the paper: the underlay, the survey lines and the
    /// measurements (section 7: 계측선 · 화면에만).
    #[test]
    fn the_screen_only_lines_do_not_print() {
        let plan = plan();
        assert!(!plan.line("underlay").is_some_and(|e| e.print), "밑그림 인쇄 안 됨");
        assert!(!find().line("find_measure").is_some_and(|e| e.print), "계측선 화면에만");
        assert!(!section().line("desca_source").is_some_and(|e| e.print), "desca 원본 인쇄 안 됨");
        assert!(plan.line("feature_poly").is_some_and(|e| e.print));
    }

    /// The four text sizes on the report paper.
    #[test]
    fn the_text_sizes_are_the_designed_ones() {
        assert_eq!(LABEL_SIZES_PT.len(), 4);
        let sizes: Vec<f64> = LABEL_SIZES_PT.iter().map(|(_, p)| *p).collect();
        assert!(sizes.contains(&10.0) && sizes.contains(&9.0) && sizes.contains(&8.0) && sizes.contains(&7.0));
        assert!(plan().line("underlay_level").is_none(), "no invented line");
    }
}
