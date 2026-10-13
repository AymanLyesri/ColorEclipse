# ColorEclipse

Light/dark theme applier for [ArchEclipse](https://github.com/AymanLyesri/ArchEclipse).
Rust replacement for hypr's former `theme/scripts/*.sh`, shipped as its own
package so the compositor config no longer compiles anything at login.

It drives the system color-scheme (`gsettings`), the GTK/icon/cursor themes, and
`cwal` color generation — in parallel, from one binary.

## Why

Six shell scripts (414 lines) each re-read the same flags file and re-spawned
`gsettings`; the appliers had to be launched as four background jobs and their
failures were swallowed. One process with a thread per component does the same
work without the shells, and `wal` (wallpaper → colors) no longer lives inside
the hypr config at all.

## Layout

```
Cargo.toml              binary `coloreclipse` (no dependencies)
src/main.rs             CLI + the four appliers + unit tests
dist/aur/PKGBUILD       coloreclipse-git (AUR)
```

## Usage

| Command | Effect |
|---|---|
| `coloreclipse get` | print the current variant (`dark`\|`light`) |
| `coloreclipse switch` | toggle the variant, then re-apply everything |
| `coloreclipse switch dark\|light` | set the variant, then re-apply everything |
| `coloreclipse apply` | re-apply cursor/GTK/icon/cwal for the current variant |
| `coloreclipse wal [wallpaper]` | regenerate `cwal` colors (defaults to walleclipse's current wallpaper) |

`switch` refuses to run while `autovariant` is enabled, and `wal` (auto-)picks
light/dark from the wallpaper's average brightness when it is — the same
behaviour the scripts had.

## Config

`$XDG_CONFIG_HOME/coloreclipse/theme.conf`

```
autocolor=true      # false = wal does nothing
autovariant=true    # false = pin the variant, wallpapers cannot flip it
```

Both default to `true` when the file (or a key) is missing. The quickshell
Settings toggle writes this file; deleting it resets to auto.

## Runtime dependencies

Used as found, each degrading gracefully when absent: `gsettings` (glib2),
`cwal`, `hyprctl` (hyprland), `magick` (imagemagick, variant detection),
`killall` (psmisc), `notify-send` (libnotify), and `ffmpeg` for the animated
wallpaper frame extraction (cached in `~/.cache/wall-thumbs`). The GTK/icon/
cursor names it sets come from `whitesur-gtk-theme`, `whitesur-icon-theme` and
`phinger-cursors`.

## Install

```sh
yay -S coloreclipse-git          # AUR
# or from a checkout:
cargo build --release            # target/release/coloreclipse
cargo test
```

## Consumers

- **ArchEclipse** (`~/.config/hypr`): `coloreclipse apply` at login; the
  quickshell bar (theme toggle, launcher commands, wallpaper panel) calls
  `coloreclipse switch` / `coloreclipse wal`.
- **WallEclipse**: `apply_theme` runs `coloreclipse wal <wallpaper>` after a
  wallpaper change, and the daemon owns `config/current.conf` which
  `coloreclipse wal` falls back to.
