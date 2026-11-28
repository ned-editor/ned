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
    /// 24-bit RGB `TrueColor`.
    Rgb(u8, u8, u8),
}

impl Color {
    /// Creates an ANSI indexed color.
    #[must_use]
    pub const fn ansi(ansi: u8) -> Self {
        Color::Ansi(ansi)
    }
    /// Creates an RGB `TrueColor`.
    #[must_use]
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Color::Rgb(red, green, blue)
    }
    /// Standard black color.
    #[must_use]
    pub const fn black() -> Self {
        Self::ansi(0)
    }
    /// Standard white color.
    #[must_use]
    pub const fn white() -> Self {
        Self::ansi(15)
    }
    /// Grey color with optional light intensity.
    #[must_use]
    pub const fn grey(light: bool) -> Self {
        Self::ansi(if light { 7 } else { 8 })
    }
    /// Red color with optional light intensity.
    #[must_use]
    pub const fn red(light: bool) -> Self {
        Self::ansi(if light { 1 + 8 } else { 1 })
    }
    /// Green color with optional light intensity.
    #[must_use]
    pub const fn green(light: bool) -> Self {
        Self::ansi(if light { 2 + 8 } else { 2 })
    }
    /// Blue color with optional light intensity.
    #[must_use]
    pub const fn blue(light: bool) -> Self {
        Self::ansi(if light { 4 + 8 } else { 4 })
    }
    /// Yellow color with optional light intensity.
    #[must_use]
    pub const fn yellow(light: bool) -> Self {
        Self::ansi(if light { 3 + 8 } else { 3 })
    }
    /// Magenta color with optional light intensity.
    #[must_use]
    pub const fn magenta(light: bool) -> Self {
        Self::ansi(if light { 5 + 8 } else { 5 })
    }
    /// Cyan color with optional light intensity.
    #[must_use]
    pub const fn cyan(light: bool) -> Self {
        Self::ansi(if light { 6 + 8 } else { 6 })
    }

    /// Converts to 3-bit color value.
    #[must_use]
    pub const fn to_3b(self) -> u8 {
        self.to_4b() % 8
    }
    /// Converts to 4-bit color value (0-15).
    #[must_use]
    pub const fn to_4b(self) -> u8 {
        match self {
            Color::Ansi(ansi) => ansi_to_4b(ansi),
            Color::Rgb(r, g, b) => rgb_to_4bit(r, g, b),
        }
    }
    /// Converts to 8-bit ANSI palette index (0-255).
    #[must_use]
    pub const fn to_8b(self) -> u8 {
        match self {
            Color::Ansi(ansi) => ansi,
            Color::Rgb(r, g, b) => rgb_to_ansi(r, g, b),
        }
    }
    /// Converts to 24-bit (r, g, b) tuple.
    #[must_use]
    pub const fn to_24b(self) -> (u8, u8, u8) {
        match self {
            Color::Ansi(ansi) => rgb_from_8bit(ansi),
            Color::Rgb(r, g, b) => (r, g, b),
        }
    }
    /// Returns true if this color is an ANSI indexed color.
    #[must_use]
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
    0x00_00_00, 0x80_00_00, 0x00_80_00, 0x80_80_00, 0x00_00_EE, 0x80_00_80, 0x00_80_80, 0xc0_c0_c0, // darks
    0x80_80_80, 0xff_66_00, 0x00_ff_00, 0xff_ff_00, 0x66_99_ff, 0xff_00_ff, 0x00_ff_ff, 0xff_ff_ff, // brights
    0x00_00_00, 0x00_00_5f, 0x00_00_87, 0x00_00_af, 0x00_00_d7, 0x00_00_ff, 0x00_5f_00, 0x00_5f_5f, // 0x0F
    0x00_5f_87, 0x00_5f_af, 0x00_5f_d7, 0x00_5f_ff, 0x00_87_00, 0x00_87_5f, 0x00_87_87, 0x00_87_af, // colors
    0x00_87_d7, 0x00_87_ff, 0x00_af_00, 0x00_af_5f, 0x00_af_87, 0x00_af_af, 0x00_af_d7, 0x00_af_ff, // colors
    0x00_d7_00, 0x00_d7_5f, 0x00_d7_87, 0x00_d7_af, 0x00_d7_d7, 0x00_d7_ff, 0x00_ff_00, 0x00_ff_5f, // colors
    0x00_ff_87, 0x00_ff_af, 0x00_ff_d7, 0x00_ff_ff, 0x5f_00_00, 0x5f_00_5f, 0x5f_00_87, 0x5f_00_af, // colors
    0x5f_00_d7, 0x5f_00_ff, 0x5f_5f_00, 0x5f_5f_5f, 0x5f_5f_87, 0x5f_5f_af, 0x5f_5f_d7, 0x5f_5f_ff, // colors
    0x5f_87_00, 0x5f_87_5f, 0x5f_87_87, 0x5f_87_af, 0x5f_87_d7, 0x5f_87_ff, 0x5f_af_00, 0x5f_af_5f, // colors
    0x5f_af_87, 0x5f_af_af, 0x5f_af_d7, 0x5f_af_ff, 0x5f_d7_00, 0x5f_d7_5f, 0x5f_d7_87, 0x5f_d7_af, // colors
    0x5f_d7_d7, 0x5f_d7_ff, 0x5f_ff_00, 0x5f_ff_5f, 0x5f_ff_87, 0x5f_ff_af, 0x5f_ff_d7, 0x5f_ff_ff, // colors
    0x87_00_00, 0x87_00_5f, 0x87_00_87, 0x87_00_af, 0x87_00_d7, 0x87_00_ff, 0x87_5f_00, 0x87_5f_5f, // colors
    0x87_5f_87, 0x87_5f_af, 0x87_5f_d7, 0x87_5f_ff, 0x87_87_00, 0x87_87_5f, 0x87_87_87, 0x87_87_af, // colors
    0x87_87_d7, 0x87_87_ff, 0x87_af_00, 0x87_af_5f, 0x87_af_87, 0x87_af_af, 0x87_af_d7, 0x87_af_ff, // colors
    0x87_d7_00, 0x87_d7_5f, 0x87_d7_87, 0x87_d7_af, 0x87_d7_d7, 0x87_d7_ff, 0x87_ff_00, 0x87_ff_5f, // colors
    0x87_ff_87, 0x87_ff_af, 0x87_ff_d7, 0x87_ff_ff, 0xaf_00_00, 0xaf_00_5f, 0xaf_00_87, 0xaf_00_af, // colors
    0xaf_00_d7, 0xaf_00_ff, 0xaf_5f_00, 0xaf_5f_5f, 0xaf_5f_87, 0xaf_5f_af, 0xaf_5f_d7, 0xaf_5f_ff, // colors
    0xaf_87_00, 0xaf_87_5f, 0xaf_87_87, 0xaf_87_af, 0xaf_87_d7, 0xaf_87_ff, 0xaf_af_00, 0xaf_af_5f, // colors
    0xaf_af_87, 0xaf_af_af, 0xaf_af_d7, 0xaf_af_ff, 0xaf_d7_00, 0xaf_d7_5f, 0xaf_d7_87, 0xaf_d7_af, // colors
    0xaf_d7_d7, 0xaf_d7_ff, 0xaf_ff_00, 0xaf_ff_5f, 0xaf_ff_87, 0xaf_ff_af, 0xaf_ff_d7, 0xaf_ff_ff, // colors
    0xd7_00_00, 0xd7_00_5f, 0xd7_00_87, 0xd7_00_af, 0xd7_00_d7, 0xd7_00_ff, 0xd7_5f_00, 0xd7_5f_5f, // colors
    0xd7_5f_87, 0xd7_5f_af, 0xd7_5f_d7, 0xd7_5f_ff, 0xd7_87_00, 0xd7_87_5f, 0xd7_87_87, 0xd7_87_af, // colors
    0xd7_87_d7, 0xd7_87_ff, 0xd7_af_00, 0xd7_af_5f, 0xd7_af_87, 0xd7_af_af, 0xd7_af_d7, 0xd7_af_ff, // colors
    0xd7_d7_00, 0xd7_d7_5f, 0xd7_d7_87, 0xd7_d7_af, 0xd7_d7_d7, 0xd7_d7_ff, 0xd7_ff_00, 0xd7_ff_5f, // colors
    0xd7_ff_87, 0xd7_ff_af, 0xd7_ff_d7, 0xd7_ff_ff, 0xff_00_00, 0xff_00_5f, 0xff_00_87, 0xff_00_af, // colors
    0xff_00_d7, 0xff_00_ff, 0xff_5f_00, 0xff_5f_5f, 0xff_5f_87, 0xff_5f_af, 0xff_5f_d7, 0xff_5f_ff, // colors
    0xff_87_00, 0xff_87_5f, 0xff_87_87, 0xff_87_af, 0xff_87_d7, 0xff_87_ff, 0xff_af_00, 0xff_af_5f, // colors
    0xff_af_87, 0xff_af_af, 0xff_af_d7, 0xff_af_ff, 0xff_d7_00, 0xff_d7_5f, 0xff_d7_87, 0xff_d7_af, // colors
    0xff_d7_d7, 0xff_d7_ff, 0xff_ff_00, 0xff_ff_5f, 0xff_ff_87, 0xff_ff_af, 0xff_ff_d7, 0xff_ff_ff, // colors
    0x08_08_08, 0x12_12_12, 0x1c_1c_1c, 0x26_26_26, 0x30_30_30, 0x3a_3a_3a, 0x44_44_44, 0x4e_4e_4e, // E8
    0x58_58_58, 0x62_62_62, 0x6c_6c_6c, 0x76_76_76, 0x80_80_80, 0x8a_8a_8a, 0x94_94_94, 0x9e_9e_9e, // greys
    0xa8_a8_a8, 0xb2_b2_b2, 0xbc_bc_bc, 0xc6_c6_c6, 0xd0_d0_d0, 0xda_da_da, 0xe4_e4_e4, 0xee_ee_ee, // greys
];

#[test]
fn same_ansi() {
    for (i, c) in COLOR_256.iter().take(16).copied().enumerate() {
        match i {
            // blues and bright red is elevated
            4 | 9 | 12 => continue,
            _ => {}
        }
        assert_eq!(
            Color::ansi(i as u8).to_24b(),
            ((c >> 16) as u8, (c >> 8) as u8, c as u8,),
            "for {i} - {c:06X}"
        );
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
    assert_eq!((0xab_cd_ef >> 16) as u8, 0xab);
    assert_eq!((0xab_cd_ef >> 8) as u8, 0xcd);
}
