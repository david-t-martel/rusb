# RUSB Testing & Parity Matrix

`rusb` aims to be feature-for-feature compatible with the original libusb C
stack.  This document enumerates the coverage we currently have and describes
how to extend it when adding new backends or APIs.

## Test Targets

| Target                     | Backend                     | Status | How to run |
|----------------------------|-----------------------------|--------|------------|
| Linux / Android            | usbfs (sysfs+ioctl)         | ✅     | `cargo test` (needs `/dev/bus/usb` access) |
| Windows (WinUSB)           | SetupDi + WinUSB API        | ✅     | `cargo test` + optional parity test below |
| macOS (IOUSBDeviceInterface) | IOKit Device/Interface APIs | ✅     | `cargo test` on macOS host |
| Web (wasm32 + WebUSB)      | Browser WebUSB APIs         | ✅     | `wasm-pack test --headless --chrome --features webusb` (requires browser) |

## Parity Harness

### Windows

1. Install a libusb runtime (`libusb-1.0.dll`) and ensure it resides on your
   `PATH`.
2. Enable the comparison test:

   ```powershell
   cd rusb
   $env:RUSB_COMPARE_LIBUSB = "1"
   cargo test --test windows_equivalence
   ```

   The test enumerates VID/PID pairs through both libusb and rusb and asserts
   the sets match exactly.

### Unix-like hosts

A similar harness can be enabled by copying `tests/windows_equivalence.rs` and
adjusting the library name (`libusb-1.0.so` or `libusb-1.0.dylib`).  Export
`RUSB_COMPARE_LIBUSB=1` before running to opt-in.

### WebUSB

Parity is validated by running the wasm test suite against a browser-provided
USB mock or real hardware.  Use `wasm-pack test --headless --chrome --features
webusb` to exercise the async code paths.

## Adding New Coverage

- Extend `tests/windows_equivalence.rs` (or platform-specific clones) whenever
  new rusb APIs are exposed.  Each code path should have a matching libusb call
  so regressions are caught quickly.
- For Linux/Android/macOS, feel free to add `#[cfg(target_os = ...)]` tests that
  open real devices when the `RUSB_HW_TEST=1` environment variable is set.
- The WebUSB backend can be integration-tested via wasm-bindgen’s test runner by
  stubbing `navigator.usb` in headless Chrome.

## Panics and Error Reporting

All public APIs return `Result<_, Error>`; internal `unwrap` calls are confined
to unit tests.  When adding new platform code prefer propagating `std::io::Error`
or OS-specific codes through `Error::Os` so parity harnesses receive identical
failure information as libusb.
