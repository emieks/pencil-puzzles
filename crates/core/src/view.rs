//! A minimal rectangular cell view for browser clients (I4, P10).
/// A cell mark offered by a genre.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct MarkOption {
    /// Stable value used when submitting answers.
    pub value: String,
    /// Human-readable accessible name.
    pub name: String,
    /// Whether the cell is filled rather than displaying a text symbol.
    pub shaded: bool,
    /// Symbol drawn in the cell; empty for an unmarked cell.
    pub symbol: String,
}
impl MarkOption {
    /// A shaded cell mark.
    pub fn shaded(value: &str, name: &str) -> Self {
        let mut mark = Self::new(value, name, "");
        mark.shaded = true;
        mark
    }

    /// A mark with its accessible name and displayed symbol.
    pub fn new(value: &str, name: &str, symbol: &str) -> Self {
        Self {
            shaded: false,
            value: value.into(),
            name: name.into(),
            symbol: symbol.into(),
        }
    }
}
/// One cell in row-major order.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct ViewCell {
    /// Current mark or clue value.
    pub value: String,
    /// Optional region index for highlighting rule violations.
    pub region: Option<u32>,
    /// Whether a player may change the cell.
    pub editable: bool,
}
impl ViewCell {
    /// An editable cell with the given mark.
    pub fn editable(value: String) -> Self {
        Self {
            value,
            editable: true,
            region: None,
        }
    }
}
/// A rectangular board with labels outside its four sides (I4).
///
/// This initial view covers cell-marking games. Additional layers can be
/// introduced when a genre requires edges, vertices or regions.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct View {
    /// Number of rows.
    pub rows: u32,
    /// Number of columns.
    pub cols: u32,
    /// Cells in row-major order.
    pub cells: Vec<ViewCell>,
    /// Column labels above the board.
    pub top: Vec<String>,
    /// Row clues to the right.
    pub right: Vec<String>,
    /// Column clues below the board.
    pub bottom: Vec<String>,
    /// Row labels to the left.
    pub left: Vec<String>,
    /// Available marks in cycling order, unmarked first.
    pub marks: Vec<MarkOption>,
    /// Row indices with a strong boundary before them.
    pub row_separators: Vec<u32>,
    /// Column indices with a strong boundary before them.
    pub col_separators: Vec<u32>,
    /// What happens to unmarked cells when checking a finished answer.
    pub finish_note: String,
    /// Short instructions for the displayed rules.
    pub instructions: String,
}

/// A numeric clue input in a visual puzzle editor.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct ClueInput {
    /// The cell, row or column carrying the clue.
    pub place: crate::Place,
    /// Current text, empty when no clue has been entered.
    pub value: String,
    /// Smallest accepted clue.
    pub min: u32,
    /// Largest accepted clue.
    pub max: u32,
    /// Whether an empty field is a valid absent clue.
    pub optional: bool,
}
impl ClueInput {
    /// A bounded numeric clue input.
    pub fn new(place: crate::Place, value: String, min: u32, max: u32, optional: bool) -> Self {
        Self {
            place,
            value,
            min,
            max,
            optional,
        }
    }
}
/// A visual clue editor supplied by a genre, independent of browser code.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct PuzzleEditor {
    /// Stable shape encoding used to submit the fields.
    pub shape: String,
    /// Background grid, labels and region boundaries.
    pub view: View,
    /// Inputs in the genre's canonical order.
    pub fields: Vec<ClueInput>,
}
impl PuzzleEditor {
    /// A visual clue editor for a validated shape.
    pub fn new(shape: String, view: View, fields: Vec<ClueInput>) -> Self {
        Self {
            shape,
            view,
            fields,
        }
    }
}
