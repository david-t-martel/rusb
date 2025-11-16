# Auricle USB Stack

This repository now centers on the `rusb` Rust rewrite of libusb.  The original
C implementation is still available under `legacy/libusb-c` for reference,
tooling, or comparison builds, but all new development, testing, and packaging
should happen in `rusb/`.

## Layout

- `rusb/` – self-contained Cargo crate with native backends for Linux/Android
  (usbfs), Windows (WinUSB), macOS (IOUSBDeviceInterface), and optional WebUSB
  support.  This crate exposes a safe Rust API that mirrors libusb semantics and
  drives the rest of the repository.
- `legacy/libusb-c/` – frozen copy of the classic C sources, build scripts, and
  documentation.  The directory structure matches the upstream project
  (examples, doc, tests, etc.) so maintainers can still run legacy build
  pipelines when needed.
- `legacy/libusb-c/tests-c/` – relocated C test harnesses.

## Building the Rust crate

```bash
cd rusb
cargo build
```

Linux/Android builds rely on `usbfs`, so the user must have permission to open
`/dev/bus/usb/*` nodes.  Windows builds automatically link against WinUSB.  For
macOS the IOKit framework is linked transparently through `io-kit-sys`.

WebAssembly/WebUSB builds require the `webusb` feature and a target of
`wasm32-unknown-unknown`:

```bash
cd rusb
wasm-pack build --target web --features webusb
```

The build script automatically enables `web_sys_unstable_apis` when the feature
is selected so that WebUSB bindings compile.

## Testing and Parity Validation

Run the default test suite (unit + doc tests):

```bash
cd rusb
cargo test
```

To compare the Rust backend against the legacy C libusb on Windows, install
`libusb-1.0.dll` somewhere on `PATH` and enable the parity test:

```bash
cd rusb
set RUSB_COMPARE_LIBUSB=1
cargo test --test windows_equivalence
```

The test dynamically loads libusb, enumerates descriptor VID/PID pairs, and
asserts that `rusb::devices()` yields the identical set.  The test is skipped by
default so that CI can run on hosts without libusb installed.

On Unix-like hosts you can follow a similar pattern by editing
`tests/windows_equivalence.rs` to point at `libusb-1.0.so` or
`libusb-1.0.dylib`—the harness is written in a portable way so extending it is
straightforward.

Refer to `rusb/TESTING.md` for the complete parity matrix and additional
instructions on extending the coverage.

## Legacy C build

All of the historical autotools/Xcode/MSVC files now live under
`legacy/libusb-c/`.  Nothing in the Rust crate depends on those files, so they
can be built or archived independently.  The original README and documentation
ship unmodified (moved to `README-c.md`).
