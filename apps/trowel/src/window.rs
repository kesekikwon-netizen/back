//! Trowel window geometry.
//!
//! The default window size is the one the design 2.1 mock-up shows (section 3:
//! 1600 × 988 — caption 32, ribbon tile 44, control bar 40, tab 32, status bar 32).

use egui::Vec2;

/// Default inner size in points, from the design 2.1 mock-up.
pub const DEFAULT_SIZE: Vec2 = Vec2::new(1600.0, 988.0);

/// Smallest inner size that still fits the drawing window: ribbon + control bar + document tabs +
/// the left tool column and the right column of the drawing palette, the legend and the properties.
/// Narrower than this and the right column would have nothing to show.
pub const MIN_SIZE: Vec2 = Vec2::new(1100.0, 700.0);
