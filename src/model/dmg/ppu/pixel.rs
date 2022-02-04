use std::ops::{Deref, DerefMut};

use remus::Device;

use super::Ppu;
use crate::model::dmg::ppu::Lcdc;

#[derive(Debug)]
pub struct Pixel {
    pub colour: Colour,
    pub palette: Palette,
}

#[derive(Copy, Clone, Debug)]
pub enum Colour {
    C0 = 0b00,
    C1 = 0b01,
    C2 = 0b10,
    C3 = 0b11,
}

impl Default for Colour {
    fn default() -> Self {
        Colour::C0
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Palette {
    BgWin,
    Obj0,
    Obj1,
}

#[derive(Debug, Default)]
pub struct Fifo(Vec<Pixel>);

impl Deref for Fifo {
    type Target = Vec<Pixel>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Fifo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Default)]
pub struct Fetch {
    stage: Stage,
    xpos: u8,
}

impl Fetch {
    pub fn exec(&mut self, fifo: &mut Fifo, ppu: &mut Ppu) {
        self.stage = std::mem::take(&mut self.stage).exec(self, fifo, ppu);
    }
}

#[derive(Debug)]
pub enum Stage {
    ReadTile,
    ReadData0 { tile: u16 },
    ReadData1 { tile: u16, data0: u8 },
    Push([Pixel; 8]),
}

impl Stage {
    fn exec(self, fetch: &mut Fetch, fifo: &mut Fifo, ppu: &mut Ppu) -> Self {
        match self {
            Stage::ReadTile => {
                // Extract scanline config
                let regs = ppu.regs.borrow();
                let lcdc = **regs.lcdc.borrow();
                let scy = **regs.scy.borrow();
                let scx = **regs.scx.borrow();
                let ly = **regs.ly.borrow();

                // Calculate index of the tile
                let idx = {
                    // Background tile
                    let bgmap = Lcdc::BgMap.get(&lcdc);
                    let base = [0x1800, 0x1c00][bgmap as usize];
                    let ypos = (scy.wrapping_add(ly) / 8) as u16;
                    let xpos = ((scx / 8).wrapping_add(fetch.xpos) & 0x1f) as u16;
                    base + (32 * ypos) + xpos
                };

                // Increment x-position to next tile
                fetch.xpos += 1;

                // Fetch the tile data index
                let tile = ppu.vram.borrow().read(idx as usize);

                // Calculate the y-index of row within the tile
                let yoff = scy.wrapping_add(ly) % 8;
                let tile = if Lcdc::BgWinData.get(&lcdc) {
                    let base = 0x0000;
                    let tile = tile as u16;
                    let offset = (16 * tile) + (2 * yoff) as u16;
                    base + offset
                } else {
                    let base = 0x0800;
                    let tile = tile as i8 as i16;
                    let offset = (16 * tile) as u16 + (2 * yoff) as u16;
                    base + offset
                };

                // Progress to next stage
                Stage::ReadData0 { tile }
            }
            Stage::ReadData0 { tile } => {
                // Fetch the first byte of the tile
                let data0 = ppu.vram.borrow().read(tile as usize);

                // Progress to next stage
                Stage::ReadData1 { tile, data0 }
            }
            Stage::ReadData1 { tile, data0 } => {
                // Fetch the seocnd byte of the tile
                let data1 = ppu.vram.borrow().read(tile as usize + 1);

                // Decode pixels from data
                let row = TileRow::from([data0, data1]).0;

                // Progress to next stage
                Stage::Push(row)
            }
            Stage::Push(row) => {
                // Push row to FIFO if there's space
                if fifo.len() <= 8 {
                    fifo.extend(row);
                    // Restart fetch
                    Stage::ReadTile
                } else {
                    // Try again next cycle
                    Stage::Push(row)
                }
            }
        }
    }
}

impl Default for Stage {
    fn default() -> Self {
        Self::ReadTile
    }
}

#[derive(Debug)]
pub struct TileRow([Pixel; 8]);

impl From<[u8; 2]> for TileRow {
    fn from(bytes: [u8; 2]) -> Self {
        Self(
            (0..u8::BITS)
                .map(|bit| Pixel {
                    colour: match (((bytes[0] & (0b1 << bit) != 0) as u8) << 1)
                        | ((bytes[1] & (0b1 << bit) != 0) as u8)
                    {
                        0b00 => Colour::C0,
                        0b01 => Colour::C1,
                        0b10 => Colour::C2,
                        0b11 => Colour::C3,
                        _ => unreachable!(),
                    },
                    palette: Palette::BgWin,
                })
                .collect::<Vec<Pixel>>()
                .try_into()
                .unwrap(),
        )
    }
}
