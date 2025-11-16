#include "mex.hpp"
#include "mexAdapter.hpp"
#include "serial_comm.h"
#include <vector>
#include <string>
#include <memory>
#include <stdexcept>

class MexFunction : public matlab::mex::Function
{
private:
	struct DeviceConfig
	{
		uint32_t timeout_ms = 1000;
		size_t buffer_size = 4096;
		bool auto_reconnect = true;
	};

	std::unique_ptr<usb_device_info> device;
	matlab::data::ArrayFactory factory;
	bool is_initialized = false;
	DeviceConfig config;
	std::vector<uint8_t> circular_buffer;

	// Add validation helpers
	void validateState()
	{
		if (!is_initialized || !device || !device->handle)
		{
			throw std::runtime_error("Device not initialized or not open");
		}
	}

	void validateInputs(const matlab::mex::ArgumentList &inputs, size_t min_args)
	{
		if (inputs.size() < min_args)
		{
			throw std::invalid_argument("Insufficient arguments");
		}
	}

	void listDevices(matlab::mex::ArgumentList &outputs,
					 matlab::mex::ArgumentList &inputs)
	{
		std::vector<usb_device_info> devices(16);
		int count = devices.size();

		if (!discover_devices(devices.data(), &count))
		{
			throw std::runtime_error("No devices found");
		}

		std::vector<std::string> fieldNames = {"vid", "pid", "serial", "description"};
		matlab::data::StructArray deviceList = factory.createStructArray({1, static_cast<size_t>(count)}, fieldNames);

		for (int i = 0; i < count; i++)
		{
			deviceList[i]["vid"] = factory.createScalar<uint16_t>(devices[i].vid);
			deviceList[i]["pid"] = factory.createScalar<uint16_t>(devices[i].pid);
			deviceList[i]["serial"] = factory.createCharArray(devices[i].serial);
			deviceList[i]["description"] = factory.createCharArray(devices[i].description);
		}

		outputs[0] = std::move(deviceList);
	}

	void openDevice(matlab::mex::ArgumentList &outputs,
					matlab::mex::ArgumentList &inputs)
	{
		validateInputs(inputs, 2);

		try
		{
			device = std::make_unique<usb_device_info>();
			device->vid = static_cast<uint16_t>(inputs[0][0]);
			device->pid = static_cast<uint16_t>(inputs[1][0]);

			if (inputs.size() > 2)
			{
				matlab::data::CharArray serial = inputs[2];
				strncpy(device->serial, serial.toAscii().c_str(), sizeof(device->serial) - 1);
			}

			if (!open_device(device.get()))
			{
				throw std::runtime_error("Failed to open device");
			}
			is_initialized = true;
		}
		catch (...)
		{
			device.reset();
			is_initialized = false;
			throw;
		}
		outputs[0] = factory.createLogicalScalar(true);
	}

	void writeData(matlab::mex::ArgumentList &outputs,
				   matlab::mex::ArgumentList &inputs)
	{
		validateState();
		validateInputs(inputs, 1);

		matlab::data::TypedArray<uint8_t> data = std::move(inputs[0]);
		if (data.getNumberOfElements() == 0)
		{
			throw std::invalid_argument("Empty data array");
		}

		if (!send_data(device.get(), data.begin(),
					   static_cast<int>(data.getNumberOfElements())))
		{
			throw std::runtime_error("Write operation failed");
		}
		outputs[0] = factory.createLogicalScalar(true);
	}

	void readData(matlab::mex::ArgumentList &outputs,
				  matlab::mex::ArgumentList &inputs)
	{
		validateState();

		size_t bytes = (inputs.empty()) ? 64 : static_cast<size_t>(inputs[0][0]);

		if (bytes == 0 || bytes > 65536)
		{
			throw std::invalid_argument("Invalid buffer size");
		}

		std::vector<uint8_t> buffer(bytes);
		int received = 0;

		if (!receive_data(device.get(), buffer.data(),
						  static_cast<int>(bytes), &received))
		{
			throw std::runtime_error("Read operation failed");
		}

		outputs[0] = factory.createArray<uint8_t>(
			{1, static_cast<size_t>(received)},
			buffer.begin(),
			buffer.begin() + received);
	}

	void closeDevice()
	{
		if (device && device->handle)
		{
			close_device(device.get());
			device.reset();
		}
		is_initialized = false;
	}

	// Device management
	void cacheDeviceList()
	{
		// Cache discovered devices
	}

	bool attemptReconnect()
	{
		if (!config.auto_reconnect)
			return false;
		// Reconnection logic
		return true;
	}

	// Error management
	class DeviceError : public std::runtime_error
	{
	public:
		enum class ErrorType
		{
			TIMEOUT,
			DISCONNECTED,
			BUFFER_OVERFLOW,
			INVALID_STATE
		};

		DeviceError(ErrorType type, const std::string &msg)
			: std::runtime_error(msg), error_type(type) {}

		ErrorType getType() const { return error_type; }

	private:
		ErrorType error_type;
	};

	// Data management
	void ensureBufferSpace(size_t required)
	{
		if (circular_buffer.size() + required > config.buffer_size)
		{
			throw DeviceError(
				DeviceError::ErrorType::BUFFER_OVERFLOW,
				"Buffer overflow detected");
		}
	}

	// Configuration management
	void setConfig(matlab::mex::ArgumentList &inputs)
	{
		validateInputs(inputs, 1);
		// Parse configuration from MATLAB struct
	}

	void getConfig(matlab::mex::ArgumentList &outputs)
	{
		// Return current configuration as MATLAB struct
	}

public:
	void operator()(matlab::mex::ArgumentList outputs,
					matlab::mex::ArgumentList inputs)
	{
		try
		{
			validateInputs(inputs, 1);
			std::string command = inputs[0][0];
			inputs.erase(inputs.begin());

			if (command == "list")
				listDevices(outputs, inputs);
			else if (command == "open")
				openDevice(outputs, inputs);
			else if (command == "write")
				writeData(outputs, inputs);
			else if (command == "read")
				readData(outputs, inputs);
			else if (command == "close")
			{
				closeDevice();
				outputs[0] = factory.createLogicalScalar(true);
			}
			else if (command == "setConfig")
				setConfig(inputs);
			else if (command == "getConfig")
				getConfig(outputs);
			else
				throw std::invalid_argument("Unknown command");
		}
		catch (const std::exception &e)
		{
			closeDevice();
			throw;
		}
	}

	~MexFunction()
	{
		closeDevice();
	}
};