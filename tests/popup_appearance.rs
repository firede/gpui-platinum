use gpui_platinum::{menu::draw_popup_button, paint::Pen, theme::GRAY_2};

#[test]
fn popup_buttons_match_the_original_pixels() {
    let reference = image::load_from_memory(include_bytes!("reference/popup-states.png"))
        .unwrap()
        .to_rgba8();
    let mut pen = Pen::new(420, 34);
    pen.rect(0, 0, 420, 34, GRAY_2);
    for (index, (text, disabled, pressed)) in [
        ("Name", false, false),
        ("Pressed", false, true),
        ("Disabled", true, false),
    ]
    .into_iter()
    .enumerate()
    {
        let mut button = Pen::new(120, 20);
        draw_popup_button(&mut button, text, disabled, pressed);
        pen.append(button, 6 + index as i32 * 126, 6);
    }
    let mut actual = image::RgbaImage::new(420, 34);
    for fill in pen.fills {
        let color = image::Rgba([
            (fill.color >> 16) as u8,
            (fill.color >> 8) as u8,
            fill.color as u8,
            255,
        ]);
        for y in fill.rect.y..fill.rect.y + fill.rect.height {
            for x in fill.rect.x..fill.rect.x + fill.rect.width {
                actual.put_pixel(x as u32, y as u32, color);
            }
        }
    }
    assert_eq!(actual, reference);
}
