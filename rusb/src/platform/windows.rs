//! Windows-specific USB backend implementation.

use crate::{
    ConfigurationDescriptor, ControlRequest, ControlTransferData, Device, DeviceDescriptor,
    DeviceList, Error, Speed, TransferBuffer, TransferDirection,
};
use std::collections::HashMap;
use std::ffi::{OsString, c_void};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::sync::Mutex;
use std::time::Duration;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA,
    SP_DEVICE_INTERFACE_DETAIL_DATA_W, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces,
    SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW,
};
use windows::Win32::Devices::Usb::{
    PIPE_TRANSFER_TIMEOUT, USB_INTERFACE_DESCRIPTOR, WINUSB_INTERFACE_HANDLE, WINUSB_SETUP_PACKET,
    WinUsb_ControlTransfer, WinUsb_Free, WinUsb_GetAssociatedInterface, WinUsb_GetDescriptor,
    WinUsb_Initialize, WinUsb_QueryInterfaceSettings, WinUsb_ReadPipe, WinUsb_ResetPipe,
    WinUsb_SetCurrentAlternateSetting, WinUsb_SetPipePolicy, WinUsb_WritePipe,
};
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::core::{GUID, PCWSTR};

// Same GUID as in libusb's windows_winusb.c
const GUID_DEVINTERFACE_USB_DEVICE: GUID = GUID::from_u128(0xA5DCBF10_6530_11D2_901F_00C04FB951ED);

/// The windows-specific device structure.
#[derive(Debug)]
pub struct WindowsDevice {
    device_path: OsString,
}

impl WindowsDevice {
    pub fn bus_number(&self) -> u8 {
        0
    }

    pub fn address(&self) -> u8 {
        0
    }

    pub fn speed(&self) -> Speed {
        Speed::Unknown
    }
}

/// The windows-specific device handle.
pub struct WindowsDeviceHandle {
    pub file: HANDLE,
    pub interface: WINUSB_INTERFACE_HANDLE,
    pub claimed_interfaces: Mutex<HashMap<u8, WINUSB_INTERFACE_HANDLE>>,
}

impl Drop for WindowsDeviceHandle {
    fn drop(&mut self) {
        if let Ok(guard) = self.claimed_interfaces.lock() {
            for (_, handle) in guard.iter() {
                if *handle != self.interface {
                    unsafe {
                        let _ = WinUsb_Free(*handle);
                    }
                }
            }
        }
        unsafe {
            let _ = WinUsb_Free(self.interface);
        }
        if self.file != INVALID_HANDLE_VALUE {
            let _ = unsafe { CloseHandle(self.file) };
        }
    }
}

pub fn devices() -> Result<DeviceList, Error> {
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
        let mut required_size = 0;

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
        inner: WindowsDeviceHandle {
            file,
            interface,
            claimed_interfaces: Mutex::new(HashMap::new()),
        },
    })
}

pub fn get_device_descriptor(device: &Device) -> Result<DeviceDescriptor, Error> {
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
    let handle_inner = &handle.inner;
    let winusb_handle = get_interface_handle(handle_inner, endpoint)?;

    maybe_set_timeout_handle(winusb_handle, endpoint, timeout)?;
    let mut transferred = 0u32;

    unsafe {
        match buffer {
            TransferBuffer::In(buf) => {
                WinUsb_ReadPipe(
                    winusb_handle,
                    endpoint,
                    Some(buf),
                    Some(&mut transferred as *mut u32),
                    None,
                )?;
            }
            TransferBuffer::Out(buf) => {
                WinUsb_WritePipe(
                    winusb_handle,
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

fn get_interface_handle(
    handle: &WindowsDeviceHandle,
    _endpoint: u8,
) -> Result<WINUSB_INTERFACE_HANDLE, Error> {
    Ok(handle.interface)
}

fn maybe_set_timeout(
    handle: &WindowsDeviceHandle,
    endpoint: u8,
    timeout: Duration,
) -> Result<(), Error> {
    maybe_set_timeout_handle(handle.interface, endpoint, timeout)
}

fn maybe_set_timeout_handle(
    interface: WINUSB_INTERFACE_HANDLE,
    endpoint: u8,
    timeout: Duration,
) -> Result<(), Error> {
    if timeout.is_zero() {
        return Ok(());
    }

    let mut value = duration_to_timeout(timeout);
    unsafe {
        WinUsb_SetPipePolicy(
            interface,
            endpoint,
            PIPE_TRANSFER_TIMEOUT,
            std::mem::size_of::<u32>() as u32,
            &value as *const u32 as *const c_void,
        )?;
    }

    Ok(())
}

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

pub fn claim_interface(handle: &crate::DeviceHandle, interface_number: u8) -> Result<(), Error> {
    let mut guard = handle
        .inner
        .claimed_interfaces
        .lock()
        .map_err(|_| Error::Unknown)?;

    if guard.contains_key(&interface_number) {
        return Ok(());
    }

    unsafe {
        let mut desc: USB_INTERFACE_DESCRIPTOR = std::mem::zeroed();
        if WinUsb_QueryInterfaceSettings(handle.inner.interface, 0, &mut desc).is_ok() {
            if desc.bInterfaceNumber == interface_number {
                guard.insert(interface_number, handle.inner.interface);
                return Ok(());
            }
        }
    }

    let mut index = 0;
    loop {
        let mut associated_handle = WINUSB_INTERFACE_HANDLE::default();
        unsafe {
            if WinUsb_GetAssociatedInterface(
                handle.inner.interface,
                index,
                &mut associated_handle,
            )
            .is_err()
            {
                break;
            }

            let mut desc: USB_INTERFACE_DESCRIPTOR = std::mem::zeroed();
            if WinUsb_QueryInterfaceSettings(associated_handle, 0, &mut desc).is_ok() {
                if desc.bInterfaceNumber == interface_number {
                    guard.insert(interface_number, associated_handle);
                    return Ok(());
                }
            }

            let _ = WinUsb_Free(associated_handle);
        }
        index += 1;
        if index > 255 {
            break;
        }
    }

    Err(Error::NotSupported)
}

pub fn release_interface(handle: &crate::DeviceHandle, interface_number: u8) -> Result<(), Error> {
    let mut guard = handle
        .inner
        .claimed_interfaces
        .lock()
        .map_err(|_| Error::Unknown)?;
    if let Some(h) = guard.remove(&interface_number) {
        if h != handle.inner.interface {
            unsafe {
                let _ = WinUsb_Free(h);
            }
        }
        Ok(())
    } else {
        Err(Error::NotSupported)
    }
}

pub fn set_interface_alt_setting(
    handle: &crate::DeviceHandle,
    interface: u8,
    alt_setting: u8,
) -> Result<(), Error> {
    let guard = handle
        .inner
        .claimed_interfaces
        .lock()
        .map_err(|_| Error::Unknown)?;
    if let Some(&h) = guard.get(&interface) {
        unsafe {
            WinUsb_SetCurrentAlternateSetting(h, alt_setting)?;
        }
        Ok(())
    } else {
        Err(Error::NotSupported)
    }
}

pub fn reset_device(_handle: &crate::DeviceHandle) -> Result<(), Error> {
    Err(Error::NotSupported)
}

pub fn clear_halt(handle: &crate::DeviceHandle, endpoint: u8) -> Result<(), Error> {
    unsafe {
        WinUsb_ResetPipe(handle.inner.interface, endpoint)?;
    }
    Ok(())
}

pub fn detach_kernel_driver(_handle: &crate::DeviceHandle, _interface: u8) -> Result<(), Error> {
    Err(Error::NotSupported)
}

pub fn attach_kernel_driver(_handle: &crate::DeviceHandle, _interface: u8) -> Result<(), Error> {
    Err(Error::NotSupported)
}

pub fn get_active_configuration(_device: &Device) -> Result<ConfigurationDescriptor, Error> {
    Err(Error::NotSupported)
}

pub fn get_configuration_descriptor(
    _device: &Device,
    _index: u8,
) -> Result<ConfigurationDescriptor, Error> {
    Err(Error::NotSupported)
}

pub fn get_config_descriptor_by_value(
    _device: &Device,
    _value: u8,
) -> Result<ConfigurationDescriptor, Error> {
    Err(Error::NotSupported)
}
