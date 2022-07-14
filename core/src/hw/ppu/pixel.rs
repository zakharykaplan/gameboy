use thiserror::Error;
#[derive(Debug)]
pub struct Pixel {
    // FIXME: Remove `pub`s
    /// Color value.
    pub col: Color,
    /// Color palette.
    pub pal: Palette,
    /// Background priority.
    pub bgp: bool,
}

impl Pixel {
    #[must_use]
    pub fn col(&self) -> Color {
        self.col
    }

    #[must_use]
    pub fn pal(&self) -> Palette {
        self.pal
    }

    #[allow(clippy::if_same_then_else)]
    pub fn blend(winbg: Self, sprite: Self) -> Self {
        // Pixels are blended as follows:
        // - If the color number of the Sprite Pixel is 0, the Background Pixel
        //   is pushed to the LCD.
        if sprite.col == Color::C0 {
            winbg
        }
        // - If the BG-to-OBJ-Priority bit is 1 and the color number of the
        //   Background Pixel is anything other than 0, the Background Pixel is
        //   pushed to the LCD.
        else if sprite.bgp && winbg.col != Color::C0 {
            winbg
        }
        // - If none of the above conditions apply, the Sprite Pixel is pushed
        //   to the LCD.
        else {
            sprite
        }
        // <https://hacktix.github.io/GBEDG/ppu/#pixel-mixing>
    }
}

/// Pixel color values.
#[derive(Copy, Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum Color {
    /// Lightest
    #[default]
    C0 = 0b00,
    /// Light
    C1 = 0b01,
    /// Dark
    C2 = 0b10,
    /// Darkest
    C3 = 0b11,
}

impl Color {
    pub fn recolor(self, pal: u8) -> Self {
        Self::try_from((pal >> (2 * (self as u8))) & 0b11).unwrap()
    }
}

impl TryFrom<u8> for Color {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0b00 => Ok(Color::C0),
            0b01 => Ok(Color::C1),
            0b10 => Ok(Color::C2),
            0b11 => Ok(Color::C3),
            _ => Err(Error::Color),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Palette {
    BgWin,
    Obj0,
    Obj1,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("unknown color")]
    Color,
}
