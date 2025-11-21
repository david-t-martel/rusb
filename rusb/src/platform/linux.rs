#![cfg(target_os = "linux")]

//! Linux-specific USB backend implementation built directly on top of sysfs
//! metadata and the usbfs device nodes. This avoids any dependency on the C
//! libusb shim or libudev by interrogating the kernel's exported files
//! directly.
//!
//! TODO: Add support for isochronous transfers via USBDEVFS_SUBMITURB
//! TODO: Add async transfer API using io_uring or epoll for better performance
//! TODO: Add interface claiming/releasing via USBDEVFS_CLAIMINTERFACE
//! TODO: Add configuration setting via USBDEVFS_SETCONFIGURATION
//! TODO: Add alternate setting selection via USBDEVFS_SETINTERFACE
//! TODO: Add kernel driver detach/attach via USBDEVFS_DISCONNECT/CONNECT
//! TODO: Add clear halt via USBDEVFS_CLEAR_HALT
//! TODO: Add device reset via USBDEVFS_RESET
//! TODO: Add string descriptor reading
//! TODO: Add hotplug support using udev or netlink

use crate::{
    ControlRequest, ControlTransferData, Device, DeviceDescriptor, DeviceHandle, DeviceList, Error,
    TransferBuffer, TransferDirection,
};
use libc::{self, c_ulong};
use std::cmp;
use std::ffi::c_void;
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind};
use std::mem::size_of;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::{Path, PathBuf};
use std::ptr;
use std::time::Duration;

const SYSFS_USB_DEVICES: &str = "/sys/bus/usb/devices";

/// Linux representation of a USB device discovered in sysfs.
/// TODO: Cache more device information (speed, manufacturer, product, serial)
/// TODO: Add port numbers for topology information
pub struct LinuxDevice {
    sysfs_path: PathBuf,
    bus_number: u16,
    device_address: u16,
    // TODO: Add device_speed: DeviceSpeed field
    // TODO: Add port_chain: Vec<u8> for USB topology
}

/// Handle that keeps the corresponding usbfs file descriptor alive.
/// TODO: Track claimed interfaces to prevent double-claim
/// TODO: Track active configuration
/// TODO: Add support for async URB submissions
pub struct LinuxDeviceHandle {
    file: File,
    caps: u32,
    // TODO: Add claimed_interfaces: HashSet<u8>
    // TODO: Add active_config: Option<u8>
}

impl LinuxDeviceHandle {
    pub(crate) fn as_raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }

    fn max_bulk_chunk(&self) -> usize {
        if self.caps & USBFS_CAP_NO_PACKET_SIZE_LIM != 0 {
            cmp::min(u32::MAX as usize, usize::MAX)
        } else {
            MAX_BULK_BUFFER_LENGTH
        }
    }
}

#[repr(C)]
struct UsbfsCtrlTransfer {
    request_type: u8,
    request: u8,
    value: u16,
    index: u16,
    length: u16,
    timeout: u32,
    data: *mut c_void,
}

#[repr(C)]
struct UsbfsBulkTransfer {
    ep: u32,
    len: u32,
    timeout: u32,
    data: *mut c_void,
}

const IOC_NRBITS: u8 = 8;
const IOC_TYPEBITS: u8 = 8;
const IOC_SIZEBITS: u8 = 14;

const IOC_WRITE: u8 = 1;
const IOC_READ: u8 = 2;

const IOC_NRSHIFT: u8 = 0;
const IOC_TYPESHIFT: u8 = IOC_NRSHIFT + IOC_NRBITS;
const IOC_SIZESHIFT: u8 = IOC_TYPESHIFT + IOC_TYPEBITS;
const IOC_DIRSHIFT: u8 = IOC_SIZESHIFT + IOC_SIZEBITS;

const fn ioc(dir: u8, ty: u8, nr: u8, size: usize) -> c_ulong {
    ((dir as c_ulong) << IOC_DIRSHIFT)
        | ((ty as c_ulong) << IOC_TYPESHIFT)
        | ((nr as c_ulong) << IOC_NRSHIFT)
        | ((size as c_ulong) << IOC_SIZESHIFT)
}

const fn ior(ty: u8, nr: u8, size: usize) -> c_ulong {
    ioc(IOC_READ, ty, nr, size)
}

const fn iorw(ty: u8, nr: u8, size: usize) -> c_ulong {
    ioc(IOC_READ | IOC_WRITE, ty, nr, size)
}

const USBDEVFS_CONTROL: c_ulong = iorw(b'U', 0, size_of::<UsbfsCtrlTransfer>());
const USBDEVFS_BULK: c_ulong = iorw(b'U', 2, size_of::<UsbfsBulkTransfer>());
const USBDEVFS_GET_CAPABILITIES: c_ulong = ior(b'U', 26, size_of::<u32>());
const MAX_BULK_BUFFER_LENGTH: usize = 16384;
const USBFS_CAP_NO_PACKET_SIZE_LIM: u32 = 0x04;

pub fn devices() -> Result<DeviceList, Error> {
    // TODO: Add caching mechanism to avoid rescanning sysfs on every call
    // TODO: Filter out devices without proper permissions and provide better error messages
    let mut devices = Vec::new();
    for entry in fs::read_dir(SYSFS_USB_DEVICES)? {
        let entry = entry?;
        let path = entry.path();

        // Interfaces lack devnum/busnum, so skip them.
        if !path.join("devnum").exists() || !path.join("busnum").exists() {
            continue;
        }

        let bus_number = read_u16_auto(&path, "busnum")?;
        let device_address = read_u16_auto(&path, "devnum")?;

        // TODO: Read and cache device speed from sysfs
        // TODO: Read and cache port numbers for topology

        devices.push(Device {
            inner: LinuxDevice {
                sysfs_path: path,
                bus_number,
                device_address,
            },
        });
    }

    Ok(DeviceList { devices })
}

pub fn open(device: &Device) -> Result<crate::DeviceHandle, Error> {
    let node_path = format!(
        "/dev/bus/usb/{:03}/{:03}",
        device.inner.bus_number, device.inner.device_address
    );

    // Most systems allow read/write, but fall back to read-only for users
    // without CAP_SYS_RAWIO.
    // TODO: Provide better error message when permission is denied (suggest udev rules)
    let file = match OpenOptions::new().read(true).write(true).open(&node_path) {
        Ok(file) => file,
        Err(err) if err.kind() == ErrorKind::PermissionDenied => {
            // TODO: Log a warning that device opened in read-only mode
            OpenOptions::new().read(true).open(&node_path)?
        }
        Err(err) => return Err(err.into()),
    };
    let fd = file.as_raw_fd();
    let mut caps = 0u32;
    let _ = unsafe { libc::ioctl(fd, USBDEVFS_GET_CAPABILITIES, &mut caps) };

    // TODO: Query and store the active configuration
    // TODO: Initialize claimed_interfaces tracking

    Ok(crate::DeviceHandle {
        inner: LinuxDeviceHandle { file, caps },
    })
}

pub fn get_device_descriptor(device: &Device) -> Result<DeviceDescriptor, Error> {
    let path = &device.inner.sysfs_path;

    Ok(DeviceDescriptor {
        length: 18,
        descriptor_type: 0x01,
        usb_version: read_u16_auto(path, "bcdUSB")?,
        device_class: read_u8_auto(path, "bDeviceClass")?,
        device_subclass: read_u8_auto(path, "bDeviceSubClass")?,
        device_protocol: read_u8_auto(path, "bDeviceProtocol")?,
        max_packet_size_0: read_u8_auto(path, "bMaxPacketSize0")?,
        vendor_id: read_u16_auto(path, "idVendor")?,
        product_id: read_u16_auto(path, "idProduct")?,
        device_version: read_u16_auto(path, "bcdDevice")?,
        manufacturer_string_index: read_u8_auto_optional(path, "iManufacturer")?,
        product_string_index: read_u8_auto_optional(path, "iProduct")?,
        serial_number_string_index: read_u8_auto_optional(path, "iSerialNumber")?,
        num_configurations: read_u8_auto(path, "bNumConfigurations")?,
    })
}

fn read_attr(path: &Path, attr: &str) -> Result<String, Error> {
    let contents = fs::read_to_string(path.join(attr))?;
    Ok(contents.trim().to_string())
}

fn read_attr_optional(path: &Path, attr: &str) -> Result<Option<String>, Error> {
    match fs::read_to_string(path.join(attr)) {
        Ok(contents) => Ok(Some(contents.trim().to_string())),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.into()),
    }
}

fn read_u8_auto(path: &Path, attr: &str) -> Result<u8, Error> {
    let value = read_attr(path, attr)?;
    parse_u8_auto(&value)
}

fn read_u8_auto_optional(path: &Path, attr: &str) -> Result<u8, Error> {
    match read_attr_optional(path, attr)? {
        Some(value) => parse_u8_auto(&value),
        None => Ok(0),
    }
}

fn read_u16_auto(path: &Path, attr: &str) -> Result<u16, Error> {
    let value = read_attr(path, attr)?;
    parse_u16_auto(&value)
}

fn parse_u8_auto(value: &str) -> Result<u8, Error> {
    parse_numeric_auto(value, u8::from_str_radix)
}

fn parse_u16_auto(value: &str) -> Result<u16, Error> {
    parse_numeric_auto(value, u16::from_str_radix)
}

fn parse_numeric_auto<T>(
    value: &str,
    parser: fn(&str, u32) -> Result<T, std::num::ParseIntError>,
) -> Result<T, Error> {
    let trimmed = value.trim();
    if let Ok(val) = parser(trimmed, 10) {
        return Ok(val);
    }

    let without_prefix = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    parser(without_prefix, 16).map_err(|_| Error::Unknown)
}

pub fn control_transfer(
    handle: &DeviceHandle,
    request: ControlRequest,
    data: ControlTransferData<'_>,
    timeout: Duration,
) -> Result<usize, Error> {
    // TODO: Add validation that request_type recipient bits are valid
    // TODO: Consider caching standard descriptor requests for performance
    if let Some(direction) = data.direction() {
        let setup_direction = if request.request_type & 0x80 != 0 {
            TransferDirection::In
        } else {
            TransferDirection::Out
        };

        if direction != setup_direction {
            return Err(invalid_argument());
        }
    }
    let length = usize_to_u16(data.len())?;
    let data_ptr = match data {
        ControlTransferData::None => ptr::null_mut(),
        ControlTransferData::In(buffer) => buffer.as_mut_ptr() as *mut c_void,
        ControlTransferData::Out(buffer) => buffer.as_ptr() as *mut c_void,
    };
    let mut transfer = UsbfsCtrlTransfer {
        request_type: request.request_type,
        request: request.request,
        value: request.value,
        index: request.index,
        length,
        timeout: duration_to_timeout(timeout),
        data: data_ptr,
    };

    let result = unsafe { libc::ioctl(handle.as_raw_fd(), USBDEVFS_CONTROL, &mut transfer) };
    if result < 0 {
        Err(Error::from(io::Error::last_os_error()))
    } else {
        Ok(result as usize)
    }
}

pub fn bulk_transfer(
    handle: &DeviceHandle,
    endpoint: u8,
    buffer: TransferBuffer<'_>,
    timeout: Duration,
) -> Result<usize, Error> {
    usbfs_data_transfer(handle, endpoint, buffer, timeout)
}

pub fn interrupt_transfer(
    handle: &DeviceHandle,
    endpoint: u8,
    buffer: TransferBuffer<'_>,
    timeout: Duration,
) -> Result<usize, Error> {
    usbfs_data_transfer(handle, endpoint, buffer, timeout)
}

fn usbfs_data_transfer(
    handle: &DeviceHandle,
    endpoint: u8,
    buffer: TransferBuffer<'_>,
    timeout: Duration,
) -> Result<usize, Error> {
    let endpoint_direction = if endpoint & 0x80 != 0 {
        TransferDirection::In
    } else {
        TransferDirection::Out
    };

    if buffer.direction() != endpoint_direction {
        return Err(invalid_argument());
    }
    let timeout_ms = duration_to_timeout(timeout);
    match buffer {
        TransferBuffer::In(buffer) => transfer_in_chunks(handle, endpoint, buffer, timeout_ms),
        TransferBuffer::Out(buffer) => transfer_out_chunks(handle, endpoint, buffer, timeout_ms),
    }
}

fn transfer_in_chunks(
    handle: &DeviceHandle,
    endpoint: u8,
    buffer: &mut [u8],
    timeout_ms: u32,
) -> Result<usize, Error> {
    // TODO: Optimize by using async URB submissions for multiple chunks in parallel
    // TODO: Handle short packet detection more efficiently
    let limit = handle.inner.max_bulk_chunk();
    let mut total = 0usize;
    while total < buffer.len() {
        let chunk = cmp::min(limit, buffer.len() - total);
        let chunk_u32 = usize_to_u32(chunk)?;
        let ptr = unsafe { buffer.as_mut_ptr().add(total) as *mut c_void };
        let read = submit_bulk(handle, endpoint, ptr, chunk_u32, timeout_ms)?;
        total += read;
        if read < chunk {
            break;
        }
    }
    Ok(total)
}

fn transfer_out_chunks(
    handle: &DeviceHandle,
    endpoint: u8,
    buffer: &[u8],
    timeout_ms: u32,
) -> Result<usize, Error> {
    let limit = handle.inner.max_bulk_chunk();
    let mut total = 0usize;
    while total < buffer.len() {
        let chunk = cmp::min(limit, buffer.len() - total);
        let chunk_u32 = usize_to_u32(chunk)?;
        let ptr = unsafe { buffer.as_ptr().add(total) as *mut c_void };
        let wrote = submit_bulk(handle, endpoint, ptr, chunk_u32, timeout_ms)?;
        total += wrote;
        if wrote < chunk {
            break;
        }
    }
    Ok(total)
}

fn submit_bulk(
    handle: &DeviceHandle,
    endpoint: u8,
    data_ptr: *mut c_void,
    len: u32,
    timeout_ms: u32,
) -> Result<usize, Error> {
    let mut transfer = UsbfsBulkTransfer {
        ep: endpoint as u32,
        len,
        timeout: timeout_ms,
        data: data_ptr,
    };
    let result = unsafe { libc::ioctl(handle.as_raw_fd(), USBDEVFS_BULK, &mut transfer) };
    if result < 0 {
        Err(Error::from(io::Error::last_os_error()))
    } else {
        Ok(result as usize)
    }
}

fn duration_to_timeout(timeout: Duration) -> u32 {
    if timeout.is_zero() {
        0
    } else {
        timeout.as_millis().min(u32::MAX as u128) as u32
    }
}

fn usize_to_u16(value: usize) -> Result<u16, Error> {
    if value > u16::MAX as usize {
        Err(invalid_argument())
    } else {
        Ok(value as u16)
    }
}

fn usize_to_u32(value: usize) -> Result<u32, Error> {
    if value > u32::MAX as usize {
        Err(invalid_argument())
    } else {
        Ok(value as u32)
    }
}

fn invalid_argument() -> Error {
    Error::from(io::Error::from_raw_os_error(libc::EINVAL))
}

#[cfg(test)]
mod tests {
    use super::{parse_u8_auto, parse_u16_auto};

    #[test]
    fn parses_decimal_values() {
        assert_eq!(parse_u8_auto("10").unwrap(), 10);
        assert_eq!(parse_u16_auto("255").unwrap(), 255);
    }

    #[test]
    fn parses_hex_values_with_and_without_prefix() {
        assert_eq!(parse_u8_auto("0x0A").unwrap(), 10);
        assert_eq!(parse_u16_auto("1d6b").unwrap(), 0x1d6b);
    }

    // TODO: Add tests for device enumeration (mock sysfs)
    // TODO: Add tests for control transfers
    // TODO: Add tests for bulk transfers with various buffer sizes
    // TODO: Add tests for timeout handling
    // TODO: Add tests for error conditions (permission denied, device not found, etc.)
    // TODO: Add tests for chunked transfer logic
    // TODO: Add benchmarks comparing against libusb-1.0
}
