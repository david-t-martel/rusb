# USB 3.x+ Feature Plan for `rusb`

## Goals

1. Reach feature parity with libusb’s mature USB 2.0 stack while exposing the
   capabilities introduced in USB 3.0/3.1/3.2 and USB4 (streams, bursts,
   SuperSpeed power management).
2. Offer optional asynchronous APIs per-platform without breaking the current
   synchronous facade.
3. Provide deterministic testing and benchmarking so regressions show up early.

## Current Capabilities Snapshot

| Platform              | Control | Bulk            | Interrupt | Isochronous | Streams | Async option |
|-----------------------|---------|-----------------|-----------|-------------|---------|--------------|
| Linux / Android usbfs | ✅       | ✅ (chunked to usbfs limits) | ✅         | ❌         | ❌       | ❌ (blocking ioctl) |
| Windows WinUSB        | ✅       | ✅ (synchronous) | ✅         | ❌ (WinUSB lacks native ISO) | ❌ | ❌ (no OVERLAPPED use) |
| macOS IOKit           | ✅       | ✅ (blocking ReadPipeTO/WritePipeTO) | ✅ | ❌ | ❌ | ❌ |
| WebUSB                | ✅ (async promises) | ✅ | ✅ | ❌ | ❌ | ✅ (Promises/Futures) |

## Phase 1 – SuperSpeed plumbing

1. **Capability detection** (DONE for Linux via `USBDEVFS_GET_CAPABILITIES`; mirror
   the logic on macOS/Windows using the relevant APIs).
2. **Max packet size logic** – map descriptor values to transfer chunking rules
   so we never stagnate at 16 KB chunks when the host allows more.
3. **USB 3.x speed reporting** – surface `Device::speed()` style helpers (libusb
   already exposes these).

## Phase 2 – Transfer primitives

1. **Bulk streams (Linux/Windows)**
   - Add API to claim/release stream IDs (usbfs `USBDEVFS_ALLOC_STREAMS` and
     WinUSB `WinUsb_RegisterIsochBuffer` equivalent for bulk streams when
     available).
   - Expose `TransferStreamId` optional argument on the Rust bulk APIs.
2. **Isochronous transfers**
   - Implement usbfs URB submission path with per-packet descriptors.
   - On Windows, offer alternative backend using the KMDF libusbK or WinUSBDK
     helpers (WinUSB itself does not support ISO – may require optional backend
     switch).
   - macOS: wire `ReadIsochPipeAsync` / `WriteIsochPipeAsync` and completion
     callbacks.
3. **Optional asynchronous APIs**
   - Extend `DeviceHandle` with `async_*` variants on Linux (using `io_uring` or
     dedicated threads submitting URBs).
   - Windows: expose OVERLAPPED handles + completion events; provide both a
     `Future`-based API and a callback helper.
   - macOS: wrap the CFRunLoop callbacks coming from IOKit’s async APIs.

## Phase 3 – Power & performance tuning

1. Expose helpers to set/clear U1/U2/U3 exit latencies where supported.
2. Allow callers to negotiate burst size (USB 3.1) via class-specific control
   messages.
3. Hook kernel notifications to expose link state transitions (suspend/resume
   events) in the Rust API.

## Phase 4 – Tooling & validation

1. **Windows test harness** (see `rusb/tests/windows_compare.rs`) – extend to run
   bulk/interrupt/iso tests once the async + ISO plumbing lands.
2. **Hardware fixture** – recommended setup: a Cypress FX3 or FTDI FT600/FT601
   SuperSpeed development board wired into a USB 3.2 Gen 2 hub with a hardware
   loopback daughtercard so OUT data re-enters as IN data. Pair it with a
   Teledyne LeCroy or Total Phase USB protocol analyzer to capture traces while
   running scripts.
3. **Command-line tooling** – add `cargo xtask usb-dump` style utilities that
   can enumerate devices, print link states, and issue canned transfers for
   smoke testing.

## Hardware Test Harness Concept

- **Device Under Test:** Cypress FX3 SuperSpeed Explorer kit configured with
  the AN75779 firmware (GPIF II to loopback FIFO). Provides bulk IN/OUT, control
  endpoint, and optional isochronous endpoints.
- **Host Setup:** USB 3.2 Gen 2 hub connected to both a Windows 11 desktop and a
  Linux workstation so both stacks can run the same scripts.
- **Instrumentation:**
  - USB protocol analyzer capturing SS/SS+ packets.
  - Bench power supply with current logging to observe U1/U2 transitions.
- **Execution Flow:**
  1. Use the Windows harness to run control/bulk/interrupt benchmarks.
  2. Mirror on Linux/macOS once parity suites exist.
  3. Collect throughput + latency numbers plus analyzer traces, checking for
     compliance (no Babble, proper link commands).

This document should be updated as each phase lands, with links to the relevant
PRs and regression tests.
