classdef SerialDevice < handle % was serialDevice
	properties (Access = private)
		% Use consistent naming
		IsOpen (1,1) logical = false
		Config struct = struct(...
			'timeout_ms', uint32(1000), ...
			'buffer_size', uint32(4096), ...
			'auto_reconnect', true)
		LastError struct = struct(...
			'code', int32(0), ...
			'message', '')
		RetryCount (1,1) uint32 = 0
		MaxRetries (1,1) uint32 = 3
	end

	properties (Constant, Access = private)
		% Add error identifiers
		ErrorIds = struct(...
			'NotOpen', 'SerialDevice:DeviceNotOpen', ...
			'Timeout', 'SerialDevice:Timeout', ...
			'Disconnected', 'SerialDevice:Disconnected')
	end

	methods
		function obj = SerialDevice()
		end

		function devices = list(obj)
			devices = serial_mex('list');
		end

		function open(obj, vid, pid, serial)
			if obj.IsOpen
				error('Device already open');
			end

			if nargin < 4
				serial_mex('open', vid, pid);
			else
				serial_mex('open', vid, pid, serial);
			end
			obj.IsOpen = true;
		end

		function write(obj, data)
			validateattributes(data, {'uint8'}, {'vector'}, 'write', 'data');
			if ~obj.verifyConnection()
				error(obj.ErrorIds.NotOpen, 'Device not open');
			end
			try
				serial_mex('write', data);
			catch ME
				obj.handleError(ME);
			end
		end

		function data = read(obj, bytes)
			if ~obj.verifyConnection()
				error('Device not open');
			end
			try
				if nargin < 2
					bytes = obj.Config.buffer_size;
				end
				data = serial_mex('read', bytes);
			catch ME
				if strcmp(ME.identifier, 'MATLAB:error:device_disconnected')
					obj.close();
				end
				rethrow(ME);
			end
		end

		function close(obj)
			if obj.IsOpen
				serial_mex('close');
				obj.IsOpen = false;
			end
		end

		function delete(obj)
			obj.close();
		end

		function setConfig(obj, name, value)
			validateattributes(value, {'numeric', 'logical'}, {'scalar'});
			if ~isfield(obj.Config, name)
				error('Invalid configuration parameter: %s', name);
			end
			obj.Config.(name) = value;
			if obj.IsOpen
				serial_mex('setConfig', name, value);
			end
		end

		function flush(obj)
			if ~obj.verifyConnection()
				error('Device not open');
			end
			serial_mex('flush');
		end

		function status = isOpen(obj)
			status = obj.IsOpen;
		end

		% Add connection verification
		function verified = verifyConnection(obj)
			if ~obj.IsOpen
				verified = false;
			else
				try
					% Send test message
					serial_mex('ping');
					verified = true;
				catch
					verified = false;
				end
			end

			% Add automatic reconnection
			function success = tryReconnect(obj)
				if ~obj.Config.auto_reconnect || obj.RetryCount >= obj.MaxRetries
					success = false;
				end
				try
					obj.close();
					obj.open(obj.LastVid, obj.LastPid);
					obj.RetryCount = obj.RetryCount + 1;
					success = true;
				catch
					success = false;
				end
			end
		end

		function handleError(obj, ME)
			switch ME.identifier
				case 'MATLAB:error:device_disconnected'
					error(obj.ErrorIds.Disconnected, 'Device disconnected');
				case 'MATLAB:error:timeout'
					error(obj.ErrorIds.Timeout, 'Operation timed out');
				otherwise
					rethrow(ME);
			end
		end
	end
end
