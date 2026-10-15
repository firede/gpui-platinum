use gpui_platinum::{AlertKind, draw_alert_icon, draw_small_alert_icon, paint::Pen};

#[test]
fn alert_icons_match_the_original_pixels() {
    let reference = image::load_from_memory(include_bytes!("reference/alert-icons.png"))
        .unwrap()
        .to_rgba8();
    let mut actual = image::RgbaImage::from_pixel(140, 44, image::Rgba([0xdd, 0xdd, 0xdd, 255]));
    for (index, kind) in [AlertKind::Note, AlertKind::Caution, AlertKind::Stop]
        .into_iter()
        .enumerate()
    {
        let mut pen = Pen::new(32, 32);
        draw_alert_icon(&mut pen, kind);
        for fill in pen.fills {
            let color = image::Rgba([
                (fill.color >> 16) as u8,
                (fill.color >> 8) as u8,
                fill.color as u8,
                255,
            ]);
            for row in 0..fill.rect.height {
                for column in 0..fill.rect.width {
                    actual.put_pixel(
                        (6 + index as i32 * 38 + fill.rect.x + column) as u32,
                        (6 + fill.rect.y + row) as u32,
                        color,
                    );
                }
            }
        }
    }
    assert_eq!(actual, reference);
}

#[test]
fn notification_icons_match_the_original_pixels() {
    let reference = image::load_from_memory(include_bytes!("reference/toasts.png"))
        .unwrap()
        .to_rgba8();
    for (kind, x, y) in [
        (AlertKind::Note, 174, 53),
        (AlertKind::Caution, 124, 89),
        (AlertKind::Stop, 87, 125),
    ] {
        let mut actual = image::RgbaImage::from_pixel(16, 16, image::Rgba([0xdd, 0xdd, 0xdd, 255]));
        let mut pen = Pen::new(16, 16);
        draw_small_alert_icon(&mut pen, kind);
        for fill in pen.fills {
            let color = image::Rgba([
                (fill.color >> 16) as u8,
                (fill.color >> 8) as u8,
                fill.color as u8,
                255,
            ]);
            for row in 0..fill.rect.height {
                for column in 0..fill.rect.width {
                    actual.put_pixel(
                        (fill.rect.x + column) as u32,
                        (fill.rect.y + row) as u32,
                        color,
                    );
                }
            }
        }
        assert_eq!(
            actual,
            image::imageops::crop_imm(&reference, x, y, 16, 16).to_image(),
            "{kind:?}"
        );
    }
}
