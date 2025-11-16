//! Target-specific helper modules that build on top of the core `rusb` API.
//!
//! These helpers are intentionally lightweight and avoid new dependencies so
//! they can be copied into downstream projects or extended as needed.

pub mod esp32;
pub mod ftdi;
pub mod logger;
pub mod stm32;

pub use esp32::Esp32SerialBridge;
pub use ftdi::FtdiDevice;
pub use logger::ChannelLogger;
pub use stm32::{Stm32DfuDevice, Stm32VirtualCom};
