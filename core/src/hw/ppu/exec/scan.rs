use std::fmt::Display;

use remus::Device;

use super::draw::Draw;
use super::hblank::HBlank;
use super::sprite::Sprite;
use super::vblank::VBlank;
use super::{Lcdc, Mode, Ppu};

#[derive(Debug, Default)]
pub struct Scan {
    pub(super) idx: usize,
    pub(super) objs: Vec<Sprite>,
}

impl Scan {
    pub fn exec(mut self, ppu: &mut Ppu) -> Mode {
        // Extract the sprite and scanline config
        let regs = ppu.ctl.borrow();
        let lcdc = **regs.lcdc.borrow();
        let size = Lcdc::ObjSize.get(&lcdc);
        let ht = [8, 16][size as usize];
        let ly = **regs.ly.borrow();

        // Scan should only run when the following conditions are met:
        // - Sprites are enabled
        // - It is currently an "on" dot (as Scan runs at 2 MiHz)
        // - Fewer than 10 sprites have been found per scanline
        if Lcdc::ObjEnable.get(&lcdc) && ppu.dot % 2 == 0 && self.objs.len() < 10 {
            // Checking an OAM entry takes 4 dots, due to the read. This means
            // we need another clock divider to disable during the "off" dot.
            if ppu.dot % 4 == 0 {
                // Scan the current OAM entry
                let mut obj = [0; 4];
                obj.iter_mut().for_each(|byte| {
                    *byte = ppu.oam.borrow().read(self.idx);
                    self.idx += 1;
                });
                // Parse entry into Sprite
                let obj = Sprite::from(obj);
                // Add sprite to be rendered if it's on the current scanline
                if obj.xpos != 0 && (obj.ypos..=obj.ypos + ht).contains(&(ly + 16)) {
                    self.objs.push(obj);
                }
            }
        } else {
            // Regardless, move to next OAM entry
            // NOTE: We're incrementing by 2 here, since the PPU has a 16-bit
            //       wide bus to the OAM, allowing it to access one word (2
            //       bytes) per dot.
            // TODO: citation needed
            self.idx += 2;
        }
        drop(regs);

        // Scan lasts 80 dots, then progresses to Draw
        ppu.dot += 1;
        if ppu.dot < 80 {
            Mode::Scan(self)
        } else {
            let mut draw = self.into();
            Draw::setup(&mut draw, ppu);
            Mode::Draw(draw)
        }
    }
}

impl Display for Scan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "┌─────────────┐")?;
        writeln!(f, "│ {:^11} │", "Scan")?;
        writeln!(f, "├─────────────┤")?;
        writeln!(f, "│ Sprite: {:>3} │", self.idx)?;
        writeln!(f, "│ Found: {:>4} │", self.objs.len())?;
        write!(f, "└─────────────┘")
    }
}

impl From<HBlank> for Scan {
    fn from(HBlank { .. }: HBlank) -> Self {
        Self {
            ..Default::default()
        }
    }
}

impl From<VBlank> for Scan {
    fn from(VBlank { .. }: VBlank) -> Self {
        Self {
            ..Default::default()
        }
    }
}
