% Add release directory to path
addpath(fullfile(pwd, 'release'));

% Build with local paths first, then custom paths if specified
try
	build_serial_mex('debug', true);
catch
	% If local build fails, try with custom paths
	build_serial_mex('libusb_include', 'C:\custom\include', ...
		'libusb_lib', 'C:\custom\lib', ...
		'debug', true);
end

% Create device object
dev = SerialDevice();

try
	% List available devices
	devices = dev.list()

	% Open first STM32 device found
	dev.open(hex2dec('0483'), hex2dec('5740'))

	% Configure device before use
	dev.setConfig('timeout_ms', 1000);
	dev.setConfig('endpoint_in', hex2dec('81'));
	dev.setConfig('endpoint_out', hex2dec('01'));

	% Add proper device verification
	if ~dev.isOpen()
		error('Failed to open device');
	end

	% Send data
	bytes_written = dev.write(uint8([1 2 3 4]));
	if bytes_written ~= 4
		warning('Incomplete write operation');
	end

	% Read response
	response = dev.read(64);
	if isempty(response)
		warning('No data received within timeout period');
	end

catch ME
	% Add specific error handling
	switch ME.identifier
		case 'MATLAB:error:device_disconnected'
			fprintf('Device disconnected unexpectedly\n');
		case 'MATLAB:error:timeout'
			fprintf('Operation timed out\n');
		otherwise
			fprintf('Error: %s\n', ME.message);
	end
	dev.close();
	rethrow(ME);
end

% Clean up
dev.close();
rmpath(fullfile(pwd, 'release'));