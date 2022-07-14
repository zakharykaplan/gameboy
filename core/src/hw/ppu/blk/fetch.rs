use remus::Device;

use super::fifo::{Fifo, TileRow};
use super::{Lcdc, Ppu};

#[derive(Debug, Default)]
pub struct Fetch {
    busy: bool,
    tcol: u8,
    stage: Stage,
}

impl Fetch {
    pub fn exec(&mut self, fifo: &mut Fifo, ppu: &mut Ppu) {
        // NOTE: Some stages of the fetch take 2 dots to complete, so only
        //       execute when we're not already busy from the current stage.
        if self.busy {
            self.busy = false;
        } else {
            self.stage = std::mem::take(&mut self.stage).exec(self, fifo, ppu);
        }
    }
}

#[derive(Debug, Default)]
pub enum Stage {
    #[default]
    ReadTile,
    ReadData0 {
        addr: u16,
    },
    ReadData1 {
        addr: u16,
        data0: u8,
    },
    Push(TileRow),
}

impl Stage {
    fn exec(self, fetch: &mut Fetch, fifo: &mut Fifo, ppu: &mut Ppu) -> Self {
        match self {
            Stage::ReadTile => {
                // Extract scanline info
                let regs = ppu.ctl.borrow();
                let lcdc = **regs.lcdc.borrow();
                let scy = **regs.scy.borrow();
                let scx = **regs.scx.borrow();
                let ly = **regs.ly.borrow();

                // Calculate the tile number (i.e. tile data index)
                // NOTE: How this is calculated depends on the type of the tile
                //       being fetched.
                let tidx = {
                    // Offset the tile column
                    let trow = (ly.wrapping_add(scy) / 8) as u16;
                    let tcol = ((fetch.tcol + (scx / 8)) & 0x1f) as u16;

                    // Combine tile row and column to get index
                    (32 * trow) + tcol
                };

                // Calculate the tile number base
                let base = {
                    let bgmap = Lcdc::BgMap.get(&lcdc);
                    [0x1800, 0x1c00][bgmap as usize]
                };

                // Calculate the tile number address
                let addr = base + tidx;

                // Fetch the tile data index
                let tile = ppu.vram.borrow().read(addr as usize);

                // FIXME: This is cheating
                // Calculate the y-index of row within the tile
                let yoff = scy.wrapping_add(ly) % 8;
                let addr = if Lcdc::BgWinData.get(&lcdc) {
                    let base = 0x0000;
                    let tile = tile as u16;
                    let offset = (16 * tile) + (2 * yoff) as u16;
                    base + offset
                } else {
                    let base = 0x1000;
                    let tile = tile as i8 as i16;
                    let offset = (16 * tile) as i16 + (2 * yoff) as i16;
                    (base + offset) as u16
                };

                // Progress to next stage
                Stage::ReadData0 { addr }
            }
            Stage::ReadData0 { addr } => {
                // Fetch the first byte of the tile
                let data0 = ppu.vram.borrow().read(addr as usize);

                // Progress to next stage
                let addr = addr + 1;
                Stage::ReadData1 { addr, data0 }
            }
            Stage::ReadData1 { addr, data0 } => {
                // Fetch the seocnd byte of the tile
                let data1 = ppu.vram.borrow().read(addr as usize);

                // Decode pixels from data
                let row = TileRow::from([data0, data1]);

                // Progress to next stage
                Stage::Push(row)
            }
            Stage::Push(row) => {
                // Attempt to push row to FIFO
                match fifo.try_append(row) {
                    Ok(()) => {
                        // Move fetch to next x-position
                        fetch.tcol += 1;
                        // Restart fetch
                        Stage::ReadTile
                    }
                    Err(row) => {
                        // Try again next cycle
                        Stage::Push(row)
                    }
                }
            }
        }
    }
}
