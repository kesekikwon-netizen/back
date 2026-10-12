//! The stroke properties a legend line stands for (design 2.1 section 7).
//!
//! A weight is a point value measured **on the report paper**, not in document units
//! (section 14, question (a)): it stays what it is when the drawing's scale changes and when
//! the drawing is reduced onto a plate. That is why nothing here is a ratio.

use serde::{Deserialize, Serialize};

/// The widest weight the interface offers, used by the survey-area boundary and the frame
/// (section 7: "1.0 pt는 조사구역 경계와 도곽에만 씁니다").
pub const WIDE_WEIGHT_PT: f64 = 1.0;
/// The thinnest a plot shows reliably (0.088 mm at 0.25 pt). The design keeps `0.2` and `0.1`
/// on the ladder for the screen and for work plotted large; this is what a submission drawing
/// guarantees.
pub const PRINTER_SAFE_WEIGHT_PT: f64 = 0.25;
/// The ladder of chips in the drawing palette, thickest first: the four steps section 10
/// decision 4 settled on, plus the printer-safe step and the frame's.
pub const WEIGHT_LADDER_PT: [f64; 6] = [1.0, 0.8, 0.5, 0.25, 0.2, 0.1];
/// Above this a weight is a mistake, not a choice: no drawing line is this thick, and a value
/// this large on a stroke makes the renderer work for nothing.
const WEIGHT_CEILING_PT: f64 = 100.0;

/// What went wrong with a weight.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum WeightError {
    #[error("a weight must be a number in points")]
    NotFinite,
    #[error("a weight must be more than zero")]
    NotPositive,
    #[error("a weight of {0} pt is more than the {WEIGHT_CEILING_PT} pt ceiling")]
    TooThick(f64),
}

/// A stroke weight in points on paper: finite, positive, within the ceiling.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Weight(pub f64);

impl Weight {
    /// The weight of a drawing line.
    pub fn new(pt: f64) -> Result<Self, WeightError> {
        if !pt.is_finite() {
            return Err(WeightError::NotFinite);
        }
        if pt <= 0.0 {
            return Err(WeightError::NotPositive);
        }
        if pt > WEIGHT_CEILING_PT {
            return Err(WeightError::TooThick(pt));
        }
        Ok(Self(pt))
    }

    pub fn pt(self) -> f64 {
        self.0
    }

    /// The same weight on the report paper, in millimetres — what the plotter is asked for.
    pub fn mm(self) -> f64 {
        self.0 * 25.4 / 72.0
    }

    /// Whether a plot shows this weight reliably (see [`PRINTER_SAFE_WEIGHT_PT`]).
    pub fn prints_reliably(self) -> bool {
        self.0 >= PRINTER_SAFE_WEIGHT_PT
    }

    /// The same weight scaled by `factor`, e.g. the legend's "×0.8" (section 7).
    /// A factor that is not a number, or one that would take the weight to nothing, is refused
    /// rather than turned into a broken weight.
    pub fn scaled(self, factor: f64) -> Result<Self, WeightError> {
        let scaled = self.0 * factor;
        // A weight is a number a person reads (0.8 × 0.8 is 0.64, not 0.6400000000000001), and
        // the panel writes it with two decimals. Rounding to a hundred-thousandth of a point
        // — 0.000035 mm on paper — takes the float noise out and changes nothing a plotter,
        // an eye or a ruler could tell.
        if !scaled.is_finite() {
            return Err(WeightError::NotFinite);
        }
        Self::new((scaled * 10_000.0).round() / 10_000.0)
    }

    /// The next step down the ladder — what "한 단계" (one step) means in the legend's
    /// weight-at-once control. Below the thinnest chip there is no step, so the weight stands.
    pub fn one_step_down(self) -> Self {
        match WEIGHT_LADDER_PT.iter().position(|w| (*w - self.0).abs() < 1e-9) {
            // `get`, not `[i + 1]`: the thinnest chip has no step below it, and indexing past
            // the end would be a panic in shipped code.
            Some(i) => WEIGHT_LADDER_PT.get(i + 1).copied().and_then(|w| Self::new(w).ok()).unwrap_or(self),
            // A weight the user added sits between two chips: the next chip below it.
            None => WEIGHT_LADDER_PT
                .iter()
                .filter(|w| **w < self.0)
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|w| Self(*w))
                .unwrap_or(self),
        }
    }
}

/// How a line is broken (section 7: 실선 · 파선 · 점선 · 일점쇄선).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Dash {
    #[default]
    Solid,
    Dashed,
    Dotted,
    DashDot,
}

impl Dash {
    /// The egui/SVG dash pattern, in points: `[dash, gap]` repeated. `None` is a solid line.
    ///
    /// The gaps are where the design's own line weight shows through: a dashed trench edge is
    /// read by its length, so the dash grows with the weight rather than staying a fixed size.
    pub fn pattern(self, weight: Weight) -> Option<Vec<f64>> {
        let w = weight.pt().max(0.05);
        match self {
            Dash::Solid => None,
            Dash::Dashed => Some(vec![w * 6.0, w * 3.0]),
            Dash::Dotted => Some(vec![0.01, w * 2.5]),
            Dash::DashDot => Some(vec![w * 8.0, w * 2.5, 0.01, w * 2.5]),
        }
    }

    /// The interface's own name, as section 3 writes it.
    pub fn label(self) -> &'static str {
        match self {
            Dash::Solid => "Solid",
            Dash::Dashed => "Dashed",
            Dash::Dotted => "Dotted",
            Dash::DashDot => "Dash dot",
        }
    }
}

/// The fill inside a cut face (section 4, section 8: 없음 · 45° · 135° · 번갈아).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Hatch {
    #[default]
    None,
    Deg45,
    Deg135,
    /// Alternating: adjacent pieces take opposite angles, so neighbouring cut faces read apart
    /// without a second line (section 14, question (e)).
    Alternating,
}

impl Hatch {
    /// The angle this hatch draws at. `Alternating` answers with one of the two, chosen by
    /// `piece` — the index of the cut face among its neighbours, so that touching pieces get
    /// opposite angles.
    pub fn angle_deg(self, piece: usize) -> f64 {
        match self {
            Hatch::None => 0.0,
            Hatch::Deg45 => 45.0,
            Hatch::Deg135 => 135.0,
            Hatch::Alternating if piece.is_multiple_of(2) => 45.0,
            Hatch::Alternating => 135.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Hatch::None => "None",
            Hatch::Deg45 => "45°",
            Hatch::Deg135 => "135°",
            Hatch::Alternating => "Alternating",
        }
    }
}

/// The gap between hatch lines on the report paper, in millimetres (section 14, question (f):
/// 1.0 mm, the guess to confirm).
pub const HATCH_PITCH_MM: f64 = 1.0;

/// The gap in points on the report paper.
pub fn hatch_pitch_pt() -> f64 {
    HATCH_PITCH_MM * 72.0 / 25.4
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    /// The four steps the design settled on, the printer-safe step and the frame's are all on
    /// the ladder, thickest first.
    #[test]
    fn the_ladder_is_the_designed_one() {
        assert_eq!(WEIGHT_LADDER_PT, [1.0, 0.8, 0.5, 0.25, 0.2, 0.1]);
        for pair in WEIGHT_LADDER_PT.windows(2) {
            assert!(pair[0] > pair[1]);
        }
    }

    /// A weight is a number on paper: not a NaN, not zero, not a hairline thinner than a plot
    /// can hold by much, and never absurd.
    #[test]
    fn a_broken_weight_is_refused() {
        assert_eq!(Weight::new(f64::NAN), Err(WeightError::NotFinite));
        assert_eq!(Weight::new(f64::INFINITY), Err(WeightError::NotFinite));
        assert_eq!(Weight::new(0.0), Err(WeightError::NotPositive));
        assert_eq!(Weight::new(-1.0), Err(WeightError::NotPositive));
        assert_eq!(Weight::new(1000.0), Err(WeightError::TooThick(1000.0)));
        assert_eq!(Weight::new(0.8), Ok(Weight(0.8)));
    }

    /// The millimetres on paper are what the plotter is asked for: 0.8 pt is the 0.28 mm the
    /// design writes next to it.
    #[test]
    fn millimetres_are_what_the_plotter_is_asked_for() {
        assert!((Weight(0.8).mm() - 0.282).abs() < 0.001, "0.8 pt is {} mm", Weight(0.8).mm());
        assert!((Weight(1.0).mm() - 0.353).abs() < 0.001);
        assert!(Weight(0.25).prints_reliably());
        assert!(!Weight(0.2).prints_reliably());
        assert!(!Weight(0.1).prints_reliably());
    }

    /// "한 단계" walks the ladder down and stops at the thinnest chip.
    #[test]
    fn one_step_down_walks_the_ladder() {
        assert_eq!(Weight(0.8).one_step_down(), Weight(0.5));
        assert_eq!(Weight(0.5).one_step_down(), Weight(0.25));
        assert_eq!(Weight(0.25).one_step_down(), Weight(0.2));
        assert_eq!(Weight(0.2).one_step_down(), Weight(0.1));
        assert_eq!(Weight(0.1).one_step_down(), Weight(0.1), "below the last chip there is no step");
        assert_eq!(Weight(1.0).one_step_down(), Weight(0.8));
    }

    /// A weight the user added sits between two chips and steps down to the chip below it.
    #[test]
    fn an_added_weight_steps_down_to_the_chip_below() {
        assert_eq!(Weight(0.6).one_step_down(), Weight(0.5));
        assert_eq!(Weight(0.3).one_step_down(), Weight(0.25));
        assert_eq!(Weight(0.05).one_step_down(), Weight(0.05));
    }

    /// A dash pattern grows with the line it belongs to: a dashed 0.8 pt edge has longer
    /// dashes than a dashed 0.2 pt one, so both read at the size they are drawn.
    #[test]
    fn dashes_grow_with_the_line() {
        let thick = Dash::Dashed.pattern(Weight(0.8)).unwrap_or_default();
        let thin = Dash::Dashed.pattern(Weight(0.2)).unwrap_or_default();
        assert!(thick[0] > thin[0], "{thick:?} vs {thin:?}");
        assert!(Dash::Solid.pattern(Weight(0.8)).is_none(), "a solid line has no pattern");
        assert!(Dash::Dotted.pattern(Weight(0.5)).is_some());
        assert_eq!(Dash::DashDot.pattern(Weight(0.5)).map(|p| p.len()), Some(4));
    }

    /// Touching cut faces get opposite angles (section 14, question (e)).
    #[test]
    fn touching_pieces_alternate() {
        let a = Hatch::Alternating.angle_deg(0);
        let b = Hatch::Alternating.angle_deg(1);
        let c = Hatch::Alternating.angle_deg(2);
        assert_eq!((a, b, c), (45.0, 135.0, 45.0));
        assert_eq!(Hatch::Deg45.angle_deg(1), 45.0, "an explicit angle does not alternate");
        assert_eq!(Hatch::Deg135.angle_deg(0), 135.0);
    }

    #[test]
    fn the_hatch_pitch_is_the_designeds_millimetre() {
        assert_eq!(HATCH_PITCH_MM, 1.0);
        assert!((hatch_pitch_pt() - 2.835).abs() < 0.001);
    }
}
