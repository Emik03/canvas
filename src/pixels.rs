use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Write};

#[derive(Deserialize, Serialize)]
pub enum Pixel {
    White,
    LightGray,
    DarkGray,
    Black,
    Pink,
    Red,
    Orange,
    Brown,
    Yellow,
    Lime,
    Green,
    Cyan,
    Teal,
    Blue,
    Magenta,
    Purple,
}

impl Pixel {
    pub const fn to_byte(&self) -> u8 {
        self.to_char() as u8
    }

    pub const fn to_char(&self) -> char {
        match self {
            Self::White => '0',
            Self::LightGray => '1',
            Self::DarkGray => '2',
            Self::Black => '3',
            Self::Pink => '4',
            Self::Red => '5',
            Self::Orange => '6',
            Self::Brown => '7',
            Self::Yellow => '8',
            Self::Lime => '9',
            Self::Green => ':',
            Self::Cyan => ';',
            Self::Teal => '<',
            Self::Blue => '=',
            Self::Magenta => '>',
            Self::Purple => '?',
        }
    }
}

impl Display for Pixel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_char(self.to_char())
    }
}
