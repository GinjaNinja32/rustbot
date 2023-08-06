#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Color {
    None = -1,
    BrightWhite,
    Black,
    Blue,
    Green,
    BrightRed,
    Red,
    Magenta,
    Yellow,
    BrightYellow,
    BrightGreen,
    Cyan,
    BrightCyan,
    BrightBlue,
    BrightMagenta,
    BrightBlack,
    White,
}

impl From<u8> for Color {
    fn from(v: u8) -> Self {
        match v {
            0 => Color::BrightWhite,
            1 => Color::Black,
            2 => Color::Blue,
            3 => Color::Green,
            4 => Color::BrightRed,
            5 => Color::Red,
            6 => Color::Magenta,
            7 => Color::Yellow,
            8 => Color::BrightYellow,
            9 => Color::BrightGreen,
            10 => Color::Cyan,
            11 => Color::BrightCyan,
            12 => Color::BrightBlue,
            13 => Color::BrightMagenta,
            14 => Color::BrightBlack,
            15 => Color::White,
            _ => Color::None,
        }
    }
}

impl std::ops::Add<Format> for Color {
    type Output = FormatColor;

    fn add(self, rhs: Format) -> FormatColor {
        FormatColor(rhs, self)
    }
}

// some hackery to get clippy to shut up about the casing of Format::None etc
pub use format::Format;
#[allow(clippy::module_inception)]
mod format {
    #![allow(non_upper_case_globals)]
    use bitflags::bitflags;
    bitflags! {
        pub struct Format: u8 {
            const None = 0x00;
            const Bold = 0x01;
            const Italic = 0x02;
            const Underline = 0x04;
        }
    }
}

impl std::ops::Add<Format> for Format {
    type Output = Format;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: Format) -> Format {
        self | rhs
    }
}

#[derive(Copy, Clone)]
pub struct FormatColor(pub Format, pub Color);

impl std::ops::Add<Format> for FormatColor {
    type Output = FormatColor;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: Format) -> FormatColor {
        FormatColor(self.0 | rhs, self.1)
    }
}

impl From<Format> for FormatColor {
    fn from(f: Format) -> Self {
        Self(f, Color::None)
    }
}

impl From<Color> for FormatColor {
    fn from(c: Color) -> Self {
        Self(Format::None, c)
    }
}
