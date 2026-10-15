use gpui_platinum::{
    ScrollBar, Slider, StepperState, Theme, draw_stepper,
    paint::Pen,
    slider::{ScrollPart, SliderState, draw_slider_thumb},
};

fn paint(pen: &Pen, pixels: &mut image::RgbaImage, x: i32, y: i32) {
    for fill in &pen.fills {
        let color = image::Rgba([
            (fill.color >> 16) as u8,
            (fill.color >> 8) as u8,
            fill.color as u8,
            255,
        ]);
        for row in 0..fill.rect.height {
            for column in 0..fill.rect.width {
                let x = x + fill.rect.x + column;
                let y = y + fill.rect.y + row;
                if x >= 0 && y >= 0 && x < pixels.width() as i32 && y < pixels.height() as i32 {
                    pixels.put_pixel(x as u32, y as u32, color);
                }
            }
        }
    }
}

fn assert_pixels(actual: &image::RgbaImage, reference: &image::RgbaImage) {
    assert_eq!(actual.dimensions(), reference.dimensions());
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
fn slider_states_match_the_original_pixels() {
    let reference = image::load_from_memory(include_bytes!("reference/slider-states.png"))
        .unwrap()
        .to_rgba8();
    let mut actual = image::RgbaImage::from_pixel(520, 40, image::Rgba([0xdd, 0xdd, 0xdd, 255]));
    for (index, value, ticks, disabled) in
        [(0, 0.3, 0, false), (1, 0.5, 5, false), (2, 0.7, 0, true)]
    {
        let mut pen = Pen::new(120, 24);
        Slider::new("slider", value, 0., 1.)
            .width(120)
            .ticks(ticks)
            .disabled(disabled)
            .draw(&mut pen, false);
        paint(&pen, &mut actual, 6 + index * 126, 6);
    }
    let mut thumb = Pen::new(11, 16);
    draw_slider_thumb(&mut thumb, SliderState::Pressed, Theme::default());
    paint(&thumb, &mut actual, 6 + 3 * 126 + 10, 6);
    assert_pixels(&actual, &reference);
}

#[test]
fn scrollbar_states_match_the_original_pixels() {
    let reference = image::load_from_memory(include_bytes!("reference/scrollbar-states.png"))
        .unwrap()
        .to_rgba8();
    let mut actual = image::RgbaImage::from_pixel(260, 130, image::Rgba([0xdd, 0xdd, 0xdd, 255]));
    for (index, value, total, pressed) in [
        (0, 0., 40., ScrollPart::None),
        (1, 15., 40., ScrollPart::Thumb),
        (2, 30., 40., ScrollPart::Forward),
        (3, 10., 40., ScrollPart::PageForward),
        (4, 0., 5., ScrollPart::None),
    ] {
        let mut pen = Pen::new(16, 116);
        ScrollBar::new("scrollbar", value, 10., total)
            .length(116)
            .draw(&mut pen, pressed);
        paint(&pen, &mut actual, 6 + index * 22, 6);
    }
    let mut pen = Pen::new(130, 16);
    ScrollBar::new("horizontal", 20., 10., 40.)
        .length(130)
        .horizontal()
        .draw(&mut pen, ScrollPart::Back);
    paint(&pen, &mut actual, 120, 10);
    assert_pixels(&actual, &reference);
}

#[test]
fn stepper_states_match_the_original_pixels() {
    let reference = image::load_from_memory(include_bytes!("reference/stepper-status.png"))
        .unwrap()
        .to_rgba8();
    for (index, pressed, state) in [
        (0, 0, StepperState::Normal),
        (1, 1, StepperState::Normal),
        (2, 0, StepperState::Disabled),
    ] {
        let mut actual = image::RgbaImage::from_pixel(13, 23, image::Rgba([0xdd, 0xdd, 0xdd, 255]));
        let mut pen = Pen::new(13, 23);
        draw_stepper(&mut pen, pressed, state);
        paint(&pen, &mut actual, 0, 0);
        let crop = image::imageops::crop_imm(&reference, 8 + index * 21, 8, 13, 23).to_image();
        assert_pixels(&actual, &crop);
    }
}
