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

/// WallEclipse's config dir: `$WALLECLIPSE_CONFIG_DIR`, else
/// `~/.config/walleclipse/config` (same lookup as WallEclipse itself).
fn walleclipse_config_dir() -> PathBuf {
    match env::var("WALLECLIPSE_CONFIG_DIR") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => home().join(".config/walleclipse/config"),
    }
}

fn current_wallpaper_conf() -> PathBuf {
    walleclipse_config_dir().join("current.conf")
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

fn apply_gtk(variant: &str) -> bool {
    let name = format!("WhiteSur-{}", capitalize(variant));
    if run("gsettings", &["set", "org.gnome.desktop.interface", "gtk-theme", &name]) {
        println!("GTK theme set to {name}");
        true
    } else {
        eprintln!("Error: Failed to set GTK theme");
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
        None if current_wallpaper_conf().is_file() => expand_home(
            fs::read_to_string(current_wallpaper_conf())
                .map_err(|e| e.to_string())?
                .trim(),
        ),
        None => return Err("Error: No wallpaper specified and current.conf not found".to_string()),
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
    fn home_expansion() {
        let home = home().to_string_lossy().into_owned();
        assert_eq!(expand_home("$HOME/x.jpg"), format!("{home}/x.jpg"));
        assert_eq!(expand_home("~/x.jpg"), format!("{home}/x.jpg"));
        assert_eq!(expand_home("/abs/x.jpg"), "/abs/x.jpg");
    }

    #[test]
    fn walleclipse_conf_dir_env_override() {
        env::set_var("WALLECLIPSE_CONFIG_DIR", "/tmp/wc-test");
        assert_eq!(current_wallpaper_conf(), PathBuf::from("/tmp/wc-test/current.conf"));
        env::remove_var("WALLECLIPSE_CONFIG_DIR");
    }

    #[test]
    fn out_dir_is_coloreclipse() {
        assert_eq!(wal_out_dir(), cache_dir().join("coloreclipse"));
    }
}
