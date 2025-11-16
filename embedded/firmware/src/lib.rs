#![cfg_attr(all(feature = "stm32", not(test)), no_std)]

#[cfg(feature = "stm32")]
use cortex_m_rt::entry;

/// Build metadata exposed to the host tools.
pub const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "stm32")]
#[entry]
fn main() -> ! {
    loop {
        cortex_m::asm::nop();
    }
}
