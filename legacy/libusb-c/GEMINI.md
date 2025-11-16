# GEMINI.md - libusb Project Overview

This document provides a summary of the libusb project, its purpose, and how to get started with it.

## Project Overview

libusb is a cross-platform C library that provides applications with direct access to USB devices. It is designed to be portable and supports Linux, macOS, Windows, OpenBSD/NetBSD, Haiku, and Solaris. The library is licensed under the GNU Lesser General Public License (LGPL) 2.1 or later.

The primary goal of libusb is to create a standardized, easy-to-use API for communicating with USB devices from userspace, without the need for kernel-level drivers. This makes it a popular choice for a wide range of applications, from hobbyist projects to commercial products.

### Key Features:

*   **Cross-Platform:** Works on a variety of operating systems.
*   **Userspace Operation:** No need for custom kernel drivers.
*   **Standard C Library:** Can be easily integrated into C and C++ applications.
*   **Versatile:** Supports all USB transfer types (Control, Bulk, Interrupt, Isochronous).
*   **Hotplug Detection:** Can detect when devices are connected or disconnected.

### Core Technologies:

*   **C:** The library is primarily written in C.
*   **Autotools:** The build system is based on GNU Autotools (`configure.ac`, `Makefile.am`).
*   **Visual Studio:** Windows builds are supported using Visual Studio solution files (`.sln`, `.vcxproj`).
*   **Doxygen:** The API documentation is generated using Doxygen.

## Building and Running

The process for building and running `libusb` varies depending on the operating system.

### Windows

On Windows, `libusb` is built using Visual Studio.

1.  **Open the Solution:** Open the `msvc/libusb.sln` file in Visual Studio 2022 or later.
2.  **Select Configuration:** Choose your desired build configuration (e.g., "Release" or "Debug") and platform (e.g., "x64").
3.  **Build the Project:** Build the solution. This will generate the `libusb-1.0.dll` and `libusb-1.0.lib` files in the `build/<PlatformToolset>/<Platform>/<Configuration>/` directory.

Alternatively, you can use the `vcpkg` dependency manager to install `libusb`.

### Unix-like Systems (Linux, macOS)

On Unix-like systems, `libusb` uses the standard GNU Autotools build system.

1.  **Bootstrap:** Run the `bootstrap.sh` script to generate the `configure` script and other necessary build files.
    ```bash
    ./bootstrap.sh
    ```
2.  **Configure:** Run the `configure` script to prepare the build. You can enable the examples and tests with the `--enable-examples-build` and `--enable-tests-build` flags.
    ```bash
    ./configure --enable-examples-build --enable-tests-build
    ```
3.  **Build:** Compile the library using `make`.
    ```bash
    make
    ```
4.  **Install:** Install the library on your system.
    ```bash
    sudo make install
    ```

### Running the Examples and Tests

After building the library, you can find the example and test applications in the `examples` and `tests` directories, respectively. These can be run from the command line to test the library and interact with USB devices.

## Development Conventions

The `libusb` project has a set of development conventions to ensure code quality and consistency.

### Coding Style

The project uses `.clang-tidy` to enforce a consistent coding style. The configuration file (`.clang-tidy`) specifies a set of checks from `clang-analyzer`, `bugprone`, `modernize`, `performance`, `portability`, and `readability`.

### Commit Messages

Commit messages should be formatted to a width of 72 characters and include a free-standing summary line. Detailed information should be included in the commit message body.

### API Documentation

When extending or changing the API, the documentation must be updated. The documentation is generated using Doxygen, and any changes should be verified by running `make -C doc` and checking for warnings.
