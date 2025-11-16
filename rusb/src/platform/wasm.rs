#![cfg(all(target_arch = "wasm32", feature = "webusb"))]

//! WebUSB-specific USB backend implementation. All operations are asynchronous and rely on the
//! browser's WebUSB APIs. Build scripts enable `web_sys_unstable_apis` automatically when this
//! backend is compiled so that the necessary DOM bindings are available.

use crate::{
    ControlRequest, ControlTransferData, Device, DeviceDescriptor, DeviceHandle, DeviceList, Error,
    TransferBuffer, TransferDirection,
};
use js_sys::{Array, DataView, Uint8Array};
use std::time::Duration;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
#[cfg(feature = "webusb-threads")]
use wasm_bindgen_rayon::init_thread_pool as rayon_init_thread_pool;
use web_sys::{
    DomException, Usb, UsbConfiguration, UsbControlTransferParameters, UsbDevice, UsbDirection,
    UsbInTransferResult, UsbInterface, UsbOutTransferResult, UsbRecipient, UsbRequestType,
    UsbTransferStatus,
};

/// The WebUSB-specific device structure.
pub struct WasmDevice(pub UsbDevice);

/// The WebUSB-specific device handle.
pub struct WasmDeviceHandle {
    pub device: UsbDevice,
}

/// Initializes the rayon thread pool when the `webusb-threads` feature is enabled.
#[cfg(feature = "webusb-threads")]
pub async fn init_thread_pool(workers: Option<usize>) -> Result<(), Error> {
    let default_threads = web_sys::window()
        .map(|win| win.navigator().hardware_concurrency())
        .filter(|count| *count > 0)
        .unwrap_or(4);
    let threads = workers.unwrap_or(default_threads as usize).max(1);
    rayon_init_thread_pool(threads).await.map_err(js_to_error)
}

/// Retrieves the list of WebUSB devices that the user has already granted access to.
pub async fn devices() -> Result<DeviceList, Error> {
    let usb = usb()?;
    let value = JsFuture::from(usb.get_devices())
        .await
        .map_err(js_to_error)?;

    let devices_js = Array::from(&value);
    let mut devices = Vec::with_capacity(devices_js.length() as usize);
    for entry in devices_js.iter() {
        if let Ok(device) = entry.dyn_into::<UsbDevice>() {
            ensure_ready(&device).await?;
            devices.push(Device {
                inner: WasmDevice(device),
            });
        }
    }

    Ok(DeviceList { devices })
}

async fn ensure_ready(device: &UsbDevice) -> Result<(), Error> {
    if !device.opened() {
        JsFuture::from(device.open()).await.map_err(js_to_error)?;
    }

    if device.configuration().is_none() {
        let configurations = device.configurations();
        if configurations.length() == 0 {
            return Err(Error::NotSupported);
        }
        if let Some(first) = configurations.get(0) {
            if let Ok(config) = first.dyn_into::<web_sys::UsbConfiguration>() {
                JsFuture::from(device.select_configuration(config.configuration_value()))
                    .await
                    .map_err(js_to_error)?;
            }
        }
    }

    if let Some(configuration) = device.configuration() {
        let interfaces = configuration.interfaces();
        for iface in interfaces.iter() {
            if let Ok(interface) = iface.dyn_into::<web_sys::UsbInterface>() {
                let number = interface.interface_number();
                if let Err(err) = JsFuture::from(device.claim_interface(number as u8)).await {
                    // If the interface is already claimed we can keep going, otherwise propagate.
                    if !is_already_claimed(&err) {
                        return Err(js_to_error(err));
                    }
                }
            }
        }
    }

    Ok(())
}

fn is_already_claimed(err: &JsValue) -> bool {
    if let Some(dom) = err.dyn_ref::<DomException>() {
        return dom.name() == "InvalidStateError";
    }
    false
}

pub async fn open(device: &Device) -> Result<DeviceHandle, Error> {
    ensure_ready(&device.inner.0).await?;
    Ok(DeviceHandle {
        inner: WasmDeviceHandle {
            device: device.inner.0.clone(),
        },
    })
}

pub fn get_device_descriptor(device: &Device) -> Result<DeviceDescriptor, Error> {
    let usb_device = &device.inner.0;
    Ok(DeviceDescriptor {
        length: 18,
        descriptor_type: 1,
        usb_version: usb_device.usb_version_major() as u16,
        device_class: usb_device.device_class(),
        device_subclass: usb_device.device_subclass(),
        device_protocol: usb_device.device_protocol(),
        max_packet_size_0: 0,
        vendor_id: usb_device.vendor_id(),
        product_id: usb_device.product_id(),
        device_version: usb_device.device_version_major() as u16,
        manufacturer_string_index: 0,
        product_string_index: 0,
        serial_number_string_index: 0,
        num_configurations: usb_device.configurations().length() as u8,
    })
}

pub async fn control_transfer(
    handle: &DeviceHandle,
    request: ControlRequest,
    data: ControlTransferData<'_>,
    _timeout: Duration,
) -> Result<usize, Error> {
    let params = build_control_parameters(request)?;
    match data {
        ControlTransferData::None => {
            let promise = handle
                .inner
                .device
                .control_transfer_out_with_u8_array(&params, &Uint8Array::new_with_length(0));
            JsFuture::from(promise).await.map_err(js_to_error)?;
            Ok(0)
        }
        ControlTransferData::In(buffer) => {
            let promise = handle
                .inner
                .device
                .control_transfer_in_with_length(&params, buffer.len() as u16);
            let result = JsFuture::from(promise).await.map_err(js_to_error)?;
            let result: UsbInTransferResult = result.dyn_into().map_err(|_| Error::Unknown)?;
            ensure_status_ok(result.status())?;
            let copied = copy_in_data(result.data(), buffer);
            Ok(copied)
        }
        ControlTransferData::Out(buffer) => {
            let payload = slice_to_uint8(buffer);
            let promise = handle
                .inner
                .device
                .control_transfer_out_with_u8_array(&params, &payload);
            let result = JsFuture::from(promise).await.map_err(js_to_error)?;
            let result: UsbOutTransferResult = result.dyn_into().map_err(|_| Error::Unknown)?;
            ensure_status_ok(result.status())?;
            Ok(result.bytes_written() as usize)
        }
    }
}

pub async fn bulk_transfer(
    handle: &DeviceHandle,
    endpoint: u8,
    buffer: TransferBuffer<'_>,
    _timeout: Duration,
) -> Result<usize, Error> {
    transfer_pipe(handle, endpoint, buffer).await
}

pub async fn interrupt_transfer(
    handle: &DeviceHandle,
    endpoint: u8,
    buffer: TransferBuffer<'_>,
    _timeout: Duration,
) -> Result<usize, Error> {
    transfer_pipe(handle, endpoint, buffer).await
}

async fn transfer_pipe(
    handle: &DeviceHandle,
    endpoint: u8,
    buffer: TransferBuffer<'_>,
) -> Result<usize, Error> {
    let expected = if endpoint & 0x80 != 0 {
        TransferDirection::In
    } else {
        TransferDirection::Out
    };

    if buffer.direction() != expected {
        return Err(Error::NotSupported);
    }

    match buffer {
        TransferBuffer::In(buf) => {
            let promise = handle.inner.device.transfer_in(endpoint, buf.len() as u32);
            let value = JsFuture::from(promise).await.map_err(js_to_error)?;
            let result: UsbInTransferResult = value.dyn_into().map_err(|_| Error::Unknown)?;
            ensure_status_ok(result.status())?;
            let copied = copy_in_data(result.data(), buf);
            Ok(copied)
        }
        TransferBuffer::Out(buf) => {
            let payload = slice_to_uint8(buf);
            let promise = handle
                .inner
                .device
                .transfer_out_with_u8_array(endpoint, &payload);
            let value = JsFuture::from(promise).await.map_err(js_to_error)?;
            let result: UsbOutTransferResult = value.dyn_into().map_err(|_| Error::Unknown)?;
            ensure_status_ok(result.status())?;
            Ok(result.bytes_written() as usize)
        }
    }
}

fn copy_in_data(view: Option<DataView>, target: &mut [u8]) -> usize {
    if let Some(view) = view {
        let start = view.byte_offset() as u32;
        let end = start + view.byte_length() as u32;
        let bytes = Uint8Array::new(&view.buffer()).subarray(start, end);
        let mut temp = vec![0u8; bytes.length() as usize];
        bytes.copy_to(&mut temp[..]);
        let len = temp.len().min(target.len());
        target[..len].copy_from_slice(&temp[..len]);
        len
    } else {
        0
    }
}

fn slice_to_uint8(data: &[u8]) -> Uint8Array {
    let array = Uint8Array::new_with_length(data.len() as u32);
    array.copy_from(data);
    array
}

fn build_control_parameters(
    request: ControlRequest,
) -> Result<UsbControlTransferParameters, Error> {
    let direction = if request.request_type & 0x80 != 0 {
        UsbDirection::In
    } else {
        UsbDirection::Out
    };

    let request_type = match (request.request_type >> 5) & 0x03 {
        0 => UsbRequestType::Standard,
        1 => UsbRequestType::Class,
        2 => UsbRequestType::Vendor,
        _ => return Err(Error::NotSupported),
    };

    let recipient = match request.request_type & 0x1F {
        0 => UsbRecipient::Device,
        1 => UsbRecipient::Interface,
        2 => UsbRecipient::Endpoint,
        _ => UsbRecipient::Other,
    };

    let params = UsbControlTransferParameters::new(direction, request_type, recipient);
    params.request(request.request);
    params.value(request.value);
    params.index(request.index);
    Ok(params)
}

fn ensure_status_ok(status: UsbTransferStatus) -> Result<(), Error> {
    match status.as_str() {
        "ok" => Ok(()),
        _ => Err(Error::Unknown),
    }
}

fn usb() -> Result<Usb, Error> {
    let window = web_sys::window().ok_or(Error::NotSupported)?;
    let navigator = window.navigator();
    Ok(navigator.usb())
}

fn js_to_error(value: JsValue) -> Error {
    if let Some(dom) = value.dyn_ref::<DomException>() {
        return Error::Os(dom.code() as i32);
    }
    if let Some(err) = value.dyn_ref::<js_sys::Error>() {
        if let Some(message) = err.message().as_string() {
            let hash = message
                .bytes()
                .fold(0i32, |acc, b| acc.wrapping_add(b as i32));
            return Error::Os(hash);
        }
    }
    Error::Unknown
}
