//! Memory bank controllers.
//!
//! Implementations of cartridge memory bank controllers.

use std::fmt::Debug;

use remus::{Block, SharedDevice};

mod mbc1;
mod nombc;

pub use self::mbc1::Mbc1;
pub use self::nombc::NoMbc;

/// Unified MBC interface.
pub(super) trait Mbc: Block + Debug {
    /// Gets a shared reference to the MBC's ROM.
    fn rom(&self) -> SharedDevice;

    /// Gets a shared reference to the MBC's RAM.
    fn ram(&self) -> SharedDevice;
}
