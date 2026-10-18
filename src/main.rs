//! ColorEclipse — theme variant and desktop-theme applier for ArchEclipse.
//!
//!   coloreclipse get                  print the current variant (`dark`|`light`)
//!   coloreclipse switch [dark|light]   set the variant (toggles when omitted), then re-apply
//!   coloreclipse apply                 re-apply every component for the current variant
//!   coloreclipse wal [wallpaper]       regenerate wallpaper colors (defaults to the current wallpaper)
//!
//! Flags live in `$XDG_CONFIG_HOME/coloreclipse/theme.conf` (`autocolor=`, `autovariant=`).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

mod palette;
mod render;

const DEFAULT_FLAG: bool = true; // theme.conf default for autocolor/autovariant
const CURSOR_SIZE: &str = "24";
const ANIMATED_EXTS: [&str; 10] = [
    "mp4", "gif", "webm", "mkv", "avi", "flv", "mpeg", "mp3", "ogg", "wav",
];

fn home() -> PathBuf {
    PathBuf::from(env::var("HOME").unwrap_or_default())
}

fn config_dir() -> PathBuf {
    env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home().join(".config"))
}

fn cache_dir() -> PathBuf {
    env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home().join(".cache"))
}

fn theme_conf() -> PathBuf {
    config_dir().join("coloreclipse/theme.conf")
}

/// The wallpaper the last generated palette was built from, recorded in
/// `colors.json`.
///
/// This is how we resolve "which wallpaper?" without asking anyone. Coloreclipse
/// deliberately knows nothing about whatever applies wallpapers: callers pass the
/// path explicitly (`coloreclipse wal <path>`), and we remember the last one we
/// were given so a later `switch`/`apply` recolors from the same image. Reaching
/// into another tool's config to guess would invert the dependency — they call
/// us, not the other way round.
fn last_generated_wallpaper(dir: &Path) -> Option<String> {
    let text = fs::read_to_string(dir.join("colors.json")).ok()?;
    // Written by `TEMPLATE_COLORS_JSON`, so the shape is known: the value is
    // the second quoted run after the `"wallpaper"` key. Paths containing a
    // double quote would already corrupt the file, so no unescaping is needed.
    let value = text.split_once("\"wallpaper\"")?.1.split_once('"')?.1;
    let path = &value[..value.find('"')?];
    (!path.is_empty()).then(|| path.to_string())
}

fn thumb_cache() -> PathBuf {
    cache_dir().join("wall-thumbs")
}

/// Canonical color output dir (cwal's `~/.cache/cwal` is gone; no shim).
fn wal_out_dir() -> PathBuf {
    cache_dir().join("coloreclipse")
}

/// Read a boolean flag from a theme.conf: `autocolor=false`, comments and spaces
/// tolerated, plus the legacy whole-file `true`/`false` format.
fn theme_bool_at(path: &Path, key: &str) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return DEFAULT_FLAG;
    };
    for raw in text.lines() {
        let line: String = raw
            .split(['#', ';'])
            .next()
            .unwrap_or("")
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        match line.as_str() {
            "" => continue,
            "true" => return true,
            "false" => return false,
            _ => {}
        }
        if let Some(value) = line.strip_prefix(&format!("{key}=")) {
            return match value.to_ascii_lowercase().as_str() {
                "true" => true,
                "false" => false,
                _ => DEFAULT_FLAG, // key present with a junk value
            };
        }
    }
    DEFAULT_FLAG
}

fn autocolor() -> bool {
    theme_bool_at(&theme_conf(), "autocolor")
}

fn autovariant() -> bool {
    theme_bool_at(&theme_conf(), "autovariant")
}

/// Copy the three rendered toolkit files from the cache into the config
/// locations GTK3/GTK4/qt6ct actually read, then delete the previous static
/// generator's leftover outputs. Missing cache files (no `wal` run yet) are a
/// tolerated no-op, never an error: `apply` must work before any generation.
fn install_toolkit_themes(out_dir: &Path, config_root: &Path) -> Result<(), String> {
    let installs = [
        ("colors-gtk3.css", "gtk-3.0/gtk.css"),
        ("colors-gtk4.css", "gtk-4.0/gtk.css"),
        ("colors-qt6ct.conf", "qt6ct/colors/coloreclipse.conf"),
    ];
    for (source, dest) in installs {
        let src = out_dir.join(source);
        if !src.is_file() {
            continue;
        }
        let body = fs::read(&src).map_err(|e| format!("Error: cannot read {}: {e}", src.display()))?;
        let target = config_root.join(dest);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Error: cannot create {}: {e}", parent.display()))?;
        }
        // tmp file + atomic rename, same shape render::write_all uses.
        let tmp = target.with_file_name(format!(
            ".tmp.{}.{}",
            std::process::id(),
            target.file_name().and_then(|n| n.to_str()).unwrap_or("file")
        ));
        fs::write(&tmp, &body).map_err(|e| format!("Error: cannot write {}: {e}", tmp.display()))?;
        fs::rename(&tmp, &target)
            .map_err(|e| format!("Error: cannot write {}: {e}", target.display()))?;
    }
    // noctalia.css is the previous generator's output in these two slots, now
    // replaced wholesale by our generated gtk.css. Anything else in the
    // directories is the user's and stays untouched.
    for dir in ["gtk-3.0", "gtk-4.0"] {
        let _ = fs::remove_file(config_root.join(dir).join("noctalia.css"));
    }
    Ok(())
}

/// Run a command with its output discarded (parity with the shell scripts this
/// replaced). False when the program is missing or exits non-zero.
fn run(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn stdout_of(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn notify(summary: &str, body: &str) {
    run("notify-send", &["-u", "normal", summary, body]);
}

fn current_variant() -> &'static str {
    match stdout_of("gsettings", &["get", "org.gnome.desktop.interface", "color-scheme"]) {
        Some(scheme) if scheme.contains("light") => "light",
        _ => "dark",
    }
}

fn set_color_scheme(variant: &str) -> bool {
    let scheme = if variant == "light" { "prefer-light" } else { "prefer-dark" };
    run("gsettings", &["set", "org.gnome.desktop.interface", "color-scheme", scheme])
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn apply_cursor(variant: &str) {
    let cursor = format!("phinger-cursors-{variant}");
    run("hyprctl", &["setcursor", &format!("theme_{cursor}"), CURSOR_SIZE]);
    run("gsettings", &["set", "org.gnome.desktop.interface", "cursor-theme", &cursor]);
    println!("Cursor theme set to {cursor}");
}

/// GTK/Qt theming component. The static-theme era is over: the colors come
/// from coloreclipse's rendered stylesheets and palette (installed from the
/// cache onto the config slots the toolkits read). The gsettings theme name is
/// parked on the toolkit's built-in "Adwaita", which only supplies widget
/// structure — it ships in the gtk3 package, no theme package is involved.
/// `variant` is already baked into the cached files by `wal`; this signature
/// is kept so it stays a drop-in component of `apply_components`.
fn apply_gtk(variant: &str) -> bool {
    if let Err(message) = install_toolkit_themes(&wal_out_dir(), &config_dir()) {
        eprintln!("{message}");
        return false;
    }
    if run("gsettings", &["set", "org.gnome.desktop.interface", "gtk-theme", "Adwaita"]) {
        let _ = variant;
        println!("GTK/Qt toolkit themes installed from the generated palette");
        true
    } else {
        eprintln!("Error: Failed to set GTK theme name to Adwaita");
        false
    }
}

fn apply_icon(variant: &str) -> bool {
    let name = format!("WhiteSur-{variant}");
    if run("gsettings", &["set", "org.gnome.desktop.interface", "icon-theme", &name]) {
        println!("Icon theme set to {name}");
        true
    } else {
        eprintln!("Error: Failed to set Icon theme");
        false
    }
}

/// Every component runs in parallel; a failing one never aborts the others.
fn apply_components() {
    let variant = current_variant();
    let mut switched = None;
    thread::scope(|scope| {
        scope.spawn(|| apply_cursor(variant));
        scope.spawn(|| {
            let _ = apply_gtk(variant);
        });
        scope.spawn(|| {
            let _ = apply_icon(variant);
        });
        let wal = scope.spawn(|| run_wal(None));
        match wal.join() {
            Ok(Ok(target)) => {
                if target != variant {
                    switched = Some(target);
                }
            }
            Ok(Err(message)) => eprintln!("{message}"),
            Err(_) => {}
        }
    });
    // wal may have auto-switched the variant while the sibling threads raced it
    // with the old one; re-assert the resolved variant so the theme is consistent.
    if let Some(target) = switched {
        apply_cursor(target);
        let _ = apply_gtk(target);
        let _ = apply_icon(target);
    }
    println!("All theme components applied successfully");
}

fn is_animated(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ANIMATED_EXTS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn is_newer(candidate: &Path, reference: &Path) -> bool {
    match (candidate.metadata().and_then(|m| m.modified()), reference.metadata().and_then(|m| m.modified())) {
        (Ok(candidate), Ok(reference)) => candidate > reference,
        _ => false,
    }
}

/// Variant from a wallpaper's average color; `dark` whenever the image cannot
/// be decoded (same fallback as the old ImageMagick probe).
fn variant_from_wallpaper(wallpaper: &Path) -> &'static str {
    if !wallpaper.is_file() {
        return "dark";
    }
    match palette::decode_pixels(wallpaper) {
        Ok(pixels) => palette::variant_for_pixels(&pixels),
        Err(_) => "dark",
    }
}

/// FNV-1a, names the extracted-frame cache file.
/// shortcut: keys differ from the old sha1-named thumbs, so those orphan in ~/.cache/wall-thumbs.
fn thumb_hash(input: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Extract one frame (ffmpeg) so generation has a still image to color from.
fn extract_frame(video: &str, cached: &Path) -> Result<(), String> {
    if !run("ffmpeg", &["-version"]) {
        return Err("Error: ffmpeg is required to extract a frame from animated wallpapers".to_string());
    }
    let dir = thumb_cache();
    fs::create_dir_all(&dir).map_err(|e| format!("Error: cannot create {}: {e}", dir.display()))?;
    // ffmpeg infers the format from the extension
    let tmp = dir.join(format!(".tmp.{}.jpg", std::process::id()));
    if !run("ffmpeg", &["-y", "-ss", "00:00:01", "-i", video, "-frames:v", "1", "-q:v", "2", &tmp.to_string_lossy()]) {
        let _ = fs::remove_file(&tmp);
        return Err(format!("Error: Failed to extract frame from animated wallpaper: {video}"));
    }
    fs::rename(&tmp, cached).map_err(|e| e.to_string())
}

fn expand_home(path: &str) -> String {
    let home = home();
    let home = home.to_string_lossy();
    if let Some(rest) = path.strip_prefix('~') {
        return format!("{home}{rest}");
    }
    path.replace("$HOME", &home)
}

/// Returns the variant in effect after the run (the auto-detected one when it
/// switched), so callers can re-assert a consistent theme.
fn run_wal(wallpaper_arg: Option<&str>) -> Result<&'static str, String> {
    if !autocolor() {
        println!("Auto color generation disabled in {} (autocolor=false)", theme_conf().display());
        return Ok(current_variant());
    }
    let variant = current_variant();

    let mut wallpaper = match wallpaper_arg {
        Some(path) => expand_home(path),
        None => last_generated_wallpaper(&wal_out_dir()).ok_or_else(|| {
            "Error: No wallpaper known. Pass one explicitly: coloreclipse wal <path>".to_string()
        })?,
    };

    if is_animated(&wallpaper) {
        println!("Detected animated wallpaper: {wallpaper}");
        let cached = thumb_cache().join(format!("{}.jpg", thumb_hash(&wallpaper)));
        // reuse the frame while it is still fresh (saves ~450ms of ffmpeg per switch)
        if cached.is_file() && is_newer(&cached, Path::new(&wallpaper)) {
            println!("Using cached thumbnail: {}", cached.display());
        } else {
            extract_frame(&wallpaper, &cached)?;
        }
        wallpaper = cached.to_string_lossy().into_owned();
    }

    if !Path::new(&wallpaper).is_file() {
        return Err(format!("Error: Wallpaper file not found: {wallpaper}"));
    }

    let target = if autovariant() {
        let detected = variant_from_wallpaper(Path::new(&wallpaper));
        println!("Detected wallpaper variant: {detected}");
        detected
    } else {
        variant
    };

    // Internal generation: same pywal-style files cwal wrote, now in
    // ~/.cache/coloreclipse. Bodies render in memory first, so a failure
    // keeps the last good cache instead of truncating it.
    let scheme = palette::generate(Path::new(&wallpaper), &wallpaper, target)
        .map_err(|_| "Error: Failed to generate color scheme".to_string())?;
    render::write_all(&scheme, &wal_out_dir())
        .map_err(|_| "Error: Failed to generate color scheme".to_string())?;
    if let Err(message) = install_toolkit_themes(&wal_out_dir(), &config_dir()) {
        eprintln!("{message}");
    }
    println!("Color scheme generated for {target} theme");

    // Apply the auto-switched variant without recursing back into wal.
    if target != variant {
        println!("Auto-switching theme from {variant} to {target}");
        set_color_scheme(target);
        apply_cursor(target);
        let _ = apply_gtk(target);
        let _ = apply_icon(target);
    }
    Ok(target)
}

fn switch_theme(target: Option<&str>) -> i32 {
    if autovariant() {
        println!(
            "Manual theme switching is disabled because autovariant is enabled in {}.",
            theme_conf().display()
        );
        notify("Theme Switch Disabled", "Manual switching is disabled while auto variant is enabled");
        return 0;
    }

    let target = match target {
        Some(theme) => theme.to_string(),
        None if current_variant() == "dark" => "light".to_string(),
        None => "dark".to_string(),
    };

    if target != "dark" && target != "light" {
        eprintln!("Error: Invalid theme. Use 'dark' or 'light'");
        return 1;
    }

    if !set_color_scheme(&target) {
        eprintln!("Error: Failed to set system color scheme");
        return 1;
    }
    println!("Theme switched to: {target}");

    apply_components();

    thread::sleep(Duration::from_secs(1));
    notify("Theme Changed", &format!("Current theme: {}", capitalize(&target)));
    0
}

fn usage() {
    eprintln!("Usage: coloreclipse {{get|switch|apply|wal}} [dark|light|wallpaper]");
    eprintln!("  get          - Display current theme");
    eprintln!("  switch       - Toggle theme (dark <-> light)");
    eprintln!("  switch dark  - Switch to dark theme");
    eprintln!("  switch light - Switch to light theme");
    eprintln!("  apply        - Apply current theme components (cursor, wal, gtk, icon)");
    eprintln!("  wal [path]   - Regenerate colors from a wallpaper (defaults to the current one)");
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("get") => {
            println!("{}", current_variant());
            0
        }
        Some("switch") => switch_theme(args.get(1).map(String::as_str)),
        Some("apply") => {
            apply_components();
            0
        }
        Some("wal") => match run_wal(args.get(1).map(String::as_str)) {
            Ok(_) => 0,
            Err(message) => {
                eprintln!("{message}");
                1
            }
        },
        _ => {
            usage();
            1
        }
    };
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_conf(name: &str, contents: &str) -> PathBuf {
        let path = env::temp_dir().join(format!("coloreclipse-{name}.conf"));
        fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn flag_lookup() {
        let path = temp_conf("flag", "# comment\nautocolor = false  ; trailing\nautovariant=true\n");
        assert!(!theme_bool_at(&path, "autocolor"));
        assert!(theme_bool_at(&path, "autovariant"));
        assert!(theme_bool_at(&path, "missing")); // default on
        assert!(!theme_bool_at(&temp_conf("legacy", "false"), "autocolor")); // legacy whole-file
        assert!(theme_bool_at(&temp_conf("junk", "autocolor=maybe"), "autocolor")); // junk -> default
        assert!(theme_bool_at(Path::new("/nonexistent/theme.conf"), "autocolor"));
    }

    #[test]
    fn variant_detection() {
        use crate::palette::{Rgb, variant_for_pixels};
        assert_eq!(variant_for_pixels(&[Rgb(255, 255, 255)]), "light");
        assert_eq!(variant_for_pixels(&[Rgb(0, 0, 0)]), "dark");
        assert_eq!(variant_for_pixels(&[Rgb(128, 128, 128)]), "dark"); // not > 128
        assert!(is_animated("a/b/Video.MP4"));
        assert!(!is_animated("a/b/wall.png"));
        assert_eq!(thumb_hash("x"), thumb_hash("x"));
        assert_ne!(thumb_hash("x"), thumb_hash("y"));
    }

    #[test]
    fn last_wallpaper_is_read_back_from_the_generated_palette() {
        let dir = env::temp_dir().join(format!("coloreclipse-lastwp-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // No palette yet.
        assert_eq!(last_generated_wallpaper(&dir), None);

        // The shape render.rs writes, including a path with a space.
        fs::write(dir.join("colors.json"), "{\n    \"wallpaper\": \"/tmp/some dir/wall.jpg\",\n    \"alpha\": \"1.00\"\n}\n").unwrap();
        assert_eq!(
            last_generated_wallpaper(&dir),
            Some("/tmp/some dir/wall.jpg".to_string())
        );

        // A palette written before this feature, or truncated by a crash.
        fs::write(dir.join("colors.json"), "{}\n").unwrap();
        assert_eq!(last_generated_wallpaper(&dir), None);
        fs::write(dir.join("colors.json"), "{\n    \"wallpaper\": \"\",\n").unwrap();
        assert_eq!(last_generated_wallpaper(&dir), None);
        fs::write(dir.join("colors.json"), "not json at all").unwrap();
        assert_eq!(last_generated_wallpaper(&dir), None);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn home_expansion() {
        let home = home().to_string_lossy().into_owned();
        assert_eq!(expand_home("$HOME/x.jpg"), format!("{home}/x.jpg"));
        assert_eq!(expand_home("~/x.jpg"), format!("{home}/x.jpg"));
        assert_eq!(expand_home("/abs/x.jpg"), "/abs/x.jpg");
    }

    /// `coloreclipse gtk` copies the three rendered toolkit files from the
    /// cache into the config locations GTK3/GTK4/qt6ct read, replacing
    /// whatever static theme files occupy those slots, and removes the
    /// generator's previous outputs. Missing cache files (no wal run yet)
    /// are tolerated, not an error.
    #[test]
    fn toolkit_themes_install_from_cache_and_remove_static_outputs() {
        let root = env::temp_dir().join(format!("coloreclipse-toolkit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let out = root.join("cache");
        fs::create_dir_all(&out).unwrap();
        fs::create_dir_all(root.join("config/gtk-3.0")).unwrap();
        fs::create_dir_all(root.join("config/gtk-4.0")).unwrap();
        fs::write(out.join("colors-gtk4.css"), "gtk4-content").unwrap();
        fs::write(out.join("colors-gtk3.css"), "gtk3-content").unwrap();
        fs::write(out.join("colors-qt6ct.conf"), "qt6ct-content").unwrap();
        fs::write(root.join("config/gtk-3.0/noctalia.css"), "old").unwrap();
        fs::write(root.join("config/gtk-4.0/other-theme.css"), "old").unwrap();

        super::install_toolkit_themes(&out, &root.join("config")).unwrap();

        assert_eq!(
            fs::read_to_string(root.join("config/gtk-4.0/gtk.css")).unwrap(),
            "gtk4-content"
        );
        assert_eq!(
            fs::read_to_string(root.join("config/gtk-3.0/gtk.css")).unwrap(),
            "gtk3-content"
        );
        assert_eq!(
            fs::read_to_string(root.join("config/qt6ct/colors/coloreclipse.conf")).unwrap(),
            "qt6ct-content"
        );
        assert!(!root.join("config/gtk-3.0/noctalia.css").exists());

        // Without a cache (no wal run yet), this is a no-op, not an error.
        let _ = fs::remove_dir_all(&out);
        super::install_toolkit_themes(&out, &root.join("config")).unwrap();
    }

    #[test]
    fn out_dir_is_coloreclipse() {
        assert_eq!(wal_out_dir(), cache_dir().join("coloreclipse"));
    }
}
