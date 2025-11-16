# Embedded Workspace

This folder demonstrates a simple Cargo-based workflow for firmware + host
utilities:

- `firmware/` – placeholder no_std crate that can be built for Cortex-M targets
  (`cargo build --package firmware --features stm32 --target thumbv7em-none-eabihf`).
- `host/` – desktop CLI tool that links against the root `rusb` crate to talk to
  the target (e.g., ESP32) and logs serial traffic.

Build/test/deploy examples:

```bash
# Build host-side tooling
cd embedded
cargo run -p host

# Build firmware for STM32 (requires rustup target add thumbv7em-none-eabihf)
cargo build -p firmware --features stm32 --target thumbv7em-none-eabihf
```

Hook this workspace into CI or custom scripts to automate deployments (for
example, run the host CLI to flash via `esp32_s3_flash` and then run board tests
with the logger helper).
