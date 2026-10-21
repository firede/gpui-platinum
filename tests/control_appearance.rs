use gpui_platinum::{
    Font, Theme, draw_bevel_content, draw_disclosure, draw_split_divider, paint::Pen, theme::*,
};

fn pixels(pen: &Pen) -> image::RgbaImage {
    let mut pixels = image::RgbaImage::from_pixel(
        pen.width as u32,
        pen.height as u32,
        image::Rgba([0xdd, 0xdd, 0xdd, 255]),
    );
    for fill in &pen.fills {
        let color = image::Rgba([
            (fill.color >> 16) as u8,
            (fill.color >> 8) as u8,
            fill.color as u8,
            255,
        ]);
        for y in fill.rect.y..fill.rect.y + fill.rect.height {
            for x in fill.rect.x..fill.rect.x + fill.rect.width {
                if (0..pen.width).contains(&x) && (0..pen.height).contains(&y) {
                    pixels.put_pixel(x as u32, y as u32, color);
                }
            }
        }
    }
    pixels
}

fn assert_crop(pen: &Pen, reference: &image::RgbaImage, x: u32, y: u32) {
    let expected =
        image::imageops::crop_imm(reference, x, y, pen.width as u32, pen.height as u32).to_image();
    let actual = pixels(pen);
    let differences: Vec<_> = actual
        .enumerate_pixels()
        .filter(|(x, y, pixel)| expected.get_pixel(*x, *y) != *pixel)
        .map(|(x, y, _)| (x, y))
        .collect();
    assert!(
        differences.is_empty(),
        "{} pixels differ in crop ({x}, {y}); first: {:?}",
        differences.len(),
        differences.first()
    );
}

#[test]
fn bevel_labels_toggles_and_icons_match_the_original_pixels() {
    const PAGE: &[&str] = &[
        "#####...", "#...##..", "#...###.", "#.....#.", "#.###.#.", "#.....#.", "#.###.#.",
        "#.....#.", "#######.",
    ];
    const BOLD: &[&str] = &[
        "######..", ".##..##.", ".##..##.", ".#####..", ".##..##.", ".##..##.", "######..",
    ];
    let reference = image::load_from_memory(include_bytes!("reference/bevel-states.png"))
        .unwrap()
        .to_rgba8();
    let mut x = 8;
    for (label, disabled, on, icon) in [
        ("Normal", false, false, None),
        ("On", false, true, None),
        ("Off", false, false, None),
        ("Disabled", true, false, None),
        ("Page", false, false, Some(PAGE)),
        ("", false, true, Some(BOLD)),
    ] {
        let width = if label.is_empty() {
            22
        } else {
            Font::Bold.measure(label) + 16 + if icon.is_some() { 12 } else { 0 }
        };
        let mut pen = Pen::new(width, 22);
        draw_bevel_content(&mut pen, label, icon, disabled, false, on, Theme::default());
        assert_crop(&pen, &reference, x, 8);
        x += width as u32 + 6;
    }
    let mut pressed = Pen::new(60, 22);
    draw_bevel_content(
        &mut pressed,
        "Pressed",
        None,
        false,
        true,
        false,
        Theme::default(),
    );
    assert_crop(&pressed, &reference, 8, 38);
}

#[test]
fn disclosure_states_match_the_original_pixels() {
    let reference = image::load_from_memory(include_bytes!("reference/disclosure-states.png"))
        .unwrap()
        .to_rgba8();
    for index in 0..6 {
        let mut pen = Pen::new(12, 12);
        draw_disclosure(
            &mut pen,
            index % 2 == 1,
            index >= 4,
            (2..4).contains(&index),
        );
        assert_crop(&pen, &reference, 6 + 18 * index, 6);
    }
}

// A split 160×127 whose panes are stacked, shared at 30%: 30% of the 120
// beside the divider is 36, under the pane's least, so the first is 40.
#[test]
fn a_percent_split_matches_the_original_pixels() {
    let reference = image::load_from_memory(include_bytes!("reference/split-percent.png"))
        .unwrap()
        .to_rgba8();
    let mut pen = Pen::new(180, 147);
    pen.text(10, 10 + Font::Small.ascent(), "30%", Font::Small, BLACK);
    let mut divider = Pen::new(160, 7);
    draw_split_divider(&mut divider, true);
    pen.append(divider, 10, 50);
    pen.text(10, 57 + Font::Small.ascent(), "70%", Font::Small, BLACK);
    assert_eq!(pixels(&pen), reference);
}
