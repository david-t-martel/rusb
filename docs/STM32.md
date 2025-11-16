# STM32 USB Helpers

The `rusb::support::stm32` module targets two common STM32 scenarios:

1. **DFU bootloaders** (VID `0x0483`, PID `0xDF11`).
2. **Virtual COM ports** exposed by STLink or native USB CDC interfaces.

## DFU Flow

```rust,no_run
use rusb::support::stm32::Stm32DfuDevice;

fn flash_image(blocks: &[Vec<u8>]) -> Result<(), rusb::Error> {
    let dev = Stm32DfuDevice::open_first()?;
    for (i, block) in blocks.iter().enumerate() {
        dev.download_block(i as u16, block)?;
        let mut status = [0u8; 6];
        dev.get_status(&mut status)?;
    }
    dev.detach(500)?;
    Ok(())
}
```

All commands are simple wrappers around the DFU class requests, so you can
extend them with firmware-specific semantics (erase, jump commands, etc.).

Additional helpers:

- `wait_while_busy(timeout)` polls `DFU_GETSTATUS` until the bootloader reports
  `OK`.
- `mass_erase()` issues the vendor-specific DFUSE mass erase sequence.
- `leave_dfu()` wraps `DFU_DETACH` plus a short delay so the MCU can reboot
  into application code.

## Virtual COM

`Stm32VirtualCom` is a convenience wrapper around bulk IN/OUT pipes. It does not
perform any automatic enumeration (endpoints vary between boards), but once the
`DeviceHandle` is opened you can construct the wrapper and reuse the `read`/
`write` helpers.

## Future Work

- Automate `bInterfaceNumber` discovery via descriptors.
- Implement asynchronous DFU downloads using the planned async transfer APIs.
- Add STM32WB/USB HS examples once USB 3.x flows land in `rusb`.
