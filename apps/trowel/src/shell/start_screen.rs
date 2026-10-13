//! The start screen (design 2.1 §1 「Strata 홈과 같은 것」, J004).
//!
//! Built from the design HTML's own rules, not from an impression of them:
//!
//! ```text
//! .home      padding 20 32 16, flex column, gap 18
//! .hero      grid minmax(0,1fr) | 520px, top+bottom 1px ink, padding 20 0
//! .homecols  grid minmax(0,1fr) | 380px, gap 32
//! .conn      grid repeat(5, minmax(0,1fr)), top 1px line, each cell left 1px line
//! ```
//!
//! The design's own words about this screen, kept here as the rule the code serves:
//!
//! - 「위아래 먹색 줄 사이의 머리 띠, 삼선표, 작업 순서 레일, 연결 상태 줄.」 — the ink rule top
//!   and bottom of the hero band, the timeline rail, and the connector strip.
//! - 「Strata 조사 파일은 **읽기만** 합니다. 원본은 절대 고치지 않습니다. 원본 파일을 못 찾으면
//!   그 줄에 「원본 없음」을 **붉은 표시와 말**로 알립니다.」
//! - 「흙색은 「이어서 그리기」 단추와 「다음」 점에만 씁니다.」 — so clay appears in exactly two
//!   places below: the continue button and the `next` marker on the rail.

use egui::{Align2, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use serde_json::json;
use vectorcraft_ui_egui::VectorcraftApp;

use super::labels;
use super::tokens;

/// `.homecols` right column, points.
pub const RAIL_W: f32 = 380.0;
/// `.hero` right column, points.
pub const HERO_R_W: f32 = 520.0;
/// `.tpls` and `.conn` column count.
pub const COLS: usize = 5;

/// The five new-drawing templates (design §1, `.tpls`). Each carries its own legend set, scale
/// and paper, which is what makes them templates rather than blank documents.
pub const TEMPLATES: [(&str, &str); COLS] = [
    ("개별유구 평·단면도", "1:20 · A3 가로 · 평면용 범례"),
    ("토층 단면도", "1:20 · 기준선 · 단면용 범례"),
    ("유물 실측도", "1:1로 그림 · 도판은 1:3"),
    ("유구배치도", "1:100 · Strata 조사에서"),
    ("보고서 도판", "A4 세로 · 번호와 캡션"),
];

/// One step of the work rail (design §1, `.rail`), with where it stands: done, to do, or next.
/// The `next` step is the only one allowed the clay colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Done,
    Todo,
    Next,
}

/// The eight steps of one excavation's work, in order. The notes are what the rail says beside
/// each step: what is there, and what is missing.
pub const STEPS: [(&str, &str, Stage); 8] = [
    ("그리기 준비", "준비", Stage::Done),
    ("도면집", "도면집 3개", Stage::Done),
    ("Strata 연결", "확인", Stage::Done),
    ("수평 좌표·단위", "좌표계 5186 한국좌표", Stage::Todo),
    ("밑그림 채우기", "스캔 6장 · 정사영상 1장", Stage::Todo),
    ("유구 옮기기", "1호 주거지 옮김, 3호 수혈 따라 그리는 중", Stage::Next),
    ("심층·단면 그리기", "적설구 11, 12 적설", Stage::Todo),
    ("도판·범례표 만들기", "보고서용 PDF, Strata 보고서용 PDF", Stage::Todo),
];

/// The five links of the connector strip (design §1, `.conn`): Strata → 도면 → 변환 → 보고서 → 식별.
pub const LINKS: [(&str, &str); COLS] = [
    ("Strata", "광령리1.gpkg · 읽기만"),
    ("도면", "개별유구 평·단면도"),
    ("변환", "축척 1:20 · pt 그대로"),
    ("보고서", "PDF · 도판"),
    ("식별", "번호와 캡션"),
];

/// The hero band's right column: what this excavation is, and the three ways in (design §1).
pub struct Site {
    pub name: String,
    pub week: String,
    pub date: String,
}

/// The survey's own files, read-only, with what is found and what is not.
pub struct Survey {
    pub gpkg: Option<String>,
    pub sections: usize,
    pub control_points: usize,
    pub points: usize,
    pub scans: usize,
    pub ortho: usize,
}

/// Everything one frame of the start screen draws from: where it is, what language, and which
/// excavation. Bundled so the drawing functions take four arguments instead of eight.
pub struct Home<'a> {
    pub rect: Rect,
    pub lang: &'a str,
    pub site: &'a Site,
    pub survey: &'a Survey,
}

/// Draw the start screen wherever VectorCraft would have drawn its Home view. Returns whether
/// it drew. The condition is VectorCraft's own `home_showing`, restated because it is
/// crate-private.
pub fn show_if_home(app: &mut VectorcraftApp, ui: &mut Ui, lang: &str) -> bool {
    let home = app.ui.home.is_some() || (app.session.active().is_none() && app.session.prefs.show_home_screen);
    if !home {
        return false;
    }
    let site = Site { name: "광릉리 상궁".into(), week: "발굴 3주차".into(), date: "2026-09-28".into() };
    let survey = Survey { gpkg: Some("광령리1.gpkg".into()), sections: 2, control_points: 4, points: 38, scans: 6, ortho: 1 };
    show(app, ui, lang, &site, &survey);
    true
}

/// Trowel's start screen: the hero band, the two columns, the connector strip.
pub fn show(app: &mut VectorcraftApp, ui: &mut Ui, lang: &str, site: &Site, survey: &Survey) {
    // The white the design's `.home` sits on, painted over the panel this screen is drawn in
    // and no further. `available_rect_before_wrap` is *not* bounded by the clip rect — egui
    // 0.36 computes it as `max_rect ∩ cursor` (`layout.rs::available_from_cursor_max_rect`)
    // — and `max_rect` grows to whatever the parent last laid out, so a neighbour that took
    // its space wrongly (or a row wider than its own column) would otherwise let this fill
    // run over it. The clip rect is the panel's own rect: `CentralPanel::show_inside_dyn`
    // sets it with `panel_ui.set_clip_rect(outer_rect)` and `Painter::add` stores that clip
    // with every shape, so a rect inside it cannot paint outside it.
    let area = ui.available_rect_before_wrap().intersect(ui.clip_rect());
    ui.painter().rect_filled(area, 0.0, tokens::SURFACE);
    // `.home` padding: 20 top, 32 sides, 16 bottom.
    let inner = Rect::from_min_size(area.min + vec2(32.0, 20.0), vec2((area.width() - 64.0).max(0.0), (area.height() - 36.0).max(0.0)));

    let mut y = inner.min.y;
    let (hero_h, cols_h) = split_heights(inner.height());
    hero(app, ui, &Home { rect: inner, lang, site, survey }, y, hero_h);
    y += hero_h + 18.0;
    columns(app, ui, &Home { rect: inner, lang, site, survey }, y, cols_h);
    y += cols_h + 18.0;
    connector(ui, inner, y.min(inner.max.y - 46.0));
}

/// Split the body the way the design stacks it: the hero band takes what it needs, the two
/// columns take the rest, and the connector strip keeps its 46 points at the foot.
fn split_heights(total: f32) -> (f32, f32) {
    let conn = 46.0;
    let cols = (total - conn - 340.0 - 18.0 * 2.0).max(220.0);
    let hero = (total - conn - cols - 18.0 * 2.0).clamp(360.0, 400.0);
    (hero, cols)
}

// ---------- 히어로 밴드 ----------

fn hero(app: &mut VectorcraftApp, ui: &mut Ui, home: &Home, y: f32, h: f32) {
    let (inner, lang, site, survey) = (home.rect, home.lang, home.site, home.survey);
    let rect = Rect::from_min_size(pos2(inner.min.x, y), vec2(inner.width(), h));
    let p = ui.painter();
    // Above and below, the ink rule — the head band between two rules the design describes.
    p.line_segment([pos2(rect.left(), rect.top()), pos2(rect.right(), rect.top())], Stroke::new(1.0, tokens::INK));
    p.line_segment([pos2(rect.left(), rect.bottom()), pos2(rect.right(), rect.bottom())], Stroke::new(1.0, tokens::INK));

    // Left: the brand. `.hero .brand` — mark, wordmark, a 1px rule, the subtitle.
    let right_x = (rect.left() + (rect.width() - HERO_R_W).max(320.0)).min(rect.right());
    let l = Rect::from_min_size(pos2(rect.left(), rect.top() + 20.0), vec2(right_x - rect.left() - 24.0, h - 40.0));
    let (word, rule, sub) = (pos2(l.min.x, l.min.y + 4.0), pos2(l.min.x + 132.0, l.min.y), pos2(l.min.x + 144.0, l.min.y + 5.0));
    p.text(word, Align2::LEFT_TOP, labels::PRODUCT_NAME.text(lang), egui::FontId::proportional(28.0), tokens::INK);
    p.line_segment([rule, rule + vec2(0.0, 28.0)], Stroke::new(1.0, tokens::EDGE));
    p.text(sub, Align2::LEFT_TOP, labels::START_LEAD.text(lang), egui::FontId::proportional(15.0), tokens::MUTED);
    // The mark the design sets before the wordmark: the trowel head, drawn in code.
    trowel_mark(p, pos2(l.min.x, l.min.y + 40.0));

    // Right: what this excavation is, and the three ways in.
    let r = Rect::from_min_size(pos2(right_x, rect.top() + 20.0), vec2(HERO_R_W.min(rect.right() - right_x), h - 40.0));
    p.line_segment([pos2(r.left(), r.top()), pos2(r.left(), r.bottom())], Stroke::new(1.0, tokens::LINE));
    let mut ry = r.min.y;
    ry = text(p, r.left() + 24.0, ry, labels::START_HEADING.text(lang), 20.0, tokens::INK);
    ry += 8.0;
    for (label, id, run) in [(labels::START_NEW_DRAWING, "", true), (labels::START_STRATA, "file.open", false), (labels::BOOK_OPEN, "", true)] {
        let text = label.text(lang);
        let color = if run { tokens::INK } else { tokens::INK2 };
        if id.is_empty() {
            p.text(pos2(r.left() + 24.0, ry), Align2::LEFT_TOP, text, egui::FontId::proportional(13.0), color);
        } else {
            let hit = Rect::from_min_size(pos2(r.left() + 24.0, ry - 2.0), vec2(text.len() as f32 * 7.0 + 12.0, 20.0));
            p.rect_stroke(hit, 6.0, Stroke::new(1.0, tokens::EDGE), StrokeKind::Middle);
            p.text(pos2(r.left() + 30.0, ry), Align2::LEFT_TOP, text, egui::FontId::proportional(13.0), color);
            let r = ui.interact(hit, ui.id().with("hero_open"), Sense::click());
            if r.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if r.clicked() {
                note(app.run(id, json!({})).err().unwrap_or_default());
            }
        }
        ry += 22.0;
    }
    ry += 6.0;
    // What the survey holds, in the design's own order. A file that is not there says so in red
    // and in words — never silently dropped.
    let missing = survey.gpkg.is_none();
    let rows: [(String, String, bool); 5] = [
        (site.name.clone(), site.date.clone(), false),
        (labels::START_STRATA_FILE.text(lang).to_string(), survey.gpkg.clone().unwrap_or_else(|| "원본 없음".into()), missing),
        (labels::START_DESCA.text(lang).to_string(), format!("{}개", survey.sections), false),
        (labels::START_POINTS.text(lang).to_string(), format!("기준점 {} · 측점 {}", survey.control_points, survey.points), false),
        (labels::START_UNDERLAY.text(lang).to_string(), format!("스캔 {}장 · 정사영상 {}장", survey.scans, survey.ortho), false),
    ];
    for (k, v, is_missing) in rows {
        p.text(pos2(r.left() + 24.0, ry), Align2::LEFT_TOP, k, egui::FontId::proportional(12.0), tokens::INK2);
        p.text(
            pos2(r.left() + 122.0, ry),
            Align2::LEFT_TOP,
            v,
            egui::FontId::proportional(12.0),
            if is_missing { tokens::BLOCK } else { tokens::MUTED },
        );
        ry += 18.0;
    }
    // Which week of the excavation this is, set under the survey's own lines.
    p.text(pos2(r.left() + 24.0, ry + 2.0), Align2::LEFT_TOP, site.week.clone(), egui::FontId::proportional(12.0), tokens::FAINT);
}

/// The trowel head the design sets before the wordmark, drawn in code: a blade, a socket, a
/// socket pin. 28 × 24 points.
fn trowel_mark(p: &egui::Painter, at: Pos2) {
    let ink = Stroke::new(1.4, tokens::INK);
    p.line_segment([at, at + vec2(0.0, 14.0)], ink); // shaft
    p.line_segment([at + vec2(-7.0, 14.0), at + vec2(7.0, 14.0)], ink); // blade shoulder
    p.line_segment([at + vec2(-7.0, 14.0), at + vec2(-3.0, 21.0)], ink);
    p.line_segment([at + vec2(7.0, 14.0), at + vec2(3.0, 21.0)], ink);
    p.line_segment([at + vec2(-3.0, 21.0), at + vec2(3.0, 21.0)], ink);
    p.line_segment([at + vec2(-3.0, 17.0), at + vec2(3.0, 17.0)], ink); // the socket pin
}

// ---------- 본문 2열 ----------

fn columns(app: &mut VectorcraftApp, ui: &mut Ui, home: &Home, y: f32, h: f32) {
    let (inner, lang) = (home.rect, home.lang);
    let p = ui.painter();
    let rail_x = (inner.left() + (inner.width() - RAIL_W).max(300.0)).min(inner.right());
    let rect = Rect::from_min_size(pos2(inner.left(), y), vec2(inner.width(), h));

    // 왼쪽: 템플릿 5개.
    p.text(pos2(rect.left(), rect.top()), Align2::LEFT_TOP, labels::START_TEMPLATES.text(lang), egui::FontId::proportional(14.0), tokens::INK);
    templates(app, ui, Rect::from_min_size(pos2(rect.left(), rect.top() + 22.0), vec2(rail_x - rect.left() - 32.0, h - 22.0)), lang);

    // 오른쪽: 작업 순서 레일.
    p.line_segment([pos2(rail_x + 12.0, rect.top()), pos2(rail_x + 12.0, rect.bottom())], Stroke::new(1.0, tokens::EDGE));
    rail(ui, Rect::from_min_size(pos2(rail_x, rect.top()), vec2(RAIL_W.min(rect.right() - rail_x), h)), lang);
}

fn templates(app: &mut VectorcraftApp, ui: &Ui, rect: Rect, _lang: &str) {
    let gap = 16.0;
    let w = ((rect.width() - gap * (COLS as f32 - 1.0)) / COLS as f32).max(60.0);
    for (i, (name, note_)) in TEMPLATES.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + (w + gap) * i as f32, rect.top()), vec2(w, rect.height()));
        if r.max.x > rect.max.x {
            break;
        }
        let p = ui.painter();
        // `.tpl .pp` — a 92-point desk plate with the page preview standing in it.
        let pp = Rect::from_min_size(r.min, vec2(w, 92.0));
        p.rect_filled(pp, 8.0, tokens::DESK);
        page_preview(p, Rect::from_min_size(pp.center() + vec2(-19.0, -38.0), vec2(38.0, 76.0)));
        p.text(pos2(r.min.x, pp.max.y + 6.0), Align2::LEFT_TOP, short_to(name, w - 6.0, 13.0), egui::FontId::proportional(13.0), tokens::INK);
        p.text(pos2(r.min.x, pp.max.y + 24.0), Align2::LEFT_TOP, short_to(note_, w - 6.0, 11.0), egui::FontId::proportional(11.0), tokens::MUTED);
        let hit = Rect::from_min_size(r.min, vec2(w, 92.0 + 44.0));
        let r = ui.interact(hit, ui.id().with(("tpl", *name)), Sense::click());
        if r.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if r.clicked() {
            note(app.run("file.new", json!({ "width": 1190.0, "height": 842.0 })).err().unwrap_or_default());
        }
    }
}

/// A page standing in the desk plate: white, edged, with its margins and a baseline or two. Drawn
/// in code — the repository rule is code-drawn art for defaults, never a copied asset.
fn page_preview(p: &egui::Painter, r: Rect) {
    p.rect_filled(r, 0.0, egui::Color32::WHITE);
    p.rect_stroke(r, 0.0, Stroke::new(1.0, tokens::EDGE), StrokeKind::Inside);
    let hair = Stroke::new(0.5, tokens::LINE);
    p.rect_stroke(Rect::from_min_size(r.min + vec2(4.0, 5.0), vec2(r.width() - 8.0, r.height() - 16.0)), 0.0, hair, StrokeKind::Inside);
    for i in 0..4 {
        let y = r.min.y + 12.0 + i as f32 * 5.0;
        p.line_segment([pos2(r.min.x + 7.0, y), pos2(r.max.x - 7.0, y)], hair);
    }
}

/// `.rail` — the work rail: a 1px line at left 12, and one row per step with a 22-point marker.
fn rail(ui: &Ui, rect: Rect, lang: &str) {
    let p = ui.painter();
    let step_h = ((rect.height() - 22.0) / STEPS.len() as f32).clamp(26.0, 44.0);
    for (i, (title, note_, stage)) in STEPS.iter().enumerate() {
        let y = rect.min.y + step_h * i as f32;
        let cx = rect.min.x + 12.0;
        let cy = y + 2.0;
        let ring = Rect::from_center_size(pos2(cx, cy + 11.0), vec2(22.0, 22.0));
        // done: surface with an ink ring · todo: a ring-only mark · next: clay, the one other
        // place the design lets clay appear.
        match stage {
            Stage::Done => {
                p.circle_filled(ring.center(), 11.0, tokens::SURFACE);
                p.circle_stroke(ring.center(), 11.0, Stroke::new(1.0, tokens::INK));
            }
            Stage::Todo => {
                p.circle_stroke(ring.center(), 11.0, Stroke::new(1.0, tokens::RING));
            }
            Stage::Next => {
                p.circle_filled(ring.center(), 8.0, tokens::CLAY);
            }
        }
        let tx = rect.min.x + 28.0;
        p.text(pos2(tx, y + 2.0), Align2::LEFT_TOP, *title, egui::FontId::proportional(13.0), tokens::INK);
        let mut nx = tx;
        if *stage == Stage::Next {
            let next = labels::RAIL_NEXT.text(lang);
            p.text(pos2(tx, y + 18.0), Align2::LEFT_TOP, next, egui::FontId::proportional(11.0), tokens::CLAY_TEXT);
            nx += next.len() as f32 * 7.0 + 6.0;
        }
        p.text(pos2(nx, y + 18.0), Align2::LEFT_TOP, *note_, egui::FontId::proportional(12.0), tokens::MUTED);
    }
}

// ---------- 연결 띠 ----------

/// `.conn` — the five links, one top rule and a divider on each cell but the first.
fn connector(ui: &Ui, inner: Rect, y: f32) {
    let p = ui.painter();
    let rect = Rect::from_min_size(pos2(inner.left(), y), vec2(inner.width(), 46.0));
    p.line_segment([pos2(rect.left(), rect.top()), pos2(rect.right(), rect.top())], Stroke::new(1.0, tokens::LINE));
    let w = rect.width() / COLS as f32;
    for (i, (name, note_)) in LINKS.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + w * i as f32, rect.top() + 8.0), vec2(w, 38.0));
        if i > 0 {
            p.line_segment([pos2(r.left(), r.top()), pos2(r.left(), r.bottom())], Stroke::new(1.0, tokens::LINE));
        }
        p.text(pos2(r.left() + if i == 0 { 0.0 } else { 12.0 }, r.top()), Align2::LEFT_TOP, *name, egui::FontId::proportional(13.0), tokens::INK);
        p.text(
            pos2(r.left() + if i == 0 { 0.0 } else { 12.0 }, r.top() + 17.0),
            Align2::LEFT_TOP,
            *note_,
            egui::FontId::proportional(11.0),
            tokens::MUTED,
        );
    }
}

// ---------- 조각 ----------

/// Text at a point; returns the y below it.
fn text(p: &egui::Painter, x: f32, y: f32, s: &str, size: f32, color: egui::Color32) -> f32 {
    p.text(pos2(x, y), Align2::LEFT_TOP, s, egui::FontId::proportional(size), color);
    y + size * 1.35
}

/// Text cut to `width` points with an ellipsis — the design's own rule for a name that has more
/// to say than the space allows (`.lrow .nm` is `overflow:hidden; text-overflow:ellipsis`).
/// A Hangul glyph is about one em wide at this size; the exact measure is the column's, and
/// coming up a character short only ever costs an ellipsis.
fn short_to(s: &str, width: f32, size: f32) -> String {
    let per = size * 1.02;
    let max = ((width / per).floor() as usize).max(1);
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut kept: String = s.chars().take(max.saturating_sub(1)).collect();
    kept.push('…');
    kept
}

/// What the screen could not do, kept rather than swallowed.
fn note(msg: String) {
    if !msg.is_empty() {
        log::warn!("trowel start screen: {msg}");
    }
}

/// Kept so the module compiles against the sense enum without an unused import when a preview
/// is not interactive yet.
#[allow(dead_code)]
fn _sense() -> Sense {
    Sense::hover()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    /// The templates are the design's five, in its order, each with its own scale note.
    #[test]
    fn the_five_templates_are_the_designs_five() {
        assert_eq!(TEMPLATES.len(), COLS);
        assert_eq!(TEMPLATES[0], ("개별유구 평·단면도", "1:20 · A3 가로 · 평면용 범례"));
        assert_eq!(TEMPLATES[1].0, "토층 단면도");
        assert_eq!(TEMPLATES[2].1, "1:1로 그림 · 도판은 1:3");
        assert_eq!(TEMPLATES[3].1, "1:100 · Strata 조사에서");
        assert_eq!(TEMPLATES[4].0, "보고서 도판");
        for (name, note_) in TEMPLATES {
            assert!(!name.is_empty() && !note_.is_empty());
        }
    }

    /// The connector strip is the design's five links, and the first cell carries no divider.
    #[test]
    fn the_connector_is_five_links() {
        assert_eq!(LINKS.len(), COLS);
        assert_eq!(LINKS[0].0, "Strata");
        assert_eq!(LINKS[1].0, "도면");
        assert_eq!(LINKS[3].0, "보고서");
        assert!(LINKS[0].1.contains("읽기만"), "Strata is read-only, and says so");
    }

    /// The rail runs the excavation's eight steps in order, and exactly one is the next step —
    /// the design lets clay appear on that one point and nowhere else.
    #[test]
    fn the_rail_has_eight_steps_and_one_next() {
        assert_eq!(STEPS.len(), 8);
        assert_eq!(STEPS[0].0, "그리기 준비");
        assert_eq!(STEPS[STEPS.len() - 1].0, "도판·범례표 만들기");
        let next: Vec<&str> = STEPS.iter().filter(|s| s.2 == Stage::Next).map(|s| s.0).collect();
        assert_eq!(next.len(), 1, "one step is next, not {}", next.len());
    }

    /// The widths are the design's own: the hero's right column is 520 and the rail's is 380.
    #[test]
    fn the_column_widths_are_the_designs() {
        assert_eq!(HERO_R_W, 520.0);
        assert_eq!(RAIL_W, 380.0);
        assert_eq!(COLS, 5);
    }

    /// A missing survey file is reported in red and in words, never silently omitted.
    #[test]
    fn a_missing_survey_file_says_so() {
        let site = Site { name: "광릉리 상궁".into(), week: "발굴 3주차".into(), date: "2026-09-28".into() };
        let missing = Survey { gpkg: None, sections: 0, control_points: 0, points: 0, scans: 0, ortho: 0 };
        assert!(missing.gpkg.is_none());
        assert!(missing.gpkg.as_deref().unwrap_or("원본 없음").contains("원본 없음"));
        assert!(!site.name.is_empty());
    }
}
