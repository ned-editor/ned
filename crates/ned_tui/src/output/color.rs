//! Comprehensive color type

/// Comprehensive color represented either in ANSI palette or as RGB true color (24 bits)
///
/// It can convert to other representations common in terminals
///
/// TODO: 6bit for windows console
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// ANSI 8-bit color index (0-255).
    Ansi(u8),
    /// 24-bit RGB TrueColor.
    Rgb(u8, u8, u8),
}

impl Color {
    /// Creates an ANSI indexed color.
    pub const fn ansi(ansi: u8) -> Self {
        Color::Ansi(ansi)
    }
    /// Creates an RGB TrueColor.
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Color::Rgb(red, green, blue)
    }
    /// Standard black color.
    pub const fn black() -> Self {
        Self::ansi(0)
    }
    /// Standard white color.
    pub const fn white() -> Self {
        Self::ansi(15)
    }
    /// Grey color with optional light intensity.
    pub const fn grey(light: bool) -> Self {
        Self::ansi(if light { 7 } else { 8 })
    }
    /// Red color with optional light intensity.
    pub const fn red(light: bool) -> Self {
        Self::ansi(if light { 1 + 8 } else { 1 })
    }
    /// Green color with optional light intensity.
    pub const fn green(light: bool) -> Self {
        Self::ansi(if light { 2 + 8 } else { 2 })
    }
    /// Blue color with optional light intensity.
    pub const fn blue(light: bool) -> Self {
        Self::ansi(if light { 4 + 8 } else { 4 })
    }
    /// Yellow color with optional light intensity.
    pub const fn yellow(light: bool) -> Self {
        Self::ansi(if light { 3 + 8 } else { 3 })
    }
    /// Magenta color with optional light intensity.
    pub const fn magenta(light: bool) -> Self {
        Self::ansi(if light { 5 + 8 } else { 5 })
    }
    /// Cyan color with optional light intensity.
    pub const fn cyan(light: bool) -> Self {
        Self::ansi(if light { 6 + 8 } else { 6 })
    }

    /// Converts to 3-bit color value.
    pub const fn to_3b(self) -> u8 {
        self.to_4b() % 8
    }
    /// Converts to 4-bit color value (0-15).
    pub const fn to_4b(self) -> u8 {
        match self {
            Color::Ansi(ansi) => ansi_to_4b(ansi),
            Color::Rgb(r, g, b) => rgb_to_4bit(r, g, b),
        }
    }
    /// Converts to 8-bit ANSI palette index (0-255).
    pub const fn to_8b(self) -> u8 {
        match self {
            Color::Ansi(ansi) => ansi,
            Color::Rgb(r, g, b) => rgb_to_ansi(r, g, b),
        }
    }
    /// Converts to 24-bit (r, g, b) tuple.
    pub const fn to_24b(self) -> (u8, u8, u8) {
        match self {
            Color::Ansi(ansi) => rgb_from_8bit(ansi),
            Color::Rgb(r, g, b) => (r, g, b),
        }
    }
    /// Returns true if this color is an ANSI indexed color.
    pub const fn is_ansi(&self) -> bool {
        matches!(self, Color::Ansi(_))
    }
}

/// Convert 256*256*256 RGB to 4bit ANSI color 0-15
const fn rgb_to_4bit(r: u8, g: u8, b: u8) -> u8 {
    const fn bits(c: u8) -> (u8, u8) {
        match c {
            230.. => (1, 1),
            180.. => (1, 0),
            80.. => (0, 1),
            _ => (0, 0),
        }
    }

    // get high and mid bits for each color
    let ((hr, r), (hg, g), (hb, b)) = (bits(r), bits(g), bits(b));
    // combine into 4 bits
    let v: u8 = r | hr | (g << 1) | (hg << 1) | (b << 2) | (hb << 2) | ((hr | hg | hb) << 3);

    match v {
        0b1111 if r & g & b == 0 => 7, // silver (white) special case
        0b0111 => 8,                   // grey (light black) special case
        0b0000..=0b1111 => v,          // well parsed color
        _ => 15,                       //unreachable!("Only 4 bits are possible"),
    }
}

const fn ansi_to_4b(ansi: u8) -> u8 {
    if ansi < 16 {
        ansi
    } else {
        let (r, g, b) = rgb_from_8bit(ansi);
        rgb_to_4bit(r, g, b)
    }
}

const fn rgb_to_ansi(r: u8, g: u8, b: u8) -> u8 {
    let c = rgb_to_4bit(r, g, b);
    let (pr, pg, pb) = rgb_from_8bit(c);
    if r == pr && g == pg && b == pb {
        // this is one of the base colors
        c
    } else if r == g && g == b {
        c256_to_ansi_grey(r)
    } else {
        rgb_to_ansi_idx(r, g, b)
    }
}
const fn c256_to_ansi_grey(c: u8) -> u8 {
    match 232u8.checked_add(c / 10) {
        Some(c) => c,
        None => 255,
    }
}
const fn rgb_to_ansi_idx(r: u8, g: u8, b: u8) -> u8 {
    16 + 36 * (r / 51) + 6 * (g / 51) + (b / 51)
}
const fn rgb_from_8bit(ansi: u8) -> (u8, u8, u8) {
    let c = COLOR_256[ansi as usize];
    ((c >> 16) as u8, (c >> 8) as u8, c as u8)
}
const COLOR_256: [u32; 256] = [
    0x000000, 0x800000, 0x008000, 0x808000, 0x0000EE, 0x800080, 0x008080, 0xc0c0c0, // darks
    0x808080, 0xff6600, 0x00ff00, 0xffff00, 0x6699ff, 0xff00ff, 0x00ffff, 0xffffff, // brights
    0x000000, 0x00005f, 0x000087, 0x0000af, 0x0000d7, 0x0000ff, 0x005f00, 0x005f5f, // 0x0F
    0x005f87, 0x005faf, 0x005fd7, 0x005fff, 0x008700, 0x00875f, 0x008787, 0x0087af, // colors
    0x0087d7, 0x0087ff, 0x00af00, 0x00af5f, 0x00af87, 0x00afaf, 0x00afd7, 0x00afff, // colors
    0x00d700, 0x00d75f, 0x00d787, 0x00d7af, 0x00d7d7, 0x00d7ff, 0x00ff00, 0x00ff5f, // colors
    0x00ff87, 0x00ffaf, 0x00ffd7, 0x00ffff, 0x5f0000, 0x5f005f, 0x5f0087, 0x5f00af, // colors
    0x5f00d7, 0x5f00ff, 0x5f5f00, 0x5f5f5f, 0x5f5f87, 0x5f5faf, 0x5f5fd7, 0x5f5fff, // colors
    0x5f8700, 0x5f875f, 0x5f8787, 0x5f87af, 0x5f87d7, 0x5f87ff, 0x5faf00, 0x5faf5f, // colors
    0x5faf87, 0x5fafaf, 0x5fafd7, 0x5fafff, 0x5fd700, 0x5fd75f, 0x5fd787, 0x5fd7af, // colors
    0x5fd7d7, 0x5fd7ff, 0x5fff00, 0x5fff5f, 0x5fff87, 0x5fffaf, 0x5fffd7, 0x5fffff, // colors
    0x870000, 0x87005f, 0x870087, 0x8700af, 0x8700d7, 0x8700ff, 0x875f00, 0x875f5f, // colors
    0x875f87, 0x875faf, 0x875fd7, 0x875fff, 0x878700, 0x87875f, 0x878787, 0x8787af, // colors
    0x8787d7, 0x8787ff, 0x87af00, 0x87af5f, 0x87af87, 0x87afaf, 0x87afd7, 0x87afff, // colors
    0x87d700, 0x87d75f, 0x87d787, 0x87d7af, 0x87d7d7, 0x87d7ff, 0x87ff00, 0x87ff5f, // colors
    0x87ff87, 0x87ffaf, 0x87ffd7, 0x87ffff, 0xaf0000, 0xaf005f, 0xaf0087, 0xaf00af, // colors
    0xaf00d7, 0xaf00ff, 0xaf5f00, 0xaf5f5f, 0xaf5f87, 0xaf5faf, 0xaf5fd7, 0xaf5fff, // colors
    0xaf8700, 0xaf875f, 0xaf8787, 0xaf87af, 0xaf87d7, 0xaf87ff, 0xafaf00, 0xafaf5f, // colors
    0xafaf87, 0xafafaf, 0xafafd7, 0xafafff, 0xafd700, 0xafd75f, 0xafd787, 0xafd7af, // colors
    0xafd7d7, 0xafd7ff, 0xafff00, 0xafff5f, 0xafff87, 0xafffaf, 0xafffd7, 0xafffff, // colors
    0xd70000, 0xd7005f, 0xd70087, 0xd700af, 0xd700d7, 0xd700ff, 0xd75f00, 0xd75f5f, // colors
    0xd75f87, 0xd75faf, 0xd75fd7, 0xd75fff, 0xd78700, 0xd7875f, 0xd78787, 0xd787af, // colors
    0xd787d7, 0xd787ff, 0xd7af00, 0xd7af5f, 0xd7af87, 0xd7afaf, 0xd7afd7, 0xd7afff, // colors
    0xd7d700, 0xd7d75f, 0xd7d787, 0xd7d7af, 0xd7d7d7, 0xd7d7ff, 0xd7ff00, 0xd7ff5f, // colors
    0xd7ff87, 0xd7ffaf, 0xd7ffd7, 0xd7ffff, 0xff0000, 0xff005f, 0xff0087, 0xff00af, // colors
    0xff00d7, 0xff00ff, 0xff5f00, 0xff5f5f, 0xff5f87, 0xff5faf, 0xff5fd7, 0xff5fff, // colors
    0xff8700, 0xff875f, 0xff8787, 0xff87af, 0xff87d7, 0xff87ff, 0xffaf00, 0xffaf5f, // colors
    0xffaf87, 0xffafaf, 0xffafd7, 0xffafff, 0xffd700, 0xffd75f, 0xffd787, 0xffd7af, // colors
    0xffd7d7, 0xffd7ff, 0xffff00, 0xffff5f, 0xffff87, 0xffffaf, 0xffffd7, 0xffffff, // colors
    0x080808, 0x121212, 0x1c1c1c, 0x262626, 0x303030, 0x3a3a3a, 0x444444, 0x4e4e4e, // E8
    0x585858, 0x626262, 0x6c6c6c, 0x767676, 0x808080, 0x8a8a8a, 0x949494, 0x9e9e9e, // greys
    0xa8a8a8, 0xb2b2b2, 0xbcbcbc, 0xc6c6c6, 0xd0d0d0, 0xdadada, 0xe4e4e4, 0xeeeeee, // greys
];

#[test]
fn same_ansi() {
    for (i, c) in COLOR_256.iter().take(16).cloned().enumerate() {
        match i {
            // blues and bright red is elevated
            4 | 9 | 12 => continue,
            _ => {}
        }
        assert_eq!(
            Color::ansi(i as u8).to_24b(),
            ((c >> 16) as u8, (c >> 8) as u8, c as u8,),
            "for {i} - {c:06X}"
        )
    }
}

#[test]
fn test_rgb_to_4bs() {
    assert!(rgb_to_4bit(255, 255, 255) == 15, "white");
    assert!(rgb_to_4bit(250, 250, 250) == 15, "white, almost");
    assert!(rgb_to_4bit(241, 242, 243) == 15, "white, approx");

    assert!(rgb_to_4bit(0, 0, 0) == 0, "black");
    assert!(rgb_to_4bit(1, 2, 3) == 0, "black, approx");

    assert!(rgb_to_4bit(170, 170, 170) == 8, "grey, lighter");
    assert!(rgb_to_4bit(80, 80, 80) == 8, "grey, darker");
    assert!(rgb_to_4bit(81, 82, 83) == 8, "grey, approx");

    assert!(rgb_to_4bit(200, 200, 200) == 7, "silver");
    assert!(rgb_to_4bit(201, 202, 203) == 7, "silver approx");

    assert_eq!(rgb_to_4bit(170, 0, 0), 1, "dark red");
    assert_eq!(rgb_to_4bit(250, 0, 0), 9, "bright red");

    assert_eq!(rgb_to_4bit(0, 170, 0), 2, "dark green");
    assert_eq!(rgb_to_4bit(0, 250, 0), 10, "bright green");

    assert_eq!(rgb_to_4bit(0, 0, 170), 4, "dark blue");
    assert_eq!(rgb_to_4bit(0, 0, 250), 12, "bright blue");

    assert_eq!(rgb_to_4bit(170, 170, 0), 3, "dark yellow");
    assert_eq!(rgb_to_4bit(250, 250, 0), 11, "bright yellow");

    assert_eq!(rgb_to_4bit(170, 0, 170), 5, "dark magenta");
    assert_eq!(rgb_to_4bit(250, 0, 250), 13, "bright magenta");

    assert_eq!(rgb_to_4bit(0, 170, 170), 6, "dark cyan");
    assert_eq!(rgb_to_4bit(0, 250, 250), 14, "bright cyan");
}

#[test]
fn explore() {
    assert_eq!((0xabcdef >> 16) as u8, 0xab);
    assert_eq!((0xabcdef >> 8) as u8, 0xcd);
}
