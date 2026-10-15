//! Reproducible example images rendered by GPUI's native offscreen renderer.

use gpui::{App, Entity, HeadlessAppContext, Render, Window, px, size};
use std::{error::Error, path::Path, sync::Arc};

pub fn render_png<V: Render + 'static>(
    path: &str,
    width: f32,
    height: f32,
    scale: f32,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> Result<(), Box<dyn Error>> {
    if !scale.is_finite() || scale <= 0. || scale > 8. {
        return Err("image scale must be greater than zero and at most eight".into());
    }
    let platform = gpui_platform::current_platform(true);
    let mut app = HeadlessAppContext::with_platform(
        platform.text_system(),
        Arc::new(()),
        gpui_platform::current_headless_renderer,
    );
    app.update(gpui_platinum::init);
    let window = app.open_window(size(px(width), px(height)), build)?;
    app.run_until_parked();
    app.update_window(window.into(), |_, window, cx| {
        let _ = window.draw(cx);
    })?;
    let rendered = app.capture_screenshot(window.into())?;
    let output_width = (width * scale).round() as u32;
    let output_height = (height * scale).round() as u32;
    let image = if rendered.dimensions() == (output_width, output_height) {
        rendered
    } else {
        image::imageops::resize(
            &rendered,
            output_width,
            output_height,
            image::imageops::FilterType::Nearest,
        )
    };
    let path = Path::new(path);
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    image.save(path)?;
    Ok(())
}

pub fn argument<'a>(arguments: &'a [String], name: &str) -> Option<&'a str> {
    arguments
        .iter()
        .position(|argument| argument == name)
        .and_then(|index| arguments.get(index + 1))
        .map(String::as_str)
}
