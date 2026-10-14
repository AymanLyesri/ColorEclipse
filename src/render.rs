//! cwal-style template placeholder expansion.
//!
//! Known placeholders (audited against `/usr/share/cwal/templates`):
//! `{wallpaper}`, `{mode}`, `{alpha}` + `{alpha.hex,pct,int,dec}`, and for
//! each of `color0..color15`, `background`, `foreground`, `cursor` the forms
//! bare (=`#rrggbb`), `.hex`, `.xhex`, `.strip`, `.rgb`, `.rgba`, `.hexa`,
//! `.red`, `.green`, `.blue`. Shell `${VAR}` sequences are left untouched.
use crate::palette::{Rgb, Scheme};

fn hex(c: &Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

fn alpha_byte(scheme: &Scheme) -> u8 {
    (scheme.alpha.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn color_value(base: Rgb, format: &str, scheme: &Scheme) -> Option<String> {
    match format {
        "" | "hex" => Some(hex(&base)),
        "xhex" => Some(format!("0x{:02x}{:02x}{:02x}", base.0, base.1, base.2)),
        "strip" => Some(format!("{:02x}{:02x}{:02x}", base.0, base.1, base.2)),
        "rgb" => Some(format!("rgb({},{},{})", base.0, base.1, base.2)),
        "rgba" => Some(format!(
            "rgba({},{},{},{:.2})",
            base.0, base.1, base.2, scheme.alpha
        )),
        "hexa" => Some(format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            base.0,
            base.1,
            base.2,
            alpha_byte(scheme)
        )),
        "red" => Some(base.0.to_string()),
        "green" => Some(base.1.to_string()),
        "blue" => Some(base.2.to_string()),
        _ => None,
    }
}

fn base_color(role: &str, scheme: &Scheme) -> Option<Rgb> {
    if let Some(index) = role.strip_prefix("color") {
        let index: usize = index.parse().ok()?;
        return scheme.colors.get(index).copied();
    }
    match role {
        "background" => Some(scheme.background),
        "foreground" => Some(scheme.foreground),
        "cursor" => Some(scheme.cursor),
        _ => None,
    }
}

fn lookup(name: &str, scheme: &Scheme) -> Option<String> {
    match name {
        "wallpaper" => Some(scheme.wallpaper.clone()),
        "mode" => Some(scheme.mode.clone()),
        "alpha" | "alpha.dec" => Some(format!("{:.2}", scheme.alpha)),
        "alpha.hex" => Some(format!("{:02x}", alpha_byte(scheme))),
        "alpha.pct" => Some(format!("{}", (scheme.alpha * 100.0).round() as u32)),
        "alpha.int" => Some(alpha_byte(scheme).to_string()),
        _ => {
            let (role, format) = match name.split_once('.') {
                Some((role, format)) => (role, format),
                None => (name, ""),
            };
            color_value(base_color(role, scheme)?, format, scheme)
        }
    }
}

/// Expand `{placeholders}` in `template`. Unknown names and `${shell}` vars
/// pass through unchanged.
pub fn render_template(template: &str, scheme: &Scheme) -> String {
    let bytes = template.as_bytes();
    let mut out = String::with_capacity(template.len());
    let mut cursor = 0;
    while cursor < bytes.len() {
        let Some(open) = template[cursor..].find('{') else {
            out.push_str(&template[cursor..]);
            break;
        };
        let open = cursor + open;
        let Some(close_rel) = template[open..].find('}') else {
            out.push_str(&template[cursor..]);
            break;
        };
        let close = open + close_rel;
        let preceded_by_dollar = open > 0 && bytes[open - 1] == b'$';
        let name = &template[open + 1..close];
        if preceded_by_dollar {
            out.push_str(&template[cursor..=close]);
        } else if let Some(value) = lookup(name, scheme) {
            out.push_str(&template[cursor..open]);
            out.push_str(&value);
        } else {
            out.push_str(&template[cursor..=close]);
        }
        cursor = close + 1;
    }
    out
}

/// Every template cwal renders (55 entries in `/usr/share/cwal/templates`).
pub fn template_names() -> Vec<&'static str> {
    vec![
        "colors",
        "colors-alacritty.toml",
        "colors-alacritty.yml",
        "colors-btop.theme",
        "colors-cava",
        "colors.css",
        "colors-discord.css",
        "colors-dunst.conf",
        "colors.fish",
        "colors-foot.ini",
        "colors-fuzzel.ini",
        "colors-ghostty.conf",
        "Colors.hs",
        "colors-hyprland.conf",
        "colors.json",
        "colors-kitty.conf",
        "colors-konsole.colorscheme",
        "colors-leftwm-theme.ron",
        "colors-mako",
        "colors--nodim-alacritty.yml",
        "colors-nqq.css",
        "colors-nvim.lua",
        "colors-obsidian.css",
        "colors-oomox",
        "colors-polybar",
        "colors.properties",
        "colors-quickshell.qml",
        "colors-retroarch.cfg",
        "colors-rgb",
        "colors-rio.toml",
        "colors-rofi-dark.rasi",
        "colors-rofi-light.rasi",
        "colors.scss",
        "colors.sh",
        "colors-speedcrunch.json",
        "colors.styl",
        "colors-sway",
        "colors-themer.js",
        "colors-tilix.json",
        "colors-tofi",
        "colors-tty.sh",
        "colors-vscode.json",
        "colors-wal-dmenu.h",
        "colors-wal-dwl.h",
        "colors-wal-dwm.h",
        "colors-wal-dwm-urg.h",
        "colors-wal-st.h",
        "colors-wal-tabbed.h",
        "colors-wal.vim",
        "colors-waybar.css",
        "colors-wezterm.toml",
        "colors.yml",
        "colors-zathura",
        "colors-zed.json",
        "cwal",
    ]
}

#[cfg(test)]
mod tests {
    use crate::palette::{Rgb, Scheme};

    fn test_scheme() -> Scheme {
        Scheme {
            wallpaper: "/wall.jpg".to_string(),
            mode: "dark".to_string(),
            alpha: 1.0,
            background: Rgb(29, 26, 27),
            foreground: Rgb(209, 209, 209),
            cursor: Rgb(209, 209, 209),
            colors: [Rgb(29, 26, 27); 16],
        }
    }

    #[test]
    fn expands_all_placeholder_forms() {
        let s = test_scheme();
        let out = super::render_template(
            "{color0} {color0.strip} {color0.rgb} {color0.red} {alpha} {alpha.hex} {wallpaper} {mode}",
            &s,
        );
        assert!(
            out.contains("#1d1a1b")
                && out.contains("1d1a1b")
                && out.contains("rgb(29,26,27)")
        );
    }

    #[test]
    fn template_list_matches_cwal() {
        assert_eq!(super::template_names().len(), 55);
    }

    #[test]
    fn shell_vars_pass_through() {
        let s = test_scheme();
        let out = super::render_template("${LS_COLORS} $FZF_DEFAULT_OPTS {color0}", &s);
        assert!(out.contains("${LS_COLORS}"));
        assert!(out.contains("#1d1a1b"));
    }
}
