#pragma once
#include "mex.hpp"
#include "mexAdapter.hpp"
#include "serial_comm.h"
#include <memory>
#include <string>
#include <vector>
#include <stdexcept>
#include "MatlabDataArray.hpp"

class MexFunction : public matlab::mex::Function
{
private:
	matlab::data::ArrayFactory factory;

	// Fix unique_ptr deleter
	struct DeviceDeleter
	{
		void operator()(device_context_t *dev)
		{
			if (dev)
				device_free(dev);
		}
	};
	std::unique_ptr<device_context_t, DeviceDeleter> device;

	// Fix circular buffer declaration
	std::vector<uint8_t> receive_buffer;
	size_t buffer_position{0};

	// Fix config handling
	void updateConfig(const matlab::data::StructArray &config)
	{
		if (!device)
			return;

		using namespace matlab::data;
		try
		{
			TypedArray<double> timeout = config[0]["timeout_ms"];
			TypedArray<bool> reconnect = config[0]["auto_reconnect"];

			serial_config_t usb_config{};
			usb_config.timeout_ms = static_cast<uint32_t>(timeout[0]);
			uint32_t flags = reconnect[0] ? DEVICE_FLAG_AUTO_RECONNECT : 0;

			device_configure_context(device.get(), &usb_config, flags);
		}
		catch (const InvalidFieldException &)
		{
			throwMatlabError("MATLAB:error:config", "Missing required fields");
		}
	}

	// Simplified device management
	void openDevice(matlab::mex::ArgumentList &outputs,
					matlab::mex::ArgumentList &inputs)
	{
		if (!device)
		{
			device.reset(device_create());
		}
		// ...rest of open logic...
	}

	bool is_initialized = false;
	DeviceConfig config;

	void validateState();
	void validateInputs(const matlab::mex::ArgumentList &inputs, size_t min_args);
	void listDevices(matlab::mex::ArgumentList &outputs, matlab::mex::ArgumentList &inputs);
	void writeData(matlab::mex::ArgumentList &outputs, matlab::mex::ArgumentList &inputs);
	void readData(matlab::mex::ArgumentList &outputs, matlab::mex::ArgumentList &inputs);
	void closeDevice();
	void setConfig(matlab::mex::ArgumentList &inputs);
	void getConfig(matlab::mex::ArgumentList &outputs);

	// Error handling utilities
	void handleDeviceError(const std::exception &err)
	{
		closeDevice();
		throwMatlabError("MATLAB:error:device_error", err.what());
	}

	void throwMatlabError(const std::string &id, const std::string &msg)
	{
		matlab::data::StringArray idArray = factory.createScalar(id);
		matlab::data::StringArray msgArray = factory.createScalar(msg);
		throwException(idArray, msgArray);
	}

	// Buffer management
	void flushBuffer()
	{
		if (device && device->handle)
		{
			uint8_t dummy[64];
			int received;
			while (receive_data(device.get(), dummy, sizeof(dummy), &received))
			{
				if (received == 0)
					break;
			}
		}
	}

	size_t getAvailableBuffer()
	{
		return config.buffer_size - circular_buffer.size();
	}

	// Add error handling helper
	void checkDeviceError()
	{
		if (device)
		{
			char error[256];
			if (device_get_error(device.get(), error, sizeof(error)))
			{
				throwMatlabError("MATLAB:error:device", error);
			}
		}
	}

	// Add configuration parser
	device_config_t parseConfig(const matlab::data::StructArray &config)
	{
		device_config_t result = {};
		try
		{
			if (config.hasField("timeout_ms"))
			{
				result.usb_config.timeout_ms =
					static_cast<uint32_t>(config[0]["timeout_ms"][0]);
			}
			if (config.hasField("auto_reconnect"))
			{
				result.flags |= config[0]["auto_reconnect"][0] ? DEVICE_FLAG_AUTO_RECONNECT : 0;
			}
		}
		catch (const matlab::data::TypeMismatchException &)
		{
			throwMatlabError("MATLAB:error:config", "Invalid configuration type");
		}
		return result;
	}

	// Update data handling
	matlab::data::TypedArray<uint8_t> readBuffer(size_t bytes)
	{
		std::vector<uint8_t> buffer(bytes);
		int received = 0;

		if (!receive_data(device.get(), buffer.data(),
						  static_cast<int>(bytes), &received))
		{
			throwMatlabError("MATLAB:error:read", "Read operation failed");
		}

		return factory.createArray<uint8_t>({1, static_cast<size_t>(received)},
											buffer.begin(), buffer.begin() + received);
	}

public:
	void operator()(matlab::mex::ArgumentList outputs, matlab::mex::ArgumentList inputs);
	~MexFunction() { closeDevice(); }
};