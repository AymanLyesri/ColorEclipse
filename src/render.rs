//! cwal-style template placeholder expansion.
//!
//! Known placeholders (audited against `/usr/share/cwal/templates`):
//! `{wallpaper}`, `{mode}`, `{alpha}` + `{alpha.hex,pct,int,dec}`, and for
//! each of `color0..color15`, `background`, `foreground`, `cursor` the forms
//! bare (=`#rrggbb`), `.hex`, `.xhex`, `.strip`, `.rgb`, `.rgba`, `.hexa`,
//! `.red`, `.green`, `.blue`. Shell `${VAR}` sequences are left untouched.
use crate::palette::{contrast_ratio, Rgb, Scheme};

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
        "accent" => Some(scheme.accent),
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
            cursor = close + 1;
        } else if let Some(value) = lookup(name, scheme) {
            out.push_str(&template[cursor..open]);
            out.push_str(&value);
            cursor = close + 1;
        } else {
            // Not a placeholder (e.g. a literal JSON/CSS brace): emit through
            // the '{' and rescan from the next byte so a later real
            // placeholder sharing this '}' still expands.
            out.push_str(&template[cursor..=open]);
            cursor = open + 1;
        }
    }
    out
}

/// Every template cwal renders (55 entries in `/usr/share/cwal/templates`)
/// plus three toolkit targets we generate ourselves: GTK3/GTK4 user
/// stylesheets and the qt6ct QPalette file.
pub fn template_names() -> Vec<&'static str> {
    vec![
        "colors-gtk3.css",
        "colors-gtk4.css",
        "colors-qt6ct.conf",
        "colors-kde.colors",
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

const TEMPLATE_COLORS_HS: &str = r#"module Colors
    ( wallpaper
    , background, foreground, cursor
    , color0, color1, color2, color3, color4, color5, color6, color7
    , color8, color9, color10, color11, color12, color13, color14, color15
    ) where

-- Shell variables
-- Generated by 'wal'
wallpaper="{wallpaper}"

-- Special
background="{background}"
foreground="{foreground}"
cursor="{cursor}"

-- Colors
color0="{color0}"
color1="{color1}"
color2="{color2}"
color3="{color3}"
color4="{color4}"
color5="{color5}"
color6="{color6}"
color7="{color7}"
color8="{color8}"
color9="{color9}"
color10="{color10}"
color11="{color11}"
color12="{color12}"
color13="{color13}"
color14="{color14}"
color15="{color15}"
"#;
const TEMPLATE_COLORS: &str = r#"{color0}
{color1}
{color2}
{color3}
{color4}
{color5}
{color6}
{color7}
{color8}
{color9}
{color10}
{color11}
{color12}
{color13}
{color14}
{color15}
"#;
const TEMPLATE_COLORS_NODIM_ALACRITTY_YML: &str = r#"colors:

  primary:
    background: "{background}"
    foreground: "{foreground}"

  cursor:
    text: "{background}"
    cursor: "{foreground}"

  vi_mode_cursor:
    text: "{background}"
    cursor: "{foreground}"

  search:
    matches:
      foreground: '{color0}'
      background: '{color15}'
    focused_match:
      foreground: "{background}"
      background: "{color1}"

  line_indicator:
    foreground: None
    background: None

  footer_bar:
    foreground: '{color8}'
    background: '{color7}'

  selection:
    text: "{background}"
    background: "{color1}"

  normal:
    black: "{color0}"
    red: "{color1}"
    green: "{color2}"
    yellow: "{color3}"
    blue: "{color4}"
    magenta: "{color5}"
    cyan: "{color6}"
    white: "{color7}"

  bright:
    black: "{color8}"
    red: "{color9}"
    green: "{color10}"
    yellow: "{color11}"
    blue: "{color12}"
    magenta: "{color13}"
    cyan: "{color14}"
    white: "{color15}"
"#;
const TEMPLATE_COLORS_ALACRITTY_TOML: &str = r#"[colors.primary]
background = "{background}"
foreground = "{foreground}"

[colors.cursor]
text = "{background}"
cursor = "{foreground}"

[colors.vi_mode_cursor]
text = "{background}"
cursor = "{foreground}"

[colors.search.matches]
foreground = "{color0}"
background = "{color15}"

[colors.search.focused_match]
foreground = "{background}"
background = "{color1}"

[colors.line_indicator]
foreground = "None"
background = "None"

[colors.footer_bar]
foreground = "{color8}"
background = "{color7}"

[colors.selection]
text = "{background}"
background = "{color1}"

[colors.normal]
black = "{color0}"
red = "{color1}"
green = "{color2}"
yellow = "{color3}"
blue = "{color4}"
magenta = "{color5}"
cyan = "{color6}"
white = "{color7}"

[colors.bright]
black = "{color8}"
red = "{color9}"
green = "{color10}"
yellow = "{color11}"
blue = "{color12}"
magenta = "{color13}"
cyan = "{color14}"
white = "{color15}"

[colors.dim]
black = "{color15}"
red = "{color9}"
green = "{color13}"
yellow = "{color11}"
blue = "{color12}"
magenta = "{color10}"
cyan = "{color14}"
white = "{color8}"

"#;
const TEMPLATE_COLORS_ALACRITTY_YML: &str = r#"colors:

  primary:
    background: "{background}"
    foreground: "{foreground}"

  cursor:
    text: "{background}"
    cursor: "{foreground}"

  vi_mode_cursor:
    text: "{background}"
    cursor: "{foreground}"

  search:
    matches:
      foreground: '{color0}'
      background: '{color15}'
    focused_match:
      foreground: "{background}"
      background: "{color1}"

  line_indicator:
    foreground: None
    background: None

  footer_bar:
    foreground: '{color8}'
    background: '{color7}'

  selection:
    text: "{background}"
    background: "{color1}"

  normal:
    black: "{color0}"
    red: "{color1}"
    green: "{color2}"
    yellow: "{color3}"
    blue: "{color4}"
    magenta: "{color5}"
    cyan: "{color6}"
    white: "{color7}"

  bright:
    black: "{color8}"
    red: "{color9}"
    green: "{color10}"
    yellow: "{color11}"
    blue: "{color12}"
    magenta: "{color13}"
    cyan: "{color14}"
    white: "{color15}"

  dim:
    black: "{color15}"
    red: "{color9}"
    green: "{color13}"
    yellow: "{color11}"
    blue: "{color12}"
    magenta: "{color10}"
    cyan: "{color14}"
    white: "{color8}"
"#;
const TEMPLATE_COLORS_BTOP_THEME: &str = r#"theme[main_bg]="{background}"
theme[main_fg]="{foreground}"
theme[title]="{foreground}"
theme[hi_fg]="{color1}"
theme[selected_bg]="{color1}"
theme[selected_fg]="{foreground}"
theme[inactive_fg]="{color8}"
theme[graph_text]="{foreground}"
theme[meter_bg]="{color8}"
theme[proc_misc]="{color1}"
theme[cpu_box]="{color1}"
theme[mem_box]="{color2}"
theme[net_box]="{color3}"
theme[proc_box]="{color4}"
theme[div_line]="{color8}"
theme[temp_start]="{color2}"
theme[temp_mid]="{color3}"
theme[temp_end]="{color1}"
theme[cpu_start]="{color2}"
theme[cpu_mid]="{color3}"
theme[cpu_end]="{color1}"
theme[free_start]="{color2}"
theme[free_mid]="{color3}"
theme[free_end]="{color1}"
theme[cached_start]="{color2}"
theme[cached_mid]="{color3}"
theme[cached_end]="{color1}"
theme[available_start]="{color2}"
theme[available_mid]="{color3}"
theme[available_end]="{color1}"
theme[used_start]="{color2}"
theme[used_mid]="{color3}"
theme[used_end]="{color1}"
theme[download_start]="{color2}"
theme[download_mid]="{color3}"
theme[download_end]="{color1}"
theme[upload_start]="{color2}"
theme[upload_mid]="{color3}"
theme[upload_end]="{color1}"
"#;
const TEMPLATE_COLORS_CAVA: &str = r#"[color]
background = '{background}'
foreground = '{foreground}'

gradient = 1

gradient_color_1 = '{color1}'
gradient_color_2 = '{color2}'
gradient_color_3 = '{color3}'
gradient_color_4 = '{color4}'
gradient_color_5 = '{color5}'
gradient_color_6 = '{color6}'
gradient_color_7 = '{color7}'
gradient_color_8 = '{color8}'
"#;
const TEMPLATE_COLORS_DISCORD_CSS: &str = r##"* {
    /* font-family: "JetBrains Mono"; */
    /* font-style: italic; */

    /* *.background */
    --background-primary: {background};
    --background-secondary: {background};
    --background-tertiary: {background};

    --background-primary-alt: {background};
    --background-secondary-alt: {background};
    --background-tertiary-alt: {background};

    --channeltextarea-background: {background};
    --custom-channel-members-bg: {background};

    --profile-gradient-primary-color: {background};
    --profile-gradient-secondary-color: {background};

    --__header-bar-background: {background} !important;

    /* -- themes update -- */
    --bg-base-tertiary: {background};

    /* slightly darker background - useful for separating */
    --card-primary-bg: color-mix(in srgb, {background}, black 20%);
    --input-background: color-mix(in srgb, {background}, black 20%);
    --autocomplete-bg: color-mix(in srgb, {background}, black 20%);
    --background-nested-floating: color-mix(in srgb, {background}, black 20%);
    --background-floating: color-mix(in srgb, {background}, black 20%);
    --scrollbar-auto-track: color-mix(in srgb, {background}, black 20%);
    --scrollbar-thin-track: color-mix(in srgb, {background}, black 20%);

    /* -- themes update -- */
    --border-subtle: color-mix(in srgb, {background}, black 20%);
    --background-base-lowest: color-mix(in srgb, {background}, black 20%);
    --background-surface-high: color-mix(in srgb, {background}, black 20%);

    --button-secondary-background: color-mix(in srgb, {background}, black 30%);
    --background-surface-higher: color-mix(in srgb, {background}, black 30%);
    --background-base-lower: color-mix(in srgb, {background}, black 35%);

    --background-message-hover: color-mix(in srgb, {background}, black 40%);
    --button-secondary-background-hover: color-mix(in srgb, {background}, black 40%);
    --background-base-low: color-mix(in srgb, {background}, black 40%);
    --background-surface-highest: color-mix(in srgb, {background}, black 40%);
    --chat-background-default: color-mix(in srgb, {background}, black 45%);
    
    --button-secondary-background-active: color-mix(in srgb, {background}, black 60%);

    /* primary.* */
    --primary-630: {background};

    /* *.color2 - secondary elements */
    --scrollbar-auto-thumb: {color2};
    --scrollbar-thin-thumb: {color2};
    --interactive-muted: {color2};
    --text-muted: {color2};
    --background-modifier-hover: color-mix(in srgb, {color2}, black 40%);
    --background-modifier-active: color-mix(in srgb, {color2}, black 20%);
    --background-modifier-accent: {color2};
    --background-accent: {color2};

    /* -- themes update -- */
    --input-border: {color2};
    --border-normal: {color2};
    --icon-secondary: {color2};
    --icon-tertiary: {color2};
    --channel-icon: {color2};
    --channels-default: {color2};
    --header-primary: {color2};
    --__lottieIconColor: {color2};
    --interactive-normal: {color2};

    /* *.color3 */
    --red-400: {color3};
    --background-modifier-selected: {color3};

    /* -- themes update -- */
    --notice-background-positive: color-mix(in srgb, {color3}, black 75%);
    --notice-text-positive: {color3};

    /* *.color4 - main theme color: flashy, important elements */
    --status-danger: {color4};
    --button-outline-danger-border: {color4};
    --button-outline-danger-text: {color4};
    --button-danger-background: {color4};

    --yellow-300: {color4};

    /* *.color5 - light theme color */
    --brand-experiment: {color5};
    --brand-experiment-360: {color5};
    --brand-experiment-500: {color5};
    --profile-gradient-button-color: {color5};

    --green-360: {color5};

    /* *.color6 - lighter theme color */
    /* -!- currently unused -!- */

    /* *.color7 - very light, text and foreground */
    --text-normal: {color7};
    --interactive-active: {color7};
}

/* status indicators */
rect[fill="#d83a41"] {
    fill: {color3} !important;
}

rect[fill="#cc954c"] {
    fill: {color4} !important;
}

rect[fill="#40a258"] {
    fill: {color5} !important;
}
"##;
const TEMPLATE_COLORS_DUNST_CONF: &str = r#"[urgency_low]
    background = "{color14}"
    foreground = "{color0}"

[urgency_normal]
    background = "{color6}"
    foreground = "{color0}"

[urgency_critical]
    background = "{color1}"
    foreground = "{color15}"
"#;
const TEMPLATE_COLORS_FOOT_INI: &str = r#"[colors-{mode}]
background={background.strip}
foreground={foreground.strip}
regular0={color0.strip}
regular1={color1.strip}
regular2={color2.strip}
regular3={color3.strip}
regular4={color4.strip}
regular5={color5.strip}
regular6={color6.strip}
regular7={color7.strip}
bright0={color8.strip}
bright1={color9.strip}
bright2={color10.strip}
bright3={color11.strip}
bright4={color12.strip}
bright5={color13.strip}
bright6={color14.strip}
bright7={color15.strip}
"#;
const TEMPLATE_COLORS_FUZZEL_INI: &str = r#"[colors]
background={background.strip}ff
prompt={foreground.strip}ff
text={foreground.strip}ff
placeholder={foreground.strip}ff
input={foreground.strip}ff
match={foreground.strip}ff
selection={color1.strip}ff
selection-text={background.strip}ff
selection-match={foreground.strip}ff
border={color1.strip}ff
"#;
const TEMPLATE_COLORS_GHOSTTY_CONF: &str = r#"background = {background}
foreground = {foreground}
selection-background = {color1}
selection-foreground = {foreground}

palette = 0={color0}
palette = 1={color1}
palette = 2={color2}
palette = 3={color3}
palette = 4={color4}
palette = 5={color5}
palette = 6={color6}
palette = 7={color7}
palette = 8={color8}
palette = 9={color9}
palette = 10={color10}
palette = 11={color11}
palette = 12={color12}
palette = 13={color13}
palette = 14={color14}
palette = 15={color15}
"#;
const TEMPLATE_COLORS_HYPRLAND_CONF: &str = r#"$wallpaper = {wallpaper}

$foreground = {foreground.rgb}
$background = {background.rgb}

$color0 = {color0.rgb}
$color1 = {color1.rgb}
$color2 = {color2.rgb}
$color3 = {color3.rgb}
$color4 = {color4.rgb}
$color5 = {color5.rgb}
$color6 = {color6.rgb}
$color7 = {color7.rgb}
$color8 = {color8.rgb}
$color9 = {color9.rgb}
$color10 = {color10.rgb}
$color11 = {color11.rgb}
$color12 = {color12.rgb}
$color13 = {color13.rgb}
$color14 = {color14.rgb}
$color15 = {color15.rgb}
"#;
const TEMPLATE_COLORS_KITTY_CONF: &str = r#"foreground         {foreground}
background         {background}
background_opacity {alpha}

active_tab_foreground     {background}
active_tab_background     {foreground}
inactive_tab_foreground   {foreground}
inactive_tab_background   {background}

active_border_color   {foreground}
inactive_border_color {background}
bell_border_color     {color1}

color0       {color0}
color8       {color8}
color1       {color1}
color9       {color9}
color2       {color2}
color10      {color10}
color3       {color3}
color11      {color11}
color4       {color4}
color12      {color12}
color5       {color5}
color13      {color13}
color6       {color6}
color14      {color14}
color7       {color7}
color15      {color15}
"#;
const TEMPLATE_COLORS_KONSOLE_COLORSCHEME: &str = r#"[Background]
Color={background.rgb}

[BackgroundIntense]
Color={background.rgb}

[Color0]
Color={color0.rgb}

[Color0Intense]
Color={color8.rgb}

[Color1]
Color={color1.rgb}

[Color1Intense]
Color={color9.rgb}

[Color2]
Color={color2.rgb}

[Color2Intense]
Color={color10.rgb}

[Color3]
Color={color3.rgb}

[Color3Intense]
Color={color11.rgb}

[Color4]
Color={color4.rgb}

[Color4Intense]
Color={color12.rgb}

[Color5]
Color={color5.rgb}

[Color5Intense]
Color={color13.rgb}

[Color6]
Color={color6.rgb}

[Color6Intense]
Color={color14.rgb}

[Color7]
Color={color7.rgb}

[Color7Intense]
Color={color15.rgb}

[Foreground]
Color={foreground.rgb}

[ForegroundIntense]
Color={foreground.rgb}

[General]
Description=Colorscheme generated by wal
Opacity=1
"#;
const TEMPLATE_COLORS_LEFTWM_THEME_RON: &str = r#"#![enable(implicit_some)]
(
    border_width: 2,
    margin: [4, 4],
    default_border_color: "{background}",
    floating_border_color: "{color6}",
    focused_border_color: "{foreground}",
    background_color: "{background}",
)"#;
const TEMPLATE_COLORS_MAKO: &str = r#"background-color={color6}
text-color={background}
border-color={color14}
progress-color={color14}

[urgency=low]
background-color={color14}
text-color={background}
border-color={color6}
progress-color={color6}

[urgency=critical]
background-color={color1}
text-color={foreground}
border-color={color9}
progress-color={color9}
"#;
const TEMPLATE_COLORS_NQQ_CSS: &str = r#".cm-s-nqqwal .CodeMirror-gutters { background: {background} !important; }
.cm-s-nqqwal .CodeMirror-foldgutter-open, .CodeMirror-foldgutter-folded { color: {foreground}; }
.cm-s-nqqwal .CodeMirror-cursor { border-left: 1px solid {foreground}; background-color: rgba({color6.rgb},0.4)}
.cm-s-nqqwal { background-color: {background}; color: {foreground}; }
.cm-s-nqqwal span.cm-builtin { color: {color2}; font-weight: bold; }
.cm-s-nqqwal span.cm-comment { color: {color8}; }
.cm-s-nqqwal span.cm-keyword { color: {color3}; font-weight: bold; }
.cm-s-nqqwal span.cm-atom { color: {color4}; }
.cm-s-nqqwal span.cm-def { color: {color5}; }
.cm-s-nqqwal span.cm-variable { color: {color6}; }
.cm-s-nqqwal span.cm-variable-2 { color: {color5}; }
.cm-s-nqqwal span.cm-string { color: {color2}; }
.cm-s-nqqwal span.cm-string-2 { color: {color2}; }
.cm-s-nqqwal span.cm-number { color: {color5}; }
.cm-s-nqqwal span.cm-tag { color: {color3}; }
.cm-s-nqqwal span.cm-property { color: {color6}; }
.cm-s-nqqwal span.cm-attribute { color: {color6}; }
.cm-s-nqqwal span.cm-qualifier { color: {color4}; }
.cm-s-nqqwal span.cm-meta { color: {color5}; }
.cm-s-nqqwal span.cm-header { color: {color3}; }
.cm-s-nqqwal span.cm-operator { color: {color3}; }
.cm-s-nqqwal span.CodeMirror-matchingbracket { box-sizing: border-box; background: transparent; border-bottom: 1px solid; }
.cm-s-nqqwal span.CodeMirror-nonmatchingbracket { border-bottom: 1px solid; background: none; }
.cm-s-nqqwal .CodeMirror-activeline-background { background: rgba({color5.rgb},0.4); }
.cm-s-nqqwal div.CodeMirror-selected { background: {color3}; }
.cm-s-nqqwal .CodeMirror-focused div.CodeMirror-selected { background: {color0}); }
"#;
const TEMPLATE_COLORS_NVIM_LUA: &str = r#"return {
    wallpaper = "{wallpaper}",

    background = "{background}",
    foreground = "{foreground}",
    cursor = "{cursor}",

    color0 = "{color0}",
    color1 = "{color1}",
    color2 = "{color2}",
    color3 = "{color3}",
    color4 = "{color4}",
    color5 = "{color5}",
    color6 = "{color6}",
    color7 = "{color7}",
    color8 = "{color8}",
    color9 = "{color9}",
    color10 = "{color10}",
    color11 = "{color11}",
    color12 = "{color12}",
    color13 = "{color13}",
    color14 = "{color14}",
    color15 = "{color15}",
}
"#;
const TEMPLATE_COLORS_OBSIDIAN_CSS: &str = r#".theme-dark {

--background-primary: {background};
--background-primary-alt: {background};
--background-secondary: {color0};
--background-secondary-alt: {color0};

--background-primary-rgb: {background.rgb};
--background-secondary-rgb: {background.rgb};

--text-normal: {foreground};
--text-muted: {color7};
--text-faint: {color8};

--interactive-accent: {color4};
--interactive-accent-hover: {color5};
--interactive-accent-rgb: {color4.rgb};

--text-accent: {color4};
--text-accent-rgb: {color4.rgb};

--divider-color: {color8};

--background-modifier-border: {color8}33;
--background-modifier-border-hover: {color4}44;
--background-modifier-active-hover: {color4}22;

--background-modifier-form-field: {color0};

--text-selection: {color4}44;

--code-normal: {color3};
--code-background: {color0};

--link-color: {color4};
--link-color-hover: {color6};

--tag-color: {color5};
--tag-background: {color5}22;

--h1-color: {color4};
--h2-color: {color5};
--h3-color: {color6};
--h4-color: {color7};

--scrollbar-bg: {background};
--scrollbar-thumb-bg: {color8};
--scrollbar-active-thumb-bg: {color4};

--caret-color: {color4};

}
"#;
const TEMPLATE_COLORS_OOMOX: &str = r#"NAME=cwal
BG={color0.strip}
FG={color15.strip}
MENU_BG={color0.strip}
MENU_FG={color15.strip}
SEL_BG={color1.strip}
SEL_FG={color0.strip}
TXT_BG={color0.strip}
TXT_FG={color15.strip}
BTN_BG={color2.strip}
BTN_FG={color15.strip}
HDR_BTN_BG={color3.strip}
HDR_BTN_FG={color15.strip}
GTK3_GENERATE_DARK=True
ROUNDNESS=0
SPACING=3
GRADIENT=0.0
"#;
const TEMPLATE_COLORS_POLYBAR: &str = r#"background = d9{color0.strip}
background-alt = {color8.strip}
foreground = {color7.strip}
foreground-alt = {color7.strip}
primary = {color1.strip}
secondary = {color2.strip}
alert = {color3.strip}
disabled = {color15.strip}
"#;
const TEMPLATE_COLORS_QUICKSHELL_QML: &str = r#"pragma Singleton

import Quickshell
import QtQuick

Singleton {
  readonly property string wallpaper: "{wallpaper}"

  readonly property color background: "{background}"
  readonly property color foreground: "{foreground}"
  readonly property color cursor: "{cursor}"

  readonly property color color0: "{color0}"
  readonly property color color1: "{color1}"
  readonly property color color2: "{color2}"
  readonly property color color3: "{color3}"
  readonly property color color4: "{color4}"
  readonly property color color5: "{color5}"
  readonly property color color6: "{color6}"
  readonly property color color7: "{color7}"
  readonly property color color8: "{color8}"
  readonly property color color9: "{color9}"
  readonly property color color10: "{color10}"
  readonly property color color11: "{color11}"
  readonly property color color12: "{color12}"
  readonly property color color13: "{color13}"
  readonly property color color14: "{color14}"
  readonly property color color15: "{color15}"
}
"#;
const TEMPLATE_COLORS_RETROARCH_CFG: &str = r#"rgui_entry_normal_color = "0xFF{color9.strip}"
rgui_entry_hover_color = "0xFF{color14.strip}"
rgui_title_color = "0xFF{color15.strip}"
rgui_bg_dark_color = "0xFF{background.strip}"
rgui_bg_light_color = "0xFF{background.strip}"
rgui_border_dark_color = "0xFF{color8.strip}"
rgui_border_light_color = "0xFF{color8.strip}"
rgui_particle_color = "0xFF{color4.strip}"
"#;
const TEMPLATE_COLORS_RGB: &str = r#"{color0.rgb}
{color1.rgb}
{color2.rgb}
{color3.rgb}
{color4.rgb}
{color5.rgb}
{color6.rgb}
{color7.rgb}
{color8.rgb}
{color9.rgb}
{color10.rgb}
{color11.rgb}
{color12.rgb}
{color13.rgb}
{color14.rgb}
{color15.rgb}
"#;
const TEMPLATE_COLORS_RIO_TOML: &str = r#"[colors]
background = '{background}8e'
black = '{color0}'
blue = '{color1}'
cursor = '{cursor}'
cyan = '{color2}'
foreground = '{foreground}'
green = '{color3}'
magenta = '{color4}'
red = '{color5}'
tabs = '{foreground}'
tabs-active = '{color6}'
white = '{foreground}'
yellow = '{color7}'
dim-black = '{color0}8e'
dim-blue = '{color1}8e'
dim-cyan = '{color2}8e'
dim-foreground = '{foreground}8e'
dim-green = '{color3}8e'
dim-magenta = '{color4}8e'
dim-red = '{color5}8e'
dim-white = '{color6}8e'
dim-yellow = '{color7}8e'
light-black = '{color9}'
light-blue = '{color10}'
light-cyan = '{color11}'
light-foreground = '{color12}'
light-green = '{color13}'
light-magenta = '{color14}'
light-red = '{color15}'
light-white = '{color9}'
light-yellow = '{color10}'
"#;
const TEMPLATE_COLORS_ROFI_DARK_RASI: &str = r#"* {
    active-background: {color2};
    active-foreground: @foreground;
    normal-background: @background;
    normal-foreground: @foreground;
    urgent-background: {color1};
    urgent-foreground: @foreground;

    alternate-active-background: @background;
    alternate-active-foreground: @foreground;
    alternate-normal-background: @background;
    alternate-normal-foreground: @foreground;
    alternate-urgent-background: @background;
    alternate-urgent-foreground: @foreground;

    selected-active-background: {color1};
    selected-active-foreground: @foreground;
    selected-normal-background: {color2};
    selected-normal-foreground: @foreground;
    selected-urgent-background: {color3};
    selected-urgent-foreground: @foreground;

    background-color: @background;
    background: {background};
    foreground: {foreground};
    border-color: @background;
    spacing: 2;
}

#window {
    background-color: @background;
    border: 0;
    padding: 2.5ch;
}

#mainbox {
    border: 0;
    padding: 0;
}

#message {
    border: 2px 0px 0px;
    border-color: @border-color;
    padding: 1px;
}

#textbox {
    text-color: @foreground;
}

#inputbar {
    children:   [ prompt,textbox-prompt-colon,entry,case-indicator ];
}

#textbox-prompt-colon {
    expand: false;
    str: ":";
    margin: 0px 0.3em 0em 0em;
    text-color: @normal-foreground;
}

#listview {
    fixed-height: 0;
    border: 2px 0px 0px;
    border-color: @border-color;
    spacing: 2px;
    scrollbar: true;
    padding: 2px 0px 0px;
}

#element {
    border: 0;
    padding: 1px;
}

#element-text, element-icon {
    background-color: inherit;
    text-color:       inherit;
}

#element.normal.normal {
    background-color: @normal-background;
    text-color: @normal-foreground;
}

#element.normal.urgent {
    background-color: @urgent-background;
    text-color: @urgent-foreground;
}

#element.normal.active {
    background-color: @active-background;
    text-color: @active-foreground;
}

#element.selected.normal {
    background-color: @selected-normal-background;
    text-color: @selected-normal-foreground;
}

#element.selected.urgent {
    background-color: @selected-urgent-background;
    text-color: @selected-urgent-foreground;
}

#element.selected.active {
    background-color: @selected-active-background;
    text-color: @selected-active-foreground;
}

#element.alternate.normal {
    background-color: @alternate-normal-background;
    text-color: @alternate-normal-foreground;
}

#element.alternate.urgent {
    background-color: @alternate-urgent-background;
    text-color: @alternate-urgent-foreground;
}

#element.alternate.active {
    background-color: @alternate-active-background;
    text-color: @alternate-active-foreground;
}

#scrollbar {
    width: 4px;
    border: 0;
    handle-width: 8px;
    padding: 0;
}

#sidebar {
    border: 2px 0px 0px;
    border-color: @border-color;
}

#button {
    text-color: @normal-foreground;
}

#button.selected {
    background-color: @selected-normal-background;
    text-color: @selected-normal-foreground;
}

#inputbar {
    spacing: 0;
    text-color: @normal-foreground;
    padding: 1px;
}

#case-indicator {
    spacing: 0;
    text-color: @normal-foreground;
}

#entry {
    spacing: 0;
    text-color: @normal-foreground;
}

#prompt {
    spacing: 0;
    text-color: @normal-foreground;
}
"#;
const TEMPLATE_COLORS_ROFI_LIGHT_RASI: &str = r#"* {
    active-background: {color2};
    active-foreground: @foreground;
    normal-background: @background;
    normal-foreground: @foreground;
    urgent-background: {color1};
    urgent-foreground: @foreground;

    alternate-active-background: @background;
    alternate-active-foreground: @foreground;
    alternate-normal-background: @background;
    alternate-normal-foreground: @foreground;
    alternate-urgent-background: @background;
    alternate-urgent-foreground: @foreground;

    selected-active-background: {color1};
    selected-active-foreground: @foreground;
    selected-normal-background: {color2};
    selected-normal-foreground: @foreground;
    selected-urgent-background: {color3};
    selected-urgent-foreground: @foreground;

    background-color: @background;
    background: {foreground};
    foreground: {background};
    border-color: @background;
    spacing: 2;
}

#window {
    background-color: @background;
    border: 0;
    padding: 2.5ch;
}

#mainbox {
    border: 0;
    padding: 0;
}

#message {
    border: 2px 0px 0px;
    border-color: @border-color;
    padding: 1px;
}

#textbox {
    text-color: @foreground;
}

#inputbar {
    children:   [ prompt,textbox-prompt-colon,entry,case-indicator ];
}

#textbox-prompt-colon {
    expand: false;
    str: ":";
    margin: 0px 0.3em 0em 0em;
    text-color: @normal-foreground;
}

#listview {
    fixed-height: 0;
    border: 2px 0px 0px;
    border-color: @border-color;
    spacing: 2px;
    scrollbar: true;
    padding: 2px 0px 0px;
}

#element {
    border: 0;
    padding: 1px;
}

#element-text, element-icon {
    background-color: inherit;
    text-color:       inherit;
}

#element.normal.normal {
    background-color: @normal-background;
    text-color: @normal-foreground;
}

#element.normal.urgent {
    background-color: @urgent-background;
    text-color: @urgent-foreground;
}

#element.normal.active {
    background-color: @active-background;
    text-color: @active-foreground;
}

#element.selected.normal {
    background-color: @selected-normal-background;
    text-color: @selected-normal-foreground;
}

#element.selected.urgent {
    background-color: @selected-urgent-background;
    text-color: @selected-urgent-foreground;
}

#element.selected.active {
    background-color: @selected-active-background;
    text-color: @selected-active-foreground;
}

#element.alternate.normal {
    background-color: @alternate-normal-background;
    text-color: @alternate-normal-foreground;
}

#element.alternate.urgent {
    background-color: @alternate-urgent-background;
    text-color: @alternate-urgent-foreground;
}

#element.alternate.active {
    background-color: @alternate-active-background;
    text-color: @alternate-active-foreground;
}

#scrollbar {
    width: 4px;
    border: 0;
    handle-width: 8px;
    padding: 0;
}

#sidebar {
    border: 2px 0px 0px;
    border-color: @border-color;
}

#button {
    text-color: @normal-foreground;
}

#button.selected {
    background-color: @selected-normal-background;
    text-color: @selected-normal-foreground;
}

#inputbar {
    spacing: 0;
    text-color: @normal-foreground;
    padding: 1px;
}

#case-indicator {
    spacing: 0;
    text-color: @normal-foreground;
}

#entry {
    spacing: 0;
    text-color: @normal-foreground;
}

#prompt {
    spacing: 0;
    text-color: @normal-foreground;
}
"#;
const TEMPLATE_COLORS_SPEEDCRUNCH_JSON: &str = r#"{
    "cursor": "{cursor}",
    "number": "{foreground}",
    "parens": "{color13}",
    "result": "{color12}",
    "comment": "{color8}",
    "matched": "{color4}",
    "function": "{color1}",
    "operator": "{color3}",
    "variable": "{color2}",
    "scrollbar": "{color3}",
    "separator": "{color3}",
    "background": "{background}",
    "editorbackground": "{background}"
}
"#;
const TEMPLATE_COLORS_SWAY: &str = r#"set $wallpaper {wallpaper}

set $background {background}
set $foreground {foreground}

set $color0 {color0}
set $color1 {color1}
set $color2 {color2}
set $color3 {color3}
set $color4 {color4}
set $color5 {color5}
set $color6 {color6}
set $color7 {color7}
set $color8 {color8}
set $color9 {color9}
set $color10 {color10}
set $color11 {color11}
set $color12 {color12}
set $color13 {color13}
set $color14 {color14}
set $color15 {color15}
"#;
const TEMPLATE_COLORS_THEMER_JS: &str = r#"module.exports.colors = {
  dark: {
    accent0: "{color0}",
    accent1: "{color1}",
    accent2: "{color2}",
    accent3: "{color3}",
    accent4: "{color4}",
    accent5: "{color5}",
    accent6: "{cursor}",
    accent7: "{color7}",
    shade0:  "{background}",
    shade1:  "{color9}",
    shade2:  "{color10}",
    shade3:  "{color11}",
    shade4:  "{color12}",
    shade5:  "{color13}",
    shade6:  "{color14}",
    shade7:  "{foreground}"
  },
};
"#;
const TEMPLATE_COLORS_TILIX_JSON: &str = r#"{
    "background-color": "{background}",
    "badge-color": "{foreground}",
    "bold-color": "{foreground}",
    "comment": "",
    "cursor-background-color": "{background}",
    "cursor-foreground-color": "{cursor}",
    "foreground-color": "{foreground}",
    "highlight-background-color": "{background}",
    "highlight-foreground-color": "{foreground}",
    "name": "wal export",
    "palette": [
        "{color0}",
        "{color1}",
        "{color2}",
        "{color3}",
        "{color4}",
        "{color5}",
        "{color6}",
        "{color7}",
        "{color8}",
        "{color9}",
        "{color10}",
        "{color11}",
        "{color12}",
        "{color13}",
        "{color14}",
        "{color15}"
    ],
    "use-badge-color": false,
    "use-bold-color": false,
    "use-cursor-color": false,
    "use-highlight-color": false,
    "use-theme-colors": false
}
"#;
const TEMPLATE_COLORS_TOFI: &str = r#"background-color = {background.strip}
border-color = {color1.strip}
selection-color = {color1.strip}
text-color = {foreground.strip}
"#;
const TEMPLATE_COLORS_TTY_SH: &str = r#"#!/bin/sh
[ "${TERM:-none}" = "linux" ] && \
    printf '%b' '\e]P0{color0.strip}
                 \e]P1{color1.strip}
                 \e]P2{color2.strip}
                 \e]P3{color3.strip}
                 \e]P4{color4.strip}
                 \e]P5{color5.strip}
                 \e]P6{color6.strip}
                 \e]P7{color7.strip}
                 \e]P8{color8.strip}
                 \e]P9{color9.strip}
                 \e]PA{color10.strip}
                 \e]PB{color11.strip}
                 \e]PC{color12.strip}
                 \e]PD{color13.strip}
                 \e]PE{color14.strip}
                 \e]PF{color15.strip}
                 \ec'
"#;
const TEMPLATE_COLORS_VSCODE_JSON: &str = r#"{
  "$schema": "vscode://schemas/color-theme",
  "name": "cwal",
  "type": "{mode}",
  "colors": {
    "focusBorder": "{color12}",
    "foreground": "{foreground}",
    "descriptionForeground": "{color7}",
    "errorForeground": "{color9}",
    "textLink.foreground": "{color12}",
    "textLink.activeForeground": "{color14}",
    "editor.background": "{background}",
    "editor.foreground": "{foreground}",
    "editorCursor.foreground": "{cursor}",
    "editorLineNumber.foreground": "{color8}",
    "editorLineNumber.activeForeground": "{color12}",
    "editor.selectionBackground": "{color8}66",
    "editor.inactiveSelectionBackground": "{color8}44",
    "editor.selectionHighlightBackground": "{color4}33",
    "editor.wordHighlightBackground": "{color4}22",
    "editor.wordHighlightStrongBackground": "{color10}22",
    "editor.findMatchBackground": "{color11}88",
    "editor.findMatchHighlightBackground": "{color11}44",
    "editor.lineHighlightBackground": "{color0}",
    "editorIndentGuide.background1": "{color8}33",
    "editorIndentGuide.activeBackground1": "{color8}88",
    "editorWhitespace.foreground": "{color8}66",
    "editorBracketMatch.background": "{color8}33",
    "editorBracketMatch.border": "{color14}",
    "editorError.foreground": "{color9}",
    "editorWarning.foreground": "{color11}",
    "editorInfo.foreground": "{color12}",
    "editorHint.foreground": "{color14}",
    "editorGutter.addedBackground": "{color10}",
    "editorGutter.modifiedBackground": "{color12}",
    "editorGutter.deletedBackground": "{color9}",
    "editorWidget.background": "{color0}",
    "editorWidget.border": "{color8}",
    "editorSuggestWidget.background": "{color0}",
    "editorSuggestWidget.border": "{color8}",
    "editorSuggestWidget.foreground": "{foreground}",
    "editorSuggestWidget.highlightForeground": "{color14}",
    "editorSuggestWidget.selectedBackground": "{color8}55",
    "editorHoverWidget.background": "{color0}",
    "editorHoverWidget.border": "{color8}",
    "editorGroupHeader.tabsBackground": "{background}",
    "editorGroupHeader.tabsBorder": "{color0}",
    "tab.activeBackground": "{color8}33",
    "tab.activeForeground": "{foreground}",
    "tab.inactiveBackground": "{background}",
    "tab.inactiveForeground": "{color7}",
    "tab.unfocusedActiveBackground": "{color8}22",
    "tab.unfocusedActiveForeground": "{foreground}",
    "tab.unfocusedInactiveBackground": "{background}",
    "tab.unfocusedInactiveForeground": "{color7}",
    "tab.lastPinnedBorder": "{color0}",
    "sideBar.background": "{color0}",
    "sideBar.foreground": "{foreground}",
    "sideBar.border": "{color0}",
    "sideBarTitle.foreground": "{color15}",
    "list.activeSelectionBackground": "{color8}55",
    "list.activeSelectionForeground": "{foreground}",
    "list.inactiveSelectionBackground": "{color8}33",
    "list.hoverBackground": "{color8}22",
    "list.highlightForeground": "{color14}",
    "activityBar.background": "{color0}",
    "activityBar.foreground": "{foreground}",
    "activityBar.activeBorder": "{color12}",
    "activityBarBadge.background": "{color12}",
    "activityBarBadge.foreground": "{foreground}",
    "statusBar.background": "{color0}",
    "statusBar.foreground": "{foreground}",
    "statusBar.border": "{color0}",
    "statusBar.noFolderBackground": "{color0}",
    "statusBar.noFolderForeground": "{foreground}",
    "statusBar.debuggingBackground": "{color11}",
    "statusBar.debuggingForeground": "{background}",
    "statusBar.debuggingBorder": "{color11}",
    "statusBarItem.activeBackground": "{color8}33",
    "statusBarItem.focusBackground": "{color8}33",
    "statusBarItem.hoverBackground": "{color8}22",
    "statusBarItem.prominentBackground": "{color0}",
    "statusBarItem.prominentForeground": "{foreground}",
    "statusBarItem.prominentHoverBackground": "{color8}22",
    "statusBarItem.warningBackground": "{color0}",
    "statusBarItem.warningForeground": "{foreground}",
    "statusBarItem.warningHoverBackground": "{color8}22",
    "statusBarItem.errorBackground": "{color0}",
    "statusBarItem.errorForeground": "{foreground}",
    "statusBarItem.errorHoverBackground": "{color8}22",
    "statusBarItem.remoteBackground": "{color0}",
    "statusBarItem.remoteForeground": "{foreground}",
    "statusBarItem.remoteHoverBackground": "{color8}22",
    "titleBar.activeBackground": "{color0}",
    "titleBar.activeForeground": "{foreground}",
    "titleBar.inactiveBackground": "{color0}",
    "titleBar.inactiveForeground": "{color7}",
    "panel.background": "{background}",
    "panel.border": "{color0}",
    "terminal.background": "{background}",
    "terminal.foreground": "{foreground}",
    "terminal.ansiBlack": "{color0}",
    "terminal.ansiRed": "{color1}",
    "terminal.ansiGreen": "{color2}",
    "terminal.ansiYellow": "{color3}",
    "terminal.ansiBlue": "{color4}",
    "terminal.ansiMagenta": "{color5}",
    "terminal.ansiCyan": "{color6}",
    "terminal.ansiWhite": "{color7}",
    "terminal.ansiBrightBlack": "{color8}",
    "terminal.ansiBrightRed": "{color9}",
    "terminal.ansiBrightGreen": "{color10}",
    "terminal.ansiBrightYellow": "{color11}",
    "terminal.ansiBrightBlue": "{color12}",
    "terminal.ansiBrightMagenta": "{color13}",
    "terminal.ansiBrightCyan": "{color14}",
    "terminal.ansiBrightWhite": "{color15}",
    "badge.background": "{color12}",
    "badge.foreground": "{foreground}",
    "notifications.background": "{color0}",
    "notifications.foreground": "{foreground}",
    "notifications.border": "{color8}"
  },
  "tokenColors": [
    {
      "name": "Comment",
      "scope": [
        "comment",
        "punctuation.definition.comment"
      ],
      "settings": {
        "foreground": "{color8}",
        "fontStyle": "italic"
      }
    },
    {
      "name": "Keyword",
      "scope": [
        "keyword",
        "storage",
        "storage.type",
        "storage.modifier"
      ],
      "settings": {
        "foreground": "{color11}",
        "fontStyle": "italic bold"
      }
    },
    {
      "name": "String",
      "scope": [
        "string",
        "string.template",
        "constant.other.symbol"
      ],
      "settings": {
        "foreground": "{color14}",
        "fontStyle": "italic"
      }
    },
    {
      "name": "Number And Constants",
      "scope": [
        "constant.numeric",
        "constant.language",
        "constant.character",
        "constant.escape",
        "support.constant"
      ],
      "settings": {
        "foreground": "{color13}",
        "fontStyle": "bold"
      }
    },
    {
      "name": "Functions",
      "scope": [
        "entity.name.function",
        "support.function",
        "meta.function-call",
        "variable.function"
      ],
      "settings": {
        "foreground": "{color12}",
        "fontStyle": "italic bold"
      }
    },
    {
      "name": "Variables",
      "scope": [
        "variable",
        "meta.definition.variable.name",
        "support.variable"
      ],
      "settings": {
        "foreground": "{foreground}"
      }
    },
    {
      "name": "Parameters",
      "scope": [
        "variable.parameter"
      ],
      "settings": {
        "foreground": "{color6}"
      }
    },
    {
      "name": "Types",
      "scope": [
        "entity.name.type",
        "entity.name.class",
        "support.type",
        "support.class",
        "storage.type"
      ],
      "settings": {
        "foreground": "{color9}",
        "fontStyle": "bold"
      }
    },
    {
      "name": "Tags",
      "scope": [
        "entity.name.tag",
        "meta.tag",
        "punctuation.definition.tag"
      ],
      "settings": {
        "foreground": "{color9}"
      }
    },
    {
      "name": "Attributes",
      "scope": [
        "entity.other.attribute-name"
      ],
      "settings": {
        "foreground": "{color12}",
        "fontStyle": "italic"
      }
    },
    {
      "name": "Operators And Punctuation",
      "scope": [
        "keyword.operator",
        "punctuation",
        "delimiter"
      ],
      "settings": {
        "foreground": "{color7}"
      }
    },
    {
      "name": "Invalid",
      "scope": [
        "invalid",
        "invalid.illegal"
      ],
      "settings": {
        "foreground": "{color9}",
        "fontStyle": "underline"
      }
    }
  ]
}



"#;
const TEMPLATE_COLORS_WAL_DMENU_H: &str = r#"static const char *colors[SchemeLast][2] = {
	/*     fg         bg       */
	[SchemeNorm] = { "{color15}", "{color0}" },
	[SchemeSel] = { "{color15}", "{color1}" },
	[SchemeOut] = { "{color15}", "{color14}" },
};
"#;
const TEMPLATE_COLORS_WAL_DWL_H: &str = r#"#define COLOR(hex)    { ((hex >> 24) & 0xFF) / 255.0f, \
                        ((hex >> 16) & 0xFF) / 255.0f, \
                        ((hex >> 8) & 0xFF) / 255.0f, \
                        (hex & 0xFF) / 255.0f }

static const float rootcolor[]             = COLOR(0x{color0.strip}ff);
static uint32_t colors[][3]                = {
	/*               fg          bg          border    */
	[SchemeNorm] = { 0x{color15.strip}ff, 0x{color0.strip}ff, 0x{color8.strip}ff },
	[SchemeSel]  = { 0x{color15.strip}ff, 0x{color2.strip}ff, 0x{color1.strip}ff },
	[SchemeUrg]  = { 0x{color15.strip}ff, 0x{color1.strip}ff, 0x{color2.strip}ff },
};
"#;
const TEMPLATE_COLORS_WAL_DWM_URG_H: &str = r#"static const char norm_fg[] = "{color15}";
static const char norm_bg[] = "{color0}";
static const char norm_border[] = "{color8}";

static const char sel_fg[] = "{color15}";
static const char sel_bg[] = "{color2}";
static const char sel_border[] = "{color15}";

static const char urg_fg[] = "{color15}";
static const char urg_bg[] = "{color1}";
static const char urg_border[] = "{color1}";

static const char *colors[][3]      = {
    /*               fg           bg         border                         */
    [SchemeNorm] = { norm_fg,     norm_bg,   norm_border }, // unfocused wins
    [SchemeSel]  = { sel_fg,      sel_bg,    sel_border },  // the focused win
    [SchemeUrg] =  { urg_fg,      urg_bg,    urg_border },
};
"#;
const TEMPLATE_COLORS_WAL_DWM_H: &str = r#"static const char norm_fg[] = "{color15}";
static const char norm_bg[] = "{color0}";
static const char norm_border[] = "{color8}";

static const char sel_fg[] = "{color15}";
static const char sel_bg[] = "{color1}";
static const char sel_border[] = "{color15}";

static const char *colors[][3]      = {
    /*               fg           bg         border                         */
    [SchemeNorm] = { norm_fg,     norm_bg,   norm_border }, // unfocused wins
    [SchemeSel]  = { sel_fg,      sel_bg,    sel_border },  // the focused win
};
"#;
const TEMPLATE_COLORS_WAL_ST_H: &str = r#"const char *colorname[] = {

  /* 8 normal colors */
  [0] = "{color0}", /* black   */
  [1] = "{color1}", /* red     */
  [2] = "{color2}", /* green   */
  [3] = "{color3}", /* yellow  */
  [4] = "{color4}", /* blue    */
  [5] = "{color5}", /* magenta */
  [6] = "{color6}", /* cyan    */
  [7] = "{color7}", /* white   */

  /* 8 bright colors */
  [8]  = "{color8}",  /* black   */
  [9]  = "{color9}",  /* red     */
  [10] = "{color10}", /* green   */
  [11] = "{color11}", /* yellow  */
  [12] = "{color12}", /* blue    */
  [13] = "{color13}", /* magenta */
  [14] = "{color14}", /* cyan    */
  [15] = "{color15}", /* white   */

  /* special colors */
  [256] = "{background}", /* background */
  [257] = "{foreground}", /* foreground */
  [258] = "{cursor}",     /* cursor */
};

/* Default colors (colorname index)
 * foreground, background, cursor */
 unsigned int defaultbg = 0;
 unsigned int defaultfg = 257;
 unsigned int defaultcs = 258;
 unsigned int defaultrcs= 258;
"#;
const TEMPLATE_COLORS_WAL_TABBED_H: &str = r#"static const char* selbgcolor   = "{color0}";
static const char* selfgcolor   = "{color15}";
static const char* normbgcolor  = "{color2}";
static const char* normfgcolor  = "{color15}";
static const char* urgbgcolor   = "{color1}";
static const char* urgfgcolor   = "{color15}";
"#;
const TEMPLATE_COLORS_WAL_VIM: &str = r#"" Special
let wallpaper  = "{wallpaper}"
let background = "{background}"
let foreground = "{foreground}"
let cursor     = "{cursor}"

" Colors
let color0  = "{color0}"
let color1  = "{color1}"
let color2  = "{color2}"
let color3  = "{color3}"
let color4  = "{color4}"
let color5  = "{color5}"
let color6  = "{color6}"
let color7  = "{color7}"
let color8  = "{color8}"
let color9  = "{color9}"
let color10 = "{color10}"
let color11 = "{color11}"
let color12 = "{color12}"
let color13 = "{color13}"
let color14 = "{color14}"
let color15 = "{color15}"
"#;
const TEMPLATE_COLORS_WAYBAR_CSS: &str = r#"@define-color foreground {foreground};
@define-color background {background};
@define-color cursor {cursor};

@define-color color0 {color0};
@define-color color1 {color1};
@define-color color2 {color2};
@define-color color3 {color3};
@define-color color4 {color4};
@define-color color5 {color5};
@define-color color6 {color6};
@define-color color7 {color7};
@define-color color8 {color8};
@define-color color9 {color9};
@define-color color10 {color10};
@define-color color11 {color11};
@define-color color12 {color12};
@define-color color13 {color13};
@define-color color14 {color14};
@define-color color15 {color15};
"#;
const TEMPLATE_COLORS_WEZTERM_TOML: &str = r#"[colors]
foreground = "{foreground}"
background = "{background}"
cursor_bg = "{cursor}"
cursor_fg = "{background}"
cursor_border = "{cursor}"
selection_fg = "{foreground}"
selection_bg = "{color1}"

ansi = [
  "{color0}",
  "{color1}",
  "{color2}",
  "{color3}",
  "{color4}",
  "{color5}",
  "{color6}",
  "{color7}",
]
brights = [
  "{color8}",
  "{color9}",
  "{color10}",
  "{color11}",
  "{color12}",
  "{color13}",
  "{color14}",
  "{color15}",
]
"#;
const TEMPLATE_COLORS_ZATHURA: &str = r#"set recolor "true"
set recolor-keephue "true"

set completion-bg "{background}"
set completion-fg "{foreground}"
set completion-group-bg "{background}"
set completion-group-fg "{color2}"
set completion-highlight-bg "{color2}"
set completion-highlight-fg "{background}"

set recolor-lightcolor "{background}"
set recolor-darkcolor "{foreground}"
set default-bg "{background}"

set inputbar-bg "{background}"
set inputbar-fg "{foreground}"
set notification-bg "{background}"
set notification-fg "{foreground}"
set notification-error-bg "{color1}"
set notification-error-fg "{foreground}"
set notification-warning-bg "{color1}"
set notification-warning-fg "{foreground}"
set statusbar-bg "{background}"
set statusbar-fg "{foreground}"
set index-bg "{background}"
set index-fg "{foreground}"
set index-active-bg "{foreground}"
set index-active-fg "{background}"
set render-loading-bg "{background}"
set render-loading-fg "{foreground}"

set window-title-home-tilde true
set statusbar-basename true
set selection-clipboard clipboard
"#;
const TEMPLATE_COLORS_ZED_JSON: &str = r#"{
    "name": "cwal",
    "author": "cwal",
    "themes": [
        {
            "name": "cwal",
            "appearance": "dark",
            "style": {
                "background": "{background}90",
                "editor.background": "{background}90",
                "editor.foreground": "{foreground}",
                "text": "{foreground}",
                "text.muted": "{foreground}70",
                "text.ignored": "{foreground}40",
                "text.placeholder": "{foreground}50",
                "ignored": "{foreground}30",
                "element.hover": "{foreground}30",
                "ghost_element.hover": "{color1}30",
                "ghost_element.selected": "{color1}30",
                "ghost_element.active": "{color1}60",
                "border": "{background}",
                "editor.highlighted_line.background": "{color1}10",
                "editor.active_line.background": "{color1}10",
                "panel.background": "{background}90",
                "title_bar.background": "{background}90",
                "title_bar.inactive_background": "{background}90",
                "status_bar.background": "{background}90",
                "drop_target.background": "{background}90",
                "elevated_surface.background": "{background}",
                "toolbar.background": "{background}90",
                "tab_bar.background": "{background}90",
                "tab.inactive_background": "{background}90",
                "tab.active_background": "{color1}30",
                "scrollbar.track.background": "transparent",
                "scrollbar.track.border": "{background}",
                "scrollbar.thumb.background": "{color1}",
                "editor.gutter.background": "{background}90",
                "terminal.background": "{background}10",
                "terminal.foreground": "{foreground}",
                "terminal.dim_foreground": "{foreground}",
                "terminal.bright_foreground": "{foreground}",
                "terminal.ansi.black": "{color0}",
                "terminal.ansi.red": "{color1}",
                "terminal.ansi.green": "{color2}",
                "terminal.ansi.yellow": "{color3}",
                "terminal.ansi.blue": "{color4}",
                "terminal.ansi.magenta": "{color5}",
                "terminal.ansi.cyan": "{color6}",
                "terminal.ansi.white": "{color7}",
                "terminal.ansi.bright_black": "{color8}",
                "terminal.ansi.bright_red": "{color9}",
                "terminal.ansi.bright_green": "{color10}",
                "terminal.ansi.bright_yellow": "{color11}",
                "terminal.ansi.bright_blue": "{color12}",
                "terminal.ansi.bright_magenta": "{color13}",
                "terminal.ansi.bright_cyan": "{color14}",
                "terminal.ansi.bright_white": "{color15}",
                "modified": "{color1}",
                "syntax": {
                    "attribute": {
                        "color": "{color1}"
                    },
                    "boolean": {
                      "color": "{color2}"
                    },
                    "comment": {
                      "color": "{color8}"
                    },
                    "comment.doc": {
                      "color": "{color8}"
                    },
                    "constant": {
                      "color": "{color10}"
                    },
                    "function": {
                      "color": "{color14}"
                    },
                    "keyword": {
                      "color": "{color4}"
                    },
                    "number": {
                      "color": "{color5}"
                    },
                    "operator": {
                      "color": "{color4}"
                    },
                    "string": {
                      "color": "{color1}"
                    },
                    "variable": {
                      "color": "{color2}"
                    }
                },
                "players": [
                  {
                    "cursor": "{color1}",
                    "background": "{background}",
                    "selection": "{color1}30"
                  }
                ]
            }
        }
    ]
}
"#;
const TEMPLATE_COLORS_CSS: &str = r#":root {
    --wallpaper: url("{wallpaper}");

    /* Special */
    --background: {background};
    --foreground: {foreground};
    --cursor: {cursor};

    /* Colors */
    --color0: {color0};
    --color1: {color1};
    --color2: {color2};
    --color3: {color3};
    --color4: {color4};
    --color5: {color5};
    --color6: {color6};
    --color7: {color7};
    --color8: {color8};
    --color9: {color9};
    --color10: {color10};
    --color11: {color11};
    --color12: {color12};
    --color13: {color13};
    --color14: {color14};
    --color15: {color15};
}
"#;
const TEMPLATE_COLORS_FISH: &str = r#"set fish_color_autosuggestion {color8.strip}
set fish_color_cancel {color1.strip} '--reverse'
set fish_color_command {color10.strip}
set fish_color_comment {color8.strip}
set fish_color_cwd {color2.strip}
set fish_color_cwd_root {color1.strip}
set fish_color_end {color3.strip}
set fish_color_error {color1.strip}
set fish_color_escape {color5.strip}
set fish_color_history_current --bold
set fish_color_host {color12.strip}
set fish_color_host_remote {color12.strip}
set fish_color_keyword {color5.strip}
set fish_color_match --background={color4.strip}
set fish_color_normal {foreground.strip}
set fish_color_operator {color6.strip}
set fish_color_option {color3.strip}
set fish_color_param {color12.strip}
set fish_color_quote {color11.strip}
set fish_color_redirection {color5.strip}
set fish_color_search_match --background={color8.strip}
set fish_color_selection --background={color8.strip}
set fish_color_status {color1.strip}
set fish_color_user {color10.strip}
set fish_color_valid_path --underline

set fish_pager_color_background {background.strip}
set fish_pager_color_completion {foreground.strip}
set fish_pager_color_description {color8.strip}
set fish_pager_color_prefix {color10.strip}
set fish_pager_color_progress {color8.strip}
set fish_pager_color_secondary_background {background.strip}
set fish_pager_color_secondary_completion {foreground.strip}
set fish_pager_color_secondary_description {color8.strip}
set fish_pager_color_secondary_prefix {color10.strip}
set fish_pager_color_selected_background --background={color8.strip}
set fish_pager_color_selected_completion {foreground.strip}
set fish_pager_color_selected_description {color8.strip}
set fish_pager_color_selected_prefix {color10.strip}
"#;
const TEMPLATE_COLORS_JSON: &str = r#"{
    "wallpaper": "{wallpaper}",
    "alpha": "{alpha}",

    "special": {
        "background": "{background}",
        "foreground": "{foreground}",
        "cursor": "{cursor}"
    },
    "colors": {
        "color0": "{color0}",
        "color1": "{color1}",
        "color2": "{color2}",
        "color3": "{color3}",
        "color4": "{color4}",
        "color5": "{color5}",
        "color6": "{color6}",
        "color7": "{color7}",
        "color8": "{color8}",
        "color9": "{color9}",
        "color10": "{color10}",
        "color11": "{color11}",
        "color12": "{color12}",
        "color13": "{color13}",
        "color14": "{color14}",
        "color15": "{color15}"
    }
}
"#;
const TEMPLATE_COLORS_PROPERTIES: &str = r#"background={background}
foreground={foreground}
cursor={cursor}

color0={color0}
color1={color1}
color2={color2}
color3={color3}
color4={color4}
color5={color5}
color6={color6}
color7={color7}
color8={color8}
color9={color9}
color10={color10}
color11={color11}
color12={color12}
color13={color13}
color14={color14}
color15={color15}
"#;
const TEMPLATE_COLORS_SCSS: &str = r#"$wallpaper: "{wallpaper}";

// Special
$background: {background};
$foreground: {foreground};
$cursor: {cursor};

// Accent: a vivid, legible color for UIs (sliders, focus rings, active states).
// cwal has no such concept — color5 is a prominence rank and is often too dim
// to use. Consumers that do not know this variable simply ignore it.
$accent: {accent};

// Colors
$color0: {color0};
$color1: {color1};
$color2: {color2};
$color3: {color3};
$color4: {color4};
$color5: {color5};
$color6: {color6};
$color7: {color7};
$color8: {color8};
$color9: {color9};
$color10: {color10};
$color11: {color11};
$color12: {color12};
$color13: {color13};
$color14: {color14};
$color15: {color15};
"#;
const TEMPLATE_COLORS_SH: &str = r#"wallpaper="{wallpaper}"

# Special
background='{background}'
foreground='{foreground}'
cursor='{cursor}'

# Colors
color0='{color0}'
color1='{color1}'
color2='{color2}'
color3='{color3}'
color4='{color4}'
color5='{color5}'
color6='{color6}'
color7='{color7}'
color8='{color8}'
color9='{color9}'
color10='{color10}'
color11='{color11}'
color12='{color12}'
color13='{color13}'
color14='{color14}'
color15='{color15}'

# FZF colors
export FZF_DEFAULT_OPTS="
    $FZF_DEFAULT_OPTS
    --color fg:7,bg:0,hl:1,fg+:232,bg+:1,hl+:255
    --color info:7,prompt:2,spinner:1,pointer:232,marker:1
"

# Fix LS_COLORS being unreadable.
export LS_COLORS="${LS_COLORS}:su=30;41:ow=30;42:st=30;44:"
"#;
const TEMPLATE_COLORS_STYL: &str = r#"$wallpaper = "{wallpaper}"

$background = {background}
$foregroud = {foreground}
$cursor = {cursor}

$color0 = {color0}
$color1 = {color1}
$color2 = {color2}
$color3 = {color3}
$color4 = {color4}
$color5 = {color5}
$color6 = {color6}
$color7 = {color7}
$color8 = {color8}
$color9 = {color9}
$color10 = {color10}
$color11 = {color11}
$color12 = {color12}
$color13 = {color13}
$color14 = {color14}
$color15 = {color15}
"#;
const TEMPLATE_COLORS_YML: &str = r#"wallpaper: "{wallpaper}"

special:
    background: "{background}"
    foreground: "{foreground}"
    cursor: "{cursor}"

colors:
    color0: "{color0}"
    color1: "{color1}"
    color2: "{color2}"
    color3: "{color3}"
    color4: "{color4}"
    color5: "{color5}"
    color6: "{color6}"
    color7: "{color7}"
    color8: "{color8}"
    color9: "{color9}"
    color10: "{color10}"
    color11: "{color11}"
    color12: "{color12}"
    color13: "{color13}"
    color14: "{color14}"
    color15: "{color15}"
"#;
const TEMPLATE_CWAL: &str = r#"{wallpaper}
"#;

/// Linear mix of two colors (`t` = fraction toward `b`), rounded to bytes.
/// The surface-ramp equivalent of palette.rs's cwal-mode darken/lighten: those
/// crush toward black/white and lose the wallpaper hue; mixing toward the
/// foreground keeps raised surfaces a shade *of the palette*, not of grey.
fn mix(a: &Rgb, b: &Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let channel = |x: u8, y: u8| -> u8 {
        let v = (f32::from(x) + (f32::from(y) - f32::from(x)) * t) + 0.5;
        v as u8
    };
    Rgb(channel(a.0, b.0), channel(a.1, b.1), channel(a.2, b.2))
}

/// Text rendered on top of the accent: whichever of the background and the
/// foreground is more readable over it. The accent is wallpaper-derived, so
/// the readable partner is not fixed (measured in palette.rs: `color5`
/// scored 2.60:1 on a dark background).
fn on_accent(scheme: &Scheme) -> Rgb {
    let over_bg = contrast_ratio(&scheme.accent, &scheme.background);
    let over_fg = contrast_ratio(&scheme.accent, &scheme.foreground);
    if over_bg >= over_fg {
        scheme.background
    } else {
        scheme.foreground
    }
}

/// The named-color set libadwaita ships and GTK4 apps consume via the user
/// stylesheet. Surfaces are small mixes of background toward foreground, so
/// popovers/cards read as raised versions of the window instead of unrelated
/// static-theme colors. Semantic colors (destructive/error/warning/success)
/// are deliberately left to Adwaita's own defaults: they must stay
/// recognizable whatever the wallpaper looks like.
fn gtk4_stylesheet(scheme: &Scheme) -> String {
    let bg = hex(&scheme.background);
    let fg = hex(&scheme.foreground);
    let accent = hex(&scheme.accent);
    let on_accent = hex(&on_accent(scheme));
    let surface = hex(&mix(&scheme.background, &scheme.foreground, 0.05));
    format!(
        "/* Generated by coloreclipse — GTK4 (libadwaita) user stylesheet. \
Values derive window/foreground/accent from the wallpaper scheme. */
@define-color accent_color {accent};
@define-color accent_bg_color {accent};
@define-color accent_fg_color {on_accent};

@define-color window_bg_color {bg};
@define-color window_fg_color {fg};

@define-color view_bg_color {bg};
@define-color view_fg_color {fg};

@define-color headerbar_bg_color {bg};
@define-color headerbar_fg_color {fg};
@define-color headerbar_backdrop_color @window_bg_color;

@define-color popover_bg_color {surface};
@define-color popover_fg_color {fg};

@define-color card_bg_color {surface};
@define-color card_fg_color {fg};

@define-color dialog_bg_color {bg};
@define-color dialog_fg_color {fg};

@define-color overview_bg_color {surface};
@define-color overview_fg_color {fg};

@define-color sidebar_bg_color {surface};
@define-color sidebar_fg_color {fg};
@define-color sidebar_backdrop_color @window_bg_color;
@define-color sidebar_border_color @window_bg_color;

@define-color secondary_sidebar_bg_color {bg};
@define-color secondary_sidebar_fg_color {fg};

/* Focused selection states */
@define-color theme_selected_bg_color @accent_bg_color;
@define-color theme_selected_fg_color @accent_fg_color;

/* Backdrop/unfocused states */
@define-color theme_unfocused_fg_color @window_fg_color;
@define-color theme_unfocused_text_color @view_fg_color;
@define-color theme_unfocused_bg_color @window_bg_color;
@define-color theme_unfocused_base_color @window_bg_color;
@define-color theme_unfocused_selected_bg_color @accent_bg_color;
@define-color theme_unfocused_selected_fg_color @accent_fg_color;
"
    )
}

/// GTK3's Adwaita recolors through the legacy `theme_*` directives; the file
/// also carries the libadwaita names so both toolkit generations follow the
/// same palette from one generator (the shape noctalia used before).
fn gtk3_stylesheet(scheme: &Scheme) -> String {
    let bg = hex(&scheme.background);
    let fg = hex(&scheme.foreground);
    let accent = hex(&scheme.accent);
    let on_accent = hex(&on_accent(scheme));
    let surface = hex(&mix(&scheme.background, &scheme.foreground, 0.05));
    let borders = hex(&mix(&scheme.background, &scheme.foreground, 0.10));
    let insensitive_bg = hex(&mix(&scheme.background, &scheme.foreground, 0.03));
    let insensitive_fg = hex(&mix(&scheme.foreground, &scheme.background, 0.45));
    format!(
        "/* Generated by coloreclipse — GTK3 user stylesheet. Same \
wallpaper-derived values as the GTK4 one, plus the legacy theme_* directs \
GTK3's Adwaita consumes. */
@define-color accent_color {accent};
@define-color accent_bg_color {accent};
@define-color accent_fg_color {on_accent};

@define-color window_bg_color {bg};
@define-color window_fg_color {fg};

@define-color view_bg_color {bg};
@define-color view_fg_color {fg};

@define-color headerbar_bg_color {surface};
@define-color headerbar_fg_color {fg};
@define-color headerbar_backdrop_color @window_bg_color;

@define-color popover_bg_color {surface};
@define-color popover_fg_color {fg};

@define-color card_bg_color {surface};
@define-color card_fg_color {fg};

@define-color dialog_bg_color {bg};
@define-color dialog_fg_color {fg};

@define-color sidebar_bg_color {surface};
@define-color sidebar_fg_color {fg};
@define-color sidebar_backdrop_color @window_bg_color;
@define-color sidebar_border_color @window_bg_color;

@define-color theme_bg_color {bg};
@define-color theme_fg_color {fg};
@define-color theme_base_color {bg};
@define-color theme_text_color {fg};
@define-color theme_selected_bg_color {accent};
@define-color theme_selected_fg_color {on_accent};
@define-color theme_unfocused_bg_color {bg};
@define-color theme_unfocused_fg_color {fg};
@define-color theme_unfocused_base_color {bg};
@define-color theme_unfocused_text_color {fg};
@define-color theme_unfocused_selected_bg_color {accent};
@define-color theme_unfocused_selected_fg_color {on_accent};
@define-color theme_insensitive_bg_color {insensitive_bg};
@define-color theme_insensitive_fg_color {insensitive_fg};
@define-color borders {borders};
@define-color link_color {accent};
"
    )
}

/// One qt6ct QPalette line per state, 22 comma-separated slots in
/// QPalette::ColorRole order: WindowText, Button, Light, Midlight, Dark, Mid,
/// Text, BrightText, ButtonText, Base, Window, Shadow, Highlight,
/// HighlightedText, Link, LinkVisited, AlternateBase, NoRole, ToolTipBase,
/// ToolTipText, PlaceholderText, Accent. Widget chrome (Button/Light/...)
/// takes its steps from background/foreground; selections and accents get the
/// scheme accent. Disabled/inactive repeat the active line ( qt6ct fills in
/// the states Fusion needs at runtime , matching noctalia's palette shape).
fn qt6ct_palette(scheme: &Scheme) -> String {
    let line = |slots: &[Rgb]| -> String {
        slots
            .iter()
            .map(|c| hex(c))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let bg = scheme.background;
    let fg = scheme.foreground;
    let accent = scheme.accent;
    let slots = vec![
        fg,                                     // WindowText
        mix(&bg, &fg, 0.05),                    // Button (raised widget face)
        mix(&bg, &fg, 0.30),                    // Light (bevel highlight)
        mix(&bg, &fg, 0.16),                    // Midlight (bevel light)
        mix(&fg, &bg, 0.60),                    // Dark (bevel shadow)
        mix(&fg, &bg, 0.75),                    // Mid (divider)
        fg,                                     // Text
        fg,                                     // BrightText
        fg,                                     // ButtonText
        bg,                                     // Base (inputs/text views)
        bg,                                     // Window
        Rgb(0, 0, 0),                           // Shadow
        accent,                                 // Highlight (selection)
        on_accent(scheme),                      // HighlightedText
        accent,                                 // Link
        mix(&accent, &fg, 0.30),                // LinkVisited (distinct)
        mix(&bg, &fg, 0.035),                   // AlternateBase (zebra rows)
        bg,                                     // NoRole (Qt reserves the slot)
        mix(&bg, &fg, 0.08),                    // ToolTipBase
        fg,                                     // ToolTipText
        mix(&fg, &bg, 0.45),                    // PlaceholderText (dim)
        accent,                                 // Accent
    ];
    let active = line(&slots);
    format!(
        "; Generated by coloreclipse; a wallpaper-derived QPalette for Fusion. \
Slots per line follow QPalette::ColorRole order.\n\
[ColorScheme]\n\
active_colors={active}\n\
disabled_colors={active}\n\
inactive_colors={active}\n"
    )
}

/// The window/view/button chrome Dolphin and friends color from: a full
/// KColorScheme `.colors` file written to
/// `~/.local/share/color-schemes/coloreclipse.colors`, selected by
/// `kdeglobals` (main.rs strips the stale inline `[Colors:*]`/
/// `[ColorEffects:*]` groups, which would otherwise override this file).
///
/// Surfaces take the same bg/fg mix steps as the Qt slots; Selection paints
/// the accent with `on_accent` text. Semantic foregrounds (Negative/
/// Positive/Neutral) are fixed hues measured ≥3:1 against both dark and
/// light backgrounds — KColorScheme requires explicit values and there is
/// no equivalent of "Adwaita's semantic defaults" to delegate to on the KDE
/// side.
fn kde_scheme(scheme: &Scheme) -> String {
    const NEGATIVE: Rgb = Rgb(224, 27, 36);
    const POSITIVE: Rgb = Rgb(46, 194, 126);
    const NEUTRAL: Rgb = Rgb(246, 166, 9);

    let rgb = |c: &Rgb| format!("{},{},{}", c.0, c.1, c.2);
    let bg = scheme.background;
    let fg = scheme.foreground;
    let accent = scheme.accent;
    let on_accent = on_accent(scheme);
    let decoration_hover = mix(&accent, &fg, 0.30);
    let link_visited = mix(&accent, &fg, 0.30);
    let inactive_fg = mix(&fg, &bg, 0.45);

    // One color-role group per KColorScheme surface: 12 keys, always all
    // present (KColorScheme applies its own defaults per missing key, and a
    // partially-written scheme silently falls back to Breeze).
    let group = |name: &str, normal: Rgb, alternate: Rgb, text: Rgb, text_inactive: Rgb| -> String {
        format!(
            "; {name}\n{name}\n\
BackgroundAlternate={}\nBackgroundNormal={}\n\
DecorationFocus={}\nDecorationHover={}\n\
ForegroundActive={}\nForegroundInactive={}\nForegroundLink={}\n\
ForegroundNegative={}\nForegroundNeutral={}\nForegroundNormal={}\n\
ForegroundPositive={}\nForegroundVisited={}\n\n",
            rgb(&alternate),
            rgb(&normal),
            rgb(&accent),
            rgb(&decoration_hover),
            rgb(&accent),
            rgb(&text_inactive),
            rgb(&accent),
            rgb(&NEGATIVE),
            rgb(&NEUTRAL),
            rgb(&text),
            rgb(&POSITIVE),
            rgb(&link_visited),
        )
    };

    let selection_alternate = mix(&accent, &bg, 0.20);
    // Surfaces: raw background, raised chrome a step toward the text (same
    // mixes the Qt slots and GTK popover use), view always the window color.
    let mut out = String::new();
    out.push_str("; Generated by coloreclipse; a wallpaper-derived KColorScheme for \
KDE Frameworks apps (Dolphin, Kate, okular, …).\n");
    out.push_str(&format!(
        "[ColorEffects:Disabled]\n\
Color={}\nColorAmount=0\nColorEffect=0\nContrastAmount=0.65\nContrastEffect=1\nIntensityAmount=0.1\nIntensityEffect=2\n\n\
[ColorEffects:Inactive]\n\
ChangeSelectionColor=true\nColor={}\nColorAmount=0.025\nColorEffect=2\nContrastAmount=0.1\nContrastEffect=2\nEnable=false\nIntensityAmount=0\nIntensityEffect=0\n\n",
        rgb(&mix(&bg, &fg, 0.85)),
        rgb(&mix(&bg, &fg, 0.50)),
    ));
    out.push_str(&group("[Colors:View]", bg, mix(&bg, &fg, 0.035), fg, inactive_fg));
    out.push_str(&group("[Colors:Window]", bg, mix(&bg, &fg, 0.035), fg, inactive_fg));
    out.push_str(&group("[Colors:Button]", mix(&bg, &fg, 0.05), mix(&bg, &fg, 0.08), fg, inactive_fg));
    out.push_str(&group("[Colors:Complementary]", mix(&bg, &fg, 0.08), mix(&bg, &fg, 0.05), fg, inactive_fg));
    out.push_str(&group("[Colors:Header]", mix(&bg, &fg, 0.10), mix(&bg, &fg, 0.05), fg, inactive_fg));
    // KWin-level titlebars color the inactive header from the raised/inset
    // swap, mirroring the noctalia scheme's [Colors:Header][Inactive] block.
    out.push_str(&group(
        "[Colors:Header][Inactive]",
        mix(&bg, &fg, 0.05),
        mix(&bg, &fg, 0.10),
        fg,
        inactive_fg,
    ));
    out.push_str(&group(
        "[Colors:Selection]",
        accent,
        selection_alternate,
        on_accent,
        mix(&on_accent, &bg, 0.45),
    ));
    out.push_str(&group("[Colors:Tooltip]", mix(&bg, &fg, 0.08), mix(&bg, &fg, 0.035), fg, inactive_fg));
    out
}

/// The four toolkit targets this project adds on top of cwal's set.
fn toolkit_body(name: &str, scheme: &Scheme) -> Option<String> {
    match name {
        "colors-gtk3.css" => Some(gtk3_stylesheet(scheme)),
        "colors-gtk4.css" => Some(gtk4_stylesheet(scheme)),
        "colors-qt6ct.conf" => Some(qt6ct_palette(scheme)),
        "colors-kde.colors" => Some(kde_scheme(scheme)),
        _ => None,
    }
}

fn template_body(name: &str) -> Option<&'static str> {
    match name {
        "Colors.hs" => Some(TEMPLATE_COLORS_HS),
        "colors" => Some(TEMPLATE_COLORS),
        "colors--nodim-alacritty.yml" => Some(TEMPLATE_COLORS_NODIM_ALACRITTY_YML),
        "colors-alacritty.toml" => Some(TEMPLATE_COLORS_ALACRITTY_TOML),
        "colors-alacritty.yml" => Some(TEMPLATE_COLORS_ALACRITTY_YML),
        "colors-btop.theme" => Some(TEMPLATE_COLORS_BTOP_THEME),
        "colors-cava" => Some(TEMPLATE_COLORS_CAVA),
        "colors-discord.css" => Some(TEMPLATE_COLORS_DISCORD_CSS),
        "colors-dunst.conf" => Some(TEMPLATE_COLORS_DUNST_CONF),
        "colors-foot.ini" => Some(TEMPLATE_COLORS_FOOT_INI),
        "colors-fuzzel.ini" => Some(TEMPLATE_COLORS_FUZZEL_INI),
        "colors-ghostty.conf" => Some(TEMPLATE_COLORS_GHOSTTY_CONF),
        "colors-hyprland.conf" => Some(TEMPLATE_COLORS_HYPRLAND_CONF),
        "colors-kitty.conf" => Some(TEMPLATE_COLORS_KITTY_CONF),
        "colors-konsole.colorscheme" => Some(TEMPLATE_COLORS_KONSOLE_COLORSCHEME),
        "colors-leftwm-theme.ron" => Some(TEMPLATE_COLORS_LEFTWM_THEME_RON),
        "colors-mako" => Some(TEMPLATE_COLORS_MAKO),
        "colors-nqq.css" => Some(TEMPLATE_COLORS_NQQ_CSS),
        "colors-nvim.lua" => Some(TEMPLATE_COLORS_NVIM_LUA),
        "colors-obsidian.css" => Some(TEMPLATE_COLORS_OBSIDIAN_CSS),
        "colors-oomox" => Some(TEMPLATE_COLORS_OOMOX),
        "colors-polybar" => Some(TEMPLATE_COLORS_POLYBAR),
        "colors-quickshell.qml" => Some(TEMPLATE_COLORS_QUICKSHELL_QML),
        "colors-retroarch.cfg" => Some(TEMPLATE_COLORS_RETROARCH_CFG),
        "colors-rgb" => Some(TEMPLATE_COLORS_RGB),
        "colors-rio.toml" => Some(TEMPLATE_COLORS_RIO_TOML),
        "colors-rofi-dark.rasi" => Some(TEMPLATE_COLORS_ROFI_DARK_RASI),
        "colors-rofi-light.rasi" => Some(TEMPLATE_COLORS_ROFI_LIGHT_RASI),
        "colors-speedcrunch.json" => Some(TEMPLATE_COLORS_SPEEDCRUNCH_JSON),
        "colors-sway" => Some(TEMPLATE_COLORS_SWAY),
        "colors-themer.js" => Some(TEMPLATE_COLORS_THEMER_JS),
        "colors-tilix.json" => Some(TEMPLATE_COLORS_TILIX_JSON),
        "colors-tofi" => Some(TEMPLATE_COLORS_TOFI),
        "colors-tty.sh" => Some(TEMPLATE_COLORS_TTY_SH),
        "colors-vscode.json" => Some(TEMPLATE_COLORS_VSCODE_JSON),
        "colors-wal-dmenu.h" => Some(TEMPLATE_COLORS_WAL_DMENU_H),
        "colors-wal-dwl.h" => Some(TEMPLATE_COLORS_WAL_DWL_H),
        "colors-wal-dwm-urg.h" => Some(TEMPLATE_COLORS_WAL_DWM_URG_H),
        "colors-wal-dwm.h" => Some(TEMPLATE_COLORS_WAL_DWM_H),
        "colors-wal-st.h" => Some(TEMPLATE_COLORS_WAL_ST_H),
        "colors-wal-tabbed.h" => Some(TEMPLATE_COLORS_WAL_TABBED_H),
        "colors-wal.vim" => Some(TEMPLATE_COLORS_WAL_VIM),
        "colors-waybar.css" => Some(TEMPLATE_COLORS_WAYBAR_CSS),
        "colors-wezterm.toml" => Some(TEMPLATE_COLORS_WEZTERM_TOML),
        "colors-zathura" => Some(TEMPLATE_COLORS_ZATHURA),
        "colors-zed.json" => Some(TEMPLATE_COLORS_ZED_JSON),
        "colors.css" => Some(TEMPLATE_COLORS_CSS),
        "colors.fish" => Some(TEMPLATE_COLORS_FISH),
        "colors.json" => Some(TEMPLATE_COLORS_JSON),
        "colors.properties" => Some(TEMPLATE_COLORS_PROPERTIES),
        "colors.scss" => Some(TEMPLATE_COLORS_SCSS),
        "colors.sh" => Some(TEMPLATE_COLORS_SH),
        "colors.styl" => Some(TEMPLATE_COLORS_STYL),
        "colors.yml" => Some(TEMPLATE_COLORS_YML),
        "cwal" => Some(TEMPLATE_CWAL),
        _ => None,
    }
}


/// Render one named template (see [`template_names`]). The three toolkit
/// targets are generated bodies rather than placeholder templates.
pub fn render_one(name: &str, scheme: &Scheme) -> Result<String, String> {
    if let Some(body) = toolkit_body(name, scheme) {
        return Ok(body);
    }
    match template_body(name) {
        Some(body) => Ok(render_template(body, scheme)),
        None => Err(format!("Error: unknown template {name}")),
    }
}

fn escape(color: &Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", color.0, color.1, color.2)
}

/// Terminal escape sequences, byte-identical in shape to cwal's `sequences`
/// file: `ESC ] 4 ; N ; #rrggbb ESC \` for colors 0..15, then special slots
/// 10=foreground, 11=background, 13/17=cursor, 19=background, 4;232=background,
/// 4;256=foreground, 4;257=background, 708=background, 12=cursor (verified
/// against cwal output bytes).
pub fn sequences_for(scheme: &Scheme) -> String {
    let mut out = String::new();
    for (index, color) in scheme.colors.iter().enumerate() {
        out.push_str(&format!("\x1b]4;{};{}\x1b\\", index, escape(color)));
    }
    let background = escape(&scheme.background);
    let foreground = escape(&scheme.foreground);
    let cursor = escape(&scheme.cursor);
    for (prefix, slot, color) in [
        ("", 10, &foreground),
        ("", 11, &background),
        ("", 13, &cursor),
        ("", 17, &cursor),
        ("", 19, &background),
        ("4;", 232, &background),
        ("4;", 256, &foreground),
        ("4;", 257, &background),
        ("", 708, &background),
        ("", 12, &cursor),
    ] {
        out.push_str(&format!("\x1b]{prefix}{slot};{color}\x1b\\"));
    }
    out
}

/// Write every template plus `sequences` into `out_dir`, each via tmp file +
/// atomic rename. All bodies render in memory first so a failure leaves the
/// previous cache untouched.
pub fn write_all(scheme: &Scheme, out_dir: &std::path::Path) -> Result<(), String> {
    let mut rendered: Vec<(&str, String)> = Vec::with_capacity(59);
    for name in template_names() {
        rendered.push((name, render_one(name, scheme)?));
    }
    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("Error: cannot create {}: {e}", out_dir.display()))?;
    let tag = std::process::id();
    for (name, body) in &rendered {
        let target = out_dir.join(name);
        let tmp = out_dir.join(format!(".tmp.{tag}.{name}"));
        std::fs::write(&tmp, body)
            .map_err(|e| format!("Error: cannot write {}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, &target)
            .map_err(|e| format!("Error: cannot write {}: {e}", target.display()))?;
    }
    let sequences = sequences_for(scheme);
    let target = out_dir.join("sequences");
    let tmp = out_dir.join(format!(".tmp.{tag}.sequences"));
    std::fs::write(&tmp, &sequences)
        .map_err(|e| format!("Error: cannot write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &target)
        .map_err(|e| format!("Error: cannot write {}: {e}", target.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::palette::{Rgb, Scheme};

    fn test_scheme() -> Scheme {
        let mut colors = [Rgb(29, 26, 27); 16];
        colors[15] = Rgb(209, 209, 209);
        Scheme {
            wallpaper: "/wall.jpg".to_string(),
            mode: "dark".to_string(),
            alpha: 1.0,
            background: Rgb(29, 26, 27),
            foreground: Rgb(209, 209, 209),
            cursor: Rgb(209, 209, 209),
            accent: Rgb(209, 209, 209),
            colors,
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

    /// The KDE color scheme KColorScheme apps (Dolphin, Kate, okular…) read:
    /// full `.colors` format with the per-role groups and keys KColorScheme
    /// requires. Backgrounds come from scheme-derived surfaces (the same
    /// bg/fg mixes as the Qt slots), the Selection group colors from the
    /// accent, and the semantic foregrounds are fixed hues measured ≥3:1
    /// against both dark and light backgrounds (KDE has no equivalent of
    /// Adwaita's semantic defaults, so they must be explicit).
    #[test]
    fn kde_scheme_has_required_kcolorscheme_groups_and_keys() {
        let out = gtk_test("colors-kde.colors");
        assert!(out.starts_with("; Generated by coloreclipse"));
        // Every group block Dolphin consumes, including the two effect blocks.
        for group in [
            "[ColorEffects:Disabled]",
            "[ColorEffects:Inactive]",
            "[Colors:View]",
            "[Colors:Window]",
            "[Colors:Button]",
            "[Colors:Complementary]",
            "[Colors:Header]",
            "[Colors:Selection]",
            "[Colors:Tooltip]",
        ] {
            assert!(out.contains(&format!("\n{group}\n")), "missing {group}");
        }
        // Backgrounds: view/window raw background, raised chrome mixes.
        assert!(out.contains("BackgroundNormal=29,26,27"));
        assert!(out.contains("BackgroundAlternate=35,32,33"));
        assert!(out.contains("BackgroundNormal=38,35,36"));
        assert!(out.contains("BackgroundNormal=43,41,42"));
        assert!(out.contains("BackgroundNormal=47,44,45"));
        // Foreground: text on surfaces, dimmed inactive.
        assert!(out.contains("ForegroundNormal=209,209,209"));
        assert!(out.contains("ForegroundInactive=128,127,127"));
        // Decorations and selection use the accent; on-accent text is light
        // mode opposite here (accent == foreground, so background wins).
        assert!(out.contains("DecorationFocus=209,209,209"));
        // Selection colors from the accent with on-accent text.
        assert!(out.contains("BackgroundNormal=209,209,209"));
        assert!(out.contains("ForegroundNormal=29,26,27"));
        // Semantics: fixed hues, same on every variant.
        assert!(out.contains("ForegroundNegative=224,27,36"));
        assert!(out.contains("ForegroundPositive=46,194,126"));
        assert!(out.contains("ForegroundNeutral=246,166,9"));
    }

    #[test]
    fn template_list_matches_cwal_plus_toolkit_targets() {
        // The 55 cwal templates must all still be present...
        for name in ["colors.json", "colors-kitty.conf", "colors.scss", "colors-waybar.css", "cwal"] {
            assert!(super::template_names().contains(&name), "cwal template {name} missing");
        }
        // ...plus our four toolkit targets.
        assert_eq!(super::template_names().len(), 59);
    }

    #[test]
    fn shell_vars_pass_through() {
        let s = test_scheme();
        let out = super::render_template("${LS_COLORS} $FZF_DEFAULT_OPTS {color0}", &s);
        assert!(out.contains("${LS_COLORS}"));
        assert!(out.contains("#1d1a1b"));
    }

    /// No unexpanded cwal placeholder may survive rendering (`${shell}` vars
    /// are not placeholders and are allowed through).
    fn has_leftover_placeholders(rendered: &str) -> Option<String> {
        let bytes = rendered.as_bytes();
        let mut cursor = 0;
        while cursor < bytes.len() {
            let Some(open) = rendered[cursor..].find('{') else {
                return None;
            };
            let open = cursor + open;
            let Some(close_rel) = rendered[open..].find('}') else {
                return None;
            };
            let close = open + close_rel;
            let preceded_by_dollar = open > 0 && bytes[open - 1] == b'$';
            let name = &rendered[open + 1..close];
            // Flag any identifier-shaped placeholder that did not expand —
            // *including names `lookup` does not know*. Skipping unknown names
            // would make this test blind to typos: a misspelled or not-yet-added
            // placeholder would slip through unrendered.
            //
            // Literal braces are still fine: JSON objects start with a quote
            // (`{"color0": ...}`), CSS blocks contain `:` (`{color:red}`), and
            // `${...}` shell vars are handled above, so none of those look like
            // a bare identifier.
            let looks_like_placeholder = !name.is_empty()
                && !preceded_by_dollar
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
                && !name.starts_with(|c: char| c.is_ascii_digit());
            if looks_like_placeholder && super::lookup(name, &test_scheme()).is_none() {
                return Some(name.to_string());
            }
            cursor = close + 1;
        }
        None
    }

    #[test]
    fn all_templates_render_without_leftover_placeholders() {
        let s = test_scheme();
        for name in super::template_names() {
            let out = super::render_one(&name, &s).unwrap();
            assert!(
                has_leftover_placeholders(&out).is_none(),
                "leftover in {name}"
            );
        }
    }

    #[test]
    fn sequences_match_cwal_shape() {
        let out = super::sequences_for(&test_scheme());
        assert!(out.contains("\x1b]4;0;#1d1a1b\x1b\\"));
        assert!(out.contains("\x1b]4;15;#d1d1d1\x1b\\"));
        assert!(out.contains("\x1b]10;#d1d1d1\x1b\\"));
        assert!(out.contains("\x1b]11;#1d1a1b\x1b\\"));
        assert!(out.contains("\x1b]12;#d1d1d1\x1b\\"));
        assert!(out.contains("\x1b]4;232;#1d1a1b\x1b\\"));
        assert!(out.contains("\x1b]4;256;#d1d1d1\x1b\\"));
    }

    #[test]
    fn write_all_produces_every_file() {
        let dir = std::env::temp_dir().join(format!("coloreclipse-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        super::write_all(&test_scheme(), &dir).unwrap();
        let mut count = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let name = entry.unwrap().file_name().into_string().unwrap();
            assert!(!name.starts_with(".tmp."), "tmp file left behind: {name}");
            count += 1;
        }
        assert_eq!(count, 60); // 55 cwal templates + 4 toolkit targets + sequences
        let sh = std::fs::read_to_string(dir.join("colors.sh")).unwrap();
        assert!(sh.contains("background='#1d1a1b'"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn gtk_test(name: &str) -> String {
        super::render_one(name, &test_scheme()).unwrap()
    }

    /// libadwaita names its colors; our GTK4 stylesheet color-schemes them from
    /// the wallpaper. bg/fg come straight from the scheme, the accent from
    /// `pick_accent` via the scheme, and on-accent text is whichever of
    /// background/foreground is more readable over the accent.
    #[test]
    fn gtk4_stylesheet_uses_scheme_colors() {
        let out = gtk_test("colors-gtk4.css");
        assert!(out.contains("@define-color window_bg_color #1d1a1b;\n"));
        assert!(out.contains("@define-color window_fg_color #d1d1d1;\n"));
        assert!(out.contains("@define-color accent_color #d1d1d1;\n"));
        // accent here equals the foreground, so the readable on-accent text is
        // the dark background, not the near-white foreground.
        assert!(out.contains("@define-color accent_fg_color #1d1a1b;\n"));
        // Raised surfaces (cards, popovers) must not be the raw window color.
        assert!(out.contains("@define-color popover_bg_color #262324;\n"));
    }

    /// GTK3's Adwaita consumes the legacy `theme_*` directs; the stylesheet
    /// must define those *and* the libadwaita names (each file is the user
    /// stylesheet for its GTK generation).
    #[test]
    fn gtk3_stylesheet_defines_theme_variables() {
        let out = gtk_test("colors-gtk3.css");
        assert!(out.contains("@define-color theme_bg_color #1d1a1b;\n"));
        assert!(out.contains("@define-color theme_fg_color #d1d1d1;\n"));
        assert!(out.contains("@define-color theme_base_color #1d1a1b;\n"));
        assert!(out.contains("@define-color theme_selected_bg_color #d1d1d1;\n"));
        assert!(out.contains("@define-color window_bg_color #1d1a1b;\n"));
    }

    /// The QPalette file qt6ct consumes: three state colors, each 22
    /// comma-separated slots in QPalette role order
    /// (windowText, button, light, midlight, dark, mid, text, brightText,
    /// buttonText, base, window, shadow, highlight, highlightedText, link,
    /// linkVisited, alternateBase, toolTipBase, toolTipText,
    /// placeholderText, accent). Slots we pin: windowText=foreground,
    /// text=foreground, base/background-ish=background,
    /// highlight/accent-on-selection=accent.
    #[test]
    fn qt6ct_palette_has_three_22_slot_state_lines() {
        let out = gtk_test("colors-qt6ct.conf");
        assert!(out.starts_with("; Generated by coloreclipse"));
        assert!(out.contains("\n[ColorScheme]\nactive_colors="));
        for key in ["active_colors", "disabled_colors", "inactive_colors"] {
            let line = out.lines().find(|l| l.starts_with(key)).unwrap_or("");
            let value = line.split_once('=').unwrap_or((" ", " ")).1;
            let slots: Vec<&str> = value.split(',').map(str::trim).collect();
            assert_eq!(slots.len(), 22, "slot count in {key}");
            assert_eq!(slots[0], "#d1d1d1", "windowText in {key}");
            assert_eq!(slots[6], "#d1d1d1", "text in {key}");
            assert_eq!(slots[9], "#1d1a1b", "base in {key}");
            assert_eq!(slots[12], "#d1d1d1", "highlight in {key}");
            assert_eq!(slots[21], "#d1d1d1", "accent in {key}");        }
    }
}

