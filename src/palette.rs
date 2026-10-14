/// Palette model: 16-color scheme plus specials, in the shape cwal emits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

#[derive(Debug, Clone, PartialEq)]
pub struct Scheme {
    pub wallpaper: String,
    pub mode: String,
    pub alpha: f32,
    pub background: Rgb,
    pub foreground: Rgb,
    pub cursor: Rgb,
    pub colors: [Rgb; 16],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_pixel_is_light_black_is_dark() {
        assert_eq!(variant_for_pixels(&[Rgb(255, 255, 255)]), "light");
        assert_eq!(variant_for_pixels(&[Rgb(0, 0, 0)]), "dark");
        assert_eq!(variant_for_pixels(&[Rgb(128, 128, 128)]), "dark");
    }
}

/// Decode an image to RGB pixels, thumbnailing the longest edge to ~256px so
/// later quantization stays cheap. Transparent pixels are composited over
/// black. Errors are strings for the CLI to print.
pub fn decode_pixels(path: &std::path::Path) -> Result<Vec<Rgb>, String> {
    let image = image::ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?;
    let (width, height) = (image.width(), image.height());
    let thumbnail = if width.max(height) > 256 {
        let scale = 256.0 / width.max(height) as f32;
        let (w, h) = (
            (width as f32 * scale).round().max(1.0) as u32,
            (height as f32 * scale).round().max(1.0) as u32,
        );
        image.thumbnail(w, h)
    } else {
        image
    };
    let rgba = thumbnail.to_rgba8();
    Ok(rgba
        .pixels()
        .map(|pixel| {
            let alpha = f32::from(pixel[3]) / 255.0;
            Rgb(
                (f32::from(pixel[0]) * alpha).round() as u8,
                (f32::from(pixel[1]) * alpha).round() as u8,
                (f32::from(pixel[2]) * alpha).round() as u8,
            )
        })
        .collect())
}

/// Mean color of a pixel slice; black when empty.
pub fn average_color(pixels: &[Rgb]) -> Rgb {
    if pixels.is_empty() {
        return Rgb(0, 0, 0);
    }
    let (mut r, mut g, mut b) = (0u64, 0u64, 0u64);
    for pixel in pixels {
        r += u64::from(pixel.0);
        g += u64::from(pixel.1);
        b += u64::from(pixel.2);
    }
    let count = pixels.len() as u64;
    Rgb((r / count) as u8, (g / count) as u8, (b / count) as u8)
}

/// Perceived-brightness split matching `variant_from_rgb` in main.rs:
/// `(r*299 + g*587 + b*114) / 1000 > 128` is light.
pub fn variant_for_pixels(pixels: &[Rgb]) -> &'static str {
    let average = average_color(pixels);
    let luminance =
        (u32::from(average.0) * 299 + u32::from(average.1) * 587 + u32::from(average.2) * 114)
            / 1000;
    if luminance > 128 { "light" } else { "dark" }
}
