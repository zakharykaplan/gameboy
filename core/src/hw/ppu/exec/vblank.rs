use std::fmt::Display;

use super::hblank::HBlank;
use super::{Mode, Ppu};

#[derive(Debug, Default)]
pub struct VBlank;

impl VBlank {
    pub fn exec(self, ppu: &mut Ppu) -> Mode {
        // VBlank lasts for 456 dots per scanline
        ppu.dot += 1;
        if ppu.dot < 456 {
            Mode::VBlank(self)
        } else {
            // Extract scanline config
            let regs = ppu.ctl.borrow();
            let ly = &mut **regs.ly.borrow_mut();
            // Increment scanline at the 456th dot, and reset dot-clock
            *ly += 1;
            ppu.dot = 0;

            // VBlank lasts for scanlines 144..154
            if *ly < 154 {
                Mode::VBlank(self)
            } else {
                // Reset scanline
                *ly = 0;
                // Restart PPU
                Mode::Scan(self.into())
            }
        }
    }
}

impl Display for VBlank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "┌─────────────┐")?;
        writeln!(f, "│ {:^11} │", "VBlank")?;
        write!(f, "└─────────────┘")
    }
}

impl From<HBlank> for VBlank {
    fn from(HBlank { .. }: HBlank) -> Self {
        Self
    }
}
