//! Trowel's own strings.
//!
//! The repository rule is that a string in code is English and is the lookup key
//! (`docs/development.md` › Localisation); the interface draws it through the
//! shared catalog. Trowel's archaeological vocabulary — 굴광선, 수혈, 토층, 표고,
//! 도면집 — has no English equivalent to translate from, and `ko.tsv` is a
//! translation of the VectorCraft interface, not a glossary of excavation terms.
//!
//! So Trowel keeps its own small catalog: each label carries the English key (what
//! a script, the control channel and the MCP see) and the Korean the design uses,
//! and picks between them from the interface language. Folding it into the shared
//! catalog is J001's business, together with the rest of the palette.

/// One label: the English key, and the Korean the design 2.1 mock-up shows.
#[derive(Debug, Clone, Copy)]
pub struct Label {
    pub key: &'static str,
    pub ko: &'static str,
}

/// The label to draw in `lang`. Anything that is not Korean shows the English key.
impl Label {
    pub fn text(self, lang: &str) -> &'static str {
        if lang.starts_with("ko") { self.ko } else { self.key }
    }
}

macro_rules! labels {
    ($($konst:ident => $key:literal / $ko:literal),* $(,)?) => {
        // The catalog is complete on purpose: every label the design names, so that a
        // surface is written in the design's own words. A label with no caller yet is
        // the one whose task has not reached it.
        $(#[allow(dead_code)] pub const $konst: Label = Label { key: $key, ko: $ko };)*
    };
}

labels! {
    // The menu bar (section 3: 파일 편집 오브젝트 문자 선택 효과 도면 보기 창 도움말).
    MENU_FILE => "File" / "파일",
    MENU_EDIT => "Edit" / "편집",
    MENU_OBJECT => "Object" / "오브젝트",
    MENU_TYPE => "Type" / "문자",
    MENU_SELECT => "Select" / "선택",
    MENU_EFFECT => "Effect" / "효과",
    MENU_DRAWING => "Drawing" / "도면",
    MENU_VIEW => "View" / "보기",
    MENU_WINDOW => "Window" / "창",
    MENU_HELP => "Help" / "도움말",

    // Ribbon tiles (section 3, the one-line ribbon).
    T_OPEN => "Open" / "열기",
    T_SAVE => "Save" / "저장",
    T_IMPORT => "Import" / "가져오기",
    G_NEW_DRAWING => "New drawing" / "새 도면",
    T_PLAN => "Plan" / "평면",
    T_SECTION => "Section" / "단면",
    T_FIND => "Find" / "유물",
    T_PLATE => "Plate" / "도판",
    G_UNDERLAY => "Underlay" / "밑그림",
    T_ALIGN_UNDERLAY => "Align" / "맞추기",
    T_UNDERLAY => "Underlay" / "밑그림",
    G_DRAW => "Draw" / "그리기",
    T_PEN => "Pen" / "펜",
    T_PENCIL => "Pencil" / "연필",
    T_SECTION_LINE => "Section line" / "단면선",
    T_LEVEL_MARK => "Level mark" / "표고",
    T_PAINT => "Paint" / "칠통",
    G_ELEMENTS => "Drawing elements" / "도면 요소",
    T_NORTH => "North" / "방위",
    T_SCALE => "Scale" / "축척",
    T_LEGEND_TABLE => "Legend table" / "범례표",
    T_TITLE_BLOCK => "Title block" / "표제란",
    G_VIEW => "View" / "보기",
    T_GRID => "Grid" / "격자",
    T_SNAP => "Snap" / "자석",
    G_EXPORT => "Export" / "내보내기",
    T_PDF => "PDF" / "PDF",
    T_DXF => "DXF" / "DXF",
    T_TO_STRATA => "To Strata" / "Strata로",
    T_FIND_COMMANDS => "Find commands" / "명령 찾기",

    // The archaeology tool group (section 3: 고고학).
    G_ARCHAEOLOGY => "Archaeology" / "고고학",
    T_TRENCH => "Trench" / "수혈",

    // The drawing book (section 3: 도면집 광령리1 ＋).
    DRAWING_BOOK => "Drawing book" / "도면집",
    BOOK_PLAN => "Plan" / "평면",
    BOOK_SECTION => "Section" / "단면",
    BOOK_FIND => "Find" / "유물",
    BOOK_PLATE => "Plate" / "도판",
    BOOK_HOME => "Home" / "홈",
    BOOK_ORIGIN => "Origin" / "원점",
    BOOK_HEIGHT => "Height" / "높이",
    BOOK_SEALE => "Sea level" / "해발",
    BOOK_DATUM => "Datum" / "지정함",

    // The right column (section 3, from the top: drawing palette, legend · layers, properties).
    DRAWING_PALETTE => "Drawing palette" / "그리기 팔레트",
    NEXT_STROKE => "Next stroke" / "다음에 그릴 선",
    WEIGHT => "Weight" / "굵기",
    HATCH => "Hatch" / "빗금",
    DASH => "Dash" / "선종",
    CUT_FACE => "Cut face" / "잘린 면",
    LEGEND_LAYERS => "Legend · Layers" / "범례 · 레이어",
    // 「범례 · 레이어」 (J005, section 7): the three sets, the weight-at-once control and the
    // strokes the user set by hand.
    SET_PLAN => "Plan set" / "평면용",
    SET_SECTION => "Section set" / "단면·입면용",
    SET_FIND => "Find set" / "유물용",
    WEIGHT_AT_ONCE => "Weight at once" / "굵기 한 번에",
    WHERE_SET => "This set" / "이 범례",
    WHERE_DRAWING => "This drawing" / "이 도면",
    WHERE_BOOK => "The whole book" / "도면집 전체",
    HOW_RATIO => "By a ratio" / "비율 값",
    HOW_STEP => "One step" / "한 단계",
    WITH_SEPARATE => "With the ones set by hand" / "따로 바꾼 선도 함께",
    TO_THE_LINE => "Back to the line" / "범례대로",
    SEPARATE => "separate" / "따로",
    FROM_SURVEY => "Survey" / "조사",
    DASH_SOLID => "Solid" / "실선",
    DASH_DASHED => "Dashed" / "파선",
    DASH_DOTTED => "Dotted" / "점선",
    DASH_DASHDOT => "Dash dot" / "일점쇄선",
    LAYER_ON_DRAWING => "On the drawing" / "도면의 레이어에 있음",
    LAYER_MISSING => "No layer with that name yet" / "같은 이름의 레이어가 아직 없음",
    TROWEL_ONLY_LINES => "Trowel only" / "도면에만",
    TEXT_SIZE => "Text" / "글자",
    HATCH_NONE => "No fill" / "칠 없음",
    HATCH_ALTERNATING => "Alternating" / "번갈아",
    PROPERTIES => "Properties" / "속성",
    PROPERTIES_STROKE => "The line" / "고른 선",
    KIND => "Kind" / "종류",
    PERIOD => "Period" / "시대",
    PICK => "Pick one" / "고르세요",
    NEEDED => "Required" / "필요",

    // The start screen (section 3, J004).
    PRODUCT_NAME => "Trowel" / "트루엘",
    START_LEAD => "고고학 기록을 위한 도면 프로그램" / "고고학 기록을 위한 도면 프로그램",
    START_NEW_DRAWING => "New drawing" / "새 도면",
    START_OPEN_STORE => "Open store" / "저장소 열기",
    START_NEW_TRACING => "New tracing" / "새 그리기",
    START_RECENT => "Recent" / "최근",
    START_STRATA => "Strata survey file" / "Strata 조사 파일",
    START_PICK_SITE => "Make a chosen survey" / "선택 조사 만들기",
    START_LAST_SITE => "The last survey" / "마지막 조사로",
    // The start screen (design §1 「Strata 홈과 같은 것」, J004) — the design's own words.
    START_HEADING => "광릉리 상궁" / "광릉리 상궁",
    START_TEMPLATES => "도면 템플릿" / "도면 템플릿",
    RAIL_NEXT => "다음" / "다음",
    BOOK_OPEN => "이어서 그리기" / "이어서 그리기",
    START_STRATA_FILE => "Strata survey file" / "Strata 조사 파일",
    START_DESCA => "desca sections" / "desca 단면",
    START_POINTS => "Survey points" / "측점",
    START_UNDERLAY => "Underlay" / "밑그림",
    LAYER_EMPTY => "Nothing on that layer yet" / "그 레이어에 아직 선이 없음",
    STROKE => "Stroke" / "선",
    NO_STROKE => "no stroke" / "획 없음",
    // The status bar (section 3).
    COORDINATES => "Coordinates" / "좌표",
    UNSAVED => "Unsaved" / "저장 안 됨",

    // The Z magnifier (section 10, J002): «왼쪽으로 끌면 확대 … 방향은 환경설정에서 바꿀 수
    // 있습니다».
    MAGNIFIER_DRAG => "Magnifier drag" / "돋보기 끌기",
    DRAG_IN_LEFT => "Drag left zooms in" / "왼쪽으로 끌면 확대",
    DRAG_IN_RIGHT => "Drag right zooms in" / "오른쪽으로 끌면 확대",

    // What is still to come (section 3: 흐린 「준비 중」).
    NOT_READY => "Not ready yet" / "준비 중",
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    /// Korean is drawn for Korean and its regional forms; everything else falls
    /// back to the English key, which is also what the control channel and MCP see.
    #[test]
    fn korean_picks_the_designed_text() {
        assert_eq!(T_OPEN.text("ko"), "열기");
        assert_eq!(T_OPEN.text("ko-KR"), "열기");
        assert_eq!(T_OPEN.text("en"), "Open");
        assert_eq!(T_OPEN.text("ja"), "Open");
    }

    /// The menu bar is the one the design lists, with 「도면」 added for Trowel.
    #[test]
    fn the_menu_bar_matches_the_mock_up() {
        assert_eq!(
            [
                MENU_FILE.ko,
                MENU_EDIT.ko,
                MENU_OBJECT.ko,
                MENU_TYPE.ko,
                MENU_SELECT.ko,
                MENU_EFFECT.ko,
                MENU_DRAWING.ko,
                MENU_VIEW.ko,
                MENU_WINDOW.ko,
                MENU_HELP.ko
            ],
            ["파일", "편집", "오브젝트", "문자", "선택", "효과", "도면", "보기", "창", "도움말"]
        );
    }

    /// The line weights are the design's four steps plus the one the survey area
    /// and the frame use (section 10, decision 4).
    #[test]
    fn the_chips_are_the_designed_steps() {
        assert_eq!(WEIGHTS, [1.0, 0.8, 0.5, 0.25, 0.2, 0.1]);
        assert_eq!(WIDE_WEIGHT, 1.0);
        assert_eq!(PRINTER_SAFE_WEIGHT, 0.25);
    }

    /// The ladder is thickest first, and it keeps the design's four steps alongside the
    /// weights a plotter can actually show.
    #[test]
    fn the_ladder_is_descending_and_keeps_both_ends() {
        for pair in WEIGHTS.windows(2) {
            assert!(pair[0] > pair[1], "{pair:?} is not thickest first");
        }
        // The four steps of design section 10, decision 4.
        for w in [0.8, 0.5, 0.2, 0.1] {
            assert!(WEIGHTS.contains(&w), "{w} pt is one of the designed steps");
        }
        // The weight a plotter shows reliably, and the frame's.
        for w in [PRINTER_SAFE_WEIGHT, WIDE_WEIGHT] {
            assert!(WEIGHTS.contains(&w), "{w} pt is on the ladder");
        }
    }
}

/// The weights of the design 2.1 interface (section 10, decision 4), thickest first: the
/// four steps `0.8 · 0.5 · 0.2 · 0.1` the 2026-10-11 change settled on, `0.25` — the thinnest
/// weight a plot shows reliably — and `1.0` for the survey-area boundary and the drawing frame.
/// The user adds chips to these.
pub const WEIGHTS: [f64; 6] = [1.0, 0.8, 0.5, 0.25, 0.2, 0.1];
/// The weight the survey-area boundary and the drawing frame take (section 7). It is one of
/// the chips above, but with its own meaning: no drawing line takes it by default.
#[allow(dead_code)] // the legend and the frame band name it (section 7); J006 draws the frame
pub const WIDE_WEIGHT: f64 = 1.0;

/// The thinnest line a plot shows reliably: 0.25 pt is 0.088 mm on paper, the smallest
/// weight a pen plotter, an inkjet and offset lithography all hold without dropping the line
/// or letting its weight wander across a sheet.
///
/// The design keeps 0.2 and 0.1 on the ladder for the screen and for work plotted large, and
/// this is the weight a submission drawing guarantees (section 14, question (a)): a feature that
/// cannot be seen on paper cannot be measured on it.
pub const PRINTER_SAFE_WEIGHT: f64 = 0.25;

/// A weight formatted the way the interface writes it: `0.8`, never `0.80`.
pub fn weight_label(pt: f64) -> String {
    if !pt.is_finite() || pt <= 0.0 {
        return "0".to_string();
    }
    let s = format!("{pt:.2}");
    // One decimal place is what the design writes; a fifth of a point keeps two.
    let s = s.trim_end_matches('0');
    let s = s.strip_suffix('.').map(str::to_owned).unwrap_or_else(|| s.to_string());
    if s.is_empty() { "0".to_string() } else { s }
}

#[cfg(test)]
mod weight_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::weight_label;

    #[test]
    fn weights_read_as_the_design_writes_them() {
        assert_eq!(weight_label(0.8), "0.8");
        assert_eq!(weight_label(0.5), "0.5");
        assert_eq!(weight_label(0.2), "0.2");
        assert_eq!(weight_label(0.1), "0.1");
        assert_eq!(weight_label(1.0), "1");
    }

    /// A weight that isn't a number never reaches the interface as one.
    #[test]
    fn a_broken_weight_reads_as_zero() {
        assert_eq!(weight_label(f64::NAN), "0");
        assert_eq!(weight_label(f64::INFINITY), "0");
        assert_eq!(weight_label(-1.0), "0");
        assert_eq!(weight_label(0.0), "0");
    }
}
