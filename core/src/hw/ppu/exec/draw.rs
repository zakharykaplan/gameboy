use std::fmt::Display;

use super::blk::Pipeline;
use super::sprite::Sprite;
use super::{Mode, Ppu, Scan, SCREEN};

#[derive(Debug, Default)]
pub struct Draw {
    pixels: Pipeline,
    objs: Vec<Sprite>,
}

impl Draw {
    pub fn setup(&mut self, ppu: &mut Ppu) {
        // Set up the pipeline
        let scx = **ppu.ctl.borrow().scx.borrow();
        self.pixels.set_discard(scx);
    }

    pub fn exec(mut self, ppu: &mut Ppu) -> Mode {
        // Execute the next fetch cycle
        self.pixels.fetch(ppu);

        // If we have a pixel to draw, draw it
        let xpos = self.pixels.xpos() as usize;
        if let Some(pixel) = self.pixels.shift() {
            // Calculate pixel index on screen
            let ypos = **ppu.ctl.borrow().ly.borrow() as usize;
            let idx = (ypos * SCREEN.width) + xpos;

            // Determine this pixel's color (according to its palette)
            let color = ppu.color(pixel);

            // Write the pixel into the framebuffer
            ppu.lcd[idx] = color;
        }

        // Retrieve updated x-position
        let xpos = self.pixels.xpos() as usize;

        // Move to the next dot
        ppu.dot += 1;

        // Either draw next pixel, or enter HBlank
        if xpos < SCREEN.width {
            Mode::Draw(self)
        } else {
            Mode::HBlank(Default::default())
        }
    }
}

impl Display for Draw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "┌─────────────┐")?;
        writeln!(f, "│ {:^11} │", "Draw")?;
        writeln!(f, "├─────────────┤")?;
        writeln!(f, "│ Column: {:>3} │", self.pixels.xpos())?;
        writeln!(f, "│ Objects: {:>2} │", self.objs.len())?;
        write!(f, "└─────────────┘")
    }
}

impl From<Scan> for Draw {
    fn from(scan: Scan) -> Self {
        let Scan { objs, .. } = scan;
        Self {
            objs,
            ..Default::default()
        }
    }
}
