use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::calculator::format_number;

/// A cell address, counted from zero at A1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cell {
    pub col: usize,
    pub row: usize,
}

impl Cell {
    pub const fn new(col: usize, row: usize) -> Self {
        Self { col, row }
    }

    pub fn parse(name: &str) -> Option<Self> {
        let name = name.trim();
        let letters = name.bytes().take_while(u8::is_ascii_alphabetic).count();
        if letters == 0 || letters == name.len() {
            return None;
        }
        let mut col = 0usize;
        for letter in name[..letters].bytes() {
            col = col
                .checked_mul(26)?
                .checked_add(usize::from(letter.to_ascii_uppercase() - b'A' + 1))?;
        }
        let digits = &name[letters..];
        if !digits.bytes().all(|digit| digit.is_ascii_digit()) {
            return None;
        }
        let row = digits.parse::<usize>().ok()?.checked_sub(1)?;
        Some(Self::new(col - 1, row))
    }

    pub fn column_name(col: usize) -> String {
        let mut index = col as u128 + 1;
        let mut letters = Vec::new();
        while index > 0 {
            letters.push((b'A' + ((index - 1) % 26) as u8) as char);
            index = (index - 1) / 26;
        }
        letters.iter().rev().collect()
    }
}

impl fmt::Display for Cell {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}{}",
            Self::column_name(self.col),
            self.row as u128 + 1
        )
    }
}

/// Raw cell contents, the selected range, and an uncommitted edit.
#[derive(Clone, Debug, Default)]
pub struct Sheet {
    pub cells: BTreeMap<Cell, String>,
    pub cursor: Cell,
    pub anchor: Cell,
    pub editing: Option<String>,
}

impl Sheet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ledger() -> Self {
        let mut sheet = Self::new();
        for (row, values) in [
            ["Item", "Cost"],
            ["Rent", "800"],
            ["Groceries", "245.5"],
            ["Lamp oil", "12"],
            ["Books", "38"],
            ["", ""],
            ["Total", "=SUM(B2:B5)"],
            ["Average", "=AVERAGE(B2:B5)"],
        ]
        .iter()
        .enumerate()
        {
            for (col, value) in values.iter().enumerate() {
                sheet.set(Cell::new(col, row), *value);
            }
        }
        sheet.select(Cell::new(1, 6), false);
        sheet
    }

    pub fn set(&mut self, cell: Cell, text: impl Into<String>) {
        let text = text.into();
        if text.is_empty() {
            self.cells.remove(&cell);
        } else {
            self.cells.insert(cell, text);
        }
    }

    pub fn raw(&self, cell: Cell) -> &str {
        self.cells.get(&cell).map(String::as_str).unwrap_or("")
    }

    /// Evaluate a formula, or return the cell's text unchanged.
    pub fn value(&self, cell: Cell) -> String {
        let raw = self.raw(cell);
        if !raw.starts_with('=') {
            return raw.to_string();
        }
        let mut evaluation = Evaluation {
            sheet: self,
            active: BTreeSet::new(),
            cached: BTreeMap::new(),
        };
        match evaluation.cell(cell) {
            Ok(value) => format_number(value),
            Err(error) => error.to_string(),
        }
    }

    pub fn select(&mut self, cell: Cell, extend: bool) {
        self.commit_edit();
        self.cursor = cell;
        if !extend {
            self.anchor = cell;
        }
    }

    pub fn move_cursor(&mut self, col: i32, row: i32, cols: usize, rows: usize, extend: bool) {
        let cell = Cell::new(
            self.cursor
                .col
                .saturating_add_signed(col as isize)
                .min(cols.saturating_sub(1)),
            self.cursor
                .row
                .saturating_add_signed(row as isize)
                .min(rows.saturating_sub(1)),
        );
        self.select(cell, extend);
    }

    pub fn begin_edit(&mut self, text: impl Into<String>) {
        self.editing = Some(text.into());
    }

    pub fn commit_edit(&mut self) {
        if let Some(text) = self.editing.take() {
            self.set(self.cursor, text);
        }
    }

    pub fn cancel_edit(&mut self) {
        self.editing = None;
    }

    pub fn selected(&self, cell: Cell) -> bool {
        (self.anchor.col.min(self.cursor.col)..=self.anchor.col.max(self.cursor.col))
            .contains(&cell.col)
            && (self.anchor.row.min(self.cursor.row)..=self.anchor.row.max(self.cursor.row))
                .contains(&cell.row)
    }

    pub fn clear_selection(&mut self) {
        self.editing = None;
        let (a, b) = (self.anchor, self.cursor);
        self.cells.retain(|cell, _| {
            cell.col < a.col.min(b.col)
                || cell.col > a.col.max(b.col)
                || cell.row < a.row.min(b.row)
                || cell.row > a.row.max(b.row)
        });
    }

    pub fn selection_bounds(&self) -> (Cell, Cell) {
        (
            Cell::new(
                self.anchor.col.min(self.cursor.col),
                self.anchor.row.min(self.cursor.row),
            ),
            Cell::new(
                self.anchor.col.max(self.cursor.col),
                self.anchor.row.max(self.cursor.row),
            ),
        )
    }

    /// Copy raw values, preserving formulas when the rectangle is pasted again.
    pub fn copy_tsv(&self) -> String {
        let (first, last) = self.selection_bounds();
        let mut lines = Vec::new();
        for row in first.row..=last.row {
            let cells = (first.col..=last.col)
                .map(|col| self.raw(Cell::new(col, row)))
                .collect::<Vec<_>>();
            lines.push(cells.join("\t"));
        }
        lines.join("\n")
    }

    /// Paste a rectangle at the cursor, clipping it to the sheet's dimensions.
    pub fn paste_tsv(&mut self, text: &str, rows: usize, cols: usize) -> Vec<Cell> {
        self.commit_edit();
        let start = self.cursor;
        let text = text
            .strip_suffix("\r\n")
            .or_else(|| text.strip_suffix('\n'))
            .unwrap_or(text);
        let mut changed = Vec::new();
        let mut last = start;
        for (offset_y, line) in text.split('\n').enumerate() {
            let Some(row) = start.row.checked_add(offset_y).filter(|row| *row < rows) else {
                break;
            };
            for (offset_x, value) in line.trim_end_matches('\r').split('\t').enumerate() {
                let Some(col) = start.col.checked_add(offset_x).filter(|col| *col < cols) else {
                    break;
                };
                let cell = Cell::new(col, row);
                if self.raw(cell) != value {
                    self.set(cell, value);
                    changed.push(cell);
                }
                last.col = last.col.max(col);
                last.row = last.row.max(row);
            }
        }
        self.anchor = start;
        self.cursor = last;
        changed
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FormulaError {
    DivisionByZero,
    Cycle,
    Name,
    Reference,
    Syntax,
}

impl fmt::Display for FormulaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::DivisionByZero => "#DIV/0!",
            Self::Cycle => "#CYCLE!",
            Self::Name => "#NAME?",
            Self::Reference => "#REF!",
            Self::Syntax => "#SYNTAX!",
        })
    }
}

type FormulaResult = Result<f64, FormulaError>;

struct Evaluation<'a> {
    sheet: &'a Sheet,
    active: BTreeSet<Cell>,
    cached: BTreeMap<Cell, FormulaResult>,
}

impl Evaluation<'_> {
    fn cell(&mut self, cell: Cell) -> FormulaResult {
        if let Some(value) = self.cached.get(&cell) {
            return *value;
        }
        if self.active.contains(&cell) {
            return Err(FormulaError::Cycle);
        }
        // A malformed document must not exhaust the stack while drawing a cell.
        if self.active.len() >= 128 {
            return Err(FormulaError::Reference);
        }
        let raw = self.sheet.raw(cell);
        let Some(formula) = raw.strip_prefix('=') else {
            return Ok(raw.trim().parse::<f64>().unwrap_or(0.0));
        };
        let source = formula
            .to_ascii_uppercase()
            .replace('−', "-")
            .replace('×', "*")
            .replace('÷', "/");
        self.active.insert(cell);
        let value = Parser {
            source: source.as_bytes(),
            position: 0,
            depth: 0,
            evaluation: self,
        }
        .parse();
        self.active.remove(&cell);
        self.cached.insert(cell, value);
        value
    }
}

struct Parser<'a, 'b, 'c> {
    source: &'a [u8],
    position: usize,
    depth: usize,
    evaluation: &'b mut Evaluation<'c>,
}

impl Parser<'_, '_, '_> {
    fn parse(mut self) -> FormulaResult {
        let value = self.expression()?;
        if self.peek().is_some() {
            return Err(FormulaError::Syntax);
        }
        if value.is_finite() {
            Ok(value)
        } else {
            Err(FormulaError::DivisionByZero)
        }
    }

    fn peek(&mut self) -> Option<u8> {
        while self
            .source
            .get(self.position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
        self.source.get(self.position).copied()
    }

    fn consume(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn expression(&mut self) -> FormulaResult {
        let mut value = self.term()?;
        loop {
            if self.consume(b'+') {
                value += self.term()?;
            } else if self.consume(b'-') {
                value -= self.term()?;
            } else {
                return Ok(value);
            }
        }
    }

    fn term(&mut self) -> FormulaResult {
        let mut value = self.factor()?;
        loop {
            if self.consume(b'*') {
                value *= self.factor()?;
            } else if self.consume(b'/') {
                let divisor = self.factor()?;
                if divisor == 0.0 {
                    return Err(FormulaError::DivisionByZero);
                }
                value /= divisor;
            } else {
                return Ok(value);
            }
        }
    }

    fn factor(&mut self) -> FormulaResult {
        if self.depth >= 128 {
            return Err(FormulaError::Syntax);
        }
        self.depth += 1;
        let value = self.factor_inner();
        self.depth -= 1;
        value
    }

    fn factor_inner(&mut self) -> FormulaResult {
        if self.consume(b'-') {
            return Ok(-self.factor()?);
        }
        if self.consume(b'+') {
            return self.factor();
        }
        if self.consume(b'(') {
            let value = self.expression()?;
            return if self.consume(b')') {
                Ok(value)
            } else {
                Err(FormulaError::Syntax)
            };
        }
        match self.peek() {
            Some(b'0'..=b'9' | b'.') => self.number(),
            Some(b'A'..=b'Z') => {
                let name = self.identifier();
                if self.consume(b'(') {
                    self.function(&name)
                } else {
                    let cell = Cell::parse(&name).ok_or(FormulaError::Name)?;
                    self.evaluation.cell(cell)
                }
            }
            _ => Err(FormulaError::Syntax),
        }
    }

    fn number(&mut self) -> FormulaResult {
        let start = self.position;
        while self
            .source
            .get(self.position)
            .is_some_and(|byte| byte.is_ascii_digit() || *byte == b'.')
        {
            self.position += 1;
        }
        if self.source.get(self.position) == Some(&b'E') {
            self.position += 1;
            if matches!(self.source.get(self.position), Some(b'+' | b'-')) {
                self.position += 1;
            }
            while self
                .source
                .get(self.position)
                .is_some_and(u8::is_ascii_digit)
            {
                self.position += 1;
            }
        }
        std::str::from_utf8(&self.source[start..self.position])
            .map_err(|_| FormulaError::Syntax)?
            .parse()
            .map_err(|_| FormulaError::Syntax)
    }

    fn identifier(&mut self) -> String {
        self.peek();
        let start = self.position;
        while self
            .source
            .get(self.position)
            .is_some_and(u8::is_ascii_alphanumeric)
        {
            self.position += 1;
        }
        String::from_utf8_lossy(&self.source[start..self.position]).into_owned()
    }

    fn function(&mut self, name: &str) -> FormulaResult {
        if !matches!(name, "SUM" | "AVERAGE" | "MIN" | "MAX") {
            return Err(FormulaError::Name);
        }
        let mut values = Vec::new();
        if !self.consume(b')') {
            loop {
                let start = self.position;
                let first = self.identifier();
                if self.consume(b':') {
                    let first = Cell::parse(&first).ok_or(FormulaError::Reference)?;
                    let last = Cell::parse(&self.identifier()).ok_or(FormulaError::Reference)?;
                    let width = first.col.abs_diff(last.col).saturating_add(1);
                    let height = first.row.abs_diff(last.row).saturating_add(1);
                    if width.saturating_mul(height) > 1_000_000 {
                        return Err(FormulaError::Reference);
                    }
                    for row in first.row.min(last.row)..=first.row.max(last.row) {
                        for col in first.col.min(last.col)..=first.col.max(last.col) {
                            values.push(self.evaluation.cell(Cell::new(col, row))?);
                        }
                    }
                } else {
                    self.position = start;
                    values.push(self.expression()?);
                }
                if self.consume(b')') {
                    break;
                }
                if !self.consume(b',') {
                    return Err(FormulaError::Syntax);
                }
            }
        }
        if values.is_empty() {
            return Ok(0.0);
        }
        Ok(match name {
            "SUM" => values.iter().sum(),
            "AVERAGE" => values.iter().sum::<f64>() / values.len() as f64,
            "MIN" => values.into_iter().fold(f64::INFINITY, f64::min),
            "MAX" => values.into_iter().fold(f64::NEG_INFINITY, f64::max),
            _ => unreachable!(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(name: &str) -> Cell {
        Cell::parse(name).unwrap()
    }

    #[test]
    fn addresses_round_trip_and_reject_invalid_rows() {
        assert_eq!(Cell::new(27, 9).to_string(), "AB10");
        assert_eq!(Cell::parse(" ab10 "), Some(Cell::new(27, 9)));
        for invalid in ["", "A", "0", "A0", "A-1", "A1B", "A+1", "É1"] {
            assert_eq!(Cell::parse(invalid), None, "{invalid}");
        }
    }

    #[test]
    fn formulas_evaluate_references_ranges_functions_and_precedence() {
        let mut sheet = Sheet::new();
        for (address, text) in [
            ("A1", "2"),
            ("A2", "3.5"),
            ("A3", "=A1*A2+1"),
            ("B1", "=SUM(A1:A3)"),
            ("B2", "=AVERAGE(A1:A2)"),
            ("C1", "=MAX(A1, 10, A2)"),
            ("C2", "=MIN(A2:A1, -1)"),
            ("C3", "=SUM((2+3)*4, MAX(1,2))"),
            ("D1", "text"),
            ("D2", "=D1+1"),
            ("D3", "=2×3−1"),
        ] {
            sheet.set(cell(address), text);
        }
        for (address, expected) in [
            ("A3", "8"),
            ("B1", "13.5"),
            ("B2", "2.75"),
            ("C1", "10"),
            ("C2", "-1"),
            ("C3", "22"),
            ("D1", "text"),
            ("D2", "1"),
            ("D3", "5"),
        ] {
            assert_eq!(sheet.value(cell(address)), expected, "{address}");
        }
    }

    #[test]
    fn errors_propagate_without_hanging_on_invalid_formulas() {
        let mut sheet = Sheet::new();
        for (formula, expected) in [
            ("=1/0", "#DIV/0!"),
            ("=A1", "#CYCLE!"),
            ("=A1+", "#CYCLE!"),
            ("=2+", "#SYNTAX!"),
            ("=SUM(1 2)", "#SYNTAX!"),
            ("=SUM(1,)", "#SYNTAX!"),
            ("=SUM(A2:?)", "#REF!"),
            ("=UNKNOWN()", "#NAME?"),
            ("=NOTACELL", "#NAME?"),
            ("=SUM(A2:A999999999)", "#REF!"),
        ] {
            sheet.set(cell("A1"), formula);
            assert_eq!(sheet.value(cell("A1")), expected, "{formula}");
        }
        sheet.set(cell("A1"), "=B1");
        sheet.set(cell("B1"), "=C1");
        sheet.set(cell("C1"), "=A1");
        assert_eq!(sheet.value(cell("A1")), "#CYCLE!");
        sheet.set(cell("C1"), "42");
        assert_eq!(sheet.value(cell("A1")), "42");
    }

    #[test]
    fn ledger_recalculates_after_an_edit_and_cancel_preserves_the_cell() {
        let mut sheet = Sheet::ledger();
        assert_eq!(sheet.value(cell("B7")), "1095.5");
        sheet.select(cell("B2"), false);
        sheet.begin_edit("900");
        sheet.commit_edit();
        assert_eq!(sheet.value(cell("B7")), "1195.5");
        sheet.begin_edit("1000");
        sheet.cancel_edit();
        assert_eq!(sheet.raw(cell("B2")), "900");
        sheet.select(cell("B5"), true);
        sheet.clear_selection();
        assert_eq!(sheet.value(cell("B7")), "0");
        assert_eq!(sheet.raw(cell("A2")), "Rent");
    }

    #[test]
    fn formulas_keep_ten_significant_digits_at_small_and_large_scales() {
        let mut sheet = Sheet::new();
        sheet.set(cell("A1"), "=1.234567891e-10");
        sheet.set(cell("A2"), "=1.234567891e12");
        assert_eq!(sheet.value(cell("A1")), "1.234567891e-10");
        assert_eq!(sheet.value(cell("A2")), "1.234567891e+12");
    }

    #[test]
    fn clipboard_rectangles_keep_formulas_and_clip_at_the_sheet_edge() {
        let mut sheet = Sheet::new();
        sheet.set(cell("A1"), "3");
        sheet.set(cell("B1"), "=A1*2");
        sheet.set(cell("A2"), "Notes");
        sheet.select(cell("A1"), false);
        sheet.select(cell("B2"), true);
        assert_eq!(sheet.copy_tsv(), "3\t=A1*2\nNotes\t");
        sheet.select(cell("C3"), false);
        let changed = sheet.paste_tsv("1\t2\t3\r\n4\t5\t6\r\n", 4, 4);
        assert_eq!(changed.len(), 4);
        assert_eq!(sheet.raw(cell("D4")), "5");
        assert_eq!(sheet.raw(cell("E3")), "");
        assert_eq!(sheet.selection_bounds(), (cell("C3"), cell("D4")));
    }
}
