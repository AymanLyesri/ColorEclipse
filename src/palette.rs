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
    /// A vivid, legible accent for UIs (sliders, focus rings, active states).
    /// Not one of the 16 slots: cwal's `color5` is a prominence rank and is
    /// often too dim to use.
    pub accent: Rgb,
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

    fn hex_color(hex: &str) -> Rgb {
        Rgb(
            u8::from_str_radix(&hex[0..2], 16).unwrap(),
            u8::from_str_radix(&hex[2..4], 16).unwrap(),
            u8::from_str_radix(&hex[4..6], 16).unwrap(),
        )
    }

    fn hsv_saturation(color: &Rgb) -> f32 {
        rgb_to_hsv(color).1
    }

    /// The reported bug: on a saturated wallpaper (a yellow cyberpunk image)
    /// the light-mode background came out as a near-fully-saturated yellow
    /// field, because cwal's `lighten(bg, 0.93 - lum)` only mixes that
    /// fraction of the way to white and so never reaches its own 0.93 target.
    /// cwal's own output for such an image is a desaturated near-neutral, and
    /// a light theme background must not be a saturated colour.
    #[test]
    fn light_background_is_near_white_with_only_a_tint() {
        // A big saturated yellow field, like the workspace-3 wallpaper.
        let path = write_rgba_png("saturated-light", 96, 96, |x, _| {
            let band = (x / 12) % 3;
            match band {
                0 => [233, 239, 45, 255],
                1 => [214, 226, 96, 255],
                _ => [246, 240, 140, 255],
            }
        });
        let s = generate(&path, "wall.png", "light").unwrap();
        let _ = std::fs::remove_file(&path);

        assert!(
            w3_luminance(&s.background) >= LIGHT_BACKGROUND_LUMINANCE,
            "light background too dark: w3 {:.3} < {LIGHT_BACKGROUND_LUMINANCE}",
            w3_luminance(&s.background)
        );
        assert!(
            hsv_saturation(&s.background) <= LIGHT_BACKGROUND_MAX_TINT,
            "light background too saturated: {:.3} > {LIGHT_BACKGROUND_MAX_TINT}",
            hsv_saturation(&s.background)
        );
        // Readable: the dark foreground must still clear WCAG AA on it.
        let fg_lum = w3_luminance(&s.foreground);
        let ratio = (fg_lum.max(w3_luminance(&s.background)) + 0.05)
            / (fg_lum.min(w3_luminance(&s.background)) + 0.05);
        assert!(ratio >= 7.0, "foreground contrast on light background: {ratio:.2}:1");
    }

    /// A dark-mode background must be left completely alone by the light
    /// normalization.
    #[test]
    fn dark_background_is_untouched_by_light_normalization() {
        let path = fixture("dark-anime.jpg");
        let s = generate(&path, &path.to_string_lossy(), "dark").unwrap();
        // Recorded 2026-10-17; the light path must not reach into dark mode.
        assert_eq!(s.background, hex_color("100f13"));
        assert!(w3_luminance(&s.background) < 0.5, "expected a dark background");
    }

    /// The shell accent must be vivid *and* legible. cwal's `color5` is a
    /// prominence rank and can be a dim near-grey (~2.6:1 on a dark
    /// background), which is useless for sliders and focus rings.
    #[test]
    fn accent_is_vivid_and_legible_against_the_background() {
        for (name, mode) in [("dark-anime.jpg", "dark"), ("light-scenic.jpg", "light")] {
            let path = fixture(name);
            let s = generate(&path, &path.to_string_lossy(), mode).unwrap();
            let bg = w3_luminance(&s.background);
            let accent_lum = w3_luminance(&s.accent);
            let ratio = (accent_lum.max(bg) + 0.05) / (accent_lum.min(bg) + 0.05);
            assert!(
                ratio >= MIN_ACCENT_CONTRAST,
                "{name} {mode}: accent {:?} only {ratio:.2}:1 on bg {:?}",
                s.accent, s.background
            );
            // Vivid: not one of the deliberately grey derived slots.
            assert!(
                !s.colors[7..9].contains(&s.accent),
                "{name} {mode}: accent fell back to a grey slot"
            );
            // Pin the wiring: `generate` must actually call `pick_accent`
            // rather than picking a fixed slot like cwal's color5. Without
            // this, swapping the call back to `colors[5]` still satisfies every
            // other assertion here on these fixtures.
            assert_eq!(
                s.accent,
                pick_accent(&s.background, &s.colors[1..7]),
                "{name} {mode}: generate() must source the accent from pick_accent"
            );
        }
    }

    /// The measured real case: on a dark wallpaper cwal's `color5` was
    /// `#36519a`, only 2.60:1 against the background. Ranking by prominence
    /// alone would keep picking it; the accent must instead come from the
    /// vivid, legible ones.
    #[test]
    fn accent_rejects_the_dim_color5_in_favour_of_a_vivid_one() {
        let background = hex_color("0c0c15");
        let accents = [
            hex_color("33427a"), // vivid but dim on this bg
            hex_color("5967a6"),
            hex_color("332b57"),
            hex_color("9697c1"), // dim, desaturated
            hex_color("36519a"), // <- cwal's color5, 2.60:1: rejected
            hex_color("da7dd1"), // vivid and legible
        ];
        let accent = pick_accent(&background, &accents);
        assert_ne!(accent, accents[4], "picked the too-dim color5");
        assert_eq!(accent, accents[5]);
    }

    /// `generate` must take the accent from `pick_accent`, not from a fixed
    /// slot. On the two image fixtures `pick_accent` happens to agree with
    /// `color5`, so they cannot tell the two apart; this band layout is a case
    /// where they genuinely differ, which pins the wiring.
    #[test]
    fn generate_sources_the_accent_from_pick_accent_not_a_fixed_slot() {
        let bands: [[u8; 3]; 8] = [
            [220, 4, 101],
            [170, 31, 173],
            [29, 90, 218],
            [229, 172, 27],
            [30, 95, 19],
            [112, 121, 108],
            [253, 16, 255],
            [25, 175, 96],
        ];
        let path = write_rgba_png("accent-pick", 64, 128, |_, y| {
            let band = bands[(y / 16) as usize % bands.len()];
            [band[0], band[1], band[2], 255]
        });
        let s = generate(&path, "wall.png", "light").unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(s.colors[5], Rgb(220, 4, 101), "fixture drifted");
        assert_ne!(
            s.accent, s.colors[5],
            "accent fell back to cwal's color5 slot"
        );
        assert_eq!(s.accent, Rgb(30, 95, 19));
        assert_eq!(s.accent, pick_accent(&s.background, &s.colors[1..7]));
    }

    /// Parity with cwal's own output on the same fixtures (recorded 2026-10-14
    /// via `cwal -i <fixture>`). The derivation structure is byte-faithful
    /// (background crush, foreground/grey derivations, boosts), so those
    /// relations hold exactly; clustering differs (Lab k-means vs ImageMagick
    /// octree), so absolute colors are bounded, not exact.
    #[test]
    fn matches_cwal_on_fixtures() {
        let dark_expected = [
            "1d1a1b", "645e79", "907064", "b89770", "a7715a", "cfaea1", "e8af79", "a99e9e",
            "836976", "7665bb", "c9856c", "e1b47b", "da8867", "ebbba8", "ffc388", "d1d1d1",
        ];
        let path = fixture("dark-anime.jpg");
        let wallpaper = path.to_string_lossy().into_owned();
        let dark = generate(&path, &wallpaper, "dark").unwrap();
        // Exact derivation relations.
        assert_eq!(dark.colors[15], lighten(&dark.background, 0.80));
        assert_eq!(dark.colors[7], saturate(&lighten(&dark.background, 0.60), 0.05));
        assert_eq!(dark.colors[8], saturate(&lighten(&dark.background, 0.40), 0.10));
        assert_eq!(dark.foreground, dark.colors[15]);
        assert_eq!(dark.cursor, dark.colors[15]);
        // Absolute neighborhood: background in cwal's dark-crush family.
        let bg_dist = channel_dist(&dark.background, &hex_color("1d1a1b"));
        assert!(bg_dist <= 32, "dark background drifts: {bg_dist}");
        // Every cwal accent hue is present somewhere in our set.
        let set_dist = accent_set_distance(&dark.colors, &dark_expected);
        assert!(set_dist <= 48, "dark accents drift: {set_dist}");

        let light_expected = [
            "dcdae2", "a3adcc", "5c73ac", "9cabd2", "768cbc", "3b51b8", "5d699a", "53535e",
            "978ea8", "6c85cb", "2c52b4", "6684d0", "3e6ac5", "1330bb", "3046a1", "161516",
        ];
        let path = fixture("light-scenic.jpg");
        let wallpaper = path.to_string_lossy().into_owned();
        let light = generate(&path, &wallpaper, "light").unwrap();
        assert_eq!(light.colors[15], darken(&light.background, 0.90));
        assert_eq!(light.colors[7], saturate(&darken(&light.background, 0.60), 0.05));
        assert_eq!(light.colors[8], saturate(&darken(&light.background, 0.30), 0.10));
        assert_eq!(light.foreground, light.colors[15]);
        let bg_dist = channel_dist(&light.background, &hex_color("dcdae2"));
        assert!(bg_dist <= 32, "light background drifts: {bg_dist}");
        let set_dist = accent_set_distance(&light.colors, &light_expected);
        assert!(set_dist <= 32, "light accents drift: {set_dist}");
    }

    fn channel_dist(a: &Rgb, b: &Rgb) -> u8 {
        a.0.abs_diff(b.0).max(a.1.abs_diff(b.1)).max(a.2.abs_diff(b.2))
    }
    /// For each expected color, distance to the nearest color we produced.
    fn accent_set_distance(a: &[Rgb; 16], b: &[&str]) -> u8 {
        b.iter()
            .map(|h| {
                let y = hex_color(h);
                a.iter().map(|x| channel_dist(x, &y)).min().unwrap_or(255)
            })
            .max()
            .unwrap_or(0)
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

// --- CIELAB helpers (clustering happens in Lab, like cwal's Lab quantize) ---

/// sRGB byte color to CIELAB (D65). Mirrors the colorspace cwal quantizes in.
fn srgb_to_lab(color: &Rgb) -> [f32; 3] {
    let linear = |channel: u8| {
        let c = f32::from(channel) / 255.0;
        if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    let (r, g, b) = (linear(color.0), linear(color.1), linear(color.2));
    let (x, y, z) = (
        (0.4124564 * r + 0.3575761 * g + 0.1804375 * b) / 0.95047,
        (0.2126729 * r + 0.7151522 * g + 0.0721750 * b) / 1.0,
        (0.0193339 * r + 0.1191920 * g + 0.9503041 * b) / 1.08883,
    );
    let f = |t: f32| {
        if t > (6.0 / 29.0_f32).powi(3) {
            t.cbrt()
        } else {
            t / (3.0 * (6.0 / 29.0_f32).powi(2)) + 4.0 / 29.0
        }
    };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn lab_distance_sq(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

fn rgb_distance_sq(a: &Rgb, b: &Rgb) -> u64 {
    let dr = i32::from(a.0) - i32::from(b.0);
    let dg = i32::from(a.1) - i32::from(b.1);
    let db = i32::from(a.2) - i32::from(b.2);
    (dr * dr + dg * dg + db * db) as u64
}

/// Deterministic k-means in Lab space. Seeds are the most frequent 5-bit
/// buckets spread by RGB distance; assignments use Lab distance; centroids
/// stay RGB means. Returns `(centroid, pixel count)` per cluster.
fn kmeans_lab(pixels: &[Rgb], count: usize, iterations: usize) -> Vec<(Rgb, u64)> {
    use std::collections::BTreeMap;
    // BTreeMap (not HashMap): bucket order must be deterministic so the same
    // image always yields the same palette.
    let mut buckets: BTreeMap<u32, (u64, [u64; 3])> = BTreeMap::new();
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
        if centroids.iter().all(|c| rgb_distance_sq(c, color) >= 48 * 48 * 3) {
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
    let mut lab_centroids: Vec<[f32; 3]> = centroids.iter().map(srgb_to_lab).collect();

    let mut totals = vec![0u64; count];
    for _ in 0..iterations {
        let mut sums = vec![[0u64; 3]; count];
        totals.fill(0);
        let labs: Vec<[f32; 3]> = pixels.iter().map(srgb_to_lab).collect();
        for (pixel, lab) in pixels.iter().zip(labs.iter()) {
            let mut best = 0;
            let mut best_dist = f32::INFINITY;
            for (index, centroid) in lab_centroids.iter().enumerate() {
                let dist = lab_distance_sq(lab, centroid);
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
                lab_centroids[index] = srgb_to_lab(&next);
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
    // Final sizes for the converged centroids.
    totals.fill(0);
    let labs: Vec<[f32; 3]> = pixels.iter().map(srgb_to_lab).collect();
    for lab in &labs {
        let mut best = 0;
        let mut best_dist = f32::INFINITY;
        for (index, centroid) in lab_centroids.iter().enumerate() {
            let dist = lab_distance_sq(lab, centroid);
            if dist < best_dist {
                best_dist = dist;
                best = index;
            }
        }
        totals[best] += 1;
    }
    centroids.into_iter().zip(totals.into_iter()).collect()
}

// --- cwal-faithful color derivation (HSL/HSV ops mirror cwal's C code) ---

fn clamp_byte(value: f32) -> u8 {
    (value.clamp(0.0, 255.0) + 0.5) as u8
}

/// cwal `darken_color`: multiplicative crush toward black (truncates).
fn darken(color: &Rgb, amount: f32) -> Rgb {
    let amount = amount.clamp(0.0, 1.0);
    Rgb(
        (f32::from(color.0) * (1.0 - amount)) as u8,
        (f32::from(color.1) * (1.0 - amount)) as u8,
        (f32::from(color.2) * (1.0 - amount)) as u8,
    )
}

/// cwal `lighten_color`: mix toward white (truncates).
fn lighten(color: &Rgb, amount: f32) -> Rgb {
    let amount = amount.clamp(0.0, 1.0);
    Rgb(
        (f32::from(color.0) + (255.0 - f32::from(color.0)) * amount) as u8,
        (f32::from(color.1) + (255.0 - f32::from(color.1)) * amount) as u8,
        (f32::from(color.2) + (255.0 - f32::from(color.2)) * amount) as u8,
    )
}

fn rgb_to_hsl(color: &Rgb) -> (f32, f32, f32) {
    let (r, g, b) = (f32::from(color.0) / 255.0, f32::from(color.1) / 255.0, f32::from(color.2) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let (sum, range) = (max + min, max - min);
    let l = sum / 2.0;
    let (mut h, mut s) = (0.0, 0.0);
    if range != 0.0 {
        s = if l <= 0.5 { range / sum } else { range / (2.0 - sum) };
        if r == max {
            h = (g - b) / range;
        } else if g == max {
            h = 2.0 + (b - r) / range;
        } else {
            h = 4.0 + (r - g) / range;
        }
        h = (h / 6.0) % 1.0;
        if h < 0.0 {
            h += 1.0;
        }
    }
    (h, l, s)
}

fn hls_to_rgb(h: f32, l: f32, s: f32) -> Rgb {
    fn calc(t: f32, temp1: f32, temp2: f32) -> f32 {
        if 6.0 * t < 1.0 {
            temp2 + (temp1 - temp2) * 6.0 * t
        } else if 2.0 * t < 1.0 {
            temp1
        } else if 3.0 * t < 2.0 {
            temp2 + (temp1 - temp2) * (2.0 / 3.0 - t) * 6.0
        } else {
            temp2
        }
    }
    let (r, g, b) = if s == 0.0 {
        (l, l, l)
    } else {
        let temp1 = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
        let temp2 = 2.0 * l - temp1;
        let tempr = (h + 1.0 / 3.0) % 1.0;
        let tempb = (h - 1.0 / 3.0 + 1.0) % 1.0;
        (calc(tempr, temp1, temp2), calc(h, temp1, temp2), calc(tempb, temp1, temp2))
    };
    Rgb(clamp_byte(r * 255.0), clamp_byte(g * 255.0), clamp_byte(b * 255.0))
}

/// cwal `saturate_color`: HSL saturation shift.
fn saturate(color: &Rgb, amount: f32) -> Rgb {
    let (h, l, s) = rgb_to_hsl(color);
    hls_to_rgb(h, l, (s + amount).clamp(0.0, 1.0))
}

fn rgb_to_hsv(color: &Rgb) -> (f32, f32, f32) {
    let (r, g, b) = (f32::from(color.0) / 255.0, f32::from(color.1) / 255.0, f32::from(color.2) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let v = max;
    if delta < 0.00001 {
        return (0.0, 0.0, v);
    }
    let s = delta / max;
    let mut h = if r >= max {
        (g - b) / delta
    } else if g >= max {
        2.0 + (b - r) / delta
    } else {
        4.0 + (r - g) / delta
    };
    h *= 60.0;
    if h < 0.0 {
        h += 360.0;
    }
    (h, s, v)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Rgb {
    let (r, g, b) = if s == 0.0 {
        (v, v, v)
    } else {
        let hh = h / 60.0;
        let i = hh as i32;
        let f = hh - i as f32;
        let p = v * (1.0 - s);
        let q = v * (1.0 - s * f);
        let t = v * (1.0 - s * (1.0 - f));
        match i {
            0 => (v, t, p),
            1 => (q, v, p),
            2 => (p, v, t),
            3 => (p, q, v),
            4 => (t, p, v),
            5 => (v, p, q),
            _ => (0.0, 0.0, 0.0),
        }
    };
    Rgb(clamp_byte(r * 255.0), clamp_byte(g * 255.0), clamp_byte(b * 255.0))
}

/// cwal `w3_luminance`: WCAG relative luminance.
fn w3_luminance(color: &Rgb) -> f32 {
    let mut channels = [f32::from(color.0) / 255.0, f32::from(color.1) / 255.0, f32::from(color.2) / 255.0];
    for channel in channels.iter_mut() {
        *channel = if *channel <= 0.04045 {
            *channel / 12.92
        } else {
            ((*channel + 0.055) / 1.055).powf(2.4)
        };
    }
    0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2]
}

/// cwal's own target for a light-mode background is w3_luminance 0.93, but it
/// gets there with `lighten(bg, 0.93 - lum)`, which only mixes that fraction
/// of the way to white and never arrives. These are the values we actually
/// want: bright enough to read as a light surface, and barely tinted at all so
/// a saturated wallpaper cannot flood the whole screen with one hue.
pub const LIGHT_BACKGROUND_LUMINANCE: f32 = 0.92;
pub const LIGHT_BACKGROUND_MAX_TINT: f32 = 0.045;

/// WCAG minimum contrast for a non-text/UI accent (WCAG 1.4.11). Below this an
/// accent is a valid palette entry but unusable for sliders and focus rings.
pub const MIN_ACCENT_CONTRAST: f32 = 3.0;

/// WCAG contrast ratio between two colors, 1.0..=21.0.
fn contrast_ratio(a: &Rgb, b: &Rgb) -> f32 {
    let (la, lb) = (w3_luminance(a), w3_luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Choose a UI accent from the six palette accents.
///
/// cwal orders accents by cluster size, so `color5` is simply the
/// fifth-biggest region of the wallpaper — which on a dark wallpaper is often
/// a dim near-grey (measured: `#36519a` at 2.60:1). That is a legitimate
/// palette entry but useless for sliders, focus rings and active states.
///
/// Rank by contrast x saturation instead. Contrast keeps the accent legible,
/// saturation keeps it from being grey, and multiplying means neither wins
/// outright: a slightly duller color at twice the contrast beats a marginally
/// more vivid but barely-visible one. The contrast floor is dropped for the
/// fallback so a palette that cannot meet it at all still yields an accent.
pub fn pick_accent(background: &Rgb, accents: &[Rgb]) -> Rgb {
    let mut best: Option<Rgb> = None;
    let mut best_score = f32::NEG_INFINITY;
    let mut fallback = accents.first().copied().unwrap_or(*background);
    let mut fallback_score = f32::NEG_INFINITY;
    for candidate in accents {
        let contrast = contrast_ratio(candidate, background);
        let score = contrast * rgb_to_hsv(candidate).1;
        if score > fallback_score {
            fallback_score = score;
            fallback = *candidate;
        }
        if contrast < MIN_ACCENT_CONTRAST {
            continue;
        }
        if score > best_score {
            best_score = score;
            best = Some(*candidate);
        }
    }
    best.unwrap_or(fallback)
}

/// Force a light-mode background to actually be light.
///
/// cwal targets w3_luminance 0.93 via `lighten(bg, 0.93 - lum)`, but that only
/// mixes that *fraction* toward white, so the result never arrives: measured
/// light backgrounds land at luminance 0.67-0.79, i.e. mid-tones. On a
/// saturated wallpaper that is a full-screen colour field — a yellow
/// cyberpunk wallpaper produced a `#e9ef2d` background, where cwal itself
/// produced a desaturated `#e0dadd`.
///
/// Mixing with white is hue-preserving: all three channels converge on 255 at
/// the same rate, so the ratios that define hue are unchanged. That raises
/// lightness and drops tint while keeping the wallpaper's hue as a hint.
fn normalize_light_background(color: &Rgb) -> Rgb {
    let lum = w3_luminance(color);
    let tint = rgb_to_hsv(color).1;
    if lum >= LIGHT_BACKGROUND_LUMINANCE && tint <= LIGHT_BACKGROUND_MAX_TINT {
        return *color;
    }
    // Both constraints improve monotonically toward white, so one binary
    // search finds the *least* change satisfying both — keeping as much of the
    // wallpaper's character as the constraints allow.
    let mut low = 0.0f32;
    let mut high = 1.0f32;
    for _ in 0..20 {
        let mid = (low + high) / 2.0;
        let candidate = lighten(color, mid);
        if w3_luminance(&candidate) >= LIGHT_BACKGROUND_LUMINANCE
            && rgb_to_hsv(&candidate).1 <= LIGHT_BACKGROUND_MAX_TINT
        {
            high = mid;
        } else {
            low = mid;
        }
    }
    lighten(color, high)
}

/// cwal `boost_dark_colors`: lift accents from dark images (slots 1..6).
fn boost_dark(accents: &mut [Rgb]) {
    for color in accents.iter_mut() {
        let (h, mut s, mut v) = rgb_to_hsv(color);
        let mut changed = false;
        if v < 0.25 {
            v += 0.3 * (1.0 - v);
            changed = true;
        }
        if s < 0.20 {
            s += 0.4 * (1.0 - s);
            changed = true;
        }
        if changed {
            *color = hsv_to_rgb(h, s, v);
        }
    }
}

/// cwal `boost_light_colors`: tame accents from light images (slots 1..6).
fn boost_light(accents: &mut [Rgb]) {
    for color in accents.iter_mut() {
        if w3_luminance(color) > 0.6 {
            *color = darken(color, 0.3);
        }
        let (h, mut s, v) = rgb_to_hsv(color);
        if s < 0.20 {
            s += 0.4 * (1.0 - s);
            *color = hsv_to_rgb(h, s, v);
        }
    }
}

/// Quantize `path` into a 16-color scheme with cwal's derivation structure:
/// 8 Lab-space base colors ordered by cluster size, `process_colors`
/// adjustments (darken/lighten/saturate/boosts), then `generate_16_colors`.
/// Reference values were probed from cwal 0.10.1 on 2026-10-14 (see the
/// `matches_cwal_on_fixtures` test): e.g. dark gives
/// `background=color0=#1d1a1b`, `foreground=cursor=color15=#d1d1d1`.
pub fn generate(
    path: &std::path::Path,
    wallpaper_abs: &str,
    mode: &str,
) -> Result<Scheme, String> {
    let pixels = decode_pixels(path)?;
    if pixels.is_empty() {
        return Err(format!("Error: no pixels decoded from {}", path.display()));
    }
    let mut sized = kmeans_lab(&pixels, 8, 12);
    sized.sort_by(|a, b| b.1.cmp(&a.1));
    let light = mode == "light";
    // cwal always derives the background from a representative base color and
    // crushes it multiplicatively (darken 0.70); picking the *darkest* cluster
    // instead latches onto small saturated regions (pure-blue backgrounds).
    // Like cwal's dominant muted base, take the largest cluster on the
    // mode's side of the luminance split.
    let base0_index = sized
        .iter()
        .position(|(c, _)| {
            let lum = luminance(c);
            if light { lum > 128 } else { lum < 128 }
        })
        .unwrap_or(0);
    let mut base: Vec<Rgb> = sized.into_iter().map(|(c, _)| c).collect();
    base.swap(0, base0_index);
    // Six accents: the next largest clusters (cwal consumes 7 of its 8 bases,
    // discarding one; we drop the smallest).
    let mut accents: Vec<Rgb> = base.iter().skip(1).take(6).cloned().collect();

    // Note: cwal `reverse_colors` mirrors its cube-ordered bases for light
    // mode; ours are size-ordered with the mode-side dominant base already at
    // [0], so no reversal is needed.
    let (accents, background) = if light {
        boost_light(&mut accents);
        let mut background = base[0];
        // cwal light background luminance targeting. Keep the derivation, then
        // force the result to actually be a light background: cwal's own step
        // above only mixes part of the way to white, so it can stop at a
        // mid-tone and, on a saturated wallpaper, at a strongly tinted one.
        let lum = w3_luminance(&background);
        if lum < 0.88 {
            background = lighten(&background, 0.93 - lum);
        } else if lum > 0.97 {
            background = darken(&background, lum - 0.93);
        }
        (accents, normalize_light_background(&background))
    } else {
        boost_dark(&mut accents);
        let first = base[0];
        let background = if first.0 < 16 || first.1 < 16 || first.2 < 16 {
            saturate(&lighten(&first, 0.03), 0.40)
        } else {
            darken(&first, 0.70)
        };
        (accents, background)
    };

    let mut colors = [Rgb(0, 0, 0); 16];
    colors[0] = background;
    for (index, accent) in accents.iter().take(6).enumerate() {
        colors[index + 1] = *accent;
    }
    if light {
        colors[7] = saturate(&darken(&background, 0.60), 0.05);
        colors[8] = saturate(&darken(&background, 0.30), 0.10);
        colors[15] = darken(&background, 0.90);
        // cwal cols16 `darken` mode (the configured default).
        for i in 0..6 {
            colors[i + 9] = saturate(&darken(&accents[i], 0.15), 0.30);
        }
    } else {
        colors[7] = saturate(&lighten(&background, 0.60), 0.05);
        colors[8] = saturate(&lighten(&background, 0.40), 0.10);
        colors[15] = lighten(&background, 0.80);
        for i in 0..6 {
            colors[i + 9] = saturate(&lighten(&accents[i], 0.25), 0.30);
        }
    }
    // Solid-color images can collapse clusters: refill from derived shades so
    // all 16 entries stay usable.
    let mut step = 0u8;
    while colors[1..15].iter().collect::<std::collections::HashSet<_>>().len() < 8 {
        step = step.wrapping_add(13);
        let anchor = if step % 2 == 0 { background } else { colors[15] };
        colors[1 + (step as usize % 14)] = Rgb(
            anchor.0.wrapping_add(step),
            anchor.1.wrapping_add(step / 2),
            anchor.2.wrapping_sub(step / 3),
        );
        if step > 200 {
            break;
        }
    }
    let foreground = colors[15];
    let accent = pick_accent(&background, &colors[1..7]);
    Ok(Scheme {
        wallpaper: wallpaper_abs.to_string(),
        mode: mode.to_string(),
        alpha: 1.0,
        background,
        foreground,
        cursor: foreground,
        accent,
        colors,
    })
}




