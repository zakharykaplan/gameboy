//! Picture processing unit.

use std::cell::RefCell;
use std::rc::Rc;

use remus::bus::Bus;
use remus::mem::Ram;
use remus::reg::Register;
use remus::{Block, Device, Machine};

use self::exec::Mode;
use self::pixel::{Color, Palette, Pixel};
use super::pic::{Interrupt, Pic};
use crate::dmg::SCREEN;

mod blk;
mod exec;
mod pixel;
mod screen;
mod sprite;

pub use self::screen::Screen;

/// PPU model.
#[rustfmt::skip]
#[derive(Debug, Default)]
pub struct Ppu {
    lcd: Screen,
    dot: usize,
    mode: Mode,
    pic: Rc<RefCell<Pic>>,
    // ┌────────┬──────────────────┬─────┬───────┐
    // │  SIZE  │       NAME       │ DEV │ ALIAS │
    // ├────────┼──────────────────┼─────┼───────┤
    // │ 8 Ki B │            Video │ RAM │ VRAM  │
    // │  160 B │ Object Attribute │ RAM │ OAM   │
    // │   12 B │      LCD Control │ Reg │       │
    // └────────┴──────────────────┴─────┴───────┘
    pub vram: Rc<RefCell<Ram<0x2000>>>,
    pub oam:  Rc<RefCell<Ram<0x00a0>>>,
    pub ctl: Rc<RefCell<Registers>>,
}

impl Ppu {
    /// Set the ppu's pic.
    pub fn set_pic(&mut self, pic: Rc<RefCell<Pic>>) {
        self.pic = pic;
    }

    /// Get a reference to the ppu's screen.
    #[must_use]
    pub fn screen(&self) -> &Screen {
        &self.lcd
    }

    /// Check if the screen is ready to be redrawn.
    #[must_use]
    pub fn ready(&self) -> bool {
        // Redraw the screen once per frame, when:
        // 1. PPU is enabled
        let enabled = self.enabled();
        // 2. Scanline is top of screen
        let topline = **self.ctl.borrow().ly.borrow() == 0;
        // 3. Dot is first of scanline
        let firstdot = self.dot == 0;

        enabled && topline && firstdot
    }

    /// Color a pixel according to the ppu's palette configuration.
    fn color(&self, pixel: Pixel) -> Color {
        let regs = self.ctl.borrow();
        let pal = **match pixel.pal() {
            Palette::BgWin => regs.bgp.borrow(),
            Palette::Obj0 => regs.obp0.borrow(),
            Palette::Obj1 => regs.obp1.borrow(),
        };
        pixel.col().recolor(pal)
    }
}

impl Block for Ppu {
    fn reset(&mut self) {
        // Reset LCD
        self.lcd = Default::default();

        // Reset mode
        self.mode = Default::default();

        // Reset memory
        self.vram.borrow_mut().reset();
        self.oam.borrow_mut().reset();

        // Reset registers
        self.ctl.borrow_mut().reset();
    }
}

impl Machine for Ppu {
    fn enabled(&self) -> bool {
        Lcdc::Enable.get(&*self.ctl.borrow().lcdc.borrow_mut())
    }

    fn cycle(&mut self) {
        self.mode = std::mem::take(&mut self.mode).exec(self);
    }
}

/// Control registers.
#[rustfmt::skip]
#[derive(Debug, Default)]
pub struct Registers {
    bus: Bus,
    // ┌──────┬────────────────────┬─────┬───────┐
    // │ SIZE │        NAME        │ DEV │ ALIAS │
    // ├──────┼────────────────────┼─────┼───────┤
    // │  1 B │        LCD Control │ Reg │ LCDC  │
    // │  1 B │         LCD Status │ Reg │ STAT  │
    // │  1 B │           Scroll Y │ Reg │ SCY   │
    // │  1 B │           Scroll X │ Reg │ SCX   │
    // │  1 B │   LCD Y Coordinate │ Reg │ LY    │
    // │  1 B │         LY Compare │ Reg │ LYC   │
    // │  1 B │ DMA Transfer Start │ Reg │ DMA   │
    // │  1 B │    BG Palette Data │ Reg │ BGP   │
    // │  1 B │ OBJ Palette 0 Data │ Reg │ OBP0  │
    // │  1 B │ OBJ Palette 1 Data │ Reg │ OBP1  │
    // │  1 B │           Window Y │ Reg │ WY    │
    // │  1 B │           Window X │ Reg │ WX    │
    // └──────┴────────────────────┴─────┴───────┘
    lcdc: Rc<RefCell<Register<u8>>>,
    stat: Rc<RefCell<Register<u8>>>,
    scy:  Rc<RefCell<Register<u8>>>,
    scx:  Rc<RefCell<Register<u8>>>,
    ly:   Rc<RefCell<Register<u8>>>,
    lyc:  Rc<RefCell<Register<u8>>>,
    dma:  Rc<RefCell<Register<u8>>>,
    bgp:  Rc<RefCell<Register<u8>>>,
    obp0: Rc<RefCell<Register<u8>>>,
    obp1: Rc<RefCell<Register<u8>>>,
    wy:   Rc<RefCell<Register<u8>>>,
    wx:   Rc<RefCell<Register<u8>>>,
}

impl Block for Registers {
    #[rustfmt::skip]
    fn reset(&mut self) {
        // Reset self
        std::mem::take(self);
        // Reset bus                           // ┌──────┬──────────────────┬─────┐
        self.bus.reset();                      // │ SIZE │      NAME        │ DEV │
                                               // ├──────┼──────────────────┼─────┤
        self.bus.map(0x00, self.lcdc.clone()); // │  1 B │      LCD Control │ Reg │
        self.bus.map(0x01, self.stat.clone()); // │  1 B │       LCD Status │ Reg │
        self.bus.map(0x02, self.scy.clone());  // │  1 B │         Scroll Y │ Reg │
        self.bus.map(0x03, self.scx.clone());  // │  1 B │         Scroll X │ Reg │
        self.bus.map(0x04, self.ly.clone());   // │  1 B │ LCD Y Coordinate │ Reg │
        self.bus.map(0x05, self.lyc.clone());  // │  1 B │       LY Compare │ Reg │
        self.bus.map(0x06, self.dma.clone());  // │  1 B │       LY Compare │ Reg │
        self.bus.map(0x07, self.bgp.clone());  // │  1 B │       LY Compare │ Reg │
        self.bus.map(0x08, self.obp0.clone()); // │  1 B │       LY Compare │ Reg │
        self.bus.map(0x09, self.obp1.clone()); // │  1 B │       LY Compare │ Reg │
        self.bus.map(0x0a, self.wy.clone());   // │  1 B │         Window Y │ Reg │
        self.bus.map(0x0b, self.wx.clone());   // │  1 B │         Window X │ Reg │
                                               // └──────┴──────────────────┴─────┘
    }
}

impl Device for Registers {
    fn contains(&self, index: usize) -> bool {
        self.bus.contains(index)
    }

    fn len(&self) -> usize {
        self.bus.len()
    }

    fn read(&self, index: usize) -> u8 {
        self.bus.read(index)
    }

    fn write(&mut self, index: usize, value: u8) {
        self.bus.write(index, value);
    }
}

#[allow(dead_code)]
#[rustfmt::skip]
#[derive(Copy, Clone, Debug)]
enum Lcdc {
    Enable      = 0b10000000,
    WinMap      = 0b01000000,
    WinEnable   = 0b00100000,
    BgWinData   = 0b00010000,
    BgMap       = 0b00001000,
    ObjSize     = 0b00000100,
    ObjEnable   = 0b00000010,
    BgWinEnable = 0b00000001,
}

impl Lcdc {
    pub fn get(self, lcdc: &u8) -> bool {
        *lcdc & self as u8 != 0
    }
}
