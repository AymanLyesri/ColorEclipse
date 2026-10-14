/// Palette model: 16-color scheme plus specials, in the shape cwal emits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("data")
            .join(name)
    }

    #[test]
    fn white_pixel_is_light_black_is_dark() {
        assert_eq!(variant_for_pixels(&[Rgb(255, 255, 255)]), "light");
        assert_eq!(variant_for_pixels(&[Rgb(0, 0, 0)]), "dark");
        assert_eq!(variant_for_pixels(&[Rgb(128, 128, 128)]), "dark");
    }

    #[test]
    fn generates_sixteen_distinct_colors() {
        let path = fixture("dark-anime.jpg");
        let wallpaper = path.to_string_lossy().into_owned();
        let s = generate(&path, &wallpaper, "dark").unwrap();
        assert_eq!(s.colors.len(), 16);
        let unique: std::collections::HashSet<_> = s.colors.iter().collect();
        assert!(unique.len() >= 8, "muddy duplicates: {unique:?}");
        assert_eq!(s.wallpaper, wallpaper);
        assert_eq!(s.mode, "dark");
    }

    #[test]
    fn light_scheme_has_light_background() {
        let path = fixture("light-scenic.jpg");
        let wallpaper = path.to_string_lossy().into_owned();
        let s = generate(&path, &wallpaper, "light").unwrap();
        assert_eq!(variant_for_pixels(&[s.background]), "light");
        assert_eq!(variant_for_pixels(&[s.foreground]), "dark");
    }

    fn write_rgba_png(name: &str, width: u32, height: u32, fill: impl Fn(u32, u32) -> [u8; 4]) -> PathBuf {
        let mut image = image::RgbaImage::new(width, height);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            *pixel = image::Rgba(fill(x, y));
        }
        let path = std::env::temp_dir().join(format!(
            "coloreclipse-{name}-{}-tmp.png",
            std::process::id()
        ));
        image.save(&path).unwrap();
        path
    }

    #[test]
    fn fully_transparent_image_is_an_error() {
        let path = write_rgba_png("alpha-empty", 8, 8, |_, _| [0, 0, 0, 0]);
        let result = generate(&path, "wall.png", "dark");
        let _ = std::fs::remove_file(&path);
        assert!(result.is_err());
    }

    #[test]
    fn transparent_regions_do_not_drag_palette_to_black() {
        // Red disc on a transparent field: only opaque pixels may vote.
        let path = write_rgba_png("alpha-disc", 64, 64, |x, y| {
            let (dx, dy) = (x as i32 - 32, y as i32 - 32);
            if dx * dx + dy * dy < 24 * 24 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 0, 0]
            }
        });
        let s = generate(&path, "wall.png", "dark").unwrap();
        let _ = std::fs::remove_file(&path);
        assert_ne!(s.background, Rgb(0, 0, 0));
    }

    #[test]
    fn corrupt_bytes_are_an_error() {
        let path = std::env::temp_dir().join(format!("coloreclipse-corrupt-{}", std::process::id()));
        std::fs::write(&path, b"this is not an image").unwrap();
        let result = generate(&path, "wall.png", "dark");
        let _ = std::fs::remove_file(&path);
        assert!(result.is_err());
    }

    #[test]
    fn solid_pixel_yields_usable_scheme() {
        let path = write_rgba_png("solid", 1, 1, |_, _| [255, 0, 0, 255]);
        let s = generate(&path, "wall.png", "dark").unwrap();
        let _ = std::fs::remove_file(&path);
        let unique: std::collections::HashSet<_> = s.colors.iter().collect();
        assert!(unique.len() >= 8);
    }

    #[test]
    fn wallpaper_path_is_carried_verbatim() {
        let path = fixture("dark-anime.jpg");
        let s = generate(&path, "/tmp/some dir/wäll.jpg", "dark").unwrap();
        assert_eq!(s.wallpaper, "/tmp/some dir/wäll.jpg");
    }

    #[test]
    fn light_image_pinned_dark_stays_readable() {
        let path = fixture("light-scenic.jpg");
        let wallpaper = path.to_string_lossy().into_owned();
        let s = generate(&path, &wallpaper, "dark").unwrap();
        assert!(luminance(&s.background) <= 64);
        assert!(luminance(&s.foreground) >= 190);
    }
}

/// Decode an image to RGB pixels, thumbnailing the longest edge to ~256px so
/// later quantization stays cheap. Fully transparent pixels are skipped;
/// semi-transparent ones are composited over black. Errors are strings for
/// the CLI to print.
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
        // Fully transparent pixels carry no color information; letting them
        // vote would drag palettes of transparent PNGs toward black.
        .filter(|pixel| pixel[3] != 0)
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

fn luminance(color: &Rgb) -> u32 {
    (u32::from(color.0) * 299 + u32::from(color.1) * 587 + u32::from(color.2) * 114) / 1000
}

/// HSV hue angle in degrees; achromatic colors sort as 0 for determinism.
fn hue_angle(color: &Rgb) -> f32 {
    let (r, g, b) = (f32::from(color.0), f32::from(color.1), f32::from(color.2));
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    if delta < 1.0 {
        return 0.0;
    }
    let hue = if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    if hue < 0.0 { hue + 360.0 } else { hue }
}

fn distance_sq(a: &Rgb, b: &Rgb) -> u64 {
    let dr = i32::from(a.0) - i32::from(b.0);
    let dg = i32::from(a.1) - i32::from(b.1);
    let db = i32::from(a.2) - i32::from(b.2);
    (dr * dr + dg * dg + db * db) as u64
}

/// Deterministic k-means: seeds are the most frequent 5-bit buckets spread by
/// a minimum RGB distance, then Lloyd iterations over all thumbnail pixels.
fn kmeans(pixels: &[Rgb], count: usize, iterations: usize) -> Vec<Rgb> {
    use std::collections::HashMap;
    let mut buckets: HashMap<u32, (u64, [u64; 3])> = HashMap::new();
    for pixel in pixels {
        let key =
            (u32::from(pixel.0 >> 3) << 10) | (u32::from(pixel.1 >> 3) << 5) | u32::from(pixel.2 >> 3);
        let entry = buckets.entry(key).or_insert((0, [0, 0, 0]));
        entry.0 += 1;
        entry.1[0] += u64::from(pixel.0);
        entry.1[1] += u64::from(pixel.1);
        entry.1[2] += u64::from(pixel.2);
    }
    let mut by_frequency: Vec<(Rgb, u64)> = buckets
        .values()
        .map(|(n, sum)| {
            (
                Rgb((sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8),
                *n,
            )
        })
        .collect();
    by_frequency.sort_by(|a, b| b.1.cmp(&a.1));

    let mut centroids: Vec<Rgb> = Vec::with_capacity(count);
    for (color, _) in &by_frequency {
        if centroids.len() >= count {
            break;
        }
        if centroids.iter().all(|c| distance_sq(c, color) >= 48 * 48 * 3) {
            centroids.push(*color);
        }
    }
    // Few distinct buckets (solid-color image): pad by reusing the top color
    // with deterministic offsets so we still return `count` entries.
    let mut pad = 0u8;
    while centroids.len() < count {
        let base = by_frequency.first().map(|(c, _)| *c).unwrap_or(Rgb(0, 0, 0));
        pad = pad.wrapping_add(17);
        centroids.push(Rgb(
            base.0.wrapping_add(pad),
            base.1.wrapping_add(pad / 2),
            base.2.wrapping_sub(pad / 3),
        ));
    }

    for _ in 0..iterations {
        let mut sums = vec![[0u64; 3]; count];
        let mut totals = vec![0u64; count];
        for pixel in pixels {
            let mut best = 0;
            let mut best_dist = u64::MAX;
            for (index, centroid) in centroids.iter().enumerate() {
                let dist = distance_sq(pixel, centroid);
                if dist < best_dist {
                    best_dist = dist;
                    best = index;
                }
            }
            sums[best][0] += u64::from(pixel.0);
            sums[best][1] += u64::from(pixel.1);
            sums[best][2] += u64::from(pixel.2);
            totals[best] += 1;
        }
        let mut moved = false;
        for (index, centroid) in centroids.iter_mut().enumerate() {
            if totals[index] == 0 {
                continue;
            }
            let next = Rgb(
                (sums[index][0] / totals[index]) as u8,
                (sums[index][1] / totals[index]) as u8,
                (sums[index][2] / totals[index]) as u8,
            );
            if next != *centroid {
                *centroid = next;
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
    centroids
}

/// Pull a color toward its own grey by `amount` (0 keeps hue, 1 is grey).
fn desaturate(color: &Rgb, amount: f32) -> Rgb {
    let grey = luminance(color) as f32;
    let mix = |channel: u8| (f32::from(channel) + (grey - f32::from(channel)) * amount).round() as u8;
    Rgb(mix(color.0), mix(color.1), mix(color.2))
}

/// Scale a color so its luminance becomes `target` (keeps hue).
fn scale_to_luminance(color: &Rgb, target: u32) -> Rgb {
    let current = luminance(color);
    if current == 0 || current == target {
        return *color;
    }
    let factor = target as f32 / current as f32;
    Rgb(
        (f32::from(color.0) * factor).round().clamp(0.0, 255.0) as u8,
        (f32::from(color.1) * factor).round().clamp(0.0, 255.0) as u8,
        (f32::from(color.2) * factor).round().clamp(0.0, 255.0) as u8,
    )
}

/// Quantize `path` into a 16-color scheme.
///
/// Ordering mirrors cwal's pywal shape (probed 2026-10-14 and pinned here):
/// dark `eula…jpg --mode dark` gave `background=color0=#1d1a1b`,
/// `foreground=cursor=color15=#d1d1d1`; light `flantia…jpg --mode light`
/// gave `background=color0=#e0d9df`, `foreground=cursor=color15=#161516`.
/// Cached dark schemes show `color15` ≈ neutral (205–209); cached light
/// schemes show `color0` ≈ light (206–229) and `color15` ≈ near-black
/// (20–22). Hence extremes are image-derived but neutralized, while
/// `background=color0` / `foreground=color15` / `cursor=foreground` holds
/// in both modes with the ramp direction flipped.
pub fn generate(
    path: &std::path::Path,
    wallpaper_abs: &str,
    mode: &str,
) -> Result<Scheme, String> {
    let pixels = decode_pixels(path)?;
    if pixels.is_empty() {
        return Err(format!("Error: no pixels decoded from {}", path.display()));
    }
    let mut clusters = kmeans(&pixels, 16, 12);
    clusters.sort_by_key(luminance);
    let (dark_half, bright_half) = clusters.split_at(8);
    let mut dark_band = dark_half.to_vec();
    let mut bright_band = bright_half.to_vec();
    dark_band.sort_by(|a, b| hue_angle(a).total_cmp(&hue_angle(b)));
    bright_band.sort_by(|a, b| hue_angle(a).total_cmp(&hue_angle(b)));

    let darkest = clusters[0];
    let lightest = clusters[15];
    let (background, foreground, mut middle): (Rgb, Rgb, Vec<Rgb>) = if mode == "light" {
        let background = {
            let lifted = desaturate(&lightest, 0.65);
            if luminance(&lifted) < 205 {
                scale_to_luminance(&lifted, 215)
            } else {
                lifted
            }
        };
        let foreground = if luminance(&darkest) > 40 {
            scale_to_luminance(&darkest, 24)
        } else {
            darkest
        };
        let mut middle = Vec::with_capacity(14);
        middle.extend(bright_band.iter().filter(|c| **c != lightest));
        middle.extend(dark_band.iter().filter(|c| **c != darkest));
        (background, foreground, middle)
    } else {
        let background = if luminance(&darkest) > 64 {
            scale_to_luminance(&darkest, 40)
        } else {
            darkest
        };
        let foreground = {
            // Neutral near-white with a whisper of the image's lightest hue.
            let tinted = desaturate(&lightest, 0.85);
            let grey = Rgb(206, 206, 206);
            Rgb(
                ((f32::from(grey.0) * 0.85 + f32::from(tinted.0) * 0.15).round()) as u8,
                ((f32::from(grey.1) * 0.85 + f32::from(tinted.1) * 0.15).round()) as u8,
                ((f32::from(grey.2) * 0.85 + f32::from(tinted.2) * 0.15).round()) as u8,
            )
        };
        let mut middle = Vec::with_capacity(14);
        middle.extend(dark_band.iter().filter(|c| **c != darkest));
        middle.extend(bright_band.iter().filter(|c| **c != lightest));
        (background, foreground, middle)
    };
    // Dedupe fallthrough (solid-color images): refill from hue-spread shades
    // of the background/foreground so all 16 entries stay usable.
    while middle.len() < 14 {
        let anchor = if middle.len() % 2 == 0 { background } else { foreground };
        let step = (middle.len() as u8).wrapping_mul(13);
        middle.push(Rgb(
            anchor.0.wrapping_add(step),
            anchor.1.wrapping_add(step / 2),
            anchor.2.wrapping_sub(step / 3),
        ));
    }
    middle.truncate(14);

    let mut colors = [Rgb(0, 0, 0); 16];
    colors[0] = background;
    for (index, color) in middle.iter().take(14).enumerate() {
        colors[index + 1] = *color;
    }
    colors[15] = foreground;
    Ok(Scheme {
        wallpaper: wallpaper_abs.to_string(),
        mode: mode.to_string(),
        alpha: 1.0,
        background,
        foreground,
        cursor: foreground,
        colors,
    })
}

