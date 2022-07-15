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
    #[must_use]
    pub fn stage(&self) -> &Stage {
        &self.stage
    }

    pub fn exec(&mut self, fifo: &mut Fifo, ppu: &mut Ppu, loc: Location) {
        // NOTE: Some stages of the fetch take 2 dots to complete, so only
        //       execute when we're not already busy from the current stage.
        if self.busy {
            self.busy = false;
        } else {
            self.stage = std::mem::take(&mut self.stage).exec(self, fifo, ppu, loc);
        }
    }

    fn tnum(&self, ppu: &Ppu, loc: Location) -> u16 {
        use Location::*;

        // Extract scanline info
        let regs = ppu.ctl.borrow();
        let lcdc = **regs.lcdc.borrow();
        let scy = **regs.scy.borrow();
        let scx = **regs.scx.borrow();
        let ly = **regs.ly.borrow();

        // Determine the tile base
        let base = match loc {
            Background => {
                let bgmap = Lcdc::BgMap.get(&lcdc);
                [0x1800, 0x1c00][bgmap as usize]
            }
            Window => {
                let winmap = Lcdc::WinMap.get(&lcdc);
                [0x1800, 0x1c00][winmap as usize]
            }
            Sprite => todo!(),
        };

        // Calculate the tile offset
        let trow: u8;
        let tcol: u8;
        match loc {
            Background => {
                trow = ly.wrapping_add(scy) / 8;
                tcol = (self.tcol + (scx / 8)) & 0x1f;
            }
            Window => {
                trow = ppu.winln / 8;
                tcol = self.tcol;
            }
            Sprite => todo!(),
        }
        let offset = (32 * trow as u16) + tcol as u16;

        // Calculate the tile number
        base + offset
    }

    fn addr(&self, ppu: &Ppu, loc: Location, tidx: u8) -> u16 {
        use Location::*;

        // Extract scanline info
        let regs = ppu.ctl.borrow();
        let lcdc = **regs.lcdc.borrow();
        let scy = **regs.scy.borrow();
        let ly = **regs.ly.borrow();

        // Calculate the y-offset within the tile
        let yoff = match loc {
            Background => ly.wrapping_add(scy) % 8,
            Window => ppu.winln % 8,
            Sprite => todo!(),
        };

        // Calculate the tile data address
        match loc {
            Background | Window => {
                if Lcdc::BgWinData.get(&lcdc) {
                    let base = 0x0000;
                    let tidx = tidx as u16;
                    let offset = (16 * tidx) + (2 * yoff) as u16;
                    base + offset
                } else {
                    let base = 0x1000;
                    let tidx = tidx as i8 as i16;
                    let offset = (16 * tidx) as i16 + (2 * yoff) as i16;
                    (base + offset) as u16
                }
            }
            Sprite => {
                let base = 0x0000;
                let tidx = tidx as u16;
                let offset = (16 * tidx) + (2 * yoff) as u16;
                base + offset
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum Location {
    #[default]
    Background,
    Window,
    Sprite,
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
    fn exec(self, fetch: &mut Fetch, fifo: &mut Fifo, ppu: &mut Ppu, loc: Location) -> Self {
        match self {
            Stage::ReadTile => {
                // Calculate the tile number
                // NOTE: How this is calculated depends on the type of the tile
                //       being fetched.
                let tnum = fetch.tnum(ppu, loc);

                // Fetch the tile number index
                use Location::*;
                let tidx = match loc {
                    Background | Window => ppu.vram.borrow().read(tnum as usize),
                    Sprite => todo!(),
                };

                // NOTE: We can calculate the tile data address in advance. This
                //       is more efficient than doing so once each data read.
                let addr = fetch.addr(ppu, loc, tidx);

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
