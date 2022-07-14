use std::fmt::Display;

use log::trace;

use self::draw::Draw;
use self::hblank::HBlank;
use self::scan::Scan;
use self::vblank::VBlank;
use super::{blk, sprite, Interrupt, Lcdc, Ppu, SCREEN};

mod draw;
mod hblank;
mod scan;
mod vblank;

#[derive(Debug)]
pub enum Mode {
    Scan(Scan),
    Draw(Draw),
    HBlank(HBlank),
    VBlank(VBlank),
}

impl Mode {
    pub fn exec(self, ppu: &mut Ppu) -> Self {
        // Handle previous state
        {
            // Update STAT
            let regs = ppu.ctl.borrow();
            let stat = &mut **regs.stat.borrow_mut();
            let ly = **regs.ly.borrow();
            let lyc = **regs.lyc.borrow();
            *stat ^= (*stat & 0x03) ^ u8::from(&self);
            *stat ^= (*stat & 0x04) ^ ((ly == lyc) as u8) << 2;

            // Trigger interrupts
            if ppu.dot == 0 {
                let mut int = 0;
                // LYC=LY
                int |= ((lyc == ly) as u8) << 6;
                // Mode 2
                int |= (matches!(self, Mode::Scan(_)) as u8) << 5;
                // Mode 1
                int |= (matches!(self, Mode::VBlank(_)) as u8) << 4;
                // Mode 0
                int |= (matches!(self, Mode::HBlank(_)) as u8) << 3;
                // Check for interrupts
                if int & (*stat & 0x78) != 0 {
                    ppu.pic.borrow_mut().req(Interrupt::LcdStat);
                }
            }
        }

        // Execute the current PPU mode
        trace!("PPU @ {:03}:\n{self}", ppu.dot);
        match self {
            Mode::Scan(scan) => scan.exec(ppu),
            Mode::Draw(draw) => draw.exec(ppu),
            Mode::HBlank(hblank) => hblank.exec(ppu),
            Mode::VBlank(vblank) => vblank.exec(ppu),
        }
    }
}

impl Default for Mode {
    fn default() -> Self {
        Self::Scan(Default::default())
    }
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scan(scan) => scan.fmt(f),
            Self::Draw(draw) => draw.fmt(f),
            Self::HBlank(hblank) => hblank.fmt(f),
            Self::VBlank(vblank) => vblank.fmt(f),
        }
    }
}

impl From<&Mode> for u8 {
    fn from(mode: &Mode) -> Self {
        match mode {
            Mode::Scan(_) => 0b10,
            Mode::Draw(_) => 0b11,
            Mode::HBlank(_) => 0b00,
            Mode::VBlank(_) => 0b01,
        }
    }
}
