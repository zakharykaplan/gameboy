use std::cell::RefCell;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::rc::Rc;

use log::{debug, error, info, warn};
use remus::bus::Bus;
use remus::dev::Device;
use remus::mem::Memory;
use remus::reg::Register;

use crate::cpu::sm83::Cpu;

const BOOTROM: [u8; 0x100] = [
    0x31, 0xfe, 0xff, 0xaf, 0x21, 0xff, 0x9f, 0x32, 0xcb, 0x7c, 0x20, 0xfb, 0x21, 0x26, 0xff, 0x0e,
    0x11, 0x3e, 0x80, 0x32, 0xe2, 0x0c, 0x3e, 0xf3, 0xe2, 0x32, 0x3e, 0x77, 0x77, 0x3e, 0xfc, 0xe0,
    0x47, 0x11, 0x04, 0x01, 0x21, 0x10, 0x80, 0x1a, 0xcd, 0x95, 0x00, 0xcd, 0x96, 0x00, 0x13, 0x7b,
    0xfe, 0x34, 0x20, 0xf3, 0x11, 0xd8, 0x00, 0x06, 0x08, 0x1a, 0x13, 0x22, 0x23, 0x05, 0x20, 0xf9,
    0x3e, 0x19, 0xea, 0x10, 0x99, 0x21, 0x2f, 0x99, 0x0e, 0x0c, 0x3d, 0x28, 0x08, 0x32, 0x0d, 0x20,
    0xf9, 0x2e, 0x0f, 0x18, 0xf3, 0x67, 0x3e, 0x64, 0x57, 0xe0, 0x42, 0x3e, 0x91, 0xe0, 0x40, 0x04,
    0x1e, 0x02, 0x0e, 0x0c, 0xf0, 0x44, 0xfe, 0x90, 0x20, 0xfa, 0x0d, 0x20, 0xf7, 0x1d, 0x20, 0xf2,
    0x0e, 0x13, 0x24, 0x7c, 0x1e, 0x83, 0xfe, 0x62, 0x28, 0x06, 0x1e, 0xc1, 0xfe, 0x64, 0x20, 0x06,
    0x7b, 0xe2, 0x0c, 0x3e, 0x87, 0xe2, 0xf0, 0x42, 0x90, 0xe0, 0x42, 0x15, 0x20, 0xd2, 0x05, 0x20,
    0x4f, 0x16, 0x20, 0x18, 0xcb, 0x4f, 0x06, 0x04, 0xc5, 0xcb, 0x11, 0x17, 0xc1, 0xcb, 0x11, 0x17,
    0x05, 0x20, 0xf5, 0x22, 0x23, 0x22, 0x23, 0xc9, 0xce, 0xed, 0x66, 0x66, 0xcc, 0x0d, 0x00, 0x0b,
    0x03, 0x73, 0x00, 0x83, 0x00, 0x0c, 0x00, 0x0d, 0x00, 0x08, 0x11, 0x1f, 0x88, 0x89, 0x00, 0x0e,
    0xdc, 0xcc, 0x6e, 0xe6, 0xdd, 0xdd, 0xd9, 0x99, 0xbb, 0xbb, 0x67, 0x63, 0x6e, 0x0e, 0xec, 0xcc,
    0xdd, 0xdc, 0x99, 0x9f, 0xbb, 0xb9, 0x33, 0x3e, 0x3c, 0x42, 0xb9, 0xa5, 0xb9, 0xa5, 0x42, 0x3c,
    0x21, 0x04, 0x01, 0x11, 0xa8, 0x00, 0x1a, 0x13, 0xbe, 0x20, 0xfe, 0x23, 0x7d, 0xfe, 0x34, 0x20,
    0xf5, 0x06, 0x19, 0x78, 0x86, 0x23, 0x05, 0x20, 0xfb, 0x86, 0x20, 0xfe, 0x3e, 0x01, 0xe0, 0x50,
];

#[derive(Debug, Default)]
pub struct GameBoy {
    cpu: Cpu,
    cart: Cartridge,
    devs: Devices,
}

impl GameBoy {
    pub fn new() -> Self {
        Self::default().reset()
    }

    #[rustfmt::skip]
    fn reset(mut self) -> Self {
        // Reset CPU
        self.cpu = self.cpu.reset();
                                                            // ┌──────────┬────────────┬─────┐
        // Reset bus                                        // │   SIZE   │    NAME    │ DEV │
        self.cpu.bus = Bus::default();                      // ├──────────┼────────────┼─────┤
        self.cpu.bus.map(0x0000, self.devs.boot.clone());   // │    256 B │       Boot │ ROM │
        self.cpu.bus.map(0x0000, self.cart.rom.clone());    // │  32 Ki B │  Cartridge │ ROM │
        self.cpu.bus.map(0x8000, self.devs.vram.clone());   // │   8 Ki B │      Video │ RAM │
        self.cpu.bus.map(0xa000, self.cart.eram.clone());   // │   8 Ki B │   External │ RAM │
        self.cpu.bus.map(0xc000, self.devs.wram.clone());   // │   8 Ki B │       Work │ RAM │
        self.cpu.bus.map(0xe000, self.devs.wram.clone());   // │   7680 B │       Echo │ RAM │
        self.cpu.bus.map(0xfe00, self.devs.oam.clone());    // │    160 B │      Video │ RAM │
                                                            // │     96 B │     Unused │ --- │
        self.cpu.bus.map(0xff00, self.devs.io.bus.clone()); // │    128 B │        I/O │ Bus │
        self.cpu.bus.map(0xff80, self.devs.hram.clone());   // │    127 B │       High │ RAM │
        self.cpu.bus.map(0xffff, self.devs.ie.clone());     // │      1 B │  Interrupt │ Reg │
                                                            // └──────────┴────────────┴─────┘
        // Reset cartridge
        self.cart = self.cart.reset();
        // Reset devices
        self.devs = self.devs.reset();
        self
    }

    pub fn load(&mut self, path: &Path) -> io::Result<()> {
        // Open the ROM file
        let mut file = File::open(path)?;
        let metadata = file.metadata()?;
        // Read its contents into memory
        let buf = &mut *self.cart.rom.borrow_mut();
        let read = file.read(buf)?;
        if read < buf.len() {
            warn!(
                r#"Read {read} bytes from "{}""; remaining {} bytes uninitialized."#,
                path.display(),
                buf.len() - read
            );
        } else if (buf.len() as u64) < metadata.len() {
            error!(
                r#"Read {read} bytes from "{}"; remaining {} bytes truncated."#,
                path.display(),
                metadata.len() - (read as u64),
            );
        } else {
            info!(r#"Read {read} bytes from "{}""#, path.display());
        }

        // Log the ROM contents
        debug!("Cartridge ROM:\n{buf}");

        Ok(())
    }

    pub fn start(&mut self) {
        self.cpu.start();

        while self.cpu.enabled() {
            self.cpu.cycle();
        }
    }
}

#[rustfmt::skip]
#[derive(Debug, Default)]
struct Devices {
                                       // ┌────────┬───────────┬─────┬───────┐
                                       // │  SIZE  │    NAME   │ DEV │ ALIAS │
                                       // ├────────┼───────────┼─────┼───────┤
    boot: Rc<RefCell<Memory<0x0100>>>, // │  256 B │      Boot │ ROM │       │
    vram: Rc<RefCell<Memory<0x2000>>>, // │ 8 Ki B │     Video │ RAM │ VRAM  │
    wram: Rc<RefCell<Memory<0x2000>>>, // │ 8 Ki B │      Work │ RAM │ WRAM  │
    oam:  Rc<RefCell<Memory<0x00a0>>>, // │  160 B │     Video │ RAM │ OAM   │
    io:   IoDevices,                   // │  128 B │       I/O │ Bus │       │
    hram: Rc<RefCell<Memory<0x007f>>>, // │  127 B │      High │ RAM │ HRAM  │
    ie:   Rc<RefCell<Register<1>>>,    // │    1 B │ Interrupt │ Reg │ IE    │
                                       // └────────┴───────────┴─────┴───────┘
}

impl Devices {
    fn reset(mut self) -> Self {
        // Reset Boot ROM
        self.boot.replace(Memory::from(&BOOTROM));
        // Reset I/O
        self.io = self.io.reset();
        self
    }
}

#[rustfmt::skip]
#[derive(Debug, Default)]
struct IoDevices {
    bus: Rc<RefCell<Bus>>,
                                      // ┌────────┬─────────────────┬─────┐
                                      // │  SIZE  │      NAME       │ DEV │
                                      // ├────────┼─────────────────┼─────┤
    con:   Rc<RefCell<Register<1>>>,  // │    1 B │      Controller │ Reg │
    com:   Rc<RefCell<Register<2>>>,  // │    2 B │   Communication │ Reg │
    timer: Rc<RefCell<Register<4>>>,  // │    4 B │ Divider & Timer │ Reg │
    iflag: Rc<RefCell<Register<1>>>,  // │    1 B │  Interrupt Flag │ Reg │
    sound: Rc<RefCell<Memory<0x17>>>, // │   23 B │           Sound │ RAM │
    wram:  Rc<RefCell<Memory<0x10>>>, // │   16 B │        Waveform │ RAM │
    lcd:   Rc<RefCell<Memory<0x0c>>>, // │   16 B │             LCD │ RAM │
    bank:  Rc<RefCell<Register<1>>>,  // │    1 B │   Boot ROM Bank │ Reg │
                                      // └────────┴─────────────────┴─────┘
}

#[rustfmt::skip]
impl IoDevices {
    fn reset(self) -> Self {
                                                             // ┌────────┬─────────────────┬─────┐
        // Reset bus                                         // │  SIZE  │      NAME       │ DEV │
        self.bus.replace(Bus::default());                    // ├────────┼─────────────────┼─────┤
        self.bus.borrow_mut().map(0x00, self.con.clone());   // │    1 B │      Controller │ Reg │
        self.bus.borrow_mut().map(0x01, self.com.clone());   // │    2 B │   Communication │ Reg │
                                                             // │    1 B │          Unused │ --- │
        self.bus.borrow_mut().map(0x04, self.timer.clone()); // │    4 B │ Divider & Timer │ Reg │
                                                             // │    7 B │          Unused │ --- │
        self.bus.borrow_mut().map(0x0f, self.iflag.clone()); // │    1 B │  Interrupt Flag │ Reg │
        self.bus.borrow_mut().map(0x10, self.sound.clone()); // │   23 B │           Sound │ RAM │
                                                             // │    9 B │          Unused │ --- │
        self.bus.borrow_mut().map(0x30, self.wram.clone());  // │   16 B │        Waveform │ RAM │
        self.bus.borrow_mut().map(0x40, self.lcd.clone());   // │   12 B │             LCD │ RAM │
                                                             // │    4 B │          Unused │ --- │
        self.bus.borrow_mut().map(0x50, self.bank.clone());  // │    1 B │   Boot ROM Bank │ Reg │
                                                             // │   47 B │          Unused │ --- │
                                                             // └────────┴─────────────────┴─────┘
        self
    }
}

#[derive(Debug, Default)]
struct Cartridge {
    rom: Rc<RefCell<Memory<0x8000>>>,
    eram: Rc<RefCell<Memory<0x2000>>>,
}

impl Cartridge {
    fn reset(self) -> Self {
        self
    }
}

#[cfg(test)]
mod tests {
    use remus::dev::Device;

    use crate::*;

    #[test]
    fn bus_works() {
        let mut gb = GameBoy::new();
        // Boot ROM
        (0x0000..=0x00ff).for_each(|addr| gb.cpu.bus.write(addr, 0x10));
        assert!((0x00..0xff)
            .map(|addr| gb.devs.boot.borrow().read(addr))
            .all(|byte| byte == 0x10));
        // Cartridge ROM
        (0x0100..=0x7fff).for_each(|addr| gb.cpu.bus.write(addr, 0x20));
        assert!((0x0100..=0x7fff)
            .map(|addr| gb.cart.rom.borrow().read(addr))
            .all(|byte| byte == 0x20));
        // Video RAM
        (0x8000..=0x9fff).for_each(|addr| gb.cpu.bus.write(addr, 0x30));
        assert!((0x0000..=0x1fff)
            .map(|addr| gb.devs.vram.borrow().read(addr))
            .all(|byte| byte == 0x30));
        // External RAM
        (0xa000..=0xbfff).for_each(|addr| gb.cpu.bus.write(addr, 0x40));
        assert!((0x0000..=0x1fff)
            .map(|addr| gb.cart.eram.borrow().read(addr))
            .all(|byte| byte == 0x40));
        // Video RAM (OAM)
        (0xfe00..=0xfe9f).for_each(|addr| gb.cpu.bus.write(addr, 0x50));
        assert!((0x00..=0x9f)
            .map(|addr| gb.devs.oam.borrow().read(addr))
            .all(|byte| byte == 0x50));
        // I/O Bus
        {
            // Controller
            (0xff00..=0xff00).for_each(|addr| gb.cpu.bus.write(addr, 0x61));
            assert!((0x00..=0x00)
                .map(|addr| gb.devs.io.bus.borrow().read(addr))
                .all(|byte| byte == 0x61));
            assert!((0x0..=0x0)
                .map(|addr| gb.devs.io.con.borrow().read(addr))
                .all(|byte| byte == 0x61));
            // Communication
            (0xff01..=0xff02).for_each(|addr| gb.cpu.bus.write(addr, 0x62));
            assert!((0x01..=0x02)
                .map(|addr| gb.devs.io.bus.borrow().read(addr))
                .all(|byte| byte == 0x62));
            assert!((0x00..=0x01)
                .map(|addr| gb.devs.io.com.borrow().read(addr))
                .all(|byte| byte == 0x62));
            // Divider & Timer
            (0xff04..=0xff07).for_each(|addr| gb.cpu.bus.write(addr, 0x63));
            assert!((0x04..=0x07)
                .map(|addr| gb.devs.io.bus.borrow().read(addr))
                .all(|byte| byte == 0x63));
            assert!((0x00..=0x03)
                .map(|addr| gb.devs.io.timer.borrow().read(addr))
                .all(|byte| byte == 0x63));
            // Interrupt Flag
            (0xff0f..=0xff0f).for_each(|addr| gb.cpu.bus.write(addr, 0x64));
            assert!((0x0f..=0x0f)
                .map(|addr| gb.devs.io.bus.borrow().read(addr))
                .all(|byte| byte == 0x64));
            assert!((0x00..=0x00)
                .map(|addr| gb.devs.io.iflag.borrow().read(addr))
                .all(|byte| byte == 0x64));
            // Sound
            (0xff10..=0xff26).for_each(|addr| gb.cpu.bus.write(addr, 0x65));
            assert!((0x10..=0x26)
                .map(|addr| gb.devs.io.bus.borrow().read(addr))
                .all(|byte| byte == 0x65));
            assert!((0x00..=0x16)
                .map(|addr| gb.devs.io.sound.borrow().read(addr))
                .all(|byte| byte == 0x65));
            // Waveform RAM
            (0xff30..=0xff3f).for_each(|addr| gb.cpu.bus.write(addr, 0x66));
            assert!((0x30..=0x3f)
                .map(|addr| gb.devs.io.bus.borrow().read(addr))
                .all(|byte| byte == 0x66));
            assert!((0x00..=0x0f)
                .map(|addr| gb.devs.io.wram.borrow().read(addr))
                .all(|byte| byte == 0x66));
            // LCD
            (0xff40..=0xff4b).for_each(|addr| gb.cpu.bus.write(addr, 0x67));
            assert!((0x40..=0x4b)
                .map(|addr| gb.devs.io.bus.borrow().read(addr))
                .all(|byte| byte == 0x67));
            assert!((0x00..=0x0b)
                .map(|addr| gb.devs.io.lcd.borrow().read(addr))
                .all(|byte| byte == 0x67));
            // Boot ROM Disable
            (0xff50..=0xff50).for_each(|addr| gb.cpu.bus.write(addr, 0x68));
            assert!((0x50..=0x50)
                .map(|addr| gb.devs.io.bus.borrow().read(addr))
                .all(|byte| byte == 0x68));
            assert!((0x00..=0x00)
                .map(|addr| gb.devs.io.bank.borrow().read(addr))
                .all(|byte| byte == 0x68));
        }
        // High RAM
        (0xff80..=0xfffe).for_each(|addr| gb.cpu.bus.write(addr, 0x70));
        assert!((0x00..=0x7e)
            .map(|addr| gb.devs.hram.borrow().read(addr))
            .all(|byte| byte == 0x70));
        // Interrupt Enable
        (0xffff..=0xffff).for_each(|addr| gb.cpu.bus.write(addr, 0x80));
        assert!((0x0..=0x0)
            .map(|addr| gb.devs.ie.borrow().read(addr))
            .all(|byte| byte == 0x80));
    }
}