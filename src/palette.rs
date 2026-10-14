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
