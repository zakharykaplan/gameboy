use std::collections::VecDeque;
use std::ops::Deref;

use super::pixel::{Palette, Pixel};

#[derive(Debug, Default)]
pub struct Fifo(VecDeque<Pixel>);

impl Fifo {
    pub fn try_append(&mut self, row: TileRow) -> Result<(), TileRow> {
        if self.0.is_empty() {
            self.0.append(&mut row.0.into());
            Ok(())
        } else {
            Err(row)
        }
    }

    pub fn pop(&mut self) -> Option<Pixel> {
        self.0.pop_front()
    }
}

#[derive(Debug)]
pub struct TileRow([Pixel; 8]);

impl Deref for TileRow {
    type Target = [Pixel; 8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<[u8; 2]> for TileRow {
    fn from(bytes: [u8; 2]) -> Self {
        // Iterate through each bit of both bytes
        let pixels = (0..u8::BITS)
            // Combine upper and lower bytes into colors for each bit
            .map(|bit| {
                // Extract color bits
                let mask = 0b1 << bit;
                let bit0 = bytes[0] & mask != 0;
                let bit1 = bytes[1] & mask != 0;
                // Combine into color value
                ((bit1 as u8) << 1) | (bit0 as u8)
            })
            // Reverse, since bit 7 represents the leftmost pixel, and bit 0 the
            // rightmost.
            .rev()
            // Convert into pixels
            .map(|col| Pixel {
                // FIXME: Properly handle `pal`, `bgp`
                col: col.try_into().unwrap(), // succeeds since values are 2-bit
                pal: Palette::BgWin,
                bgp: false,
            })
            // Collect into an array of [Pixel; 8]
            .collect::<Vec<Pixel>>()
            .try_into()
            .unwrap(); // this will succeed on any sane platform where there are
                       // 8 bits in a byte. We won't support whatever archaic
                       // (or futuristic) platform does otherwise. Sorry :P

        Self(pixels)
    }
}
