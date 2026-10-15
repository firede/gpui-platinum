use gpui_platinum::{draw_button, draw_default_ring, draw_focus_ring, paint::Pen, theme::*};

fn render(pen: &Pen, pixels: &mut image::RgbaImage, x: i32, y: i32) {
    for fill in &pen.fills {
        let color = image::Rgba([
            (fill.color >> 16) as u8,
            (fill.color >> 8) as u8,
            fill.color as u8,
            255,
        ]);
        for row in 0..fill.rect.height {
            for col in 0..fill.rect.width {
                let px = x + fill.rect.x + col;
                let py = y + fill.rect.y + row;
                if px >= 0 && py >= 0 && px < pixels.width() as i32 && py < pixels.height() as i32 {
                    pixels.put_pixel(px as u32, py as u32, color);
                }
            }
        }
    }
}

#[test]
fn push_button_states_match_the_original_pixels() {
    let reference = image::load_from_memory(include_bytes!("reference/button-states.png"))
        .unwrap()
        .to_rgba8();
    let mut actual = image::RgbaImage::from_pixel(400, 40, image::Rgba([0xdd, 0xdd, 0xdd, 255]));
    for (index, label) in ["Normal", "Pressed", "Disabled", "Default", "Focus"]
        .iter()
        .enumerate()
    {
        let x = 6 + index as i32 * 76;
        if index == 3 {
            let mut ring = Pen::new(70, 28);
            draw_default_ring(&mut ring, false);
            render(&ring, &mut actual, x, 6);
        }
        let mut button = Pen::new(62, 20);
        draw_button(&mut button, label, index == 2, index == 1);
        if index == 4 {
            draw_focus_ring(&mut button, 3, ACCENTS[0].dark);
        }
        render(&button, &mut actual, x + 4, 10);
    }
    let differences: Vec<_> = actual
        .enumerate_pixels()
        .filter(|(x, y, pixel)| reference.get_pixel(*x, *y) != *pixel)
        .map(|(x, y, _)| (x, y))
        .collect();
    assert!(
        differences.is_empty(),
        "{} pixels differ; first: {:?}",
        differences.len(),
        differences.first()
    );
}

#[test]
fn checkbox_states_match_the_original_pixels() {
    use gpui_platinum::{CheckState, draw_checkbox};
    let reference = image::load_from_memory(include_bytes!("reference/checkbox-states.png"))
        .unwrap()
        .to_rgba8();
    let mut actual = image::RgbaImage::from_pixel(260, 30, image::Rgba([0xdd, 0xdd, 0xdd, 255]));
    for index in 0..10 {
        let mut pen = Pen::new(12, 12);
        let mark = if index == 9 {
            CheckState::On
        } else {
            [CheckState::Off, CheckState::On, CheckState::Mixed][index % 3]
        };
        draw_checkbox(
            &mut pen,
            mark,
            (6..9).contains(&index),
            (3..6).contains(&index),
        );
        if index == 9 {
            draw_focus_ring(&mut pen, 0, ACCENTS[0].dark);
        }
        render(&pen, &mut actual, 9 + index as i32 * 24, 9);
    }
    assert_pixels(&actual, &reference);
}

#[test]
fn radio_states_match_the_original_pixels() {
    use gpui_platinum::{draw_focus_circle, draw_radio};
    let reference = image::load_from_memory(include_bytes!("reference/radio-states.png"))
        .unwrap()
        .to_rgba8();
    let mut actual = image::RgbaImage::from_pixel(200, 30, image::Rgba([0xdd, 0xdd, 0xdd, 255]));
    for index in 0..7 {
        let mut pen = Pen::new(12, 12);
        draw_radio(
            &mut pen,
            index % 2 == 1 || index == 6,
            (4..6).contains(&index),
            (2..4).contains(&index),
        );
        if index == 6 {
            draw_focus_circle(&mut pen, ACCENTS[0].dark);
        }
        render(&pen, &mut actual, 9 + index * 24, 9);
    }
    assert_pixels(&actual, &reference);
}

fn assert_pixels(actual: &image::RgbaImage, reference: &image::RgbaImage) {
    let differences: Vec<_> = actual
        .enumerate_pixels()
        .filter(|(x, y, pixel)| reference.get_pixel(*x, *y) != *pixel)
        .map(|(x, y, _)| (x, y))
        .collect();
    assert!(
        differences.is_empty(),
        "{} pixels differ; first: {:?}",
        differences.len(),
        differences.first().map(|&(x, y)| (
            x,
            y,
            actual.get_pixel(x, y),
            reference.get_pixel(x, y)
        ))
    );
}

#[test]
fn progress_states_match_the_original_pixels() {
    use gpui_platinum::{draw_progress, draw_progress_indeterminate};
    let reference = image::load_from_memory(include_bytes!("reference/progress-states.png"))
        .unwrap()
        .to_rgba8();
    let mut actual = image::RgbaImage::from_pixel(520, 30, image::Rgba([0xdd, 0xdd, 0xdd, 255]));
    for index in 0..4 {
        let mut pen = Pen::new(120, 14);
        if index == 3 {
            draw_progress_indeterminate(&mut pen, 3, ACCENTS[0]);
        } else {
            draw_progress(&mut pen, [0., 0.4, 1.][index], ACCENTS[0]);
        }
        render(&pen, &mut actual, 6 + index as i32 * 126, 6);
    }
    assert_pixels(&actual, &reference);
}
