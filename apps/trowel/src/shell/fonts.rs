//! Fallback fonts for Trowel's own text.
//!
//! The interface fonts (`SourceSans3`, `JetBrainsMono`) carry no Hangul, so every Korean
//! label in the design — 굴광선, 수혈, 토층, 도면집 — would draw as missing-glyph boxes.
//! `vectorcraft_ui_egui::ui_fonts` solves this for VectorCraft, but it is `pub(crate)` and
//! runs from the frame layout Trowel replaces, so Trowel does it itself here: after a frame
//! is laid out, the characters it painted that the UI fonts lack are collected, an installed
//! font covering them is looked for off the UI thread, and it joins every egui family as their
//! last fallback from the next frame on. Text the UI fonts cover costs no font loading.
//!
//! Nothing is bundled and nothing is downloaded: the fonts are the ones already installed on
//! the machine, read through the engine's font database.

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use egui::epaint::Shape;
use egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};
use egui::{Color32, FontData, FontFamily, FontId, LayerId};
use vectorcraft_text::{FontDb, FontFace};

/// Characters already looked at: the interface fonts have them, a fallback was added or is
/// being looked for, or no installed font has them.
#[derive(Default)]
pub struct Fallback {
    seen: HashSet<char>,
    /// Faces already added to egui, by the database's own face id.
    added: HashSet<u32>,
    /// The search running, if any.
    pending: Option<Receiver<Vec<Arc<FontFace>>>>,
}

impl Fallback {
    /// Start reading the installed fonts, off the UI thread.
    pub fn new() -> Self {
        FontDb::global().scan_in_background();
        Self::default()
    }

    /// After a frame is laid out: add the fonts a finished search found, then look for fonts
    /// covering the characters this frame painted that the interface fonts lack.
    pub fn after_frame(&mut self, ctx: &egui::Context) {
        if let Some(faces) = self.pending.as_ref() {
            match faces.try_recv() {
                Ok(found) => {
                    self.pending = None;
                    self.install(ctx, &found);
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => self.pending = None,
            }
        }
        let missing = self.missing(ctx);
        if missing.is_empty() {
            return;
        }
        let (tx, rx) = channel();
        let repaint = ctx.clone();
        let spawned = std::thread::Builder::new().name("trowel-font-fallback".into()).spawn(move || {
            let mut faces: Vec<Arc<FontFace>> = Vec::new();
            for c in missing {
                if let Some(f) = FontDb::global().face_covering(c)
                    && !faces.iter().any(|x| x.id() == f.id())
                {
                    faces.push(f);
                }
            }
            // The receiver is gone only when the app is.
            let _ = tx.send(faces);
            repaint.request_repaint();
        });
        if spawned.is_ok() {
            self.pending = Some(rx);
        }
    }

    /// The characters painted this frame that the interface fonts lack and that haven't been
    /// looked for, now marked seen. Empty while a search is running.
    fn missing(&mut self, ctx: &egui::Context) -> Vec<char> {
        if self.pending.is_some() {
            return Vec::new();
        }
        let mut layers: Vec<LayerId> = ctx.memory(|m| m.layer_ids().collect());
        if !layers.contains(&LayerId::background()) {
            layers.push(LayerId::background());
        }
        let mut chars: Vec<char> = Vec::new();
        let seen = &self.seen;
        let mut note = |text: &str| {
            // Most interface text is ASCII, which the interface fonts cover.
            if !text.is_ascii() {
                chars.extend(text.chars().filter(|c| !c.is_ascii() && !c.is_whitespace() && !c.is_control() && !seen.contains(c)));
            }
        };
        ctx.graphics(|g| {
            for l in &layers {
                for s in g.get(*l).into_iter().flat_map(|list| list.all_entries()) {
                    painted_text(&s.shape, &mut note);
                }
            }
        });
        if chars.is_empty() {
            return chars;
        }
        chars.sort_unstable();
        chars.dedup();
        let probe = FontId::new(13.0, FontFamily::Proportional);
        let missing: Vec<char> = ctx.fonts_mut(|f| chars.iter().copied().filter(|c| !f.has_glyph(&probe, *c)).collect());
        self.seen.extend(chars);
        missing
    }

    /// Add `faces` to every egui family, after the fonts already in it.
    fn install(&mut self, ctx: &egui::Context, faces: &[Arc<FontFace>]) {
        let families: Vec<FontFamily> = ctx.fonts(|f| f.definitions().families.keys().cloned().collect());
        for face in faces {
            if !self.added.insert(face.id()) {
                continue;
            }
            let name = format!("trowel-fallback {} {} #{}", face.family, face.style, face.face_index());
            // The font database already holds the bytes for the session; leaking one handle
            // per face added is cheaper than copying them.
            let face: &'static FontFace = Box::leak(Box::new(face.clone()));
            let data = FontData { index: face.face_index(), ..FontData::from_static(face.file_data()) };
            let into = families.iter().map(|family| InsertFontFamily { family: family.clone(), priority: FontPriority::Lowest }).collect();
            ctx.add_font(FontInsert { name, data, families: into });
        }
        if !faces.is_empty() {
            ctx.request_repaint();
        }
    }
}

/// Call `f` with the text of `shape` and the shapes it holds.
fn painted_text(shape: &Shape, f: &mut impl FnMut(&str)) {
    match shape {
        Shape::Text(t) => f(t.galley.text()),
        Shape::Vec(v) => v.iter().for_each(|s| painted_text(s, f)),
        _ => {}
    }
}

/// Keep the import of `Color32` honest: it is only here because `FontData`'s default fill.
#[allow(dead_code)]
fn _color_is_used(_: Color32) {}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    /// Latin text costs nothing: the interface fonts have it, so no search starts.
    #[test]
    fn latin_text_searches_no_fonts() {
        let ctx = egui::Context::default();
        vectorcraft_ui_egui::theme::install_fonts(&ctx);
        let mut fonts = Fallback::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.label("Layer 1 — Café · Open · Save");
            fonts.after_frame(ui.ctx());
        });
        out.textures_delta.clear();
        assert!(fonts.pending.is_none(), "no search for text the UI fonts cover");
        assert!(fonts.added.is_empty());
    }

    /// The characters the design writes are Hangul, which the interface fonts lack, so they
    /// are collected for a search. Whether an installed font covers them is the machine's
    /// business — on a machine with none, they stay boxes and nothing is installed.
    #[test]
    fn hangul_is_collected_for_a_search() {
        let ctx = egui::Context::default();
        vectorcraft_ui_egui::theme::install_fonts(&ctx);
        let mut fonts = Fallback::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.label("굴광선 수혈 토층 도면집 표고");
            fonts.after_frame(ui.ctx());
        });
        out.textures_delta.clear();
        assert!(fonts.pending.is_some(), "Hangul needs an installed font to draw");
        // Wait for it, as the next frame would.
        if let Some(rx) = fonts.pending.take()
            && let Ok(found) = rx.recv()
        {
            fonts.install(&ctx, &found);
            assert!(!found.is_empty(), "the machine has a font that draws Hangul");
        }
    }

    /// A character is looked for once: a later frame starts no second search.
    #[test]
    fn a_character_is_looked_for_once() {
        let ctx = egui::Context::default();
        vectorcraft_ui_egui::theme::install_fonts(&ctx);
        let mut fonts = Fallback::default();
        for _ in 0..2 {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.label("굴광선");
                fonts.after_frame(ui.ctx());
            });
            out.textures_delta.clear();
            if let Some(rx) = fonts.pending.take() {
                let _ = rx.recv();
            }
        }
        assert!(fonts.pending.is_none(), "the second frame must not start a search");
    }
}
