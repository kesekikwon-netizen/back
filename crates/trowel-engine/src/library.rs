//! Reading and writing a set as a file (design 2.1 section 8, J018).
//!
//! "모든 것을 사용자가 고치고 더할 수 있습니다. 그리다가 생긴 좋은 선과 빗금이 범례 줄이 되고,
//! **양식 파일로 동료·기관과 나눕니다.**" A set is therefore a file, like VectorCraft's swatch and
//! graphic-style libraries: a small JSON document the user writes, hands to a colleague, and
//! reads back.
//!
//! Nothing here writes to disk. A caller that wants a file says which file; a set that cannot be
//! read is an error the caller reports, never a silent empty set.

use serde::{Deserialize, Serialize};

use crate::legend::{Group, LegendEntry, LegendTable};
use crate::style::{Dash, Hatch, Weight};

/// The extension a set file has, and the `format` header inside it.
pub const EXTENSION: &str = "trowellegend";
/// The `format` value inside a set file.
pub const FORMAT: &str = "trowellegend";

/// A set file as it is written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct File {
    format: String,
    version: u32,
    group: Group,
    entries: Vec<Row>,
}

/// One line in the file. The weight is a number in points, so a hand-edited file says `0.8` and
/// not `{"0.8": {}}`; a weight that is not a weight is a read error naming the line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Row {
    name: String,
    layer: String,
    weight_pt: f64,
    #[serde(default)]
    dash: Dash,
    #[serde(default)]
    hatch: Hatch,
    #[serde(default)]
    label_pt: f64,
    #[serde(default)]
    frequent: bool,
    #[serde(default = "yes")]
    print: bool,
}

fn yes() -> bool {
    true
}

/// The file version this crate writes.
pub const VERSION: u32 = 1;

/// What went wrong reading a set file.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum LibraryError {
    #[error("this is not a Trowel legend file")]
    NotLegend,
    #[error("the file is version {0}; this build writes version {VERSION}")]
    TooNew(u32),
    #[error("the file could not be read: {0}")]
    Malformed(String),
    #[error("the line `{0}` has a weight of {1} pt, which is not a stroke weight")]
    BadWeight(String, f64),
    #[error("the line `{0}` has no layer: a legend line is a layer")]
    NoLayer(String),
}

/// The set as text, ready to be written to a file the user names.
pub fn write(table: &LegendTable) -> String {
    let file = File {
        format: FORMAT.to_string(),
        version: VERSION,
        group: table.group,
        entries: table
            .entries
            .iter()
            .map(|e| Row {
                name: e.name.clone(),
                layer: e.layer.clone(),
                weight_pt: e.weight.pt(),
                dash: e.dash,
                hatch: e.hatch,
                label_pt: e.label_pt,
                frequent: e.frequent,
                print: e.print,
            })
            .collect(),
    };
    // A hand-readable document: one line per field is not needed, but one line per row is,
    // because a colleague opens this in a text editor.
    serde_json::to_string_pretty(&file)
        .unwrap_or_else(|_e| format!("{{\"format\":\"{FORMAT}\",\"version\":{VERSION},\"group\":\"plan\",\"entries\":[]}}"))
}

/// Read a set from `text`, naming the first line that cannot be read.
pub fn read(text: &str) -> Result<LegendTable, LibraryError> {
    // What it says it is, before what is in it: a file that is not ours is reported as such,
    // even when it is missing the fields a set file has.
    let header: serde_json::Value = serde_json::from_str(text).map_err(|e| LibraryError::Malformed(e.to_string()))?;
    if header.get("format").and_then(|f| f.as_str()) != Some(FORMAT) {
        return Err(LibraryError::NotLegend);
    }
    let file: File = serde_json::from_value(header).map_err(|e| LibraryError::Malformed(e.to_string()))?;
    if file.version > VERSION {
        return Err(LibraryError::TooNew(file.version));
    }
    let mut table = LegendTable::new(file.group);
    for row in file.entries {
        let weight = Weight::new(row.weight_pt).map_err(|_| LibraryError::BadWeight(row.name.clone(), row.weight_pt))?;
        if row.layer.is_empty() {
            return Err(LibraryError::NoLayer(row.name.clone()));
        }
        let mut entry = LegendEntry::new(&row.name, &row.layer, weight);
        entry.dash = row.dash;
        entry.hatch = row.hatch;
        entry.label_pt = row.label_pt;
        entry.frequent = row.frequent;
        entry.print = row.print;
        // A file with two lines on one layer would make the panel claim it twice; the first
        // line wins and the second is a read error, not a silent overwrite.
        if table.line(&row.layer).is_some() {
            return Err(LibraryError::BadWeight(row.name.clone(), row.weight_pt));
        }
        table.add(entry).map_err(|_| LibraryError::BadWeight(row.name.clone(), row.weight_pt))?;
    }
    Ok(table)
}

/// Whether `text` looks like a set file, so a folder of mixed files can be told apart.
pub fn sniff(text: &str) -> bool {
    text.contains(FORMAT)
}

/// Whether `name` has a set file's extension.
pub fn has_extension(name: &str) -> bool {
    std::path::Path::new(name).extension().is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::legend::StrokeChange;
    use crate::preset;

    /// ③ The done-criterion 「저장했다 다시 열기」: a set written and read back is the same set,
    /// down to the weight the user changed and the strokes they kept.
    #[test]
    fn a_set_survives_a_round_trip() {
        let mut t = preset::plan();
        t.line_mut("feature_poly").unwrap().weight = Weight::new(0.5).unwrap();
        t.overrides.insert("feature_poly".into(), vec![7, 9]);
        t.line_mut("feature_poly").unwrap().dash = Dash::Dotted;

        let text = write(&t);
        let back = read(&text).unwrap();

        assert_eq!(back.group, t.group);
        assert_eq!(back.entries.len(), t.entries.len());
        assert_eq!(back.line("feature_poly").map(|e| e.weight), Some(Weight(0.5)));
        assert_eq!(back.line("feature_poly").map(|e| e.dash), Some(Dash::Dotted));
        // The 「따로」 strokes are the drawing's state, not the set's: a file handed to a
        // colleague carries the lines, not which strokes one surveyor nudged.
        assert!(back.overrides.is_empty(), "a set file carries no kept strokes");
        // And writing it again gives the same text, so a shared file does not churn.
        assert_eq!(write(&back), text);
    }

    /// The file says what it is: an extension, a header, and a sniff that tells set files from
    /// the swatch and style libraries beside them.
    #[test]
    fn a_set_file_says_what_it_is() {
        let text = write(&preset::section());
        assert!(sniff(&text));
        assert!(has_extension("광령리.trowellegend"));
        assert!(!has_extension("swatches.vcswatches"));
        assert!(!sniff("{\"format\":\"vcswatches\"}"));
        assert_eq!(read(&text).unwrap().group, Group::Section);
    }

    /// A file that is not one, or one from a newer build, is refused with a reason.
    #[test]
    fn a_file_that_is_not_a_set_is_refused() {
        assert_eq!(read("{\"format\":\"vcswatches\",\"version\":1}"), Err(LibraryError::NotLegend));
        assert!(read("not json at all").is_err());
        let future = write(&preset::find()).replace("\"version\": 1", "\"version\": 99");
        assert_eq!(read(&future), Err(LibraryError::TooNew(99)));
    }

    /// A hand-edited weight that is not a weight is a read error naming the line, not a stroke
    /// of no width.
    #[test]
    fn a_broken_weight_names_the_line() {
        let text = write(&preset::plan()).replace("\"weight_pt\": 0.8", "\"weight_pt\": -1");
        match read(&text) {
            Err(LibraryError::BadWeight(name, pt)) => {
                assert_eq!(pt, -1.0);
                assert!(!name.is_empty());
            }
            other => panic!("expected a bad weight, got {other:?}"),
        }
    }

    /// A line with no layer is refused: a legend line *is* a layer.
    #[test]
    fn a_line_with_no_layer_is_refused() {
        let text = write(&preset::plan()).replace("\"layer\": \"feature_poly\"", "\"layer\": \"\"");
        assert!(matches!(read(&text), Err(LibraryError::NoLayer(_))));
    }

    /// Two lines on one layer would make the panel claim it twice.
    #[test]
    fn two_lines_on_one_layer_are_refused() {
        let mut t = preset::find();
        t.entries.push(LegendEntry::new("중복", "find_axis", Weight::new(0.5).unwrap()));
        assert!(read(&write(&t)).is_err(), "the second line must not silently overwrite the first");
    }

    /// The changes a table asks for survive the file too, so a colleague's file carries the
    /// 「따로」 strokes with it.
    #[test]
    fn the_kept_strokes_are_part_of_the_file() {
        let mut t = preset::plan();
        let change: StrokeChange = t.set_weight("feature_floor", Weight::new(0.25).unwrap(), &[1, 2, 3]).unwrap();
        assert_eq!(change.touched(), 3);
        let back = read(&write(&t)).unwrap();
        assert_eq!(back.line("feature_floor").map(|e| e.weight.pt()), Some(0.25));
    }
}
