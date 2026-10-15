use gpui_platinum::{
    paint::Pen,
    theme::*,
    window::{PlatinumWindowOptions, WindowPart, draw_grow_box, draw_title_box, draw_window_frame},
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

#[test]
fn title_boxes_and_resize_corner_match_the_original_pixels() {
    let reference = image::load_from_memory(include_bytes!("reference/titlebox-states.png"))
        .unwrap()
        .to_rgba8();
    let mut pen = Pen::new(220, 30);
    pen.rect(0, 0, pen.width, pen.height, GRAY_2);
    for (index, (part, pressed)) in [WindowPart::Close, WindowPart::Zoom, WindowPart::Collapse]
        .into_iter()
        .flat_map(|part| [(part, false), (part, true)])
        .enumerate()
    {
        let mut button = Pen::new(11, 11);
        draw_title_box(&mut button, part, pressed);
        pen.append(button, 6 + index as i32 * 21, 6);
    }
    let mut utility = Pen::new(9, 9);
    draw_title_box(&mut utility, WindowPart::Close, false);
    pen.append(utility, 132, 6);
    let mut grow = Pen::new(15, 15);
    draw_grow_box(&mut grow, true);
    pen.append(grow, 153, 6);
    assert_eq!(pixels(&pen), reference);
}

#[test]
fn active_and_background_window_frames_match_the_original_pixels() {
    let reference = image::load_from_memory(include_bytes!("reference/window-frames.png"))
        .unwrap()
        .to_rgba8();
    let mut pen = Pen::new(300, 220);
    pen.rect(0, 0, 300, 220, GRAY_5);
    let mut back = Pen::new(160, 110);
    draw_window_frame(
        &mut back,
        "Background",
        PlatinumWindowOptions {
            active: false,
            ..Default::default()
        },
        false,
        WindowPart::None,
    );
    pen.append(back, 120, 90);
    let mut front = Pen::new(180, 120);
    draw_window_frame(
        &mut front,
        "Untitled",
        PlatinumWindowOptions::default(),
        false,
        WindowPart::None,
    );
    pen.append(front, 30, 30);
    let actual = pixels(&pen);
    let differences: Vec<_> = actual
        .enumerate_pixels()
        .filter(|(x, y, _)| !((36..204).contains(x) && (49..144).contains(y)))
        .filter(|(x, y, pixel)| reference.get_pixel(*x, *y) != *pixel)
        .map(|(x, y, _)| (x, y))
        .collect();
    assert!(
        differences.is_empty(),
        "{} frame pixels differ; first: {:?}",
        differences.len(),
        differences.first()
    );
}

#[test]
fn utility_window_matches_the_original_pixels() {
    let reference = image::load_from_memory(include_bytes!("reference/iconview-utility.png"))
        .unwrap()
        .to_rgba8();
    let expected = image::imageops::crop_imm(&reference, 220, 100, 90, 60).to_image();
    let mut pen = Pen::new(90, 60);
    draw_window_frame(
        &mut pen,
        "Tools",
        PlatinumWindowOptions {
            utility: true,
            ..Default::default()
        },
        false,
        WindowPart::None,
    );
    assert_eq!(pixels(&pen), expected);
}
