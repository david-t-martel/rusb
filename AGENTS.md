# Repository Guidelines

## Project Structure & Module Organization
Core cross-platform sources live under `libusb/`, including the shared host backends and headers. Reference drivers, helpers, and legacy build glue sit in `build/`, while portable tooling for Android, macOS, and Windows lives in `android/`, `Xcode/`, and `msvc/` respectively. Regression harnesses and stress tools are in `tests/`, and sample host applications are kept in `examples/` for quick API demonstrations. API docs and Doxygen configs reside in `doc/`; run updates there whenever you change public headers.

## Build, Test, and Development Commands
Use autotools for the canonical flow: `./bootstrap.sh && ./configure --enable-examples-build` initializes freshly cloned trees, followed by `make -j$(nproc)` to build the core library. Run `make check` before every submission; it drives the binaries declared in `tests/Makefile.am`. Regenerate documentation with `make -C doc` so Doxygen warnings surface early. On Windows, open `msvc/libusb.sln` or run `powershell msvc/build_all.ps1` to compile both static and DLL targets. macOS developers without autotools can open `Xcode/libusb.xcodeproj`.

## Coding Style & Naming Conventions
Match the repository defaults declared in each file header: tabs for indentation, `c-basic-offset:8`, and K&R brace placement. Public APIs must retain the `libusb_*` prefix; internal helpers use `usbi_*` and stay in `libusb/` only. Keep enums, macros, and constants screaming snake case (`LIBUSB_TRANSFER_TYPE`). Favor Doxygen block comments (`/** ... */`) whenever documenting exported symbols, and update accompanying `.h` files and `doc/api.dox` together. Avoid touching `version_nano.h`; release engineering handles it.

## Testing Guidelines
Name new tests after the behavior under scrutiny (`tests/set_option.c`, `tests/stress_mt.c`). Extend `tests/Makefile.am` so `make check` picks them up, and gate platform-specific binaries with the existing `OS_*` conditionals. Provide umockdev fixtures when exercising Linux-specific code (`BUILD_UMOCKDEV_TEST=1 make check`). For driver-facing changes, add or update the matching sample in `examples/` so downstream maintainers can manually probe devices.

## Commit & Pull Request Guidelines
Write commits with a standalone summary line ≤72 characters plus wrapped body text explaining context, rationale, and testing. Reference GitHub issues or mailing-list threads inline (e.g., `Fixes #123`). Large feature work should start as a branch discussed on the mailing list; never bundle unrelated fixes. Do not edit `AUTHORS` directly. Pull requests should describe platform impact, doc updates (`make -C doc` output), and test evidence (command transcript or CI link); include screenshots only when UI tooling is affected (e.g., examples demonstrating hotplug dialogs).
