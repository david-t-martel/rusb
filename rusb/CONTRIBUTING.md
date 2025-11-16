# Contributing to rusb

This document provides a high-level overview of the `rusb` architecture and implementation plan.

## Architecture

The `rusb` crate is a wrapper around the platform-specific USB APIs. The crate is structured as follows:

- `src/lib.rs`: The main library file. It contains the public API and dispatches calls to the platform-specific backends.
- `src/platform/`: This directory contains the platform-specific backends. Each backend is in its own file and is conditionally compiled based on the target OS.
- `src/platform/mod.rs`: This file uses `#[cfg]` attributes to select the correct platform-specific backend.

## Implementation Plan

The following is a list of tasks that need to be completed to finish the `rusb` implementation:

- [x] Scaffold the project.
- [x] Implement `devices()` for Linux, Windows, and macOS.
- [x] Implement `open()` for Linux, Windows, and macOS.
- [x] Implement `get_device_descriptor()` for Linux, Windows, and macOS.
- [ ] Implement `control_transfer()` for all platforms.
- [ ] Implement `bulk_transfer()` for all platforms.
- [ ] Implement `interrupt_transfer()` for all platforms.
- [ ] Implement `get_string_descriptor()` for all platforms.
- [ ] Implement `get_configuration_descriptor()` for all platforms.
- [ ] Implement `get_interface_descriptor()` for all platforms.
- [ ] Implement `get_endpoint_descriptor()` for all platforms.
- [ ] Add tests.
- [ ] Add examples.
- [ ] Add documentation.
