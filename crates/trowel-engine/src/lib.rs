//! Trowel's archaeological domain.
//!
//! The app shell (`apps/trowel`) draws the window; this crate is what the drawing means.
//! It sits at L6 beside `vectorcraft-ui-egui` and depends on nothing of the engine, so every
//! rule here is a plain value that can be written down and tested:
//!
//! - [`legend`] — the legend, and the rule that a legend line **is** a layer (design 2.1 §7).
//! - [`style`] — the stroke shape a line stands for: weight on the report paper, dash, hatch.
//! - [`preset`] — the three sets of lines a new drawing lays out (§7).
//! - [`library`] — reading and writing a set as a file to share with colleagues and institutions (§8, J018).
//!
//! Nothing here panics and nothing reaches for a file the user did not name; `Result` and
//! `Option` all the way, as `AGENTS.md` requires of shipped code.

pub mod legend;
pub mod library;
pub mod preset;
pub mod style;

pub use legend::{Group, LegendEntry, LegendTable, StrokeChange};
pub use style::{Dash, Hatch, Weight};

/// The product this crate is part of, as the window and the file header write it.
pub const PRODUCT: &str = "Trowel";

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    #[test]
    fn the_product_is_trowel() {
        assert_eq!(super::PRODUCT, "Trowel");
    }
}
