use std::fmt::Display;

use super::draw::Draw;
use super::{Interrupt, Mode, Ppu, SCREEN};

#[derive(Debug, Default)]
pub struct HBlank;

impl HBlank {
    pub fn exec(self, ppu: &mut Ppu) -> Mode {
        // HBlank lasts until the 456th dot
        ppu.dot += 1;
        if ppu.dot < 456 {
            Mode::HBlank(self)
        } else {
            // Extract scanline config
            let regs = ppu.ctl.borrow();
            let ly = &mut **regs.ly.borrow_mut();
            // Increment scanline at the 456th dot, and reset dot-clock
            *ly += 1;
            ppu.dot = 0;

            // Schedule VBlank interrupt
            ppu.pic.borrow_mut().req(Interrupt::VBlank);

            // Either begin next scanline, or enter VBlank
            if *ly < SCREEN.height as u8 {
                Mode::Scan(self.into())
            } else {
                // Reset internal window line counter
                ppu.winln = 0;
                Mode::VBlank(Default::default())
            }
        }
    }
}

impl Display for HBlank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "┌─────────────┐")?;
        writeln!(f, "│ {:^11} │", "HBlank")?;
        write!(f, "└─────────────┘")
    }
}

impl From<Draw> for HBlank {
    fn from(Draw { .. }: Draw) -> Self {
        Self
    }
}
