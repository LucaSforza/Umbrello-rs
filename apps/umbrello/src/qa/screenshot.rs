//! In-memory screenshot encoding and correlation helpers.

use egui::ColorImage;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScreenshotResult {
    pub png: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub state_revision: u64,
    pub rendered_revision: u64,
}

pub(crate) fn encode_png(
    image: &ColorImage,
    state_revision: u64,
    rendered_revision: u64,
) -> Result<ScreenshotResult, String> {
    let width = image.size[0];
    let height = image.size[1];
    let mut rgba = Vec::with_capacity(width.saturating_mul(height).saturating_mul(4));
    for pixel in &image.pixels {
        rgba.extend_from_slice(&pixel.to_array());
    }
    let mut png = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png);
    image::ImageEncoder::write_image(
        encoder,
        &rgba,
        width as u32,
        height as u32,
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|error| error.to_string())?;
    Ok(ScreenshotResult {
        png,
        width,
        height,
        state_revision,
        rendered_revision,
    })
}
