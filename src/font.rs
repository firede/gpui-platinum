//! The Platinum bitmap faces, with regional East Asian fallback.
//!
//! Glyph coordinates are pixels from the baseline, with y growing downward.
//! The original font data and licenses are in `assets/fonts`.

use std::{
    cell::RefCell,
    sync::{Arc, OnceLock},
};
use unicode_segmentation::UnicodeSegmentation;

thread_local! {
    static SYSTEM_TEXT: RefCell<Option<Arc<gpui::WindowTextSystem>>> = const { RefCell::new(None) };
}

/// Enables the platform's font fallback for characters outside the bitmap faces.
pub fn init(cx: &gpui::App) {
    SYSTEM_TEXT.with(|system| {
        *system.borrow_mut() = Some(Arc::new(gpui::WindowTextSystem::new(
            cx.text_system().clone(),
        )));
    });
}

/// The language whose East Asian letterforms the text uses.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Language {
    #[default]
    English,
    SimplifiedChinese,
    TraditionalChinese,
    Japanese,
    Korean,
}

/// A face of Platinum Sans or Platinum Sans Small.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Font {
    Sans,
    #[default]
    Bold,
    Small,
    SmallBold,
}

impl Font {
    /// The glyph of `character`, trying the Latin face before East Asian fallback.
    /// English uses Simplified Chinese forms for mixed-language text.
    pub fn glyph(self, character: char, language: Language) -> Option<Glyph<'static>> {
        if let Some(glyph) = self.face().glyph(character) {
            return Some(glyph);
        }
        let mut glyph = east_asian_face(language).glyph(character)?;
        if matches!(self, Self::Bold | Self::SmallBold) {
            glyph.embolden_open();
        }
        Some(glyph)
    }

    /// The glyph of `character`, or the face's replacement character.
    pub fn glyph_or_replacement(self, character: char, language: Language) -> Glyph<'static> {
        self.glyph(character, language)
            .unwrap_or_else(|| self.face().glyph('\u{fffd}').expect("replacement glyph"))
    }

    /// The width of one line in pixels, without its final tracking column.
    pub fn measure(self, text: &str) -> i32 {
        self.measure_in(text, Language::English)
    }

    /// The width of one line using the language's East Asian letterforms.
    pub fn measure_in(self, text: &str, language: Language) -> i32 {
        let advance: i32 = text
            .graphemes(true)
            .map(|grapheme| self.advance(grapheme, language))
            .sum();
        (advance - 1).max(0)
    }

    /// Cuts a line at a grapheme boundary and appends an ellipsis when needed.
    pub fn truncate(self, text: &str, width: i32) -> String {
        self.truncate_in(text, width, Language::English)
    }

    /// Truncates using the language's East Asian letterforms and native fallback.
    pub fn truncate_in(self, text: &str, width: i32, language: Language) -> String {
        if self.measure_in(text, language) <= width {
            return text.to_owned();
        }
        let ellipsis = self.advance("…", language);
        if ellipsis - 1 > width {
            return String::new();
        }
        let mut advance = 0;
        let mut end = 0;
        for (index, grapheme) in text.grapheme_indices(true) {
            let next = self.advance(grapheme, language);
            if advance + next + ellipsis - 1 > width {
                break;
            }
            advance += next;
            end = index + grapheme.len();
        }
        format!("{}…", &text[..end])
    }

    /// Advances one grapheme, retaining the bitmap face's one-pixel tracking.
    /// Unsupported graphemes are shaped together so combining marks and emoji
    /// sequences use the platform's normal font fallback.
    pub fn advance(self, grapheme: &str, language: Language) -> i32 {
        if self.has_bitmap_grapheme(grapheme, language) {
            return grapheme
                .chars()
                .map(|ch| self.glyph_or_replacement(ch, language).advance)
                .sum();
        }
        self.system_line(grapheme, 1, false)
            .map(|line| f32::from(line.width).ceil() as i32 + 1)
            .unwrap_or_else(|| self.glyph_or_replacement('\u{fffd}', language).advance)
    }

    pub(crate) fn has_bitmap_grapheme(self, text: &str, language: Language) -> bool {
        text.chars().all(|ch| self.glyph(ch, language).is_some())
    }

    pub(crate) fn system_line(
        self,
        text: &str,
        scale: u8,
        italic: bool,
    ) -> Option<Arc<gpui::LineLayout>> {
        SYSTEM_TEXT.with(|system| {
            let system = system.borrow();
            let system = system.as_ref()?;
            let mut font = gpui::Font::default();
            if matches!(self, Self::Bold | Self::SmallBold) {
                font = font.bold();
            }
            if italic {
                font = font.italic();
            }
            let size = if matches!(self, Self::Small | Self::SmallBold) {
                11.
            } else {
                12.
            };
            Some(system.layout_line(
                text,
                gpui::px(size * scale.clamp(1, 4) as f32),
                &[gpui::TextRun {
                    len: text.len(),
                    font,
                    ..Default::default()
                }],
                None,
            ))
        })
    }

    /// The rows of the Latin face above its baseline.
    pub fn ascent(self) -> i32 {
        self.face().ascent
    }

    /// The height of the Latin face's lines, without leading.
    pub fn line_height(self) -> i32 {
        self.face().ascent + self.face().descent
    }

    /// The height of the face's capitals.
    pub fn cap_height(self) -> i32 {
        self.ascent()
    }

    /// The rows above and below the baseline, including East Asian glyphs.
    pub fn line_ink(self, language: Language) -> (i32, i32) {
        let mut above = self.face().ascent;
        let mut below = self.face().descent;
        for character in "永中あ한".chars() {
            if let Some(glyph) = self.glyph(character, language) {
                above = above.max(-glyph.bounds.y);
                below = below.max(glyph.bounds.y + glyph.bounds.height);
            }
        }
        (above, below)
    }

    fn face(self) -> &'static Face<'static> {
        static FACES: [OnceLock<Face<'static>>; 4] = [const { OnceLock::new() }; 4];
        let (index, data): (_, &[u8]) = match self {
            Self::Sans => (0, include_bytes!("../assets/fonts/sans.bin")),
            Self::Bold => (1, include_bytes!("../assets/fonts/sans-bold.bin")),
            Self::Small => (2, include_bytes!("../assets/fonts/small.bin")),
            Self::SmallBold => (3, include_bytes!("../assets/fonts/small-bold.bin")),
        };
        FACES[index].get_or_init(|| Face::decode(data).expect("valid bundled Platinum face"))
    }
}

fn east_asian_face(language: Language) -> &'static Face<'static> {
    static FACES: [OnceLock<Face<'static>>; 4] = [const { OnceLock::new() }; 4];
    let (index, data): (_, &[u8]) = match language {
        Language::English | Language::SimplifiedChinese => {
            (0, include_bytes!("../assets/fonts/cjk12/sc.bin"))
        }
        Language::TraditionalChinese => (1, include_bytes!("../assets/fonts/cjk12/tc.bin")),
        Language::Japanese => (2, include_bytes!("../assets/fonts/cjk12/jp.bin")),
        Language::Korean => (3, include_bytes!("../assets/fonts/cjk12/kr.bin")),
    };
    FACES[index].get_or_init(|| Face::decode(data).expect("valid bundled East Asian face"))
}

/// The rectangle of a glyph's pixels, from its baseline origin.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlyphBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// A bitmap glyph. Its bits are borrowed from the bundled face.
#[derive(Clone, Copy, Debug)]
pub struct Glyph<'a> {
    pub advance: i32,
    pub bounds: GlyphBounds,
    source_bounds: GlyphBounds,
    bits: &'a [u8],
    bold: bool,
}

impl Glyph<'_> {
    /// Whether the pixel at `x, y`, relative to the baseline, is ink.
    pub fn at(self, x: i32, y: i32) -> bool {
        self.source_at(x, y) || (self.bold && self.source_at(x - 1, y) && !self.source_at(x + 1, y))
    }

    /// Calls `draw(x, y, width)` for each horizontal run of ink.
    /// Coordinates are relative to the baseline; every run is one pixel high.
    pub fn for_each_run(self, mut draw: impl FnMut(i32, i32, i32)) {
        let right = self.bounds.x + self.bounds.width;
        for y in self.bounds.y..self.bounds.y + self.bounds.height {
            let mut x = self.bounds.x;
            while x < right {
                if !self.at(x, y) {
                    x += 1;
                    continue;
                }
                let start = x;
                while x < right && self.at(x, y) {
                    x += 1;
                }
                draw(start, y, x - start);
            }
        }
    }

    fn source_at(self, x: i32, y: i32) -> bool {
        let bounds = self.source_bounds;
        if x < bounds.x
            || y < bounds.y
            || x >= bounds.x + bounds.width
            || y >= bounds.y + bounds.height
        {
            return false;
        }
        let index = ((y - bounds.y) * bounds.width + x - bounds.x) as usize;
        self.bits[index / 8] & (0x80 >> (index % 8)) != 0
    }

    // The upstream EmboldenOpen rule preserves one-pixel gaps between strokes.
    fn embolden_open(&mut self) {
        self.advance += 1;
        if self.bounds.width > 0 && self.bounds.height > 0 {
            self.bounds.width += 1;
            self.bold = true;
        }
    }
}

#[derive(Debug)]
struct Metrics {
    advance: i32,
    bounds: GlyphBounds,
}

#[derive(Debug)]
struct Span {
    start: u32,
    count: u32,
    first: usize,
}

#[derive(Debug)]
struct Face<'a> {
    ascent: i32,
    descent: i32,
    metrics: Vec<Metrics>,
    spans: Vec<Span>,
    glyph_metrics: Vec<usize>,
    offsets: Vec<usize>,
    bits: &'a [u8],
}

impl<'a> Face<'a> {
    // PXF1 stores little-endian metrics, sorted Unicode spans, per-glyph metric
    // indices and packed pixels. Each glyph starts at a byte boundary.
    fn decode(data: &'a [u8]) -> Result<Self, &'static str> {
        let mut reader = Reader { data, position: 0 };
        if reader.take(4)? != b"PXF1" {
            return Err("not a bitmap face");
        }
        let ascent = reader.u16()? as i16 as i32;
        let descent = reader.u16()? as i16 as i32;
        let metric_count = reader.u16()? as usize;
        let metric_data = reader.take(metric_count * 5)?;
        let mut metrics = Vec::with_capacity(metric_count);
        for bytes in metric_data.as_chunks::<5>().0 {
            let width = bytes[3] as i8 as i32;
            let height = bytes[4] as i8 as i32;
            if width < 0 || height < 0 {
                return Err("negative glyph dimensions");
            }
            metrics.push(Metrics {
                advance: bytes[0] as i8 as i32,
                bounds: GlyphBounds {
                    x: bytes[1] as i8 as i32,
                    y: bytes[2] as i8 as i32,
                    width,
                    height,
                },
            });
        }
        let span_count = reader.u32()? as usize;
        if span_count > reader.remaining() / 8 {
            return Err("truncated character spans");
        }
        let mut spans = Vec::with_capacity(span_count);
        let mut glyph_count = 0usize;
        let mut previous_end = 0;
        for _ in 0..span_count {
            let start = reader.u32()?;
            let count = reader.u32()?;
            let end = start.checked_add(count).ok_or("character span overflow")?;
            if count == 0 || start < previous_end || end > 0x110000 {
                return Err("invalid character span");
            }
            spans.push(Span {
                start,
                count,
                first: glyph_count,
            });
            glyph_count = glyph_count
                .checked_add(count as usize)
                .ok_or("glyph count overflow")?;
            previous_end = end;
        }
        if glyph_count > reader.remaining() / 2 {
            return Err("truncated glyph metrics");
        }
        let mut glyph_metrics = Vec::with_capacity(glyph_count);
        let mut offsets = Vec::with_capacity(glyph_count + 1);
        let mut offset = 0usize;
        for _ in 0..glyph_count {
            let index = reader.u16()? as usize;
            let metric = metrics.get(index).ok_or("invalid metric index")?;
            glyph_metrics.push(index);
            offsets.push(offset);
            let pixels = (metric.bounds.width * metric.bounds.height) as usize;
            offset = offset
                .checked_add(pixels.div_ceil(8))
                .ok_or("bitmap size overflow")?;
        }
        offsets.push(offset);
        if reader.remaining() != offset {
            return Err("incorrect bitmap length");
        }
        Ok(Self {
            ascent,
            descent,
            metrics,
            spans,
            glyph_metrics,
            offsets,
            bits: reader.take(offset)?,
        })
    }

    fn glyph(&self, character: char) -> Option<Glyph<'a>> {
        let code = character as u32;
        let span_index = self.spans.partition_point(|span| span.start <= code);
        let span = self.spans.get(span_index.checked_sub(1)?)?;
        if code - span.start >= span.count {
            return None;
        }
        let index = span.first + (code - span.start) as usize;
        let metric = &self.metrics[self.glyph_metrics[index]];
        Some(Glyph {
            advance: metric.advance,
            bounds: metric.bounds,
            source_bounds: metric.bounds,
            bits: &self.bits[self.offsets[index]..self.offsets[index + 1]],
            bold: false,
        })
    }
}

struct Reader<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, length: usize) -> Result<&'a [u8], &'static str> {
        let end = self
            .position
            .checked_add(length)
            .ok_or("bitmap size overflow")?;
        let bytes = self.data.get(self.position..end).ok_or("truncated face")?;
        self.position = end;
        Ok(bytes)
    }

    fn remaining(&self) -> usize {
        self.data.len() - self.position
    }

    fn u16(&mut self) -> Result<u16, &'static str> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Result<u32, &'static str> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A 3-by-3 x glyph, packed continuously across row boundaries:
    // #.# / .#. / #.#, at (0, -2), advancing four pixels.
    const FIXTURE: &[u8] = &[
        b'P', b'X', b'F', b'1', 2, 0, 1, 0, 1, 0, 4, 0, 254, 3, 3, 1, 0, 0, 0, b'x', 0, 0, 0, 1, 0,
        0, 0, 0, 0, 0b10101010, 0b10000000,
    ];

    #[test]
    fn packed_pixels_cross_byte_and_row_boundaries() {
        let face = Face::decode(FIXTURE).unwrap();
        assert_eq!((face.ascent, face.descent), (2, 1));
        let glyph = face.glyph('x').unwrap();
        assert_eq!(glyph.advance, 4);
        let mut runs = Vec::new();
        glyph.for_each_run(|x, y, width| runs.push((x, y, width)));
        assert_eq!(
            runs,
            [(0, -2, 1), (2, -2, 1), (1, -1, 1), (0, 0, 1), (2, 0, 1)]
        );
        assert!(!glyph.at(-1, -2));
        assert!(!glyph.at(3, 0));
        assert!(!glyph.at(0, 1));
        assert!(face.glyph('w').is_none());
        assert!(face.glyph('y').is_none());
    }

    #[test]
    fn decoder_rejects_truncated_and_corrupt_faces() {
        for end in 0..FIXTURE.len() {
            assert!(Face::decode(&FIXTURE[..end]).is_err(), "cut at {end}");
        }
        let mut extra = FIXTURE.to_vec();
        extra.push(0);
        assert!(Face::decode(&extra).is_err());
        for (index, value) in [(0, b'?'), (13, 255), (23, 0), (27, 1)] {
            let mut corrupt = FIXTURE.to_vec();
            corrupt[index] = value;
            assert!(Face::decode(&corrupt).is_err(), "corrupt byte {index}");
        }
    }

    #[test]
    fn bold_keeps_single_pixel_gaps_open() {
        let face = Face::decode(FIXTURE).unwrap();
        let mut glyph = face.glyph('x').unwrap();
        glyph.embolden_open();
        assert_eq!(glyph.advance, 5);
        assert_eq!(glyph.bounds.width, 4);
        let mut runs = Vec::new();
        glyph.for_each_run(|x, y, width| runs.push((x, y, width)));
        assert_eq!(
            runs,
            [(0, -2, 1), (2, -2, 2), (1, -1, 2), (0, 0, 1), (2, 0, 2)]
        );
    }

    #[test]
    fn latin_faces_keep_the_original_metrics_and_coverage() {
        for font in [Font::Sans, Font::Bold, Font::Small, Font::SmallBold] {
            let small = matches!(font, Font::Small | Font::SmallBold);
            assert_eq!(font.ascent(), if small { 8 } else { 9 });
            assert_eq!(font.line_height(), if small { 11 } else { 12 });
            for character in "Aaé€�".chars() {
                assert!(
                    font.face().glyph(character).is_some(),
                    "{font:?}: {character:?}"
                );
            }
            assert!(font.face().glyph('中').is_none());
            assert_eq!(font.measure(""), 0);
            let a = font.glyph('A', Language::English).unwrap();
            assert_eq!(font.measure("AA"), 2 * a.advance - 1);
            let replacement = font.glyph('\u{fffd}', Language::English).unwrap();
            assert_eq!(font.measure("\u{10ffff}"), replacement.advance - 1);
        }
    }

    #[test]
    fn truncation_fits_the_available_width_and_preserves_graphemes() {
        let font = Font::Small;
        let text = "Platinum Controls";
        assert_eq!(font.truncate(text, font.measure(text)), text);
        assert_eq!(font.truncate(text, 0), "");
        assert_eq!(font.truncate(text, font.measure("Pla…")), "Pla…");
        let chinese = font.truncate_in("控制面板设置", 30, Language::SimplifiedChinese);
        assert!(chinese.ends_with('…'));
        assert!(font.measure_in(&chinese, Language::SimplifiedChinese) <= 30);
        let combined = "e\u{301}".repeat(8);
        let width = font.measure("e\u{301}…");
        assert_eq!(font.truncate(&combined, width), "e\u{301}…");
    }

    #[test]
    fn regional_faces_cover_each_script_and_preserve_ideograph_metrics() {
        for (language, sample) in [
            (Language::SimplifiedChinese, "永和九年，岁在癸丑。藏鑫覆盖"),
            (Language::TraditionalChinese, "永和九年，歲在癸丑。藏鑫覆蓋"),
            (Language::Japanese, "いろはにほへと　カタカナ　鬱"),
            (Language::Korean, "다람쥐　헌　쳇바퀴에　타고파"),
        ] {
            let face = east_asian_face(language);
            assert_eq!(face.glyph_metrics.len(), 34651);
            for character in sample.chars() {
                assert!(
                    Font::Small.glyph(character, language).is_some(),
                    "{character}"
                );
            }
            let regular = Font::Small.glyph('中', language).unwrap();
            assert_eq!(regular.advance, 12);
            assert_eq!(
                regular.bounds,
                GlyphBounds {
                    x: 0,
                    y: -10,
                    width: 11,
                    height: 11
                }
            );
            let bold = Font::Bold.glyph('中', language).unwrap();
            assert_eq!(bold.advance, 13);
            assert_eq!(
                bold.bounds,
                GlyphBounds {
                    x: 0,
                    y: -10,
                    width: 12,
                    height: 11
                }
            );
            assert_eq!(Font::Small.line_ink(language), (10, 3));
            assert_eq!(Font::Small.measure_in("中中", language), 23);
        }
    }
}
