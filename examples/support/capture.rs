//! Reproducible example images rendered by GPUI's native offscreen renderer.

use gpui::{App, Entity, HeadlessAppContext, Render, Window, px, size};
use image::{
    ImageEncoder, RgbImage, RgbaImage,
    buffer::ConvertBuffer,
    codecs::png::{CompressionType, FilterType, PngEncoder},
};
use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    sync::Arc,
};

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
    let mut output = BufWriter::new(File::create(path)?);
    encode_png(&image, &mut output)?;
    output.flush()?;
    Ok(())
}

fn encode_png(image: &RgbaImage, output: impl Write) -> image::ImageResult<()> {
    let encoder = PngEncoder::new_with_quality(output, CompressionType::Best, FilterType::Adaptive);
    if image.pixels().all(|pixel| pixel[3] == u8::MAX) {
        let rgb: RgbImage = image.convert();
        encoder.write_image(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
    } else {
        encoder.write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )
    }
}

pub fn argument<'a>(arguments: &'a [String], name: &str) -> Option<&'a str> {
    arguments
        .iter()
        .position(|argument| argument == name)
        .and_then(|index| arguments.get(index + 1))
        .map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_preserves_pixels_and_only_omits_opaque_alpha() {
        for (alphas, color) in [
            ([255, 255, 255, 255, 255, 255], image::ColorType::Rgb8),
            ([255, 254, 128, 1, 0, 255], image::ColorType::Rgba8),
        ] {
            let original = RgbaImage::from_fn(3, 2, |x, y| {
                let i = (y * 3 + x) as usize;
                image::Rgba([i as u8 * 43, 255 - i as u8 * 31, 127, alphas[i]])
            });
            let mut png = Vec::new();
            encode_png(&original, &mut png).unwrap();
            let decoded = image::load_from_memory(&png).unwrap();
            assert_eq!(decoded.color(), color);
            assert_eq!(decoded.to_rgba8(), original);
        }
    }
}
