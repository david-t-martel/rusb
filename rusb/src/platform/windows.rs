//! Windows-specific USB backend implementation.
//!
//! TODO: Add support for isochronous transfers
//! TODO: Add interface claiming/releasing via WinUsb_ClaimInterface/WinUsb_ReleaseInterface
//! TODO: Add configuration descriptor parsing
//! TODO: Add string descriptor reading via WinUsb_GetDescriptor
//! TODO: Add support for multiple interfaces per device
//! TODO: Add support for composite devices with multiple WinUSB interfaces
//! TODO: Add device reset support
//! TODO: Add clear halt via WinUsb_ResetPipe or WinUsb_AbortPipe
//! TODO: Add hotplug notification using RegisterDeviceNotification
//! TODO: Cache device descriptors to avoid repeated queries

use crate::{
    ControlRequest, ControlTransferData, Device, DeviceDescriptor, DeviceList, Error,
    TransferBuffer, TransferDirection,
};
use std::ffi::{OsString, c_void};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::time::Duration;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA,
    SP_DEVICE_INTERFACE_DETAIL_DATA_W, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces,
    SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW,
};
use windows::Win32::Devices::Usb::{
    PIPE_TRANSFER_TIMEOUT, WINUSB_INTERFACE_HANDLE, WINUSB_SETUP_PACKET, WinUsb_ControlTransfer,
    WinUsb_Free, WinUsb_GetDescriptor, WinUsb_Initialize, WinUsb_ReadPipe, WinUsb_SetPipePolicy,
    WinUsb_WritePipe,
};
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::core::{GUID, PCWSTR};

// Same GUID as in libusb's windows_winusb.c
const GUID_DEVINTERFACE_USB_DEVICE: GUID = GUID::from_u128(0xA5DCBF10_6530_11D2_901F_00C04FB951ED);

/// The windows-specific device structure.
/// TODO: Cache device descriptor to avoid reopening device for descriptor queries
/// TODO: Store instance ID and hardware ID for better device identification
/// TODO: Add support for multiple interfaces (composite devices)
#[derive(Debug)]
pub struct WindowsDevice {
    device_path: OsString,
    // TODO: Add cached_descriptor: Option<DeviceDescriptor>
    // TODO: Add instance_id: Option<String>
}

/// The windows-specific device handle.
/// TODO: Track claimed interfaces to support composite devices
/// TODO: Store multiple interface handles for multi-interface devices
pub struct WindowsDeviceHandle {
    pub file: HANDLE,
    pub interface: WINUSB_INTERFACE_HANDLE,
    // TODO: Add additional_interfaces: Vec<WINUSB_INTERFACE_HANDLE> for composite devices
    // TODO: Add claimed_interfaces: HashSet<u8>
}

impl Drop for WindowsDeviceHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = WinUsb_Free(self.interface);
        }
        if self.file != INVALID_HANDLE_VALUE {
            let _ = unsafe { CloseHandle(self.file) };
        }
    }
}

pub fn devices() -> Result<DeviceList, Error> {
    // TODO: Add caching mechanism to avoid expensive device enumeration on every call
    // TODO: Support filtering by device class/interface class
    // TODO: Add better error handling and error messages for Setup API failures
    let dev_info_set = unsafe {
        SetupDiGetClassDevsW(
            Some(&GUID_DEVINTERFACE_USB_DEVICE),
            None,
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )
    }?;

    if dev_info_set.is_invalid() {
        return Err(Error::from(windows::core::Error::from_win32()));
    }

    let mut devices = Vec::new();
    let mut dev_interface_data: SP_DEVICE_INTERFACE_DATA = unsafe { std::mem::zeroed() };
    dev_interface_data.cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;
    let mut i = 0;

    while unsafe {
        SetupDiEnumDeviceInterfaces(
            dev_info_set,
            None,
            &GUID_DEVINTERFACE_USB_DEVICE,
            i,
            &mut dev_interface_data,
        )
    }
    .is_ok()
    {
        i += 1;
        // TODO: Extract device instance ID for better tracking
        // TODO: Check if device is accessible before adding to list
        let mut required_size = 0;

        // First call to get the required buffer size
        unsafe {
            let _ = SetupDiGetDeviceInterfaceDetailW(
                dev_info_set,
                &dev_interface_data,
                None,
                0,
                Some(&mut required_size),
                None,
            );
        };

        let mut detail_data_buffer = vec![0u8; required_size as usize];
        let detail_data = detail_data_buffer.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
        unsafe {
            (*detail_data).cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32
        };

        if unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                dev_info_set,
                &dev_interface_data,
                Some(detail_data),
                required_size,
                Some(&mut required_size),
                None,
            )
        }
        .is_err()
        {
            continue;
        }

        let device_path_wide = unsafe { (*detail_data).DevicePath.as_slice() };
        let null_pos = device_path_wide
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(device_path_wide.len());
        let device_path = OsString::from_wide(&device_path_wide[..null_pos]);

        devices.push(Device {
            inner: WindowsDevice { device_path },
        });
    }

    unsafe {
        let _ = SetupDiDestroyDeviceInfoList(dev_info_set);
    };

    Ok(DeviceList { devices })
}

pub fn open(device: &Device) -> Result<crate::DeviceHandle, Error> {
    let path_wide: Vec<u16> = device
        .inner
        .device_path
        .encode_wide()
        .chain(Some(0))
        .collect();

    let file = unsafe {
        CreateFileW(
            PCWSTR(path_wide.as_ptr()),
            0x80000000 | 0x40000000, // GENERIC_READ | GENERIC_WRITE
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED,
            None,
        )
    }?;

    if file == INVALID_HANDLE_VALUE {
        return Err(Error::from(windows::core::Error::from_win32()));
    }

    let mut interface = WINUSB_INTERFACE_HANDLE::default();
    unsafe {
        WinUsb_Initialize(file, &mut interface)?;
    }

    Ok(crate::DeviceHandle {
        inner: WindowsDeviceHandle { file, interface },
    })
}

pub fn get_device_descriptor(device: &Device) -> Result<DeviceDescriptor, Error> {
    // TODO: Cache descriptor in WindowsDevice to avoid reopening the device
    // TODO: Validate descriptor length and type fields
    let handle = open(device)?;
    let mut descriptor: DeviceDescriptor = unsafe { std::mem::zeroed() };
    let mut length = 0;

    let result = unsafe {
        WinUsb_GetDescriptor(
            handle.inner.interface,
            0x01, // USB_DEVICE_DESCRIPTOR_TYPE
            0,
            0,
            Some(std::slice::from_raw_parts_mut(
                &mut descriptor as *mut _ as *mut u8,
                std::mem::size_of::<DeviceDescriptor>(),
            )),
            &mut length,
        )
    };

    if result.is_err() {
        return Err(Error::from(windows::core::Error::from_win32()));
    }

    Ok(descriptor)
}

pub fn control_transfer(
    handle: &crate::DeviceHandle,
    request: ControlRequest,
    data: ControlTransferData<'_>,
    timeout: Duration,
) -> Result<usize, Error> {
    // TODO: Validate request_type bits are correct
    // TODO: Add retry logic for transient errors
    if let Some(direction) = data.direction() {
        let setup_dir = if request.request_type & 0x80 != 0 {
            TransferDirection::In
        } else {
            TransferDirection::Out
        };
        if direction != setup_dir {
            return Err(Error::NotSupported);
        }
    }

    let length = usize_to_u16(data.len())?;
    let setup = WINUSB_SETUP_PACKET {
        RequestType: request.request_type,
        Request: request.request,
        Value: request.value,
        Index: request.index,
        Length: length,
    };

    maybe_set_timeout(&handle.inner, 0, timeout)?;
    let mut transferred = 0u32;

    unsafe {
        match data {
            ControlTransferData::None => {
                WinUsb_ControlTransfer(
                    handle.inner.interface,
                    setup,
                    None,
                    Some(&mut transferred as *mut u32),
                    None,
                )?;
            }
            ControlTransferData::In(buffer) => {
                ensure_u32_len(buffer.len())?;
                WinUsb_ControlTransfer(
                    handle.inner.interface,
                    setup,
                    Some(buffer),
                    Some(&mut transferred as *mut u32),
                    None,
                )?;
            }
            ControlTransferData::Out(buffer) => {
                ensure_u32_len(buffer.len())?;
                let mut owned = buffer.to_vec();
                WinUsb_ControlTransfer(
                    handle.inner.interface,
                    setup,
                    Some(owned.as_mut_slice()),
                    Some(&mut transferred as *mut u32),
                    None,
                )?;
            }
        }
    }

    Ok(transferred as usize)
}

pub fn bulk_transfer(
    handle: &crate::DeviceHandle,
    endpoint: u8,
    buffer: TransferBuffer<'_>,
    timeout: Duration,
) -> Result<usize, Error> {
    pipe_transfer(handle, endpoint, buffer, timeout)
}

pub fn interrupt_transfer(
    handle: &crate::DeviceHandle,
    endpoint: u8,
    buffer: TransferBuffer<'_>,
    timeout: Duration,
) -> Result<usize, Error> {
    pipe_transfer(handle, endpoint, buffer, timeout)
}

fn pipe_transfer(
    handle: &crate::DeviceHandle,
    endpoint: u8,
    buffer: TransferBuffer<'_>,
    timeout: Duration,
) -> Result<usize, Error> {
    let expected = if endpoint & 0x80 != 0 {
        TransferDirection::In
    } else {
        TransferDirection::Out
    };
    if buffer.direction() != expected {
        return Err(Error::NotSupported);
    }

    ensure_u32_len(buffer.len())?;
    maybe_set_timeout(&handle.inner, endpoint, timeout)?;
    let mut transferred = 0u32;

    unsafe {
        match buffer {
            TransferBuffer::In(buf) => {
                WinUsb_ReadPipe(
                    handle.inner.interface,
                    endpoint,
                    Some(buf),
                    Some(&mut transferred as *mut u32),
                    None,
                )?;
            }
            TransferBuffer::Out(buf) => {
                WinUsb_WritePipe(
                    handle.inner.interface,
                    endpoint,
                    buf,
                    Some(&mut transferred as *mut u32),
                    None,
                )?;
            }
        }
    }

    Ok(transferred as usize)
}

fn maybe_set_timeout(
    handle: &WindowsDeviceHandle,
    endpoint: u8,
    timeout: Duration,
) -> Result<(), Error> {
    // TODO: Cache timeout settings per endpoint to avoid redundant calls
    if timeout.is_zero() {
        return Ok(());
    }

    let mut value = duration_to_timeout(timeout);
    unsafe {
        WinUsb_SetPipePolicy(
            handle.interface,
            endpoint,
            PIPE_TRANSFER_TIMEOUT,
            std::mem::size_of::<u32>() as u32,
            &value as *const u32 as *const c_void,
        )?;
    }

    Ok(())
}

// TODO: Add tests for Windows-specific functionality
// TODO: Add tests comparing against libusb-1.0 on Windows
// TODO: Add benchmarks for transfer performance
// TODO: Add helper functions to query pipe information (type, max packet size, etc.)

fn ensure_u32_len(len: usize) -> Result<(), Error> {
    if len > u32::MAX as usize {
        Err(Error::NotSupported)
    } else {
        Ok(())
    }
}

fn usize_to_u16(value: usize) -> Result<u16, Error> {
    if value > u16::MAX as usize {
        Err(Error::NotSupported)
    } else {
        Ok(value as u16)
    }
}

fn duration_to_timeout(timeout: Duration) -> u32 {
    timeout.as_millis().min(u32::MAX as u128) as u32
}
