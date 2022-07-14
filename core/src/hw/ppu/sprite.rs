use super::pixel::Palette;

#[derive(Debug)]
pub struct Sprite {
    // Byte 0 - Y Position
    pub ypos: u8,
    // Byte 1 - X Position
    pub xpos: u8,
    // Byte 2 - Tile Index
    pub idx: u8,
    // Byte 3 - Attributes/Flags
    // Layout: 0b[Z, Y, X, P, 0000]
    // - P: palette
    // - X: x-flip
    // - Y: y-flip
    // - Z: priority
    pub priority: bool,
    pub yflip: bool,
    pub xflip: bool,
    pub palette: Palette,
}

impl From<[u8; 4]> for Sprite {
    #[rustfmt::skip]
    fn from(bytes: [u8; 4]) -> Self {
        Self {
            ypos:     bytes[0],
            xpos:     bytes[1],
            idx:      bytes[2],
            priority: bytes[3] & 0x08 != 0,
            yflip:    bytes[3] & 0x07 != 0,
            xflip:    bytes[3] & 0x06 != 0,
            palette: [
                Palette::Obj0,
                Palette::Obj1,
            ][(bytes[3] & 0x05 != 0) as usize],
        }
    }
}
