#pragma once
#ifdef __cplusplus
extern "C"
{
#endif
#include < libusb.h>
#include <stdint.h>
#include <stdbool.h>

#define FTDI_VID 0x0403
#define STM32_VID 0x0483

	typedef enum
	{
		DEVICE_STATE_CLOSED = 0,
		DEVICE_STATE_OPENING,
		DEVICE_STATE_OPEN,
		DEVICE_STATE_ERROR
	} device_state_t;

	typedef enum
	{
		SERIAL_SUCCESS = 0,
		SERIAL_ERROR_NOT_FOUND = -1,
		SERIAL_ERROR_ACCESS = -2,
		SERIAL_ERROR_BUSY = -3,
		SERIAL_ERROR_TIMEOUT = -4,
		SERIAL_ERROR_INVALID_CONFIG = -5
	} serial_error_t;

	typedef struct
	{
		uint32_t timeout_ms;
		uint32_t buffer_size;
		uint8_t endpoint_in;
		uint8_t endpoint_out;
	} serial_config_t;

	typedef struct
	{
		uint16_t vid;
		uint16_t pid;
		char serial[256];
		char description[256];
		libusb_device_handle *handle;
		device_state_t state;
		serial_error_t last_error;
		serial_config_t config;
	} usb_device_info;

	// Add unified configuration structure
	typedef struct
	{
		serial_config_t usb_config;
		struct
		{
			bool auto_reconnect;
			bool debug_logging;
			char device_filter[64];
		} mex_config;
	} device_config_t;

// Add configuration flags
#define DEVICE_FLAG_AUTO_RECONNECT (1 << 0)
#define DEVICE_FLAG_DEBUG_LOGGING (1 << 1)
#define DEVICE_FLAG_NONBLOCKING (1 << 2)

	typedef struct
	{
		serial_config_t usb_config;
		uint32_t flags;
		void *user_context;
		char error_message[256];
	} device_context_t;

	// Add simplified interface
	typedef struct
	{
		usb_device_info usb;
		device_config_t config;
		void *user_data; // For MEX-specific data
	} device_handle_t;

	// Function declarations
	bool discover_devices(usb_device_info *devices, int *count);
	bool open_device(usb_device_info *device);
	bool send_data(usb_device_info *device, uint8_t *data, int length);
	bool receive_data(usb_device_info *device, uint8_t *buffer, int length, int *received);
	void close_device(usb_device_info *device);
	bool set_device_config(usb_device_info *device, const serial_config_t *config);
	serial_error_t get_last_error(usb_device_info *device);

	// Simplified API
	device_handle_t *device_create(void);
	void device_destroy(device_handle_t *dev);
	bool device_configure_handle(device_handle_t *dev, const device_config_t *config);
	bool device_open(device_handle_t *dev, uint16_t vid, uint16_t pid, const char *serial);

	// Add simplified API
	device_context_t *device_init(void);
	void device_free(device_context_t *ctx);
	bool device_configure_context(device_context_t *ctx, const serial_config_t *config, uint32_t flags);
	int device_get_error(device_context_t *ctx, char *buffer, size_t size);

#ifdef __cplusplus
}
#endif