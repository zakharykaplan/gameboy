use std::cell::RefCell;
use std::rc::Rc;

use log::{info, trace};
use remus::bus::Bus;
use remus::mem::Ram;
use remus::{Block, Device, Machine};

/// Direct memory access.
#[derive(Debug, Default)]
pub struct Dma {
    page: u8,
    idx: Option<u8>,
    bus: Rc<RefCell<Bus>>,
    oam: Rc<RefCell<Ram<0x00a0>>>,
}

impl Dma {
    pub fn set_bus(&mut self, bus: Rc<RefCell<Bus>>) {
        self.bus = bus;
    }

    pub fn set_oam(&mut self, oam: Rc<RefCell<Ram<0x00a0>>>) {
        self.oam = oam;
    }
}

impl Block for Dma {
    fn reset(&mut self) {
        std::mem::take(self);
    }
}

impl Device for Dma {
    fn contains(&self, index: usize) -> bool {
        (0..self.len()).contains(&index)
    }

    fn len(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    fn read(&self, _: usize) -> u8 {
        self.page
    }

    fn write(&mut self, _: usize, value: u8) {
        info!("Starting DMA @ {value:#04x}00");
        self.page = value;
        self.idx = Some(0);
    }
}

impl Machine for Dma {
    fn enabled(&self) -> bool {
        self.idx.is_some()
    }

    fn cycle(&mut self) {
        // Calculate the address to read from
        let idx = self.idx.as_mut().unwrap();
        let addr = ((self.page as u16) << 8) | (*idx as u16);
        // Read this byte
        let data = self.bus.borrow().read(addr as usize);
        trace!("Transferring OAM({idx:#04x}) <- *{addr:#06x} = {data:#04x}");
        // Write this byte
        self.oam.borrow_mut().write(*idx as usize, data);
        // Increment the address
        self.idx = match *idx + 1 {
            160 => None,
            idx => Some(idx),
        };
    }
}
