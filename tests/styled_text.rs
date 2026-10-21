use gpui::TestAppContext;
use gpui_platinum::{Font, Language, TextStyle, paint::Pen, theme::BLACK};

/// `measure_styled` measures what `text_styled` draws, in every style, in
/// every face, East Asian text and text no face draws among it; nothing for
/// an empty run.
#[gpui::test]
fn styled_measure_matches_the_drawn_width(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_platinum::font::init(cx));
    let texts = ["", " ", "Ani", "Wide WWW", "粗体 mixed", "→ ✓ ∑", "ไทย"];
    let bold = TextStyle {
        bold: true,
        ..TextStyle::default()
    };
    let italic = TextStyle {
        italic: true,
        ..TextStyle::default()
    };
    let underline = TextStyle {
        underline: true,
        ..TextStyle::default()
    };
    let bold_italic = TextStyle {
        bold: true,
        ..italic
    };
    let styles = [
        TextStyle::default(),
        bold,
        italic,
        underline,
        bold_italic,
        bold.scaled(2),
        bold_italic.scaled(3),
    ];
    for font in [Font::Sans, Font::Bold, Font::Small, Font::SmallBold] {
        for language in [Language::English, Language::Japanese] {
            for text in texts {
                assert_eq!(
                    font.measure_styled_in(text, TextStyle::default(), language),
                    font.measure_in(text, language),
                    "{font:?} {language:?}: unstyled {text:?}"
                );
                for style in styles {
                    let mut pen = Pen::new(400, 40);
                    pen.language = language;
                    let after = pen.text_styled(0, 20, text, font, BLACK, style);
                    // Drawing leaves italic's room after an empty run too.
                    let drawn = if text.is_empty() {
                        0
                    } else {
                        after - i32::from(style.scale)
                    };
                    assert_eq!(
                        font.measure_styled_in(text, style, language),
                        drawn,
                        "{font:?} {language:?}: {text:?} in {style:?}"
                    );
                }
            }
        }
    }
    assert!(Font::Small.measure_styled("ไทย", bold.scaled(2)) > Font::Small.measure("ไทย"));
}
